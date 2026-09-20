//! Attach proxy credentials only to the configured proxy's resource URLs.

use std::sync::OnceLock;

use regex::{Captures, Regex};
use url::Url;

pub(super) fn inject_access_token(
    content: &str,
    token: Option<&str>,
    public_base_url: &str,
) -> String {
    let Some(token) = token.map(str::trim).filter(|value| !value.is_empty()) else {
        return content.to_string();
    };
    let Ok(base) = Url::parse(public_base_url) else {
        return content.to_string();
    };
    if !matches!(base.scheme(), "http" | "https")
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return content.to_string();
    }
    content
        .split('\n')
        .map(|line| {
            if !line.starts_with('#') {
                return credentialed_uri(line, token, &base);
            }
            let tag = line.split(':').next().unwrap_or("");
            if !matches!(
                tag,
                "#EXT-X-MEDIA"
                    | "#EXT-X-I-FRAME-STREAM-INF"
                    | "#EXT-X-RENDITION-REPORT"
                    | "#EXT-X-KEY"
                    | "#EXT-X-SESSION-KEY"
                    | "#EXT-X-MAP"
                    | "#EXT-X-PART"
                    | "#EXT-X-PRELOAD-HINT"
            ) {
                return line.to_string();
            }
            // Consume complete attributes, including quoted commas. URI need not be last;
            // text in NAME, GROUP-ID or a comment must never receive a credential.
            attribute_regex()
                .replace_all(line, |captures: &Captures<'_>| {
                    let value = &captures[2];
                    if &captures[1] != "URI" {
                        return captures[0].to_string();
                    }
                    let Some(uri) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"'))
                    else {
                        return captures[0].to_string();
                    };
                    format!("URI=\"{}\"", credentialed_uri(uri, token, &base))
                })
                .into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn credentialed_uri(uri: &str, token: &str, base: &Url) -> String {
    let Ok(mut url) = Url::parse(uri) else {
        return uri.to_string();
    };
    let prefix = format!("{}/media/vod/", base.path().trim_end_matches('/'));
    if url.origin() != base.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || !url
            .path()
            .strip_prefix(&prefix)
            .is_some_and(|asset| matches!(asset, "m3u8" | "segment" | "key"))
    {
        return uri.to_string();
    }
    let pairs = url.query_pairs().into_owned().collect::<Vec<_>>();
    for required in ["source", "url"] {
        let values = pairs
            .iter()
            .filter(|(key, _)| key == required)
            .collect::<Vec<_>>();
        if values.len() != 1 || values[0].1.is_empty() {
            return uri.to_string();
        }
    }
    // Replace only the top-level token. The upstream URL and its own query stay intact.
    url.query_pairs_mut()
        .clear()
        .extend_pairs(pairs.iter().filter(|(key, _)| key != "token"))
        .append_pair("token", token);
    url.to_string()
}

fn attribute_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"([A-Z0-9-]+)=("[^"]*"|[^,]*)"#).expect("valid HLS attribute regex")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://proxy.test/addon";
    const TOKEN: &str = "a+b&c=中文";

    fn resource(asset: &str) -> String {
        crate::build_vod_proxy_url(
            BASE,
            &format!("/media/vod/{asset}"),
            "source1",
            "https://cdn.test/a.ts?token=upstream&signature=abc",
        )
    }

    fn assert_credential(uri: &str) {
        let parsed = Url::parse(uri).unwrap();
        let tokens = parsed
            .query_pairs()
            .filter(|(key, _)| key == "token")
            .map(|(_, value)| value.into_owned())
            .collect::<Vec<_>>();
        assert_eq!(tokens, vec![TOKEN]);
        assert!(parsed.query_pairs().any(|(key, value)| {
            key == "url" && value == "https://cdn.test/a.ts?token=upstream&signature=abc"
        }));
    }

    #[test]
    fn credentials_only_the_uri_attribute_regardless_of_attribute_order() {
        for tag in [
            "MEDIA",
            "I-FRAME-STREAM-INF",
            "RENDITION-REPORT",
            "KEY",
            "SESSION-KEY",
            "MAP",
            "PART",
            "PRELOAD-HINT",
        ] {
            let line = format!(
                "#EXT-X-{tag}:TYPE=AUDIO,URI=\"{}\",NAME=\"English,token=display\",GROUP-ID=\"audio\"",
                resource("m3u8")
            );
            let output = inject_access_token(&line, Some(TOKEN), BASE);
            let attribute = attribute_regex()
                .captures_iter(&output)
                .find(|capture| &capture[1] == "URI")
                .unwrap();
            assert_credential(&attribute[2][1..attribute[2].len() - 1]);
            assert!(output.ends_with("NAME=\"English,token=display\",GROUP-ID=\"audio\""));
        }
    }

    #[test]
    fn credentials_standalone_resources_once_and_replaces_stale_top_level_tokens() {
        for asset in ["m3u8", "segment", "key"] {
            let input = format!("{}&token=old&token=duplicate#fragment", resource(asset));
            let once = inject_access_token(&input, Some(TOKEN), BASE);
            assert_credential(&once);
            assert_eq!(once, inject_access_token(&once, Some(TOKEN), BASE));
            assert_eq!(Url::parse(&once).unwrap().fragment(), Some("fragment"));
        }
    }

    #[test]
    fn never_credentials_foreign_origins_wrong_prefixes_comments_or_other_attributes() {
        let own = resource("key");
        let inputs = [
            own.replace("proxy.test", "evil.test"),
            own.replace("https:", "http:"),
            own.replace("proxy.test", "user:password@proxy.test"),
            own.replace("/addon/", "/other/"),
            own.replace("/key?", "/key/extra?"),
            own.replace("/key?", "/unknown?"),
            own.replace("source=source1", "source="),
            format!("{own}&source=duplicate"),
            "#EXT-X-KEY:URI=\"中".to_string(),
            "#EXT-X-KEY:URI=\"".to_string(),
            "#EXT-X-KEY:URI=".to_string(),
            format!("#EXT-X-KEY:URI={own}"),
            format!("# a comment containing {own}"),
            format!("#EXT-X-SESSION-DATA:URI=\"{own}\""),
            format!("#EXT-X-KEY:METHOD=NONE,NAME=\"{own}\""),
        ];
        for input in inputs {
            assert_eq!(input, inject_access_token(&input, Some(TOKEN), BASE));
        }
    }

    #[test]
    fn disabled_or_invalid_configuration_does_not_inject_credentials() {
        let input = resource("segment");
        for token in [None, Some(""), Some("   ")] {
            assert_eq!(input, inject_access_token(&input, token, BASE));
        }
        for base in [
            "not a URL",
            "ftp://proxy.test/addon",
            "https://user:pass@proxy.test/addon",
            "https://proxy.test/addon?private=true",
            "https://proxy.test/addon#fragment",
        ] {
            assert_eq!(input, inject_access_token(&input, Some(TOKEN), base));
        }
    }
}

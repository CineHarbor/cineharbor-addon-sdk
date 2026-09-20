//! Sign only resource URI attributes, never names, comments, or foreign origins.

use super::access::{MediaAuthorization, now};
use regex::{Captures, Regex};
use std::sync::OnceLock;

pub(super) fn authorize_manifest(
    content: &str,
    auth: &MediaAuthorization,
    public_base: &str,
) -> String {
    let issued = now();
    let sign = |uri: &str| {
        auth.sign_proxy_url_at(uri, public_base, issued)
            .unwrap_or_else(|| uri.to_string())
    };
    content
        .split('\n')
        .map(|line| {
            if !line.starts_with('#') {
                return sign(line);
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
            attribute_regex()
                .replace_all(line, |captures: &Captures<'_>| {
                    if &captures[1] != "URI" {
                        return captures[0].to_string();
                    }
                    let Some(uri) = captures[2]
                        .strip_prefix('"')
                        .and_then(|value| value.strip_suffix('"'))
                    else {
                        return captures[0].to_string();
                    };
                    format!("URI=\"{}\"", sign(uri))
                })
                .into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
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
    use url::Url;

    const BASE: &str = "https://proxy.test/addon";
    const SECRET: &[u8] = b"0123456789abcdef0123456789abcdef";
    fn auth() -> MediaAuthorization {
        MediaAuthorization::new(SECRET).unwrap()
    }

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
        let pairs = parsed
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert!(!pairs.contains_key("token"));
        assert!(auth().verify(
            BASE,
            parsed.path().strip_prefix("/addon/media/").unwrap(),
            &pairs["source"],
            &pairs["url"],
            pairs["expires"].parse().unwrap(),
            &pairs["sig"]
        ));
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
            let output = authorize_manifest(&line, &auth(), BASE);
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
            let once = authorize_manifest(&input, &auth(), BASE);
            assert_credential(&once);
            assert_eq!(once, authorize_manifest(&once, &auth(), BASE));
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
            assert_eq!(input, authorize_manifest(&input, &auth(), BASE));
        }
    }

    #[test]
    fn invalid_configuration_does_not_sign_resources() {
        let input = resource("segment");
        for base in [
            "not a URL",
            "ftp://proxy.test/addon",
            "https://user:pass@proxy.test/addon",
            "https://proxy.test/addon?private=true",
            "https://proxy.test/addon#fragment",
        ] {
            assert_eq!(input, authorize_manifest(&input, &auth(), base));
        }
    }
}

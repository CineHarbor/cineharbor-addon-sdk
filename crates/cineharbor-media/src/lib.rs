//! 媒体代理的纯逻辑共享核心：HLS（m3u8）清单重写 + 代理 URL 构造（无 IO，可离线单测）。
//!
//! P2 的媒体代理 slice：把 local-service 内 `vod_proxy`/`live_proxy` 依赖的「清单重写 + URL
//! 构造」纯逻辑抽为共享 crate，供 standalone vod/live addon（服务自身媒体）与（后续）
//! local-service 共用。当前与 local-service 内置版并行（strangler）；HTTP 抓取/流式/缓存
//! handler 留在本地，后续按 addon 独立服务迁移。
//!
//! 与 local-service `lib.rs` 的 `rewrite_vod_manifest_content` / `rewrite_live_manifest_content`
//! / `build_vod_proxy_*` / `build_live_proxy_*` 逐行对齐。

use std::sync::OnceLock;

use regex::Regex;
use url::Url;

mod serve;
pub use serve::{live_proxy_router, vod_proxy_router, ProxyParts, SourceHeaders, DEFAULT_WEB_UA};

/// HLS 资产类别，决定重写到哪个代理端点。
#[derive(Clone, Copy)]
pub enum VodAssetKind {
    M3u8,
    Segment,
    Key,
}

// —— 代理 URL 构造 ——

pub fn build_vod_proxy_m3u8_url(base_url: &str, source: &str, url: &str) -> String {
    build_vod_proxy_url(base_url, "/media/vod/m3u8", source, url)
}

pub fn build_vod_proxy_segment_url(base_url: &str, source: &str, url: &str) -> String {
    build_vod_proxy_url(base_url, "/media/vod/segment", source, url)
}

pub fn build_vod_proxy_key_url(base_url: &str, source: &str, url: &str) -> String {
    build_vod_proxy_url(base_url, "/media/vod/key", source, url)
}

pub fn build_vod_proxy_url(base_url: &str, path: &str, source: &str, url: &str) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer.append_pair("source", source);
    serializer.append_pair("url", url);
    let query = serializer.finish();

    format!("{}{}?{}", base_url.trim_end_matches('/'), path, query)
}

pub fn build_live_proxy_m3u8_url(
    base_url: &str,
    source_key: &str,
    url: &str,
    allow_cors: bool,
) -> String {
    build_live_proxy_url(base_url, "/media/live/m3u8", source_key, url, allow_cors)
}

pub fn build_live_proxy_segment_url(base_url: &str, source_key: &str, url: &str) -> String {
    build_live_proxy_url(base_url, "/media/live/segment", source_key, url, false)
}

pub fn build_live_proxy_key_url(base_url: &str, source_key: &str, url: &str) -> String {
    build_live_proxy_url(base_url, "/media/live/key", source_key, url, false)
}

pub fn build_live_proxy_url(
    base_url: &str,
    path: &str,
    source_key: &str,
    url: &str,
    allow_cors: bool,
) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer.append_pair("cineharbor-source", source_key);
    serializer.append_pair("url", url);
    if allow_cors {
        serializer.append_pair("allowCORS", "true");
    }
    let query = serializer.finish();

    format!("{}{}?{}", base_url.trim_end_matches('/'), path, query)
}

// —— VOD 清单重写 ——

pub fn rewrite_vod_manifest_content(
    content: &str,
    final_url: &str,
    source: &str,
    public_base_url: &str,
) -> String {
    let base_url = get_base_url(final_url);
    let lines = sanitize_vod_manifest_lines(
        content
            .split('\n')
            .map(|line| line.to_string())
            .collect::<Vec<_>>(),
    );
    let mut rewritten_lines = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let trimmed_line = lines[index].trim();

        if trimmed_line.is_empty() {
            rewritten_lines.push(String::new());
            index += 1;
            continue;
        }

        if trimmed_line.starts_with("#EXT-X-STREAM-INF:") {
            rewritten_lines.push(trimmed_line.to_string());
            let next_line = lines
                .get(index + 1)
                .map(|line| line.trim().to_string())
                .unwrap_or_default();

            if !next_line.is_empty() && !next_line.starts_with('#') {
                let resolved_url = resolve_url(&base_url, &next_line);
                rewritten_lines.push(build_vod_proxy_m3u8_url(
                    public_base_url,
                    source,
                    &resolved_url,
                ));
                index += 2;
                continue;
            }

            index += 1;
            continue;
        }

        if trimmed_line.starts_with("#EXT-X-MEDIA:")
            || trimmed_line.starts_with("#EXT-X-I-FRAME-STREAM-INF:")
            || trimmed_line.starts_with("#EXT-X-RENDITION-REPORT:")
        {
            rewritten_lines.push(rewrite_attribute_uri(
                trimmed_line,
                &base_url,
                source,
                public_base_url,
                VodAssetKind::M3u8,
            ));
            index += 1;
            continue;
        }

        if trimmed_line.starts_with("#EXT-X-KEY:")
            || trimmed_line.starts_with("#EXT-X-SESSION-KEY:")
        {
            rewritten_lines.push(rewrite_attribute_uri(
                trimmed_line,
                &base_url,
                source,
                public_base_url,
                VodAssetKind::Key,
            ));
            index += 1;
            continue;
        }

        if trimmed_line.starts_with("#EXT-X-MAP:")
            || trimmed_line.starts_with("#EXT-X-PART:")
            || trimmed_line.starts_with("#EXT-X-PRELOAD-HINT:")
        {
            rewritten_lines.push(rewrite_attribute_uri(
                trimmed_line,
                &base_url,
                source,
                public_base_url,
                VodAssetKind::Segment,
            ));
            index += 1;
            continue;
        }

        if !trimmed_line.starts_with('#') {
            let resolved_url = resolve_url(&base_url, trimmed_line);
            if looks_like_manifest_url(&resolved_url) {
                rewritten_lines.push(build_vod_proxy_m3u8_url(
                    public_base_url,
                    source,
                    &resolved_url,
                ));
            } else {
                rewritten_lines.push(build_vod_proxy_segment_url(
                    public_base_url,
                    source,
                    &resolved_url,
                ));
            }

            index += 1;
            continue;
        }

        rewritten_lines.push(trimmed_line.to_string());
        index += 1;
    }

    rewritten_lines.join("\n")
}

pub fn rewrite_attribute_uri(
    line: &str,
    base_url: &str,
    source: &str,
    public_base_url: &str,
    asset_kind: VodAssetKind,
) -> String {
    let Some(capture) = uri_attribute_regex().captures(line) else {
        return line.to_string();
    };
    let Some(uri_match) = capture.get(1) else {
        return line.to_string();
    };

    let resolved_url = resolve_url(base_url, uri_match.as_str());
    let proxied_url = match asset_kind {
        VodAssetKind::M3u8 => build_vod_proxy_m3u8_url(public_base_url, source, &resolved_url),
        VodAssetKind::Segment => build_vod_proxy_segment_url(public_base_url, source, &resolved_url),
        VodAssetKind::Key => build_vod_proxy_key_url(public_base_url, source, &resolved_url),
    };

    line.replace(
        &format!("URI=\"{}\"", uri_match.as_str()),
        &format!("URI=\"{proxied_url}\""),
    )
}

// —— Live 清单重写 ——

pub fn rewrite_live_manifest_content(
    content: &str,
    final_url: &str,
    source_key: &str,
    public_base_url: &str,
    allow_cors: bool,
) -> String {
    let base_url = get_base_url(final_url);
    let lines = content.split('\n').collect::<Vec<_>>();
    let mut rewritten_lines = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let trimmed_line = lines[index].trim();
        if trimmed_line.is_empty() {
            rewritten_lines.push(String::new());
            index += 1;
            continue;
        }

        if trimmed_line.starts_with("#EXT-X-STREAM-INF:") {
            rewritten_lines.push(trimmed_line.to_string());
            let next_line = lines
                .get(index + 1)
                .map(|line| line.trim())
                .unwrap_or_default();
            if !next_line.is_empty() && !next_line.starts_with('#') {
                let resolved_url = resolve_url(&base_url, next_line);
                rewritten_lines.push(build_live_proxy_m3u8_url(
                    public_base_url,
                    source_key,
                    &resolved_url,
                    false,
                ));
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if trimmed_line.starts_with("#EXT-X-MEDIA:")
            || trimmed_line.starts_with("#EXT-X-I-FRAME-STREAM-INF:")
            || trimmed_line.starts_with("#EXT-X-RENDITION-REPORT:")
        {
            rewritten_lines.push(rewrite_manifest_uri_attribute(
                trimmed_line,
                &base_url,
                |resolved_url| {
                    build_live_proxy_m3u8_url(public_base_url, source_key, resolved_url, false)
                },
            ));
            index += 1;
            continue;
        }

        if trimmed_line.starts_with("#EXT-X-KEY:")
            || trimmed_line.starts_with("#EXT-X-SESSION-KEY:")
        {
            rewritten_lines.push(rewrite_manifest_uri_attribute(
                trimmed_line,
                &base_url,
                |resolved_url| build_live_proxy_key_url(public_base_url, source_key, resolved_url),
            ));
            index += 1;
            continue;
        }

        if trimmed_line.starts_with("#EXT-X-MAP:")
            || trimmed_line.starts_with("#EXT-X-PART:")
            || trimmed_line.starts_with("#EXT-X-PRELOAD-HINT:")
        {
            rewritten_lines.push(rewrite_manifest_uri_attribute(
                trimmed_line,
                &base_url,
                |resolved_url| {
                    build_live_proxy_segment_url(public_base_url, source_key, resolved_url)
                },
            ));
            index += 1;
            continue;
        }

        if !trimmed_line.starts_with('#') {
            let resolved_url = resolve_url(&base_url, trimmed_line);
            rewritten_lines.push(if allow_cors {
                resolved_url
            } else {
                build_live_proxy_segment_url(public_base_url, source_key, &resolved_url)
            });
            index += 1;
            continue;
        }

        rewritten_lines.push(trimmed_line.to_string());
        index += 1;
    }

    rewritten_lines.join("\n")
}

fn rewrite_manifest_uri_attribute<F>(line: &str, base_url: &str, builder: F) -> String
where
    F: Fn(&str) -> String,
{
    let Some(uri) = extract_quoted_attribute(line, "URI") else {
        return line.to_string();
    };
    let resolved_url = resolve_url(base_url, &uri);
    let target = format!(r#"URI="{uri}""#);
    let replacement = format!(r#"URI="{}""#, builder(&resolved_url));
    line.replacen(&target, &replacement, 1)
}

// —— 清理（广告跳段）——

fn sanitize_vod_manifest_lines(lines: Vec<String>) -> Vec<String> {
    let mut sanitized = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let trimmed_line = lines[index].trim().to_string();

        if trimmed_line == "#EXT-X-DISCONTINUITY" {
            let mut cursor = index + 1;
            let mut found_unsupported_segment = false;

            while cursor + 1 < lines.len() {
                let duration_line = lines[cursor].trim().to_string();
                let resource_line = lines[cursor + 1].trim().to_string();

                if duration_line.starts_with("#EXTINF:")
                    && is_unsupported_vod_segment_uri(&resource_line)
                {
                    found_unsupported_segment = true;
                    cursor += 2;
                    continue;
                }

                break;
            }

            if found_unsupported_segment {
                if lines
                    .get(cursor)
                    .map(|line| line.trim() == "#EXT-X-DISCONTINUITY")
                    .unwrap_or(false)
                {
                    cursor += 1;
                }

                index = cursor;
                continue;
            }
        }

        if trimmed_line.starts_with("#EXTINF:")
            && lines
                .get(index + 1)
                .map(|line| is_unsupported_vod_segment_uri(line.trim()))
                .unwrap_or(false)
        {
            index += 2;
            continue;
        }

        if (trimmed_line.starts_with("#EXT-X-PART:")
            || trimmed_line.starts_with("#EXT-X-PRELOAD-HINT:")
            || trimmed_line.starts_with("#EXT-X-MAP:"))
            && is_unsupported_vod_segment_uri(&trimmed_line)
        {
            index += 1;
            continue;
        }

        if is_unsupported_vod_segment_uri(&trimmed_line) {
            index += 1;
            continue;
        }

        sanitized.push(trimmed_line);
        index += 1;
    }

    sanitized
}

fn is_unsupported_vod_segment_uri(line: &str) -> bool {
    let target = uri_attribute_regex()
        .captures(line)
        .and_then(|capture| capture.get(1))
        .map(|item| item.as_str())
        .unwrap_or(line.trim());

    unsupported_segment_regex().is_match(target)
}

// —— 属性抽取 ——

fn extract_quoted_attribute(line: &str, attribute: &str) -> Option<String> {
    let needle = format!(r#"{attribute}=""#);
    let start = line.find(&needle)? + needle.len();
    let tail = &line[start..];
    let end = tail.find('"')?;
    Some(tail[..end].to_string())
}

// —— URL 解析 ——

pub fn get_base_url(manifest_url: &str) -> String {
    if let Ok(mut url) = Url::parse(manifest_url) {
        if url.path().ends_with(".m3u8") {
            let mut path = url.path().to_string();
            if let Some(last_slash_index) = path.rfind('/') {
                path.truncate(last_slash_index + 1);
            }
            url.set_path(&path);
        } else if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        url.set_query(None);
        url.to_string()
    } else if manifest_url.ends_with('/') {
        manifest_url.to_string()
    } else {
        format!("{manifest_url}/")
    }
}

pub fn resolve_url(base_url: &str, relative_path: &str) -> String {
    if relative_path.starts_with("http://") || relative_path.starts_with("https://") {
        return relative_path.to_string();
    }

    if relative_path.starts_with("//") {
        if let Ok(base_url) = Url::parse(base_url) {
            return format!("{}{}", base_url.scheme(), relative_path);
        }
    }

    match Url::parse(base_url)
        .and_then(|base| base.join(relative_path))
        .map(|url| url.to_string())
    {
        Ok(url) => url,
        Err(_) => fallback_resolve_url(base_url, relative_path),
    }
}

fn fallback_resolve_url(base_url: &str, relative_path: &str) -> String {
    let mut base = base_url.to_string();
    if !base.ends_with('/') {
        if let Some(last_slash_index) = base.rfind('/') {
            base.truncate(last_slash_index + 1);
        }
    }

    if relative_path.starts_with('/') {
        if let Ok(url) = Url::parse(&base) {
            return format!(
                "{}://{}{}",
                url.scheme(),
                url.host_str().unwrap_or(""),
                relative_path
            );
        }
    }

    if relative_path.starts_with("../") {
        let mut segments = base
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
            .collect::<Vec<_>>();
        let relative_segments = relative_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();

        for segment in relative_segments {
            if segment == ".." {
                let _ = segments.pop();
            } else if segment != "." {
                segments.push(segment.to_string());
            }
        }

        if let Ok(url) = Url::parse(&base) {
            return format!(
                "{}://{}/{}",
                url.scheme(),
                url.host_str().unwrap_or(""),
                segments.join("/")
            );
        }
    }

    if let Some(cleaned_relative) = relative_path.strip_prefix("./") {
        return format!("{base}{cleaned_relative}");
    }

    format!("{base}{relative_path}")
}

pub fn looks_like_manifest_url(url: &str) -> bool {
    manifest_url_regex().is_match(url)
}

// —— regex ——

fn uri_attribute_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"URI="([^"]+)""#).expect("valid uri regex"))
}

fn unsupported_segment_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"(^|/)video/adjump/").expect("valid unsupported segment regex"))
}

fn manifest_url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)\.m3u8($|[?#])").expect("valid manifest url regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_vod_proxy_url_with_encoding() {
        assert_eq!(
            build_vod_proxy_url("http://pub", "/media/vod/segment", "src", "https://cdn.test/x.ts"),
            "http://pub/media/vod/segment?source=src&url=https%3A%2F%2Fcdn.test%2Fx.ts"
        );
    }

    #[test]
    fn builds_live_proxy_url_with_source_key() {
        let url = build_live_proxy_m3u8_url("http://pub", "live1", "https://cdn.test/live.m3u8", true);
        assert!(url.starts_with(
            "http://pub/media/live/m3u8?cineharbor-source=live1&url="
        ));
        assert!(url.ends_with("&allowCORS=true") || url.contains("allowCORS=true"));
    }

    #[test]
    fn rewrites_vod_master_manifest() {
        let master = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nlow.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2000000\nhttp://other.test/high.m3u8\n";
        let out = rewrite_vod_manifest_content(
            master,
            "https://cdn.test/path/index.m3u8",
            "src",
            "http://pub",
        );
        assert!(out.contains("http://pub/media/vod/m3u8?source=src&url="));
        assert!(out.contains("https%3A%2F%2Fcdn.test%2Fpath%2Flow.m3u8"));
        assert!(out.contains("http%3A%2F%2Fother.test%2Fhigh.m3u8"));
    }

    #[test]
    fn rewrites_vod_media_manifest_key_and_segments() {
        let media = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXTINF:4,\nseg1.ts\n#EXT-X-MAP:URI=\"init.mp4\"\n";
        let out = rewrite_vod_manifest_content(
            media,
            "https://cdn.test/path/index.m3u8",
            "src",
            "http://pub",
        );
        assert!(out.contains("URI=\"http://pub/media/vod/key?source=src&url="));
        assert!(out.contains("http://pub/media/vod/segment?source=src&url="));
        assert!(out.contains("https%3A%2F%2Fcdn.test%2Fpath%2Fseg1.ts"));
        assert!(out.contains("https%3A%2F%2Fcdn.test%2Fpath%2Finit.mp4"));
    }

    #[test]
    fn rewrites_live_master_manifest() {
        let master = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nch.m3u8\n";
        let out = rewrite_live_manifest_content(
            master,
            "https://cdn.test/live/index.m3u8",
            "live1",
            "http://pub",
            false,
        );
        assert!(out.contains("http://pub/media/live/m3u8?cineharbor-source=live1&url="));
        assert!(out.contains("https%3A%2F%2Fcdn.test%2Flive%2Fch.m3u8"));
    }

    #[test]
    fn sanitizes_adjump_segments() {
        let media = "#EXTM3U\n#EXT-X-DISCONTINUITY\n#EXTINF:4,\nvideo/adjump/ad1.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:4,\nreal.ts\n";
        let out = rewrite_vod_manifest_content(
            media,
            "https://cdn.test/path/index.m3u8",
            "src",
            "http://pub",
        );
        assert!(!out.contains("adjump"));
        assert!(out.contains("real.ts"));
    }

    #[test]
    fn resolves_urls() {
        assert_eq!(get_base_url("https://cdn.test/a/b/index.m3u8"), "https://cdn.test/a/b/");
        assert_eq!(
            resolve_url("https://cdn.test/a/b/", "segment.ts"),
            "https://cdn.test/a/b/segment.ts"
        );
        assert_eq!(resolve_url("https://cdn.test/a/b/", "/root.ts"), "https://cdn.test/root.ts");
        assert_eq!(
            resolve_url("https://cdn.test/a/b/", "https://other.test/x.ts"),
            "https://other.test/x.ts"
        );
        assert_eq!(
            resolve_url("https://cdn.test/a/b/", "../up.ts"),
            "https://cdn.test/a/up.ts"
        );
    }
}
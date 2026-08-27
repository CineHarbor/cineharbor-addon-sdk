//! 直播参考 addon：解析 M3U8 播放列表，暴露为 `type=tv` 的 catalog/meta/stream。

use async_trait::async_trait;
use cineharbor_addon_protocol::{
    Catalog, CatalogResponse, ContentType, Manifest, MetaDetail, MetaPreview, MetaResponse,
    Resource, Stream, StreamsResponse,
};
use cineharbor_addon_sdk::addon::{Addon, CatalogRequest};

/// 解析出的单个直播频道。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub name: String,
    pub group: Option<String>,
    pub logo: Option<String>,
    pub url: String,
}

/// 解析 M3U8（`#EXTINF` 属性 + 紧随的 URL 行）。
pub fn parse_m3u8(text: &str) -> Vec<Channel> {
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix("#EXTINF") {
            let name = rest.rsplit(',').next().unwrap_or("").trim().to_string();
            let group = attr(rest, "group-title");
            let logo = attr(rest, "tvg-logo");
            let url = loop {
                match lines.next() {
                    Some(l) if l.starts_with('#') || l.trim().is_empty() => continue,
                    Some(l) => break l.trim().to_string(),
                    None => break String::new(),
                }
            };
            if !name.is_empty() && !url.is_empty() {
                out.push(Channel {
                    name,
                    group,
                    logo,
                    url,
                });
            }
        }
    }
    out
}

fn attr(line: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let start = line.find(&pat)? + pat.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

pub struct LiveAddon {
    name: String,
    channels: Vec<Channel>,
}

#[derive(Debug, thiserror::Error)]
pub enum LiveAddonError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
}

impl LiveAddon {
    pub fn from_playlist(name: impl Into<String>, m3u8: &str) -> Self {
        Self {
            name: name.into(),
            channels: parse_m3u8(m3u8),
        }
    }

    pub async fn load(name: impl Into<String>, url: &str) -> Result<Self, LiveAddonError> {
        let text = reqwest::get(url).await?.error_for_status()?.text().await?;
        Ok(Self::from_playlist(name, &text))
    }

    fn channel_of(&self, id: &str) -> Option<&Channel> {
        let idx = id.strip_prefix("live:")?.parse::<usize>().ok()?;
        self.channels.get(idx)
    }
}

#[async_trait]
impl Addon for LiveAddon {
    async fn manifest(&self) -> Manifest {
        Manifest {
            id: "community.live".into(),
            version: "0.1.0".into(),
            name: self.name.clone(),
            description: Some("直播频道（M3U8）参考 addon".into()),
            resources: vec![Resource::Catalog, Resource::Meta, Resource::Stream],
            types: vec![ContentType::Tv],
            catalogs: vec![Catalog {
                r#type: ContentType::Tv,
                id: "channels".into(),
                name: "频道".into(),
                extra: vec![],
                extra_supported: vec![],
            }],
            id_prefixes: Some(vec!["live".into()]),
            icon: None,
            logo: None,
            background: None,
            behavior_hints: None,
        }
    }

    async fn catalog(&self, _req: CatalogRequest) -> CatalogResponse {
        let metas = self
            .channels
            .iter()
            .enumerate()
            .map(|(i, c)| MetaPreview {
                poster: c.logo.clone(),
                description: c.group.clone(),
                ..MetaPreview::new(format!("live:{i}"), ContentType::Tv, &c.name)
            })
            .collect();
        CatalogResponse { metas }
    }

    async fn meta(&self, _ty: ContentType, id: &str) -> Option<MetaResponse> {
        let c = self.channel_of(id)?;
        Some(MetaResponse {
            meta: MetaDetail {
                poster: c.logo.clone(),
                description: c.group.clone(),
                ..MetaDetail::new(id, ContentType::Tv, &c.name)
            },
        })
    }

    async fn streams(&self, _ty: ContentType, id: &str) -> StreamsResponse {
        let streams = match self.channel_of(id) {
            Some(c) => vec![Stream {
                name: Some(c.name.clone()),
                url: Some(c.url.clone()),
                ..Stream::default()
            }],
            None => vec![],
        };
        StreamsResponse { streams }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYLIST: &str = "\
#EXTM3U
#EXTINF:-1 tvg-id=\"c1\" tvg-name=\"CCTV1\" group-title=\"央视\" tvg-logo=\"http://x/1.png\",CCTV-1 综合
http://example.test/1.m3u8
#EXTINF:-1,ChannelTwo
http://example.test/2.m3u8
";

    #[test]
    fn parses_m3u8() {
        let chans = parse_m3u8(PLAYLIST);
        assert_eq!(chans.len(), 2);
        assert_eq!(chans[0].name, "CCTV-1 综合");
        assert_eq!(chans[0].group.as_deref(), Some("央视"));
        assert_eq!(chans[0].logo.as_deref(), Some("http://x/1.png"));
        assert_eq!(chans[0].url, "http://example.test/1.m3u8");
        assert_eq!(chans[1].name, "ChannelTwo");
        assert_eq!(chans[1].url, "http://example.test/2.m3u8");
    }

    #[tokio::test]
    async fn streams_serve_channel_url() {
        let addon = LiveAddon::from_playlist("Live", PLAYLIST);
        let s = addon.streams(ContentType::Tv, "live:0").await;
        assert_eq!(s.streams.len(), 1);
        assert_eq!(s.streams[0].url.as_deref(), Some("http://example.test/1.m3u8"));
        assert!(addon.streams(ContentType::Tv, "live:99").await.streams.is_empty());
    }
}
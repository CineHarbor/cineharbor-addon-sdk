//! 直播 addon：解析**多源** M3U8 播放列表，暴露为 `type=tv` 的 catalog/meta/stream。
//!
//! 对齐 native live 的「多直播源」语义：每个直播源 = 一个 catalog（`catalog/tv/{key}`）；
//! 频道 id 形如 `live:{key}:{idx}`；stream url 经媒体代理按 source key 转链（per-source
//! UA/Referer 由 main.rs 灌入 `ProxyParts.sources`）。

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

/// 一个直播源（一个 M3U8 播放列表 + 播放所需 UA/Referer）。
#[derive(Debug, Clone)]
pub struct LiveSource {
    pub key: String,
    pub name: String,
    pub ua: Option<String>,
    pub referer: Option<String>,
    pub channels: Vec<Channel>,
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
    addon_name: String,
    sources: Vec<LiveSource>,
    public_base_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LiveAddonError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
}

impl LiveAddon {
    /// 单源便捷构造（key=`default`，频道 id `live:default:{idx}`）。
    pub fn from_playlist(name: impl Into<String>, m3u8: &str) -> Self {
        Self::from_sources(
            name,
            vec![LiveSource {
                key: "default".into(),
                name: "default".into(),
                ua: None,
                referer: None,
                channels: parse_m3u8(m3u8),
            }],
        )
    }

    pub fn from_sources(name: impl Into<String>, sources: Vec<LiveSource>) -> Self {
        Self {
            addon_name: name.into(),
            sources,
            public_base_url: None,
        }
    }

    pub fn set_public_base_url(&mut self, base: Option<String>) {
        self.public_base_url = base;
    }

    /// 供 main.rs 把 per-source UA/Referer 灌入媒体代理 `ProxyParts.sources`。
    pub fn sources(&self) -> &[LiveSource] {
        &self.sources
    }

    fn channel_of(&self, id: &str) -> Option<(&LiveSource, &Channel)> {
        let rest = id.strip_prefix("live:")?;
        let (key, idx) = rest.split_once(':')?;
        let idx: usize = idx.parse().ok()?;
        let source = self.sources.iter().find(|s| s.key == key)?;
        Some((source, source.channels.get(idx)?))
    }

    fn stream_url(&self, source: &LiveSource, channel: &Channel) -> String {
        match &self.public_base_url {
            Some(base) => {
                cineharbor_media::build_live_proxy_m3u8_url(base, &source.key, &channel.url, false)
            }
            None => channel.url.clone(),
        }
    }
}

#[async_trait]
impl Addon for LiveAddon {
    async fn manifest(&self) -> Manifest {
        Manifest {
            id: "community.live".into(),
            version: "0.2.0".into(),
            name: self.addon_name.clone(),
            description: Some("多源直播频道（M3U8）addon".into()),
            resources: vec![Resource::Catalog, Resource::Meta, Resource::Stream],
            types: vec![ContentType::Tv],
            catalogs: self
                .sources
                .iter()
                .map(|source| Catalog {
                    r#type: ContentType::Tv,
                    id: source.key.clone(),
                    name: source.name.clone(),
                    extra: vec![],
                    extra_supported: vec![],
                })
                .collect(),
            id_prefixes: Some(vec!["live".into()]),
            icon: None,
            logo: None,
            background: None,
            behavior_hints: None,
        }
    }

    async fn catalog(&self, req: CatalogRequest) -> CatalogResponse {
        let Some(source) = self.sources.iter().find(|s| s.key == req.id) else {
            return CatalogResponse { metas: vec![] };
        };
        let metas = source
            .channels
            .iter()
            .enumerate()
            .map(|(i, c)| MetaPreview {
                poster: c.logo.clone(),
                description: c.group.clone(),
                ..MetaPreview::new(format!("live:{}:{i}", source.key), ContentType::Tv, &c.name)
            })
            .collect();
        CatalogResponse { metas }
    }

    async fn meta(&self, _ty: ContentType, id: &str) -> Option<MetaResponse> {
        let (_source, c) = self.channel_of(id)?;
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
            Some((source, c)) => vec![Stream {
                name: Some(c.name.clone()),
                url: Some(self.stream_url(source, c)),
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
    async fn manifest_exposes_one_catalog_per_source() {
        let sources = vec![
            LiveSource {
                key: "cctv".into(),
                name: "央视".into(),
                ua: None,
                referer: None,
                channels: parse_m3u8(PLAYLIST),
            },
            LiveSource {
                key: "hunan".into(),
                name: "卫视".into(),
                ua: None,
                referer: None,
                channels: parse_m3u8(PLAYLIST),
            },
        ];
        let addon = LiveAddon::from_sources("live", sources);
        let manifest = addon.manifest().await;
        assert_eq!(manifest.catalogs.len(), 2);
        assert_eq!(manifest.catalogs[0].id, "cctv");
        assert_eq!(manifest.catalogs[0].name, "央视");
        assert_eq!(manifest.catalogs[1].id, "hunan");
    }

    #[tokio::test]
    async fn catalog_filters_by_source_key() {
        let sources = vec![
            LiveSource {
                key: "cctv".into(),
                name: "央视".into(),
                ua: None,
                referer: None,
                channels: parse_m3u8(PLAYLIST),
            },
            LiveSource {
                key: "hunan".into(),
                name: "卫视".into(),
                ua: None,
                referer: None,
                channels: parse_m3u8("#EXTM3U\n#EXTINF:-1,Hunan\nhttp://h.test/1.m3u8\n"),
            },
        ];
        let addon = LiveAddon::from_sources("live", sources);
        let req = |id: &str| CatalogRequest {
            ty: ContentType::Tv,
            id: id.into(),
            extra: None,
            skip: None,
        };

        let cctv = addon.catalog(req("cctv")).await;
        assert_eq!(cctv.metas.len(), 2);
        assert_eq!(cctv.metas[0].id, "live:cctv:0");
        assert_eq!(cctv.metas[0].name, "CCTV-1 综合");

        let hunan = addon.catalog(req("hunan")).await;
        assert_eq!(hunan.metas.len(), 1);
        assert_eq!(hunan.metas[0].id, "live:hunan:0");

        assert!(addon.catalog(req("missing")).await.metas.is_empty());
    }

    #[tokio::test]
    async fn streams_serve_channel_url() {
        let addon = LiveAddon::from_playlist("Live", PLAYLIST);
        let s = addon.streams(ContentType::Tv, "live:default:0").await;
        assert_eq!(s.streams.len(), 1);
        assert_eq!(
            s.streams[0].url.as_deref(),
            Some("http://example.test/1.m3u8")
        );
        assert!(
            addon
                .streams(ContentType::Tv, "live:default:99")
                .await
                .streams
                .is_empty()
        );
    }

    #[tokio::test]
    async fn proxies_stream_url_when_configured() {
        let mut addon = LiveAddon::from_playlist("Live", PLAYLIST);
        addon.set_public_base_url(Some("http://addon".into()));
        let s = addon.streams(ContentType::Tv, "live:default:0").await;
        let url = s.streams[0].url.as_deref().unwrap();
        assert!(
            url.starts_with("http://addon/media/live/m3u8?cineharbor-source=default&url=")
        );
        assert!(url.contains("http%3A%2F%2Fexample.test%2F1.m3u8"));
    }
}
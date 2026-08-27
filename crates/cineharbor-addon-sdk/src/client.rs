//! 消费侧 addon HTTP 客户端：以 base URL 拉取任意 Stremio 兼容 addon。

use cineharbor_addon_protocol::{
    CatalogResponse, ContentType, Manifest, MetaResponse, StreamsResponse, SubtitlesResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum AddonClientError {
    #[error("base url 非法: {0}")]
    InvalidBaseUrl(String),
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("响应解析失败: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct AddonClient {
    base_url: String,
    http: reqwest::Client,
}

impl AddonClient {
    /// 规范化 base URL（去尾部斜杠、校验为 http/https）。
    pub fn new(base_url: impl Into<String>) -> Result<Self, AddonClientError> {
        let trimmed = base_url.into().trim_end_matches('/').to_string();
        let parsed = url::Url::parse(&trimmed)
            .map_err(|e| AddonClientError::InvalidBaseUrl(e.to_string()))?;
        match parsed.scheme() {
            "http" | "https" => {}
            s => {
                return Err(AddonClientError::InvalidBaseUrl(format!(
                    "不支持的 scheme: {s}"
                )))
            }
        }
        Ok(Self {
            base_url: trimmed,
            http: reqwest::Client::new(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// 目录浏览；`extra` 为 (扩展名, 值)，`skip` 为分页偏移。
    pub fn catalog_url(
        &self,
        ty: ContentType,
        id: &str,
        extra: Option<(&str, &str)>,
        skip: Option<u32>,
    ) -> String {
        match (extra, skip) {
            (Some((name, value)), _) => {
                self.endpoint(&format!("/catalog/{ty}/{id}/{name}={value}.json"))
            }
            (None, Some(n)) => self.endpoint(&format!("/catalog/{ty}/{id}/skip={n}.json")),
            (None, None) => self.endpoint(&format!("/catalog/{ty}/{id}.json")),
        }
    }

    pub async fn manifest(&self) -> Result<Manifest, AddonClientError> {
        let body = self
            .http
            .get(self.endpoint("/manifest.json"))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn catalog(
        &self,
        ty: ContentType,
        id: &str,
        extra: Option<(&str, &str)>,
        skip: Option<u32>,
    ) -> Result<CatalogResponse, AddonClientError> {
        let url = self.catalog_url(ty, id, extra, skip);
        let body = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn meta(&self, ty: ContentType, id: &str) -> Result<MetaResponse, AddonClientError> {
        let body = self
            .http
            .get(self.endpoint(&format!("/meta/{ty}/{id}.json")))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn streams(
        &self,
        ty: ContentType,
        id: &str,
    ) -> Result<StreamsResponse, AddonClientError> {
        let body = self
            .http
            .get(self.endpoint(&format!("/stream/{ty}/{id}.json")))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn subtitles(
        &self,
        ty: ContentType,
        id: &str,
        extra: &str,
    ) -> Result<SubtitlesResponse, AddonClientError> {
        let body = self
            .http
            .get(self.endpoint(&format!("/subtitles/{ty}/{id}/{extra}.json")))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(serde_json::from_str(&body)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_catalog_urls() {
        let c = AddonClient::new("https://addon.example.test/").expect("valid base");
        assert_eq!(
            c.catalog_url(ContentType::Movie, "top", None, None),
            "https://addon.example.test/catalog/movie/top.json"
        );
        assert_eq!(
            c.catalog_url(ContentType::Movie, "top", Some(("search", "matrix")), None),
            "https://addon.example.test/catalog/movie/top/search=matrix.json"
        );
        assert_eq!(
            c.catalog_url(ContentType::Series, "top", None, Some(30)),
            "https://addon.example.test/catalog/series/top/skip=30.json"
        );
    }

    #[test]
    fn rejects_bad_base_url() {
        assert!(AddonClient::new("ftp://x.test").is_err());
        assert!(AddonClient::new("not a url").is_err());
    }
}

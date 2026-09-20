//! Resource-bound capabilities. The shared signing key never appears in playback URLs.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{
    hmac,
    rand::{SecureRandom, SystemRandom},
};
use url::Url;

pub const MEDIA_URL_TTL_SECONDS: u64 = 6 * 60 * 60;

pub struct MediaAuthorization {
    key: hmac::Key,
}

impl MediaAuthorization {
    pub fn new(secret: &[u8]) -> Result<Self, &'static str> {
        if secret.len() < 32 {
            return Err("media signing secret must contain at least 32 bytes");
        }
        Ok(Self {
            key: hmac::Key::new(hmac::HMAC_SHA256, secret),
        })
    }

    /// A stable operator-provided secret supports restarts and multiple replicas. Without
    /// one, use a process-local CSPRNG key, never an unauthenticated/open-proxy mode.
    pub fn from_environment() -> Result<Self, &'static str> {
        match std::env::var("CINEHARBOR_MEDIA_PROXY_TOKEN") {
            Ok(secret) => Self::new(secret.trim().as_bytes()),
            Err(std::env::VarError::NotPresent) => {
                let mut secret = [0u8; 32];
                SystemRandom::new()
                    .fill(&mut secret)
                    .map_err(|_| "media entropy unavailable")?;
                Self::new(&secret)
            }
            Err(_) => Err("media signing secret must be UTF-8"),
        }
    }

    pub fn sign_proxy_url(&self, uri: &str, public_base: &str) -> Option<String> {
        self.sign_proxy_url_at(uri, public_base, now())
    }

    pub(crate) fn sign_proxy_url_at(
        &self,
        uri: &str,
        public_base: &str,
        now: u64,
    ) -> Option<String> {
        let base = valid_base(public_base)?;
        let mut url = Url::parse(uri).ok()?;
        if url.origin() != base.origin() || !url.username().is_empty() || url.password().is_some() {
            return None;
        }
        let prefix = format!("{}/media/", base.path().trim_end_matches('/'));
        let scope = url.path().strip_prefix(&prefix)?.to_string();
        let source_key = source_key(&scope)?;
        let pairs = url.query_pairs().into_owned().collect::<Vec<_>>();
        let unique = |key: &str| {
            let mut values = pairs.iter().filter(|(name, _)| name == key);
            let value = &values.next()?.1;
            (values.next().is_none() && !value.is_empty()).then_some(value.as_str())
        };
        let source = unique(source_key)?;
        let target = unique("url")?;
        // Only already-constructed proxy resources can be signed, not arbitrary URL shapes.
        if pairs.iter().any(|(key, _)| {
            ![source_key, "url", "sig", "expires", "token", "allowCORS"].contains(&key.as_str())
        }) {
            return None;
        }
        let expires = now.checked_add(MEDIA_URL_TTL_SECONDS)?;
        let tag = hmac::sign(
            &self.key,
            &message(public_base, &scope, source, target, expires),
        );
        url.query_pairs_mut()
            .clear()
            .extend_pairs(
                pairs
                    .iter()
                    .filter(|(key, _)| !["sig", "expires", "token"].contains(&key.as_str())),
            )
            .append_pair("expires", &expires.to_string())
            .append_pair("sig", &URL_SAFE_NO_PAD.encode(tag.as_ref()));
        Some(url.to_string())
    }

    pub(crate) fn verify(
        &self,
        public_base: &str,
        scope: &str,
        source: &str,
        target: &str,
        expires: u64,
        signature: &str,
    ) -> bool {
        self.verify_at(
            public_base,
            scope,
            source,
            target,
            signature,
            now()..expires,
        )
    }

    fn verify_at(
        &self,
        base: &str,
        scope: &str,
        source: &str,
        target: &str,
        signature: &str,
        validity: std::ops::Range<u64>,
    ) -> bool {
        let now = validity.start;
        let expires = validity.end;
        if source_key(scope).is_none()
            || expires <= now
            || expires.saturating_sub(now) > MEDIA_URL_TTL_SECONDS
            || signature.len() != 43
        {
            return false;
        }
        let Ok(tag) = URL_SAFE_NO_PAD.decode(signature) else {
            return false;
        };
        hmac::verify(
            &self.key,
            &message(base, scope, source, target, expires),
            &tag,
        )
        .is_ok()
    }
}

pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn source_key(scope: &str) -> Option<&'static str> {
    match scope {
        "vod/m3u8" | "vod/segment" | "vod/key" => Some("source"),
        "live/m3u8" | "live/segment" | "live/key" => Some("cineharbor-source"),
        _ => None,
    }
}

fn valid_base(input: &str) -> Option<Url> {
    let base = Url::parse(input).ok()?;
    (matches!(base.scheme(), "http" | "https")
        && base.host_str().is_some()
        && base.username().is_empty()
        && base.password().is_none()
        && base.query().is_none()
        && base.fragment().is_none())
    .then_some(base)
}

fn message(base: &str, scope: &str, source: &str, target: &str, expires: u64) -> Vec<u8> {
    let mut message = b"CineHarbor media capability v1\0".to_vec();
    for field in [base.trim_end_matches('/'), scope, source, target] {
        message.extend_from_slice(&(field.len() as u64).to_be_bytes());
        message.extend_from_slice(field.as_bytes());
    }
    message.extend_from_slice(&expires.to_be_bytes());
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_bind_every_security_field_and_expire() {
        let auth = MediaAuthorization::new(&[7; 32]).unwrap();
        let base = "https://proxy.test/addon";
        let uri =
            crate::build_vod_proxy_segment_url(base, "src", "https://cdn.test/file?token=upstream");
        let signed = auth.sign_proxy_url_at(&uri, base, 100).unwrap();
        let parsed = Url::parse(&signed).unwrap();
        let pairs = parsed
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        let expires = pairs["expires"].parse().unwrap();
        let signature = &pairs["sig"];
        let target = "https://cdn.test/file?token=upstream";
        assert!(auth.verify_at(base, "vod/segment", "src", target, signature, 100..expires));
        for (b, scope, source, url, time, sig) in [
            (
                "https://other.test",
                "vod/segment",
                "src",
                target,
                expires,
                signature.as_ref(),
            ),
            (base, "vod/key", "src", target, expires, signature.as_ref()),
            (
                base,
                "live/segment",
                "src",
                target,
                expires,
                signature.as_ref(),
            ),
            (
                base,
                "vod/segment",
                "other",
                target,
                expires,
                signature.as_ref(),
            ),
            (
                base,
                "vod/segment",
                "src",
                "http://127.0.0.1/secret",
                expires,
                signature.as_ref(),
            ),
            (
                base,
                "vod/segment",
                "src",
                target,
                expires + 1,
                signature.as_ref(),
            ),
            (
                base,
                "vod/segment",
                "src",
                target,
                expires,
                "not-a-signature",
            ),
        ] {
            assert!(!auth.verify_at(b, scope, source, url, sig, 100..time));
        }
        assert!(!auth.verify_at(
            base,
            "vod/segment",
            "src",
            target,
            signature,
            expires..expires
        ));
        assert!(!MediaAuthorization::new(&[8; 32]).unwrap().verify_at(
            base,
            "vod/segment",
            "src",
            target,
            signature,
            100..expires
        ));
        assert!(MediaAuthorization::new(b"short").is_err());
        assert_eq!(auth.sign_proxy_url_at(&signed, base, 100).unwrap(), signed);
    }
}

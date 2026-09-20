//! No ambient proxy, unsafe literal address, DNS rebinding, or implicit redirect egress.

use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use reqwest::{
    Client, Method, Url,
    dns::{Addrs, Name, Resolve, Resolving},
    header::{HeaderMap, LOCATION},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const MAX_REDIRECTS: usize = 5;

#[derive(Clone)]
pub struct MediaClient {
    client: Client,
    slots: Arc<Semaphore>,
    allowed_ports: Vec<u16>,
    #[cfg(test)]
    fixture: Option<SocketAddr>,
}

pub(super) struct Upstream {
    pub response: reqwest::Response,
    pub _permit: OwnedSemaphorePermit,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum FetchError {
    Forbidden,
    Unavailable,
    Upstream,
}

impl MediaClient {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: Client::builder()
                .no_proxy()
                .no_gzip()
                .no_brotli()
                .no_deflate()
                .no_zstd()
                .referer(false)
                .redirect(reqwest::redirect::Policy::none())
                .dns_resolver(Arc::new(PublicDns))
                .connect_timeout(Duration::from_secs(5))
                .read_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(60))
                .pool_idle_timeout(Duration::from_secs(30))
                .pool_max_idle_per_host(2)
                .build()?,
            slots: Arc::new(Semaphore::new(16)),
            allowed_ports: vec![80, 443],
            #[cfg(test)]
            fixture: None,
        })
    }

    /// Nonstandard public CDN ports require an explicit operator configuration. There is
    /// intentionally no environment switch that disables private-address filtering.
    pub fn from_environment() -> Result<Self, &'static str> {
        let mut client = Self::new().map_err(|_| "media client initialization failed")?;
        match std::env::var("CINEHARBOR_MEDIA_ALLOWED_PORTS") {
            Ok(value) => client.allowed_ports = parse_ports(&value)?,
            Err(std::env::VarError::NotPresent) => {}
            Err(_) => return Err("invalid media allowed ports"),
        }
        Ok(client)
    }

    #[cfg(test)]
    pub(super) fn available_slots(&self) -> usize {
        self.slots.available_permits()
    }

    pub(super) async fn send(
        &self,
        method: Method,
        input: &str,
        headers: HeaderMap,
    ) -> Result<Upstream, FetchError> {
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| FetchError::Unavailable)?;
        let mut url = Url::parse(input).map_err(|_| FetchError::Forbidden)?;
        for hop in 0..=MAX_REDIRECTS {
            self.validate(&url)?;
            let response = self
                .client
                .request(method.clone(), url.clone())
                .headers(headers.clone())
                .send()
                .await
                .map_err(|_| FetchError::Upstream)?;
            if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                return Ok(Upstream {
                    response,
                    _permit: permit,
                });
            }
            if hop == MAX_REDIRECTS {
                return Err(FetchError::Upstream);
            }
            let next = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| url.join(value).ok())
                .ok_or(FetchError::Upstream)?;
            if url.scheme() == "https" && next.scheme() != "https" {
                return Err(FetchError::Forbidden);
            }
            // Revalidate before every request; reqwest itself must never follow redirects.
            url = next;
        }
        Err(FetchError::Upstream)
    }

    fn validate(&self, url: &Url) -> Result<(), FetchError> {
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err(FetchError::Forbidden);
        }
        // This narrowly scoped loopback exception does not exist in production builds.
        #[cfg(test)]
        if self.fixture.is_some_and(|addr| {
            url.host_str() == Some(&addr.ip().to_string())
                && url.port_or_known_default() == Some(addr.port())
        }) {
            return Ok(());
        }
        if !url
            .port_or_known_default()
            .is_some_and(|port| self.allowed_ports.contains(&port))
        {
            return Err(FetchError::Forbidden);
        }
        match url.host() {
            Some(url::Host::Ipv4(ip)) if public_ip(IpAddr::V4(ip)) => Ok(()),
            Some(url::Host::Ipv6(ip)) if public_ip(IpAddr::V6(ip)) => Ok(()),
            Some(url::Host::Domain(host))
                if host.contains('.')
                    && !host.ends_with('.')
                    && !host.ends_with(".localhost")
                    && !host.ends_with(".local")
                    && !host.ends_with(".internal") =>
            {
                Ok(())
            }
            _ => Err(FetchError::Forbidden),
        }
    }

    #[cfg(test)]
    pub(super) fn for_fixture(addr: SocketAddr) -> Self {
        let mut client = Self::new().unwrap();
        client.fixture = Some(addr);
        client
    }
}

fn parse_ports(value: &str) -> Result<Vec<u16>, &'static str> {
    if value.len() > 256 {
        return Err("invalid media allowed ports");
    }
    let mut ports = value
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<u16>()
                .map_err(|_| "invalid media allowed ports")
        })
        .collect::<Result<Vec<_>, _>>()?;
    if ports.is_empty() || ports.len() > 32 || ports.contains(&0) {
        return Err("invalid media allowed ports");
    }
    ports.sort_unstable();
    ports.dedup();
    Ok(ports)
}

struct PublicDns;

impl Resolve for PublicDns {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let result = tokio::time::timeout(
                Duration::from_secs(5),
                tokio::net::lookup_host((name.as_str(), 0)),
            )
            .await
            .map_err(|_| io::Error::other("media DNS timeout"))??;
            // These exact vetted socket addresses are returned to the connector. There is
            // no second DNS lookup between validation and connection establishment.
            let addresses = vetted_addresses(result.take(33).collect())?;
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

fn vetted_addresses(addresses: Vec<SocketAddr>) -> io::Result<Vec<SocketAddr>> {
    if addresses.is_empty()
        || addresses.len() > 32
        || addresses.iter().any(|addr| !public_ip(addr.ip()))
    {
        return Err(io::Error::other("media DNS destination denied"));
    }
    Ok(addresses)
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192
                    && ((b == 0 && (c == 0 || c == 2)) || (b == 88 && c == 99) || b == 168))
                || (a == 198 && ((18..=19).contains(&b) || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            // Conservative global-unicast-only policy excludes mapped/compatible IPv4,
            // NAT64, 6to4, Teredo, local/link-local, multicast and documentation ranges.
            (s[0] & 0xe000) == 0x2000
                && s[0] != 0x2002
                && s[0] != 0x3ffe
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && !(s[0] == 0x3fff && (s[1] & 0xf000) == 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_special_addresses_and_normalized_literal_bypasses() {
        let client = MediaClient::new().unwrap();
        for input in [
            "http://127.0.0.1/",
            "http://127.1/",
            "http://2130706433/",
            "http://0x7f000001/",
            "http://0177.0.0.1/",
            "http://0.0.0.0/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/",
            "http://100.64.0.1/",
            "http://198.18.0.1/",
            "http://224.0.0.1/",
            "http://255.255.255.255/",
            "http://192.0.2.1/",
            "http://198.51.100.1/",
            "http://203.0.113.1/",
            "http://[::1]/",
            "http://[::]/",
            "http://[::ffff:127.0.0.1]/",
            "http://[64:ff9b::7f00:1]/",
            "http://[2002:7f00:1::]/",
            "http://[2001:db8::1]/",
            "http://[fc00::1]/",
            "http://[fe80::1]/",
            "http://[3fff::1]/",
            "http://localhost/",
            "http://metadata.google.internal/",
            "http://cdn.local/",
            "file:///etc/passwd",
            "ftp://cdn.test/file",
            "https://user:pass@cdn.test/a",
            "https://cdn.test:22/a",
            "https://cdn.test/a#fragment",
        ] {
            assert!(
                client.validate(&Url::parse(input).unwrap()).is_err(),
                "{input}"
            );
        }
        for input in [
            "https://cdn.test/a",
            "http://8.8.8.8/a",
            "https://[2606:4700:4700::1111]/a",
        ] {
            assert!(
                client.validate(&Url::parse(input).unwrap()).is_ok(),
                "{input}"
            );
        }
    }

    #[test]
    fn dns_rejects_empty_mixed_and_rebound_answers_without_a_second_lookup() {
        let public: SocketAddr = "8.8.8.8:0".parse().unwrap();
        let private: SocketAddr = "127.0.0.1:0".parse().unwrap();
        assert_eq!(vetted_addresses(vec![public]).unwrap(), vec![public]);
        assert!(vetted_addresses(vec![public, private]).is_err());
        assert!(vetted_addresses(vec![private]).is_err());
        assert!(vetted_addresses(vec![]).is_err());
        assert!(vetted_addresses(vec![public; 33]).is_err());
    }

    #[tokio::test]
    async fn actual_resolver_rejects_loopback_resolution() {
        assert!(
            PublicDns
                .resolve("localhost".parse().unwrap())
                .await
                .is_err()
        );
    }
    #[test]
    fn operator_port_configuration_is_bounded_and_does_not_allow_private_addresses() {
        assert_eq!(
            parse_ports("443, 80,8080,443").unwrap(),
            vec![80, 443, 8080]
        );
        for input in ["", "*", "0", "65536", "80,,443"] {
            assert!(parse_ports(input).is_err());
        }
        let mut client = MediaClient::new().unwrap();
        client.allowed_ports = parse_ports("8080").unwrap();
        assert!(
            client
                .validate(&Url::parse("http://cdn.test:8080/file").unwrap())
                .is_ok()
        );
        assert!(
            client
                .validate(&Url::parse("http://127.0.0.1:8080/file").unwrap())
                .is_err()
        );
    }

    #[tokio::test]
    async fn saturation_is_rejected_without_queueing_or_network_io() {
        let client = MediaClient::new().unwrap();
        let permits = (0..16)
            .map(|_| client.slots.clone().try_acquire_owned().unwrap())
            .collect::<Vec<_>>();
        assert!(matches!(
            client
                .send(Method::GET, "https://cdn.test/file", HeaderMap::new())
                .await,
            Err(FetchError::Unavailable)
        ));
        drop(permits);
        assert_eq!(client.available_slots(), 16);
    }
}

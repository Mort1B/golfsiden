use std::{net::IpAddr, sync::Arc};

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub const PROXY_CLIENT_IP_HEADER: &str = "x-golf-client-ip";
pub const PROXY_SHARED_SECRET_HEADER: &str = "x-golf-proxy-secret";

#[derive(Clone)]
pub struct ProxyTrustConfig {
    shared_secret: Option<Arc<str>>,
}

impl std::fmt::Debug for ProxyTrustConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProxyTrustConfig")
            .field(
                "shared_secret",
                &self.shared_secret.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientIdentity([u8; 32]);

impl ProxyTrustConfig {
    pub fn direct() -> Self {
        Self {
            shared_secret: None,
        }
    }

    pub fn trusted(shared_secret: String) -> Self {
        Self {
            shared_secret: Some(Arc::from(shared_secret)),
        }
    }

    pub fn client_identity(&self, headers: &HeaderMap) -> ClientIdentity {
        let Some(expected) = &self.shared_secret else {
            return ClientIdentity::DIRECT;
        };
        let Some(presented) = headers.get(PROXY_SHARED_SECRET_HEADER) else {
            return ClientIdentity::DIRECT;
        };
        if !bool::from(expected.as_bytes().ct_eq(presented.as_bytes())) {
            return ClientIdentity::DIRECT;
        }
        let Some(ip) = headers
            .get(PROXY_CLIENT_IP_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<IpAddr>().ok())
        else {
            return ClientIdentity::DIRECT;
        };
        ClientIdentity::from_ip(ip)
    }
}

impl ClientIdentity {
    const DIRECT: Self = Self([0; 32]);

    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn from_ip(ip: IpAddr) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"trusted-proxy-client-ip\0");
        match ip {
            IpAddr::V4(ip) => hasher.update(ip.octets()),
            IpAddr::V6(ip) => hasher.update(ip.octets()),
        }
        Self(hasher.finalize().into())
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    const SECRET: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    fn headers(ip: &str, secret: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(PROXY_CLIENT_IP_HEADER, HeaderValue::from_str(ip).unwrap());
        if let Some(secret) = secret {
            headers.insert(
                PROXY_SHARED_SECRET_HEADER,
                HeaderValue::from_str(secret).unwrap(),
            );
        }
        headers
    }

    #[test]
    fn untrusted_private_and_forwarding_headers_cannot_change_direct_identity() {
        let trust = ProxyTrustConfig::trusted(SECRET.to_owned());
        let direct = trust.client_identity(&HeaderMap::new());
        let mut spoofed = headers("198.51.100.10", Some("wrong-secret"));
        spoofed.insert("forwarded", HeaderValue::from_static("for=203.0.113.8"));
        spoofed.insert("x-forwarded-for", HeaderValue::from_static("192.0.2.44"));
        assert_eq!(trust.client_identity(&spoofed), direct);
        assert_eq!(ProxyTrustConfig::direct().client_identity(&spoofed), direct);
    }

    #[test]
    fn authenticated_proxy_clients_receive_distinct_canonical_ip_identities() {
        let trust = ProxyTrustConfig::trusted(SECRET.to_owned());
        let debug = format!("{trust:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains(SECRET));
        let first = trust.client_identity(&headers("198.51.100.10", Some(SECRET)));
        let same = trust.client_identity(&headers("198.51.100.10", Some(SECRET)));
        let second = trust.client_identity(&headers("198.51.100.11", Some(SECRET)));
        assert_eq!(first, same);
        assert_ne!(first, second);
        assert_ne!(first, ClientIdentity::DIRECT);
    }
}

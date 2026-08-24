use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use http::HeaderMap;
use ipnet::IpNet;

#[derive(Clone, Debug)]
pub struct ClientIdentity {
    header: Option<String>,
    trusted_proxies: Vec<IpNet>,
}

impl ClientIdentity {
    pub fn new(header: Option<String>, trusted_proxies: Vec<IpNet>) -> Self {
        Self {
            header,
            trusted_proxies,
        }
    }

    pub fn resolve(&self, peer: IpAddr, headers: &HeaderMap) -> IpAddr {
        let trusted = self
            .trusted_proxies
            .iter()
            .any(|network| network.contains(&peer));
        if !trusted {
            return peer;
        }

        self.header
            .as_deref()
            .and_then(|name| headers.get(name))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(peer)
    }
}

pub fn network_key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(ip) => IpAddr::V4(ip),
        IpAddr::V6(ip) => {
            let bits = u128::from(ip) & (u128::MAX << 64);
            IpAddr::V6(Ipv6Addr::from(bits))
        }
    }
}

pub fn unspecified() -> IpAddr {
    IpAddr::V4(Ipv4Addr::UNSPECIFIED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_spoofed_header_from_untrusted_peer() {
        let identity = ClientIdentity::new(
            Some("cf-connecting-ip".into()),
            vec!["10.0.0.0/8".parse().unwrap()],
        );
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "203.0.113.10".parse().unwrap());

        assert_eq!(
            identity.resolve("192.0.2.4".parse().unwrap(), &headers),
            "192.0.2.4".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn accepts_header_from_trusted_peer() {
        let identity = ClientIdentity::new(
            Some("x-forwarded-for".into()),
            vec!["10.0.0.0/8".parse().unwrap()],
        );
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.10".parse().unwrap());

        assert_eq!(
            identity.resolve("10.0.0.2".parse().unwrap(), &headers),
            "203.0.113.10".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn rejects_ambiguous_forwarding_chains() {
        let identity = ClientIdentity::new(
            Some("x-forwarded-for".into()),
            vec!["10.0.0.0/8".parse().unwrap()],
        );
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.10, 10.0.0.2".parse().unwrap());

        assert_eq!(
            identity.resolve("10.0.0.2".parse().unwrap(), &headers),
            "10.0.0.2".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn groups_ipv6_clients_by_64() {
        assert_eq!(
            network_key("2001:db8:42:7:dead:beef::1".parse().unwrap()),
            "2001:db8:42:7::".parse::<IpAddr>().unwrap()
        );
    }
}

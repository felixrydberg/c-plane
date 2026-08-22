use std::{env, net::SocketAddr, str::FromStr};

use ipnet::IpNet;

use crate::{
    limit::{LimitMode, LimitPolicy, LimitSettings, TrafficClass},
    routing::{Authority, Service, Upstreams},
};

#[derive(Clone, Debug)]
pub struct Config {
    pub listen: Vec<String>,
    pub forwarded_proto: String,
    pub client_ip_header: Option<String>,
    pub trusted_proxies: Vec<IpNet>,
    pub authorities: Authorities,
    pub upstreams: Upstreams,
    pub limits: LimitSettings,
}

#[derive(Clone, Debug)]
pub struct Authorities {
    pub platform: Vec<Authority>,
    pub api: Vec<Authority>,
    pub storage: Vec<Authority>,
    pub registry: Vec<Authority>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            listen: csv(
                "INGRESS_LISTEN",
                "0.0.0.0:3000,0.0.0.0:8080,0.0.0.0:8081,0.0.0.0:5000",
            ),
            forwarded_proto: value("INGRESS_FORWARDED_PROTO", "http"),
            client_ip_header: optional("INGRESS_CLIENT_IP_HEADER"),
            trusted_proxies: parse_csv("INGRESS_TRUSTED_PROXY_CIDRS")?,
            authorities: Authorities {
                platform: authorities("INGRESS_PLATFORM_HOSTS", "localhost:3000")?,
                api: authorities("INGRESS_API_HOSTS", "localhost:8080")?,
                storage: authorities_with_suffixes(
                    "INGRESS_STORAGE_HOSTS",
                    "localhost:8081",
                    "INGRESS_STORAGE_HOST_SUFFIXES",
                )?,
                registry: authorities("INGRESS_REGISTRY_HOSTS", "localhost:5000")?,
            },
            upstreams: Upstreams {
                ui: upstream("INGRESS_UI_UPSTREAM", "ui:3000")?,
                api: upstream("INGRESS_API_UPSTREAM", "api:8080")?,
                storage: upstream("INGRESS_STORAGE_UPSTREAM", "storage:8081")?,
                registry: upstream("INGRESS_REGISTRY_UPSTREAM", "registry:5000")?,
            },
            limits: LimitSettings {
                mode: LimitMode::from_str(&value("INGRESS_RATE_MODE", "observe"))?,
                auth: policy("INGRESS_RATE_AUTH", LimitPolicy::new(20, 200, 32))?,
                ui: policy("INGRESS_RATE_UI", LimitPolicy::new(50, 1_000, 128))?,
                api_read: policy("INGRESS_RATE_API_READ", LimitPolicy::new(30, 600, 64))?,
                api_write: policy("INGRESS_RATE_API_WRITE", LimitPolicy::new(10, 120, 32))?,
                storage: policy("INGRESS_RATE_STORAGE", LimitPolicy::new(200, 6_000, 64))?,
                registry: policy("INGRESS_RATE_REGISTRY", LimitPolicy::new(100, 3_000, 64))?,
            },
        })
    }

    pub fn policy(&self, class: TrafficClass) -> LimitPolicy {
        self.limits.policy(class)
    }
}

fn value(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn optional(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn csv(name: &str, default: &str) -> Vec<String> {
    value(name, default)
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

fn parse_csv<T>(name: &str) -> Result<Vec<T>, String>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    csv(name, "")
        .into_iter()
        .map(|item| {
            item.parse()
                .map_err(|error| format!("invalid {name} value {item:?}: {error}"))
        })
        .collect()
}

fn authorities(name: &str, default: &str) -> Result<Vec<Authority>, String> {
    Ok(csv(name, default)
        .into_iter()
        .map(Authority::exact)
        .collect())
}

fn authorities_with_suffixes(
    exact_name: &str,
    exact_default: &str,
    suffix_name: &str,
) -> Result<Vec<Authority>, String> {
    let mut values = authorities(exact_name, exact_default)?;
    values.extend(csv(suffix_name, "").into_iter().map(Authority::suffix));
    Ok(values)
}

fn upstream(name: &str, default: &str) -> Result<SocketAddr, String> {
    use std::net::ToSocketAddrs;

    let address = value(name, default);
    address
        .to_socket_addrs()
        .map_err(|error| format!("could not resolve {name}={address:?}: {error}"))?
        .next()
        .ok_or_else(|| format!("{name}={address:?} did not resolve to an address"))
}

fn policy(name: &str, default: LimitPolicy) -> Result<LimitPolicy, String> {
    let Some(raw) = optional(name) else {
        return Ok(default);
    };
    let values = raw
        .split(',')
        .map(str::trim)
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid {name}: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    match values.as_slice() {
        [per_second, per_minute, in_flight] => {
            Ok(LimitPolicy::new(*per_second, *per_minute, *in_flight))
        }
        _ => Err(format!(
            "{name} must contain per-second,per-minute,in-flight"
        )),
    }
}

impl Upstreams {
    pub fn get(&self, service: Service) -> SocketAddr {
        match service {
            Service::Ui => self.ui,
            Service::Api => self.api,
            Service::Storage => self.storage,
            Service::Registry => self.registry,
        }
    }
}

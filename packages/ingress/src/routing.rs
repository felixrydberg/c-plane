use std::net::SocketAddr;

use http::Method;

use crate::{config::Authorities, limit::TrafficClass};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Service {
    Ui,
    Api,
    Storage,
    Registry,
}

#[derive(Clone, Debug)]
pub struct Upstreams {
    pub ui: SocketAddr,
    pub api: SocketAddr,
    pub storage: SocketAddr,
    pub registry: SocketAddr,
}

#[derive(Clone, Debug)]
pub struct Authority(String);

impl Authority {
    pub fn exact(value: String) -> Self {
        Self(normalize_authority(&value))
    }

    fn matches(&self, authority: &str) -> bool {
        normalize_authority(authority) == self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteMatch {
    pub id: &'static str,
    pub service: Service,
    pub class: TrafficClass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Proxy(RouteMatch),
    NotFound,
    Misdirected,
}

#[derive(Clone, Debug)]
pub struct Router {
    authorities: Authorities,
}

impl Router {
    pub fn new(authorities: Authorities) -> Self {
        Self { authorities }
    }

    pub fn route(&self, authority: &str, path: &str, method: &Method) -> Decision {
        if matches_any(&self.authorities.platform, authority) {
            if path_prefix(path, "/api/registry/token") {
                return proxy("platform.registry-token", Service::Api, TrafficClass::Auth);
            }
            if path_prefix(path, "/api") {
                return proxy("platform.api", Service::Api, api_class(method));
            }
            if path_prefix(path, "/ui-api/auth") {
                return proxy("platform.auth", Service::Ui, TrafficClass::Auth);
            }
            if path_prefix(path, "/ui-api") {
                return proxy("platform.ui-api", Service::Ui, api_class(method));
            }
            return proxy("platform.ui", Service::Ui, TrafficClass::Ui);
        }

        if matches_any(&self.authorities.api, authority) {
            if path_prefix(path, "/api/registry/token") {
                return proxy("api.registry-token", Service::Api, TrafficClass::Auth);
            }
            if path_prefix(path, "/api") || path == "/health" {
                return proxy("api", Service::Api, api_class(method));
            }
            return Decision::NotFound;
        }

        if matches_any(&self.authorities.storage, authority) {
            if path_prefix(path, "/.cplane") {
                return Decision::NotFound;
            }
            return proxy("storage", Service::Storage, TrafficClass::Storage);
        }

        if matches_any(&self.authorities.registry, authority) {
            if path_prefix(path, "/v2") {
                return proxy("registry", Service::Registry, TrafficClass::Registry);
            }
            return Decision::NotFound;
        }

        Decision::Misdirected
    }
}

fn proxy(id: &'static str, service: Service, class: TrafficClass) -> Decision {
    Decision::Proxy(RouteMatch { id, service, class })
}

fn matches_any(matchers: &[Authority], authority: &str) -> bool {
    matchers.iter().any(|matcher| matcher.matches(authority))
}

fn path_prefix(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn api_class(method: &Method) -> TrafficClass {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        TrafficClass::ApiRead
    } else {
        TrafficClass::ApiWrite
    }
}

fn normalize_authority(value: &str) -> String {
    value.trim().trim_end_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn router() -> Router {
        Router::new(Authorities {
            platform: vec![Authority::exact("app.example.com".into())],
            api: vec![Authority::exact("api.example.com".into())],
            storage: vec![Authority::exact("storage.example.com".into())],
            registry: vec![Authority::exact("registry.example.com".into())],
        })
    }

    #[test]
    fn routes_platform_paths_without_rewriting_them() {
        let router = router();
        assert_eq!(
            router.route("app.example.com", "/api/projects", &Method::GET),
            proxy("platform.api", Service::Api, TrafficClass::ApiRead)
        );
        assert_eq!(
            router.route("app.example.com", "/ui-api/auth/session", &Method::POST),
            proxy("platform.auth", Service::Ui, TrafficClass::Auth)
        );
        assert_eq!(
            router.route("app.example.com", "/dashboard", &Method::GET),
            proxy("platform.ui", Service::Ui, TrafficClass::Ui)
        );
    }

    #[test]
    fn prefix_matching_respects_path_boundaries() {
        assert!(!path_prefix("/apix", "/api"));
        assert!(path_prefix("/api", "/api"));
        assert!(path_prefix("/api/projects", "/api"));
    }

    #[test]
    fn storage_requires_the_exact_authority() {
        let router = router();
        assert_eq!(
            router.route("storage.example.com", "/bucket/object", &Method::GET),
            proxy("storage", Service::Storage, TrafficClass::Storage)
        );
        assert_eq!(
            router.route("bucket.storage.example.com", "/object", &Method::GET),
            Decision::Misdirected
        );
    }

    #[test]
    fn known_authority_with_wrong_path_is_not_found() {
        let router = router();
        assert_eq!(
            router.route("registry.example.com", "/admin", &Method::GET),
            Decision::NotFound
        );
    }
}

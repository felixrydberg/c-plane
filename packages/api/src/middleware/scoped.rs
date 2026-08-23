use axum::{Extension, routing::MethodRouter};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use crate::errors::AppError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Member,
    Admin,
    Owner,
}

impl Role {
    pub fn parse(raw: &str) -> Role {
        match raw {
            "owner" => Role::Owner,
            "admin" => Role::Admin,
            _ => Role::Member,
        }
    }
}

#[derive(Clone, Copy)]
pub struct RouteGuard {
    pub scope: &'static str,
    pub min_role: Role,
}

pub struct Scoped<S> {
    pub(super) method: &'static str,
    pub(super) guard: RouteGuard,
    pub(super) router: MethodRouter<S>,
}

pub fn get<S, H, T>(handler: H, scope: &'static str, min_role: Role) -> Scoped<S>
where
    H: axum::handler::Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    Scoped {
        method: "GET",
        guard: RouteGuard { scope, min_role },
        router: axum::routing::get(handler),
    }
}

pub fn post<S, H, T>(handler: H, scope: &'static str, min_role: Role) -> Scoped<S>
where
    H: axum::handler::Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    Scoped {
        method: "POST",
        guard: RouteGuard { scope, min_role },
        router: axum::routing::post(handler),
    }
}

pub fn patch<S, H, T>(handler: H, scope: &'static str, min_role: Role) -> Scoped<S>
where
    H: axum::handler::Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    Scoped {
        method: "PATCH",
        guard: RouteGuard { scope, min_role },
        router: axum::routing::patch(handler),
    }
}

pub fn delete<S, H, T>(handler: H, scope: &'static str, min_role: Role) -> Scoped<S>
where
    H: axum::handler::Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    Scoped {
        method: "DELETE",
        guard: RouteGuard { scope, min_role },
        router: axum::routing::delete(handler),
    }
}

pub trait ScopedRouter<S>: Sized {
    fn scoped_route(self, path: &'static str, specs: impl IntoIterator<Item = Scoped<S>>) -> Self;
}

impl<S: Clone + Send + Sync + 'static> ScopedRouter<S> for axum::Router<S> {
    fn scoped_route(self, path: &'static str, specs: impl IntoIterator<Item = Scoped<S>>) -> Self {
        let mut router: Option<MethodRouter<S>> = None;
        for spec in specs {
            register_policy(spec.method, path, spec.guard.scope);
            let routed = spec.router.layer(Extension(spec.guard));
            router = Some(match router {
                Some(built) => built.merge(routed),
                None => routed,
            });
        }
        self.route(
            path,
            router.expect("scoped_route requires at least one method spec"),
        )
    }
}

fn policies() -> &'static Mutex<Vec<(&'static str, &'static str, &'static str)>> {
    static POLICIES: OnceLock<Mutex<Vec<(&'static str, &'static str, &'static str)>>> =
        OnceLock::new();
    POLICIES.get_or_init(|| Mutex::new(Vec::new()))
}

fn register_policy(method: &'static str, path: &'static str, scope: &'static str) {
    let mut policies = policies().lock().unwrap();
    if !policies
        .iter()
        .any(|(m, p, s)| *m == method && *p == path && *s == scope)
    {
        policies.push((method, path, scope));
    }
}

pub(crate) fn registered_scope(method: &str, path: &str) -> Option<&'static str> {
    let method = if method == "HEAD" { "GET" } else { method };
    policies()
        .lock()
        .unwrap()
        .iter()
        .find(|(m, p, _)| *m == method && *p == path)
        .map(|(_, _, s)| *s)
}

#[cfg(test)]
pub(crate) fn seed_policy_for_tests(method: &'static str, path: &'static str, scope: &'static str) {
    register_policy(method, path, scope);
}

pub fn check_api_key(
    guard: Option<RouteGuard>,
    key_scopes: &HashSet<String>,
) -> Result<(), AppError> {
    let Some(guard) = guard else {
        return Err(AppError::Forbidden(
            "API keys cannot access this route".into(),
        ));
    };
    if key_scopes.contains(guard.scope) {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "API key is missing required scope: {}",
            guard.scope
        )))
    }
}

pub fn role_sufficient(member_role: Option<Role>, min_role: Role) -> bool {
    member_role.is_some_and(|role| role >= min_role)
}

fn organization_id_from_path(path: &str) -> Option<uuid::Uuid> {
    path.split('/').nth(3).and_then(|s| s.parse().ok())
}

pub fn check_role(
    guard: RouteGuard,
    request_path: &str,
    roles: &HashMap<uuid::Uuid, Role>,
) -> Result<(), AppError> {
    let Some(organization_id) = organization_id_from_path(request_path) else {
        return Err(AppError::Forbidden(
            "Route has no organization context".into(),
        ));
    };
    if role_sufficient(roles.get(&organization_id).copied(), guard.min_role) {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "{:?} role or higher is required",
            guard.min_role
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_keys_fail_closed_without_a_guard_and_need_the_exact_scope() {
        let scopes = HashSet::from(["project:read".to_string()]);
        assert!(check_api_key(None, &scopes).is_err());
        assert!(
            check_api_key(
                Some(RouteGuard {
                    scope: "project:delete",
                    min_role: Role::Owner
                }),
                &scopes
            )
            .is_err()
        );
        assert!(
            check_api_key(
                Some(RouteGuard {
                    scope: "project:read",
                    min_role: Role::Member
                }),
                &scopes
            )
            .is_ok()
        );
    }

    #[test]
    fn roles_rank_from_weakest_to_strongest() {
        assert!(role_sufficient(Some(Role::Owner), Role::Owner));
        assert!(role_sufficient(Some(Role::Member), Role::Member));
        assert!(!role_sufficient(Some(Role::Member), Role::Admin));
        assert!(!role_sufficient(None, Role::Member));
    }

    #[test]
    fn unknown_role_strings_count_as_member() {
        assert_eq!(Role::parse("owner"), Role::Owner);
        assert_eq!(Role::parse("something-new"), Role::Member);
    }

    #[test]
    fn organization_id_is_read_from_the_fourth_segment() {
        let org = uuid::Uuid::new_v4();
        let path = format!("/api/organization/{org}/projects");
        assert_eq!(super::organization_id_from_path(&path), Some(org));
        assert_eq!(super::organization_id_from_path("/health"), None);
    }
}

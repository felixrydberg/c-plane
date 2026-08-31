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

pub fn put<S, H, T>(handler: H, scope: &'static str, min_role: Role) -> Scoped<S>
where
    H: axum::handler::Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    Scoped {
        method: "PUT",
        guard: RouteGuard { scope, min_role },
        router: axum::routing::put(handler),
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
            register_policy(spec.method, path, spec.guard.scope, spec.guard.min_role)
                .unwrap_or_else(|err| panic!("{err}"));
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

type RegisteredPolicy = (&'static str, &'static str, &'static str, Role);

fn policies() -> &'static Mutex<Vec<RegisteredPolicy>> {
    static POLICIES: OnceLock<Mutex<Vec<RegisteredPolicy>>> = OnceLock::new();
    POLICIES.get_or_init(|| Mutex::new(Vec::new()))
}

fn register_policy(
    method: &'static str,
    path: &'static str,
    scope: &'static str,
    min_role: Role,
) -> Result<(), AppError> {
    let mut policies = policies().lock().unwrap();
    if let Some((_, _, registered_scope, registered_min_role)) = policies
        .iter()
        .find(|(m, p, _, _)| *m == method && *p == path)
    {
        if *registered_scope != scope || *registered_min_role != min_role {
            return Err(AppError::Conflict(format!(
                "Conflicting route policy for {method} {path}"
            )));
        }
        return Ok(());
    }

    policies.push((method, path, scope, min_role));
    Ok(())
}

pub(crate) fn registered_scope(method: &str, path: &str) -> Option<&'static str> {
    let method = if method == "HEAD" { "GET" } else { method };
    policies()
        .lock()
        .unwrap()
        .iter()
        .find(|(m, p, _, _)| *m == method && *p == path)
        .map(|(_, _, scope, _)| *scope)
}

#[cfg(test)]
pub(crate) fn registered_min_role(method: &str, path: &str) -> Option<Role> {
    let method = if method == "HEAD" { "GET" } else { method };
    policies()
        .lock()
        .unwrap()
        .iter()
        .find(|(m, p, _, _)| *m == method && *p == path)
        .map(|(_, _, _, min_role)| *min_role)
}

#[cfg(test)]
pub(crate) fn seed_policy_for_tests(method: &'static str, path: &'static str, scope: &'static str) {
    register_policy(method, path, scope, Role::Member).unwrap();
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

pub fn check_role(
    guard: RouteGuard,
    organization_id: Option<uuid::Uuid>,
    roles: &HashMap<uuid::Uuid, Role>,
) -> Result<(), AppError> {
    let Some(organization_id) = organization_id else {
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
    fn check_role_fails_closed_without_a_route_organization() {
        let org = uuid::Uuid::new_v4();
        let roles = HashMap::from([(org, Role::Owner)]);
        let guard = RouteGuard {
            scope: "project:delete",
            min_role: Role::Owner,
        };

        assert!(check_role(guard, Some(org), &roles).is_ok());
        assert!(check_role(guard, None, &roles).is_err());
    }

    #[test]
    fn conflicting_policy_is_rejected_and_existing_scope_is_preserved() {
        let path = "/test/conflicting-policy";
        assert!(register_policy("GET", path, "scope:one", Role::Member).is_ok());

        let error = register_policy("GET", path, "scope:two", Role::Member).unwrap_err();
        assert!(matches!(error, AppError::Conflict(_)));
        assert_eq!(registered_scope("GET", path), Some("scope:one"));
        assert_eq!(registered_min_role("GET", path), Some(Role::Member));
    }
}

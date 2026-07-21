use dioxus::prelude::*;

mod backend;
mod models;
mod views;

use views::*;

const CSS: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(feature = "server")]
    dioxus::serve(|| async {
        backend::server::initialize()
            .await
            .unwrap_or_else(|error| panic!("control-plane initialization failed: {error}"));
        Ok(dioxus::server::router(App)
            .route(
                "/internal/s3-providers/{id}/credentials",
                dioxus::server::axum::routing::get(backend::server::credentials_handler),
            )
            .route(
                "/internal/s3-buckets/{id}/sse-key",
                dioxus::server::axum::routing::put(backend::server::ensure_bucket_sse_key_handler),
            )
            .route(
                "/internal/s3-access-tokens/{id}",
                dioxus::server::axum::routing::put(
                    backend::server::store_access_token_secret_handler,
                )
                .delete(backend::server::delete_access_token_secret_handler),
            )
            .route(
                "/internal/s3-access-tokens/resolve/{access_key}",
                dioxus::server::axum::routing::get(backend::server::resolve_access_token_handler),
            )
            .route(
                "/internal/s3-access-token-cache",
                dioxus::server::axum::routing::delete(
                    backend::server::invalidate_access_token_cache_handler,
                ),
            )
            .route(
                "/internal/regions",
                dioxus::server::axum::routing::get(backend::server::eligible_regions_handler),
            ))
    });

    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Route {
    #[route("/")]
    Dashboard {},
    #[route("/organizations")]
    Organizations {},
    #[route("/api-keys")]
    ApiKeys {},
    #[route("/regions")]
    Regions {},
    #[route("/clusters")]
    Clusters {},
    #[route("/s3-providers")]
    S3Providers {},
    #[route("/audit-logs")]
    AuditLogs {},
}

#[allow(non_snake_case)]
fn App() -> Element {
    let maintenance_version = use_signal(|| 0_u64);
    use_context_provider(|| maintenance_version);
    rsx! {
        document::Stylesheet { href: CSS }
        Router::<Route> {}
    }
}

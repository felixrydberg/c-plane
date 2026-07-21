use crate::{Route, backend::*, models::*};
use dioxus::prelude::*;

#[component]
fn Shell(title: &'static str, children: Element) -> Element {
    let maintenance_version = use_context::<Signal<u64>>();
    let maintenance = use_resource(move || {
        let _ = maintenance_version();
        registry_gc_status()
    });
    let maintenance_state = maintenance.read().clone();
    rsx! {
        div { class: "shell",
            aside {
                div { class: "brand", "C-Plane" }
                nav {
                    Link { to: Route::Dashboard {}, "Overview" }
                    div { class: "nav-label", "Customers" }
                    Link { to: Route::Organizations {}, "Organizations" }
                    Link { to: Route::ApiKeys {}, "API Keys" }
                    div { class: "nav-label", "Infrastructure" }
                    Link { to: Route::Regions {}, "Regions" }
                    Link { to: Route::Clusters {}, "Clusters" }
                    Link { to: Route::S3Providers {}, "S3 Providers" }
                    Link { to: Route::AuditLogs {}, "Audit Logs" }
                }
            }
            main {
                header {
                    h1 { "{title}" }
                    span { class: "private", "Loopback / private network" }
                }
                if let Some(Ok(status)) = maintenance_state {
                    if status.phase != "idle" {
                        div { class: "maintenance-banner", role: "alert",
                            strong { "Registry maintenance in progress" }
                            span { "The Registry is read-only while garbage collection is {status.phase}. Pulls remain available." }
                        }
                    }
                }
                {children}
            }
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        Shell { title: "Overview",
            section { class: "intro",
                h2 { "Rust control plane" }
                p { "Infrastructure administration is served by one Dioxus application and audited by request identity and source IP." }
            }
            RegistryGarbageCollection {}
            div { class: "cards",
                SummaryCard { label: "Organizations", route: Route::Organizations {} }
                SummaryCard { label: "Regions", route: Route::Regions {} }
                SummaryCard { label: "Clusters", route: Route::Clusters {} }
                SummaryCard { label: "S3 Providers", route: Route::S3Providers {} }
                SummaryCard { label: "Audit Logs", route: Route::AuditLogs {} }
            }
        }
    }
}

#[component]
fn RegistryGarbageCollection() -> Element {
    let mut status = use_resource(registry_gc_status);
    let status_state = status.read().clone();
    let mut maintenance_version = use_context::<Signal<u64>>();
    let mut confirmed = use_signal(|| false);
    let mut queueing = use_signal(|| false);
    let mut message = use_signal(|| None::<String>);

    rsx! {
        section { class: "maintenance",
            h2 { "Registry garbage collection" }
            p { "Queue Distribution garbage collection. Workers make the Registry read-only before reclaiming blobs; pulls remain available." }
            match status_state {
                Some(Ok(current)) => rsx! {
                    p { class: "maintenance-status",
                        strong { "Status: " }
                        "{current.phase}"
                        if let Some(result) = current.last_result.as_deref() { " · Last run: {result}" }
                    }
                    if let Some(error) = current.last_error.as_deref() {
                        p { class: "form-error", "{error}" }
                    }
                    if current.phase == "idle" {
                        if confirmed() {
                            p { class: "confirmation", "This queues a global maintenance job and disables Registry writes until it finishes." }
                            div { class: "actions",
                                button {
                                    class: "danger",
                                    disabled: queueing(),
                                    onclick: move |_| async move {
                                        queueing.set(true);
                                        match enqueue_registry_gc().await {
                                            Ok(()) => {
                                                confirmed.set(false);
                                                message.set(None);
                                                status.restart();
                                                maintenance_version.set(maintenance_version().wrapping_add(1));
                                            }
                                            Err(error) => message.set(Some(error.to_string())),
                                        }
                                        queueing.set(false);
                                    },
                                    if queueing() { "↻ Queueing…" } else { "✓ Confirm garbage collection" }
                                }
                                button { disabled: queueing(), onclick: move |_| confirmed.set(false), "× Cancel" }
                            }
                        } else {
                            button { class: "primary", onclick: move |_| confirmed.set(true), "↻ Queue garbage collection" }
                        }
                    } else {
                        button { onclick: move |_| status.restart(), "↻ Refresh status" }
                    }
                    ErrorMessage { message }
                },
                Some(Err(error)) => rsx! { p { class: "state error", "{error}" } },
                None => rsx! { p { class: "state", "Loading…" } },
            }
        }
    }
}

#[component]
fn SummaryCard(label: &'static str, route: Route) -> Element {
    rsx! { Link { class: "card", to: route, strong { "{label}" } span { "Open →" } } }
}

#[component]
fn ErrorMessage(message: Signal<Option<String>>) -> Element {
    rsx! { if let Some(message) = message() { p { class: "form-error", "{message}" } } }
}

#[component]
pub fn Regions() -> Element {
    let mut rows = use_resource(list_regions);
    let providers = use_resource(list_s3_providers);
    let rows_state = rows.read().clone();
    let provider_items = providers
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    rsx! {
        Shell { title: "Regions",
            CreateRegion { providers: provider_items.clone(), on_saved: move |_| rows.restart() }
            match rows_state {
                Some(Ok(items)) => rsx! { table {
                    thead { tr { th { "Name" } th { "Slug" } th { "Status" } th { "S3 Provider" } th { "Actions" } } }
                    tbody { for item in items { RegionRow { key: "{item.id}", item, providers: provider_items.clone(), on_saved: move |_| rows.restart() } } }
                } },
                Some(Err(error)) => rsx! { p { class: "state error", "{error}" } },
                None => rsx! { p { class: "state", "Loading…" } },
            }
        }
    }
}

#[component]
fn CreateRegion(providers: Vec<S3Provider>, on_saved: EventHandler<()>) -> Element {
    let mut slug = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut status = use_signal(|| "active".to_string());
    let mut provider = use_signal(String::new);
    let mut message = use_signal(|| None::<String>);
    rsx! {
        details { class: "create", summary { "+ New region" }
            div { class: "form-grid",
                label { "Display name" input { value: name, oninput: move |e| name.set(e.value()) } }
                label { "Slug" input { value: slug, oninput: move |e| slug.set(e.value()) } }
                label { "Status" select { value: status, onchange: move |e| status.set(e.value()), option { value: "active", "Active" } option { value: "inactive", "Inactive" } option { value: "maintenance", "Maintenance" } } }
                label { "S3 provider" select { value: provider, onchange: move |e| provider.set(e.value()), option { value: "", "None" } for item in providers { option { value: "{item.id}", "{item.provider_type} — {item.endpoint_url}" } } } }
            }
            ErrorMessage { message }
            button { class: "primary", onclick: move |_| async move {
                match create_region(slug(), name(), status(), optional(provider())).await {
                    Ok(()) => { slug.set(String::new()); name.set(String::new()); message.set(None); on_saved.call(()); }
                    Err(error) => message.set(Some(error.to_string())),
                }
            }, "Create region" }
        }
    }
}

#[component]
fn RegionRow(item: Region, providers: Vec<S3Provider>, on_saved: EventHandler<()>) -> Element {
    let mut slug = use_signal(|| item.slug.clone());
    let mut name = use_signal(|| item.display_name.clone());
    let mut status = use_signal(|| item.status.clone());
    let mut provider = use_signal(|| item.s3_provider_id.clone().unwrap_or_default());
    let mut message = use_signal(|| None::<String>);
    let id = item.id.clone();
    let delete_id = item.id.clone();
    rsx! {
        tr {
            td { "{item.display_name}" } td { code { "{item.slug}" } } td { "{item.status}" } td { code { {item.s3_provider_id.as_deref().unwrap_or("None")} } }
            td { details { summary { "Edit" } div { class: "popover form-grid",
                label { "Name" input { value: name, oninput: move |e| name.set(e.value()) } }
                label { "Slug" input { value: slug, oninput: move |e| slug.set(e.value()) } }
                label { "Status" select { value: status, onchange: move |e| status.set(e.value()), option { value: "active", "Active" } option { value: "inactive", "Inactive" } option { value: "maintenance", "Maintenance" } } }
                label { "S3 provider" select { value: provider, onchange: move |e| provider.set(e.value()), option { value: "", "None" } for value in providers { option { value: "{value.id}", "{value.provider_type} — {value.endpoint_url}" } } } }
                ErrorMessage { message }
                div { class: "actions",
                    button { class: "primary", onclick: move |_| { let id = id.clone(); async move { match update_region(id, slug(), name(), status(), optional(provider())).await { Ok(()) => { message.set(None); on_saved.call(()); }, Err(error) => message.set(Some(error.to_string())) } } }, "Save" }
                    button { class: "danger", onclick: move |_| { let id = delete_id.clone(); async move { match delete_region(id).await { Ok(()) => on_saved.call(()), Err(error) => message.set(Some(error.to_string())) } } }, "Delete" }
                }
            } } }
        }
    }
}

#[component]
pub fn Clusters() -> Element {
    let mut rows = use_resource(list_clusters);
    let regions = use_resource(list_regions);
    let rows_state = rows.read().clone();
    let region_items = regions
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    rsx! {
        Shell { title: "Clusters",
            CreateCluster { regions: region_items.clone(), on_saved: move |_| rows.restart() }
            match rows_state {
                Some(Ok(items)) => rsx! { table { thead { tr { th { "Name" } th { "Slug" } th { "Status" } th { "Health" } th { "Capacity" } th { "Actions" } } } tbody { for item in items { ClusterRow { key: "{item.id}", item, regions: region_items.clone(), on_saved: move |_| rows.restart() } } } } },
                Some(Err(error)) => rsx! { p { class: "state error", "{error}" } }, None => rsx! { p { class: "state", "Loading…" } },
            }
        }
    }
}

#[component]
fn CreateCluster(regions: Vec<Region>, on_saved: EventHandler<()>) -> Element {
    let mut region = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut slug = use_signal(String::new);
    let mut message = use_signal(|| None::<String>);
    let mut token = use_signal(|| None::<String>);
    rsx! {
        details { class: "create", summary { "+ New cluster" }
            div { class: "form-grid",
                label { "Region" select { value: region, onchange: move |e| region.set(e.value()), option { value: "", "Select region" } for item in regions { option { value: "{item.id}", "{item.display_name}" } } } }
                label { "Name" input { value: name, oninput: move |e| name.set(e.value()) } }
                label { "Slug" input { value: slug, oninput: move |e| slug.set(e.value()) } }
            }
            ErrorMessage { message }
            if let Some(value) = token() { p { class: "token", "Join token (shown once): " code { "{value}" } } }
            button { class: "primary", onclick: move |_| async move { match create_cluster(region(), name(), slug()).await { Ok(value) => { token.set(Some(value.token)); message.set(None); on_saved.call(()); }, Err(error) => message.set(Some(error.to_string())) } }, "Create cluster" }
        }
    }
}

#[component]
fn ClusterRow(item: Cluster, regions: Vec<Region>, on_saved: EventHandler<()>) -> Element {
    let mut region = use_signal(|| item.region_id.clone());
    let mut name = use_signal(|| item.name.clone());
    let mut slug = use_signal(|| item.slug.clone());
    let mut agent_id = use_signal(|| item.agent_id.clone().unwrap_or_default());
    let mut agent_endpoint = use_signal(|| item.agent_endpoint.clone().unwrap_or_default());
    let mut status = use_signal(|| item.status.clone());
    let mut health = use_signal(|| item.health_status.clone());
    let mut allocatable = use_signal(|| item.capacity_allocatable.to_string());
    let mut used = use_signal(|| item.capacity_used.to_string());
    let mut message = use_signal(|| None::<String>);
    let mut token = use_signal(|| None::<String>);
    let update_id = item.id.clone();
    let token_id = item.id.clone();
    let delete_id = item.id.clone();
    rsx! { tr {
        td { "{item.name}" } td { code { "{item.slug}" } } td { "{item.status}" } td { "{item.health_status}" } td { "{item.capacity_used} / {item.capacity_allocatable}" }
        td { details { summary { "Manage" } div { class: "popover form-grid",
            label { "Region" select { value: region, onchange: move |e| region.set(e.value()), for value in regions { option { value: "{value.id}", "{value.display_name}" } } } }
            label { "Name" input { value: name, oninput: move |e| name.set(e.value()) } } label { "Slug" input { value: slug, oninput: move |e| slug.set(e.value()) } }
            label { "Agent ID" input { value: agent_id, oninput: move |e| agent_id.set(e.value()) } } label { "Agent endpoint" input { value: agent_endpoint, oninput: move |e| agent_endpoint.set(e.value()) } }
            label { "Status" select { value: status, onchange: move |e| status.set(e.value()), for value in ["pending", "bootstrapping", "healthy", "draining", "offline", "removed"] { option { value, "{value}" } } } }
            label { "Health" select { value: health, onchange: move |e| health.set(e.value()), for value in ["healthy", "degraded", "offline"] { option { value, "{value}" } } } }
            label { "Allocatable" input { r#type: "number", value: allocatable, oninput: move |e| allocatable.set(e.value()) } } label { "Used" input { r#type: "number", value: used, oninput: move |e| used.set(e.value()) } }
            ErrorMessage { message } if let Some(value) = token() { p { class: "token", code { "{value}" } } }
            div { class: "actions",
                button { class: "primary", onclick: move |_| { let id = update_id.clone(); async move { match update_cluster(id, region(), name(), slug(), optional(agent_id()), optional(agent_endpoint()), status(), health(), parse_i32(allocatable()), parse_i32(used())).await { Ok(()) => { message.set(None); on_saved.call(()); }, Err(error) => message.set(Some(error.to_string())) } } }, "Save" }
                button { onclick: move |_| { let id = token_id.clone(); async move { match issue_join_credential(id).await { Ok(value) => token.set(Some(value.token)), Err(error) => message.set(Some(error.to_string())) } } }, "New join token" }
                button { class: "danger", onclick: move |_| { let id = delete_id.clone(); async move { match delete_cluster(id).await { Ok(()) => on_saved.call(()), Err(error) => message.set(Some(error.to_string())) } } }, "Delete" }
            }
        } } }
    } }
}

#[component]
pub fn S3Providers() -> Element {
    let mut rows = use_resource(list_s3_providers);
    let rows_state = rows.read().clone();
    rsx! { Shell { title: "S3 Providers",
        CreateS3Provider { on_saved: move |_| rows.restart() }
        match rows_state { Some(Ok(items)) => rsx! { table { thead { tr { th { "Provider" } th { "Endpoint" } th { "Region" } th { "Active" } th { "Actions" } } } tbody { for item in items { S3ProviderRow { key: "{item.id}", item, on_saved: move |_| rows.restart() } } } } }, Some(Err(error)) => rsx! { p { class: "state error", "{error}" } }, None => rsx! { p { class: "state", "Loading…" } } }
    } }
}

#[component]
fn CreateS3Provider(on_saved: EventHandler<()>) -> Element {
    let mut provider_type = use_signal(|| "aws_s3".to_string());
    let mut endpoint = use_signal(String::new);
    let mut region = use_signal(String::new);
    let mut access = use_signal(String::new);
    let mut secret = use_signal(String::new);
    let mut session = use_signal(String::new);
    let mut active = use_signal(|| true);
    let mut message = use_signal(|| None::<String>);
    rsx! { details { class: "create", summary { "+ New S3 provider" } div { class: "form-grid",
        label { "Provider" select { value: provider_type, onchange: move |e| provider_type.set(e.value()), option { value: "aws_s3", "AWS S3" } option { value: "cloudflare_r2", "Cloudflare R2" } } }
        label { "Endpoint URL" input { value: endpoint, oninput: move |e| endpoint.set(e.value()) } } label { "Provider region" input { value: region, oninput: move |e| region.set(e.value()) } }
        label { "Access key ID" input { value: access, oninput: move |e| access.set(e.value()) } } label { "Secret access key" input { r#type: "password", value: secret, oninput: move |e| secret.set(e.value()) } }
        label { "Session token" input { r#type: "password", value: session, oninput: move |e| session.set(e.value()) } } label { class: "checkbox", input { r#type: "checkbox", checked: active, onchange: move |e| active.set(e.checked()) } "Active" }
    } ErrorMessage { message } button { class: "primary", onclick: move |_| async move { match create_s3_provider(provider_type(), endpoint(), region(), access(), secret(), optional(session()), active()).await { Ok(()) => { message.set(None); access.set(String::new()); secret.set(String::new()); session.set(String::new()); on_saved.call(()); }, Err(error) => message.set(Some(error.to_string())) } }, "Create provider" } } }
}

#[component]
fn S3ProviderRow(item: S3Provider, on_saved: EventHandler<()>) -> Element {
    let mut provider_type = use_signal(|| item.provider_type.clone());
    let mut endpoint = use_signal(|| item.endpoint_url.clone());
    let mut region = use_signal(|| item.provider_region.clone().unwrap_or_default());
    let mut access = use_signal(String::new);
    let mut secret = use_signal(String::new);
    let mut session = use_signal(String::new);
    let mut active = use_signal(|| item.is_active);
    let mut message = use_signal(|| None::<String>);
    let update_id = item.id.clone();
    let delete_id = item.id.clone();
    rsx! { tr { td { "{item.provider_type}" } td { "{item.endpoint_url}" } td { {item.provider_region.as_deref().unwrap_or("-")} } td { if item.is_active { "Yes" } else { "No" } }
        td { details { summary { "Edit" } div { class: "popover form-grid",
            label { "Provider" select { value: provider_type, onchange: move |e| provider_type.set(e.value()), option { value: "aws_s3", "AWS S3" } option { value: "cloudflare_r2", "Cloudflare R2" } } }
            label { "Endpoint URL" input { value: endpoint, oninput: move |e| endpoint.set(e.value()) } } label { "Provider region" input { value: region, oninput: move |e| region.set(e.value()) } }
            label { "Replacement access key" input { value: access, oninput: move |e| access.set(e.value()) } } label { "Replacement secret key" input { r#type: "password", value: secret, oninput: move |e| secret.set(e.value()) } }
            label { "Replacement session token" input { r#type: "password", value: session, oninput: move |e| session.set(e.value()) } } label { class: "checkbox", input { r#type: "checkbox", checked: active, onchange: move |e| active.set(e.checked()) } "Active" }
            ErrorMessage { message } div { class: "actions",
                button { class: "primary", onclick: move |_| { let id = update_id.clone(); async move { match update_s3_provider(id, provider_type(), endpoint(), region(), optional(access()), optional(secret()), optional(session()), active()).await { Ok(()) => { message.set(None); access.set(String::new()); secret.set(String::new()); on_saved.call(()); }, Err(error) => message.set(Some(error.to_string())) } } }, "Save" }
                button { class: "danger", onclick: move |_| { let id = delete_id.clone(); async move { match delete_s3_provider(id).await { Ok(()) => on_saved.call(()), Err(error) => message.set(Some(error.to_string())) } } }, "Delete" }
            }
        } } }
    } }
}

#[component]
pub fn Organizations() -> Element {
    let mut rows = use_resource(list_organizations);
    let rows_state = rows.read().clone();
    rsx! { Shell { title: "Organizations", match rows_state { Some(Ok(items)) => rsx! { table { thead { tr { th { "Name" } th { "Email" } th { "Slug" } th { "Members" } th { "Created" } th { "Actions" } } } tbody { for item in items { OrganizationRow { key: "{item.id}", item, on_saved: move |_| rows.restart() } } } } }, Some(Err(error)) => rsx! { p { class: "state error", "{error}" } }, None => rsx! { p { class: "state", "Loading…" } } } } }
}

#[component]
fn OrganizationRow(item: Organization, on_saved: EventHandler<()>) -> Element {
    let mut name = use_signal(|| item.name.clone());
    let mut message = use_signal(|| None::<String>);
    let update_id = item.id.clone();
    let delete_id = item.id.clone();
    rsx! { tr { td { "{item.name}" } td { "{item.email}" } td { code { "{item.slug}" } } td { "{item.member_count}" } td { "{item.created_at}" } td { details { summary { "Edit" } div { class: "popover", label { "Name" input { value: name, oninput: move |e| name.set(e.value()) } } ErrorMessage { message } div { class: "actions",
        button { class: "primary", onclick: move |_| { let id = update_id.clone(); async move { match update_organization(id, name()).await { Ok(()) => on_saved.call(()), Err(error) => message.set(Some(error.to_string())) } } }, "Save" }
        button { class: "danger", onclick: move |_| { let id = delete_id.clone(); async move { match delete_organization(id).await { Ok(()) => on_saved.call(()), Err(error) => message.set(Some(error.to_string())) } } }, "Delete" }
    } } } } } }
}

#[component]
pub fn ApiKeys() -> Element {
    let mut rows = use_resource(list_api_keys);
    let rows_state = rows.read().clone();
    let mut message = use_signal(|| None::<String>);
    rsx! { Shell { title: "API Keys", ErrorMessage { message } match rows_state { Some(Ok(items)) => rsx! { table { thead { tr { th { "Name" } th { "Organization" } th { "Created" } th { "Actions" } } } tbody { for item in items { tr { key: "{item.id}", td { "{item.name}" } td { code { "{item.organization_slug}" } } td { "{item.created_at}" } td { button { class: "danger", onclick: move |_| { let id = item.id.clone(); async move { match delete_api_key(id).await { Ok(()) => rows.restart(), Err(error) => message.set(Some(error.to_string())) } } }, "Delete" } } } } } } }, Some(Err(error)) => rsx! { p { class: "state error", "{error}" } }, None => rsx! { p { class: "state", "Loading…" } } } } }
}

#[component]
pub fn AuditLogs() -> Element {
    let rows = use_resource(list_audit_logs);
    let rows_state = rows.read().clone();
    rsx! { Shell { title: "Audit Logs", match rows_state { Some(Ok(items)) => rsx! { table { thead { tr { th { "Time" } th { "Actor" } th { "IP" } th { "Action" } th { "Resource" } th { "Changes" } } } tbody { for item in items { tr { key: "{item.id}", td { "{item.created_at}" } td { "{item.actor_identifier}" } td { code { "{item.source_ip}" } } td { "{item.action}" } td { "{item.resource_type} " code { {item.resource_id.as_deref().unwrap_or("")} } } td { code { "{item.changes}" } } } } } } }, Some(Err(error)) => rsx! { p { class: "state error", "{error}" } }, None => rsx! { p { class: "state", "Loading…" } } } } }
}

fn optional(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn parse_i32(value: String) -> i32 {
    value.parse().unwrap_or_default()
}

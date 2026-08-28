use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub status: String,
    pub s3_provider_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EligibleRegion {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub status: String,
    pub routing_mode: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cluster {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub region_id: String,
    pub agent_id: Option<String>,
    pub agent_endpoint: Option<String>,
    pub status: String,
    pub health_status: String,
    pub capacity_allocatable: i32,
    pub capacity_used: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct S3Provider {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
    pub provider_region: Option<String>,
    pub is_active: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub email: String,
    pub slug: String,
    pub member_count: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub organization_slug: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub actor_identifier: String,
    pub source_ip: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub changes: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegistryGcStatus {
    pub phase: String,
    pub active_job_id: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub last_result: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JoinCredential {
    pub token: String,
    pub expires_at: String,
}

#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize)]
pub struct S3Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

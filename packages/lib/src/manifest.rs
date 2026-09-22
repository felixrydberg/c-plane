use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use utoipa::ToSchema;
use uuid::Uuid;

pub const MANIFEST_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionManifest {
    pub schema_version: i32,
    pub containers: HashMap<Uuid, ContainerConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ContainerConfig {
    pub name: String,
    pub region_id: Uuid,
    pub image: String,
    pub resolved_image: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_registry_id: Option<Uuid>,
    pub replica_count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    pub public: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_check: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<serde_json::Value>,
}

impl Default for RevisionManifest {
    fn default() -> Self {
        Self {
            schema_version: MANIFEST_SCHEMA_VERSION,
            containers: HashMap::new(),
        }
    }
}

impl RevisionManifest {
    pub fn from_json_value(
        value: &serde_json::Value,
        schema_version: i32,
    ) -> Result<Self, AppError> {
        if schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(AppError::Conflict(
                "Unsupported revision manifest schema version".into(),
            ));
        }
        if value
            .get("schema_version")
            .and_then(serde_json::Value::as_i64)
            != Some(i64::from(schema_version))
        {
            return Err(AppError::Conflict(
                "Revision manifest schema version is missing or inconsistent".into(),
            ));
        }
        serde_json::from_value(value.clone())
            .map_err(|_| AppError::Conflict("Invalid revision manifest configuration".into()))
    }

    pub fn to_json_value(&self) -> Result<serde_json::Value, AppError> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(AppError::Conflict(
                "Unsupported revision manifest schema version".into(),
            ));
        }
        serde_json::to_value(self)
            .map_err(|_| AppError::Internal("Failed to serialize revision manifest".into()))
    }

    pub fn external_registry_ids(&self) -> Vec<Uuid> {
        let ids: BTreeSet<Uuid> = self
            .containers
            .values()
            .filter_map(|config| config.external_registry_id)
            .collect();
        ids.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(registry: Option<Uuid>) -> ContainerConfig {
        ContainerConfig {
            name: "web".into(),
            region_id: Uuid::nil(),
            image: "nginx:1.27".into(),
            resolved_image: "nginx@sha256:abc".into(),
            external_registry_id: registry,
            replica_count: 1,
            port: None,
            public: false,
            cpu: None,
            memory: None,
            health_check: None,
            env: None,
        }
    }

    #[test]
    fn round_trips_and_collects_distinct_registry_ids() {
        let mut manifest = RevisionManifest::default();
        let registry = Uuid::new_v4();
        manifest
            .containers
            .insert(Uuid::new_v4(), config(Some(registry)));
        manifest
            .containers
            .insert(Uuid::new_v4(), config(Some(registry)));
        manifest.containers.insert(Uuid::new_v4(), config(None));

        let restored = RevisionManifest::from_json_value(
            &manifest.to_json_value().unwrap(),
            MANIFEST_SCHEMA_VERSION,
        )
        .unwrap();
        assert_eq!(restored, manifest);
        assert_eq!(restored.external_registry_ids(), vec![registry]);
    }

    #[test]
    fn rejects_invalid_or_unsupported_manifests() {
        assert!(RevisionManifest::from_json_value(&serde_json::json!({}), 1).is_err());
        let mut value = RevisionManifest::default().to_json_value().unwrap();
        assert!(RevisionManifest::from_json_value(&value, 2).is_err());
        value["schema_version"] = serde_json::json!(2);
        assert!(RevisionManifest::from_json_value(&value, 1).is_err());
        value["schema_version"] = serde_json::json!(1);
        value["functions"] = serde_json::json!({});
        assert!(RevisionManifest::from_json_value(&value, 1).is_err());
        value.as_object_mut().unwrap().remove("functions");
        value["containers"] = serde_json::json!({Uuid::nil().to_string(): {"name": "incomplete"}});
        assert!(RevisionManifest::from_json_value(&value, 1).is_err());
        value["containers"] = serde_json::json!({Uuid::nil().to_string(): config(None)});
        value["containers"][Uuid::nil().to_string()]["future_setting"] = serde_json::json!(true);
        assert!(RevisionManifest::from_json_value(&value, 1).is_err());
    }
}

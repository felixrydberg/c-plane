use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimelinePins {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub container: HashMap<Uuid, Uuid>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub secret: HashMap<Uuid, Uuid>,
}

impl TimelinePins {
    pub fn from_json_value(value: &serde_json::Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn set_container(&mut self, container_id: Uuid, version_id: Uuid) {
        self.container.insert(container_id, version_id);
    }

    pub fn remove_container(&mut self, container_id: &Uuid) {
        self.container.remove(container_id);
    }
}

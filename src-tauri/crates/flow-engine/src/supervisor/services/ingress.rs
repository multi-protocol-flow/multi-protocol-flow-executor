//! Ingress gateway background service adapter.
#![allow(missing_docs)]

use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::supervisor::probe::probe_http_health;
use crate::supervisor::store::{list_instances, save_instance};
use crate::supervisor::{ServiceInstance, ServiceKind};

/// Configuration for starting an Ingress background gateway service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressServiceConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub managed_by: String,
    #[serde(default)]
    pub flow_routes: Vec<IngressRouteMapping>,
}

/// Route path to flow UUID mapping for Ingress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRouteMapping {
    pub path: String,
    pub method: String,
    pub flow_uuid: String,
}

/// Probe health of an Ingress gateway service instance
pub async fn probe_ingress_health(host: &str, port: u16) -> bool {
    probe_http_health(host, port, "/healthz", Duration::from_millis(1000))
        .await
        .is_some()
}

/// List all managed Ingress gateway services
pub async fn list_ingress_services(heal: bool) -> Vec<ServiceInstance> {
    list_instances(Some(ServiceKind::Ingress), heal).await
}

/// Register an active Ingress service instance
pub fn register_ingress_instance(
    id: &str,
    name: &str,
    pid: u32,
    host: &str,
    port: u16,
    managed_by: &str,
) -> Result<ServiceInstance, String> {
    let instance = ServiceInstance {
        id: id.to_string(),
        name: name.to_string(),
        kind: ServiceKind::Ingress,
        pid,
        transport: "http".to_string(),
        host: host.to_string(),
        port,
        flow_target: None,
        dir_target: None,
        tools_count: 0,
        started_at: chrono::Utc::now().timestamp_millis() as u64,
        managed_by: managed_by.to_string(),
        status: "running".to_string(),
        metadata: serde_json::Map::new(),
    };

    save_instance(&instance)?;
    Ok(instance)
}

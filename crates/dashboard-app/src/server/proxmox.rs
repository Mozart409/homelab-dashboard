//! Proxmox VE API integration.

use crate::types::{NodeStatus, ProxmoxNode, ProxmoxStatus, ProxmoxVm, VmStatus, VmType};
use chrono::Utc;
use leptos::prelude::*;
use serde::Deserialize;

// ============================================================================
// Proxmox API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct ProxmoxApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct ApiNode {
    node: String,
    status: String,
    cpu: Option<f64>,
    mem: Option<u64>,
    maxmem: Option<u64>,
    uptime: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ApiVm {
    vmid: u32,
    name: Option<String>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    vm_type: Option<String>,
    status: String,
    cpu: Option<f64>,
    mem: Option<u64>,
    maxmem: Option<u64>,
}

// ============================================================================
// Server Functions
// ============================================================================

/// Fetch Proxmox cluster status including nodes and VMs.
#[server]
#[allow(clippy::collapsible_if, clippy::too_many_lines)]
pub async fn get_proxmox_status() -> Result<ProxmoxStatus, ServerFnError> {
    use std::sync::LazyLock;
    use std::time::Duration;
    use moka::future::Cache;

    // Cache for 30 seconds
    static PROXMOX_CACHE: LazyLock<Cache<(), ProxmoxStatus>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(30))
            .max_capacity(1)
            .build()
    });

    if let Some(cached) = PROXMOX_CACHE.get(&()).await {
        tracing::debug!("Proxmox cache hit");
        return Ok(cached);
    }

    // Get config from environment/state
    let base_url = std::env::var("PROXMOX_URL")
        .map_err(|_| ServerFnError::new("PROXMOX_URL not configured"))?;
    let token_id = std::env::var("PROXMOX_TOKEN_ID")
        .map_err(|_| ServerFnError::new("PROXMOX_TOKEN_ID not configured"))?;
    let token_secret = std::env::var("PROXMOX_TOKEN_SECRET")
        .map_err(|_| ServerFnError::new("PROXMOX_TOKEN_SECRET not configured"))?;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // Proxmox often uses self-signed certs
        .build()
        .map_err(|e| ServerFnError::new(format!("Failed to create HTTP client: {e}")))?;

    let auth_header = format!("PVEAPIToken={token_id}={token_secret}");

    // Fetch nodes
    let nodes_url = format!("{base_url}/api2/json/nodes");
    let nodes_response: ProxmoxApiResponse<Vec<ApiNode>> = client
        .get(&nodes_url)
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch Proxmox nodes: {e}")))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse Proxmox nodes: {e}")))?;

    let nodes: Vec<ProxmoxNode> = nodes_response
        .data
        .into_iter()
        .map(|n| ProxmoxNode {
            name: n.node,
            status: match n.status.as_str() {
                "online" => NodeStatus::Online,
                "offline" => NodeStatus::Offline,
                _ => NodeStatus::Unknown,
            },
            cpu_usage: n.cpu.unwrap_or(0.0),
            memory_used: n.mem.unwrap_or(0),
            memory_total: n.maxmem.unwrap_or(0),
            uptime_seconds: n.uptime.unwrap_or(0),
        })
        .collect();

    // Fetch VMs/containers from each node
    let mut vms = Vec::new();
    for node in &nodes {
        // Fetch QEMU VMs
        let qemu_url = format!("{base_url}/api2/json/nodes/{}/qemu", node.name);
        if let Ok(response) = client
            .get(&qemu_url)
            .header("Authorization", &auth_header)
            .send()
            .await
        {
            if let Ok(qemu_response) = response.json::<ProxmoxApiResponse<Vec<ApiVm>>>().await {
                for vm in qemu_response.data {
                    vms.push(ProxmoxVm {
                        vmid: vm.vmid,
                        name: vm.name.unwrap_or_else(|| format!("VM {}", vm.vmid)),
                        node: node.name.clone(),
                        status: parse_vm_status(&vm.status),
                        vm_type: VmType::Qemu,
                        cpu_usage: vm.cpu.unwrap_or(0.0),
                        memory_used: vm.mem.unwrap_or(0),
                        memory_total: vm.maxmem.unwrap_or(0),
                    });
                }
            }
        }

        // Fetch LXC containers
        let lxc_url = format!("{base_url}/api2/json/nodes/{}/lxc", node.name);
        if let Ok(response) = client
            .get(&lxc_url)
            .header("Authorization", &auth_header)
            .send()
            .await
        {
            if let Ok(lxc_response) = response.json::<ProxmoxApiResponse<Vec<ApiVm>>>().await {
                for vm in lxc_response.data {
                    vms.push(ProxmoxVm {
                        vmid: vm.vmid,
                        name: vm.name.unwrap_or_else(|| format!("CT {}", vm.vmid)),
                        node: node.name.clone(),
                        status: parse_vm_status(&vm.status),
                        vm_type: VmType::Lxc,
                        cpu_usage: vm.cpu.unwrap_or(0.0),
                        memory_used: vm.mem.unwrap_or(0),
                        memory_total: vm.maxmem.unwrap_or(0),
                    });
                }
            }
        }
    }

    let status = ProxmoxStatus {
        nodes,
        vms,
        fetched_at: Utc::now(),
    };

    PROXMOX_CACHE.insert((), status.clone()).await;
    Ok(status)
}

fn parse_vm_status(status: &str) -> VmStatus {
    match status {
        "running" => VmStatus::Running,
        "stopped" => VmStatus::Stopped,
        "paused" => VmStatus::Paused,
        "suspended" => VmStatus::Suspended,
        _ => VmStatus::Unknown,
    }
}

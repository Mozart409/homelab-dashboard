//! Service status cards for Proxmox, Jellyfin, and Home Assistant.

use leptos::prelude::*;
use crate::server::{get_proxmox_status, get_jellyfin_status, get_homeassistant_status};
use crate::types::{HomeAssistantStatus, JellyfinItemType, JellyfinStatus, NodeStatus, ProxmoxStatus, VmStatus};

// ============================================================================
// Proxmox Card
// ============================================================================

#[component]
pub fn ProxmoxCard() -> impl IntoView {
    let resource = Resource::new(
        || (),
        |()| async move { get_proxmox_status().await },
    );

    view! {
        <div class="card proxmox-card">
            <div class="card-header">
                <h3 class="card-title">
                    <span class="service-icon">"🖥️"</span>
                    "Proxmox"
                </h3>
                <button
                    class="refresh-btn"
                    on:click=move |_| resource.refetch()
                    title="Refresh"
                >
                    <RefreshIcon/>
                </button>
            </div>

            <Suspense fallback=move || view! { <ServiceSkeleton rows=4/> }>
                {move || {
                    resource.get().map(|result| {
                        match result {
                            Ok(status) => view! { <ProxmoxContent status=status/> }.into_any(),
                            Err(e) => view! { <CardError message=e.to_string()/> }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
#[allow(clippy::cast_possible_truncation)]
fn ProxmoxContent(status: ProxmoxStatus) -> impl IntoView {
    let running_vms = status.vms.iter().filter(|vm| vm.status == VmStatus::Running).count();
    let total_vms = status.vms.len();

    view! {
        <div class="service-content">
            // Nodes overview
            <div class="service-section">
                <h4 class="section-title">"Nodes"</h4>
                <div class="node-grid">
                    <For
                        each=move || status.nodes.clone()
                        key=|node| node.name.clone()
                        children=move |node| {
                            let status_class = match node.status {
                                NodeStatus::Online => "status-online",
                                NodeStatus::Offline => "status-offline",
                                NodeStatus::Unknown => "status-unknown",
                            };
                            let mem_percent: u8 = if node.memory_total > 0 {
                                #[allow(
                                    clippy::cast_possible_truncation,
                                    clippy::cast_sign_loss,
                                    clippy::cast_precision_loss
                                )]
                                {
                                    (node.memory_used as f64 / node.memory_total as f64 * 100.0)
                                        as u8
                                }
                            } else {
                                0
                            };

                            view! {
                                <div class=format!("node-item {}", status_class)>
                                    <span class="node-name">{node.name.clone()}</span>
                                    <div class="node-stats">
                                        <span class="stat">{format!("CPU: {:.0}%", node.cpu_usage * 100.0)}</span>
                                        <span class="stat">{format!("RAM: {mem_percent}%")}</span>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </div>

            // VMs summary
            <div class="service-section">
                <h4 class="section-title">"Virtual Machines"</h4>
                <div class="vm-summary">
                    <div class="vm-stat">
                        <span class="stat-value running">{running_vms}</span>
                        <span class="stat-label">"Running"</span>
                    </div>
                    <div class="vm-stat">
                        <span class="stat-value total">{total_vms}</span>
                        <span class="stat-label">"Total"</span>
                    </div>
                </div>
            </div>

            <div class="service-updated">
                "Updated: " {status.fetched_at.format("%H:%M:%S").to_string()}
            </div>
        </div>
    }
}

// ============================================================================
// Jellyfin Card
// ============================================================================

#[component]
pub fn JellyfinCard() -> impl IntoView {
    let resource = Resource::new(
        || (),
        |()| async move { get_jellyfin_status().await },
    );

    view! {
        <div class="card jellyfin-card">
            <div class="card-header">
                <h3 class="card-title">
                    <span class="service-icon">"🎬"</span>
                    "Jellyfin"
                </h3>
                <button
                    class="refresh-btn"
                    on:click=move |_| resource.refetch()
                    title="Refresh"
                >
                    <RefreshIcon/>
                </button>
            </div>

            <Suspense fallback=move || view! { <ServiceSkeleton rows=3/> }>
                {move || {
                    resource.get().map(|result| {
                        match result {
                            Ok(status) => view! { <JellyfinContent status=status/> }.into_any(),
                            Err(e) => view! { <CardError message=e.to_string()/> }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn JellyfinContent(status: JellyfinStatus) -> impl IntoView {
    let recently_added = status.recently_added.clone();
    let items = recently_added.into_iter().take(3).collect::<Vec<_>>();
    let (items_signal, _) = signal(items);
    
    view! {
        <div class="service-content">
            <div class="jellyfin-header">
                <span class="server-name">{status.server_name.clone()}</span>
                <span class="server-version">"v"{status.version.clone()}</span>
            </div>

            <div class="jellyfin-stats">
                <div class="stat-item">
                    <span class="stat-icon">"📺"</span>
                    <span class="stat-value">{status.active_streams}</span>
                    <span class="stat-label">"Streaming"</span>
                </div>
                <div class="stat-item">
                    <span class="stat-icon">"🎥"</span>
                    <span class="stat-value">{status.total_movies}</span>
                    <span class="stat-label">"Movies"</span>
                </div>
                <div class="stat-item">
                    <span class="stat-icon">"📺"</span>
                    <span class="stat-value">{status.total_series}</span>
                    <span class="stat-label">"Series"</span>
                </div>
            </div>

            {move || {
                let items = items_signal.get();
                if items.is_empty() {
                    ().into_any()
                } else {
                    view! {
                        <div class="service-section">
                            <h4 class="section-title">"Recently Added"</h4>
                            <ul class="recent-items">
                                {items.into_iter().map(|item| {
                                let display_name = match (&item.item_type, &item.series_name) {
                                    (JellyfinItemType::Episode, Some(series)) => {
                                        format!("{} - {}", series, item.name)
                                    }
                                    _ => item.name.clone(),
                                };
                                let display_name_clone = display_name.clone();

                                view! {
                                    <li class="recent-item">
                                        <span class="item-type-icon">
                                            {match item.item_type {
                                                JellyfinItemType::Movie => "🎬",
                                                JellyfinItemType::Episode => "📺",
                                                _ => "📁",
                                            }}
                                        </span>
                                        <span class="item-name" title={display_name}>
                                            {display_name_clone}
                                        </span>
                                    </li>
                                }
                                }).collect_view()}
                            </ul>
                        </div>
                    }.into_any()
                }
            }}

            <div class="service-updated">
                "Updated: " {status.fetched_at.format("%H:%M:%S").to_string()}
            </div>
        </div>
    }
}

// ============================================================================
// Home Assistant Card
// ============================================================================

/// Props for `HomeAssistantCard`
#[derive(Clone)]
pub struct HomeAssistantConfig {
    /// Entity IDs to display (empty = show all)
    pub entity_ids: Vec<String>,
}

#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn HomeAssistantCard(config: HomeAssistantConfig) -> impl IntoView {
    let entity_ids = config.entity_ids.clone();
    
    let resource = Resource::new(
        move || entity_ids.clone(),
        |ids| async move { get_homeassistant_status(ids).await },
    );

    view! {
        <div class="card homeassistant-card">
            <div class="card-header">
                <h3 class="card-title">
                    <span class="service-icon">"🏠"</span>
                    "Home Assistant"
                </h3>
                <button
                    class="refresh-btn"
                    on:click=move |_| resource.refetch()
                    title="Refresh"
                >
                    <RefreshIcon/>
                </button>
            </div>

            <Suspense fallback=move || view! { <ServiceSkeleton rows=4/> }>
                {move || {
                    resource.get().map(|result| {
                        match result {
                            Ok(status) => view! { <HomeAssistantContent status=status/> }.into_any(),
                            Err(e) => view! { <CardError message=e.to_string()/> }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn HomeAssistantContent(status: HomeAssistantStatus) -> impl IntoView {
    view! {
        <div class="service-content">
            <div class="ha-version">
                "Home Assistant " {status.version.clone()}
            </div>

            <div class="entity-grid">
                <For
                    each=move || status.entities.clone()
                    key=|entity| entity.entity_id.clone()
                    children=move |entity| {
                        let state_class = get_entity_state_class(&entity.state);
                        let display_value = match &entity.unit {
                            Some(unit) => format!("{} {}", entity.state, unit),
                            None => entity.state.clone(),
                        };

                        view! {
                            <div class=format!("entity-item {}", state_class)>
                                <span class="entity-icon">
                                    {entity.icon.clone().unwrap_or_else(|| "📊".to_string())}
                                </span>
                                <div class="entity-info">
                                    <span class="entity-name">{entity.friendly_name.clone()}</span>
                                    <span class="entity-state">{display_value}</span>
                                </div>
                            </div>
                        }
                    }
                />
            </div>

            <div class="service-updated">
                "Updated: " {status.fetched_at.format("%H:%M:%S").to_string()}
            </div>
        </div>
    }
}

fn get_entity_state_class(state: &str) -> &'static str {
    match state.to_lowercase().as_str() {
        "on" | "home" | "open" | "playing" => "state-active",
        "off" | "away" | "closed" | "idle" => "state-inactive",
        "unavailable" | "unknown" => "state-unavailable",
        _ => "state-neutral",
    }
}

// ============================================================================
// Shared Components
// ============================================================================

#[component]
fn RefreshIcon() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
            <path d="M3 3v5h5"/>
            <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/>
            <path d="M16 21h5v-5"/>
        </svg>
    }
}

#[component]
fn CardError(message: String) -> impl IntoView {
    view! {
        <div class="card-error">
            <span class="error-icon">"⚠️"</span>
            <span class="error-message">{message}</span>
        </div>
    }
}

#[component]
fn ServiceSkeleton(rows: usize) -> impl IntoView {
    view! {
        <div class="service-content skeleton">
            {(0..rows).map(|_| view! {
                <div class="skeleton-row"/>
            }).collect_view()}
        </div>
    }
}

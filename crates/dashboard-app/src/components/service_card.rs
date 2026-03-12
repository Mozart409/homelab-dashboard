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
        <div class="bg-bg-card border border-border rounded-xl overflow-hidden">
            <div class="flex items-center justify-between p-4 border-b border-border-subtle bg-bg-secondary">
                <h3 class="font-mono text-sm font-semibold text-text-primary flex items-center gap-2">
                    <span class="text-lg">"🖥️"</span>
                    "Proxmox"
                </h3>
                <button
                    class="flex items-center justify-center w-7 h-7 bg-transparent border border-border rounded text-text-muted cursor-pointer transition-all duration-150 hover:text-text-primary hover:border-text-muted"
                    on:click=move |_| resource.refetch()
                    title="Refresh"
                >
                    <RefreshIcon/>
                </button>
            </div>

            <Suspense fallback=move || view! { <ServiceSkeleton rows=4/> }>
                {move || Suspend::new(async move {
                    match resource.await {
                        Ok(status) => view! { <ProxmoxContent status=status/> }.into_any(),
                        Err(e) => view! { <CardError message=e.to_string()/> }.into_any(),
                    }
                })}
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
        <div class="p-4">
            // Nodes overview
            <div class="mb-4">
                <h4 class="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">"Nodes"</h4>
                <div class="flex flex-col gap-2">
                    <For
                        each=move || status.nodes.clone()
                        key=|node| node.name.clone()
                        children=move |node| {
                            let border_color = match node.status {
                                NodeStatus::Online => "border-l-accent-green",
                                NodeStatus::Offline => "border-l-accent-red",
                                NodeStatus::Unknown => "border-l-text-muted",
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
                                <div class=format!("flex justify-between items-center p-2 bg-bg-elevated rounded border-l-[3px] {}", border_color)>
                                    <span class="font-mono text-sm font-medium">{node.name.clone()}</span>
                                    <div class="flex gap-4">
                                        <span class="font-mono text-xs text-text-secondary">{format!("CPU: {:.0}%", node.cpu_usage * 100.0)}</span>
                                        <span class="font-mono text-xs text-text-secondary">{format!("RAM: {mem_percent}%")}</span>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </div>

            // VMs summary
            <div class="mb-4">
                <h4 class="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">"Virtual Machines"</h4>
                <div class="flex gap-6">
                    <div class="flex flex-col items-center">
                        <span class="font-mono text-2xl font-semibold text-accent-green">{running_vms}</span>
                        <span class="text-xs text-text-muted">"Running"</span>
                    </div>
                    <div class="flex flex-col items-center">
                        <span class="font-mono text-2xl font-semibold text-text-secondary">{total_vms}</span>
                        <span class="text-xs text-text-muted">"Total"</span>
                    </div>
                </div>
            </div>

            <div class="text-[0.7rem] text-text-muted pt-2 border-t border-border-subtle">
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
        <div class="bg-bg-card border border-border rounded-xl overflow-hidden">
            <div class="flex items-center justify-between p-4 border-b border-border-subtle bg-bg-secondary">
                <h3 class="font-mono text-sm font-semibold text-text-primary flex items-center gap-2">
                    <span class="text-lg">"🎬"</span>
                    "Jellyfin"
                </h3>
                <button
                    class="flex items-center justify-center w-7 h-7 bg-transparent border border-border rounded text-text-muted cursor-pointer transition-all duration-150 hover:text-text-primary hover:border-text-muted"
                    on:click=move |_| resource.refetch()
                    title="Refresh"
                >
                    <RefreshIcon/>
                </button>
            </div>

            <Suspense fallback=move || view! { <ServiceSkeleton rows=3/> }>
                {move || Suspend::new(async move {
                    match resource.await {
                        Ok(status) => view! { <JellyfinContent status=status/> }.into_any(),
                        Err(e) => view! { <CardError message=e.to_string()/> }.into_any(),
                    }
                })}
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
        <div class="p-4">
            <div class="flex justify-between items-center mb-4">
                <span class="font-medium">{status.server_name.clone()}</span>
                <span class="font-mono text-xs text-text-muted">"v"{status.version.clone()}</span>
            </div>

            <div class="flex gap-6 mb-4">
                <div class="flex flex-col items-center gap-1">
                    <span class="text-xl">"📺"</span>
                    <span class="font-mono text-xl font-semibold">{status.active_streams}</span>
                    <span class="text-[0.7rem] text-text-muted">"Streaming"</span>
                </div>
                <div class="flex flex-col items-center gap-1">
                    <span class="text-xl">"🎥"</span>
                    <span class="font-mono text-xl font-semibold">{status.total_movies}</span>
                    <span class="text-[0.7rem] text-text-muted">"Movies"</span>
                </div>
                <div class="flex flex-col items-center gap-1">
                    <span class="text-xl">"📺"</span>
                    <span class="font-mono text-xl font-semibold">{status.total_series}</span>
                    <span class="text-[0.7rem] text-text-muted">"Series"</span>
                </div>
            </div>

            <Show when=move || !items_signal.with_untracked(|items| items.is_empty())>
                <div class="mb-4">
                    <h4 class="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">"Recently Added"</h4>
                    <ul class="flex flex-col gap-1">
                        <For
                            each=move || items_signal.get()
                            key=|item| item.id.clone()
                            children=move |item| {
                                let display_name = match (&item.item_type, &item.series_name) {
                                    (JellyfinItemType::Episode, Some(series)) => {
                                        format!("{} - {}", series, item.name)
                                    }
                                    _ => item.name.clone(),
                                };
                                let display_name_clone = display_name.clone();

                                view! {
                                    <li class="flex items-center gap-2 py-1">
                                        <span class="shrink-0">
                                            {match item.item_type {
                                                JellyfinItemType::Movie => "🎬",
                                                JellyfinItemType::Episode => "📺",
                                                _ => "📁",
                                            }}
                                        </span>
                                        <span class="text-xs text-text-secondary truncate" title={display_name}>
                                            {display_name_clone}
                                        </span>
                                    </li>
                                }
                            }
                        />
                    </ul>
                </div>
            </Show>

            <div class="text-[0.7rem] text-text-muted pt-2 border-t border-border-subtle">
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
        <div class="bg-bg-card border border-border rounded-xl overflow-hidden">
            <div class="flex items-center justify-between p-4 border-b border-border-subtle bg-bg-secondary">
                <h3 class="font-mono text-sm font-semibold text-text-primary flex items-center gap-2">
                    <span class="text-lg">"🏠"</span>
                    "Home Assistant"
                </h3>
                <button
                    class="flex items-center justify-center w-7 h-7 bg-transparent border border-border rounded text-text-muted cursor-pointer transition-all duration-150 hover:text-text-primary hover:border-text-muted"
                    on:click=move |_| resource.refetch()
                    title="Refresh"
                >
                    <RefreshIcon/>
                </button>
            </div>

            <Suspense fallback=move || view! { <ServiceSkeleton rows=4/> }>
                {move || Suspend::new(async move {
                    match resource.await {
                        Ok(status) => view! { <HomeAssistantContent status=status/> }.into_any(),
                        Err(e) => view! { <CardError message=e.to_string()/> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn HomeAssistantContent(status: HomeAssistantStatus) -> impl IntoView {
    view! {
        <div class="p-4">
            <div class="text-xs text-text-muted mb-4">
                "Home Assistant " {status.version.clone()}
            </div>

            <div class="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2">
                <For
                    each=move || status.entities.clone()
                    key=|entity| entity.entity_id.clone()
                    children=move |entity| {
                        let border_color = match entity.state.to_lowercase().as_str() {
                            "on" | "home" | "open" | "playing" => "border-l-accent-green",
                            "off" | "away" | "closed" | "idle" => "border-l-text-muted",
                            "unavailable" | "unknown" => "border-l-accent-red opacity-60",
                            _ => "border-l-text-muted",
                        };
                        let display_value = match &entity.unit {
                            Some(unit) => format!("{} {}", entity.state, unit),
                            None => entity.state.clone(),
                        };

                        view! {
                            <div class=format!("flex items-center gap-2 p-2 bg-bg-elevated rounded border-l-[3px] {}", border_color)>
                                <span class="text-xl">
                                    {entity.icon.clone().unwrap_or_else(|| "📊".to_string())}
                                </span>
                                <div class="flex-1 min-w-0">
                                    <span class="block text-xs text-text-muted truncate">{entity.friendly_name.clone()}</span>
                                    <span class="block font-mono text-sm font-medium">{display_value}</span>
                                </div>
                            </div>
                        }
                    }
                />
            </div>

            <div class="text-[0.7rem] text-text-muted pt-2 mt-4 border-t border-border-subtle">
                "Updated: " {status.fetched_at.format("%H:%M:%S").to_string()}
            </div>
        </div>
    }
}

// ============================================================================
// Shared Components
// ============================================================================

#[component]
fn RefreshIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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
        <div class="flex items-center gap-2 p-4 text-accent-red text-sm">
            <span>"⚠️"</span>
            <span>{message}</span>
        </div>
    }
}

#[component]
fn ServiceSkeleton(rows: usize) -> impl IntoView {
    view! {
        <div class="p-4 animate-pulse">
            {(0..rows).map(|_| view! {
                <div class="w-full h-8 bg-bg-elevated rounded mb-2"/>
            }).collect_view()}
        </div>
    }
}

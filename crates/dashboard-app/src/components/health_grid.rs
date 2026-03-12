//! Health check status grid component.

use leptos::prelude::*;
use crate::server::get_health_overview;
use crate::types::{HealthCheck, HealthOverview, HealthStatus};

/// Displays a grid of health check statuses.
#[component]
pub fn HealthGrid() -> impl IntoView {
    let resource = Resource::new(
        || (),
        |()| async move { get_health_overview().await },
    );

    view! {
        <div class="bg-bg-card border border-border rounded-xl overflow-hidden">
            <div class="flex items-center justify-between p-4 border-b border-border-subtle bg-bg-secondary">
                <h3 class="font-mono text-sm font-semibold text-text-primary flex items-center gap-2">
                    <span class="text-lg">"🩺"</span>
                    "Service Health"
                </h3>
                <button
                    class="flex items-center justify-center w-7 h-7 bg-transparent border border-border rounded text-text-muted cursor-pointer transition-all duration-150 hover:text-text-primary hover:border-text-muted"
                    on:click=move |_| resource.refetch()
                    title="Refresh"
                >
                    <RefreshIcon/>
                </button>
            </div>

            <Suspense fallback=move || view! { <HealthSkeleton/> }>
                {move || Suspend::new(async move {
                    match resource.await {
                        Ok(overview) => view! { <HealthContent overview=overview/> }.into_any(),
                        Err(e) => view! {
                            <div class="flex items-center gap-2 p-4 text-accent-red text-sm">
                                <span>"⚠️"</span>
                                <span>{e.to_string()}</span>
                            </div>
                        }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn HealthContent(overview: HealthOverview) -> impl IntoView {
    let all_healthy = overview.unhealthy_count == 0;

    view! {
        <div class="p-4">
            // Summary bar
            <div class=move || format!(
                "flex items-center gap-2 px-4 py-2 rounded mb-4 text-sm font-medium {}",
                if all_healthy { "bg-accent-green/10 text-accent-green" } else { "bg-accent-red/10 text-accent-red" }
            )>
                <Show
                    when=move || all_healthy
                    fallback=move || view! {
                        <span>"⚠️"</span>
                        <span>{overview.unhealthy_count} " service(s) unhealthy"</span>
                    }
                >
                    <span>"✅"</span>
                    <span>"All systems operational"</span>
                </Show>
            </div>

            // Health check grid
            <div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2">
                <For
                    each=move || overview.checks.clone()
                    key=|check| check.id
                    children=move |check| {
                        view! { <HealthCheckItem check=check/> }
                    }
                />
            </div>

            <div class="text-[0.7rem] text-text-muted pt-2 mt-4 border-t border-border-subtle">
                "Checked: " {overview.fetched_at.format("%H:%M:%S").to_string()}
            </div>
        </div>
    }
}

#[component]
fn HealthCheckItem(check: HealthCheck) -> impl IntoView {
    let (border_color, status_icon) = match check.status {
        HealthStatus::Healthy => ("border-l-accent-green", "🟢"),
        HealthStatus::Degraded => ("border-l-accent-yellow", "🟡"),
        HealthStatus::Unhealthy => ("border-l-accent-red", "🔴"),
        HealthStatus::Unknown => ("border-l-text-muted", "⚪"),
    };

    let response_time = check
        .response_time_ms
        .map_or_else(|| "—".to_string(), |ms| format!("{ms}ms"));

    view! {
        <div class=format!("flex flex-col gap-1 px-4 py-2 bg-bg-elevated rounded border-l-[3px] {}", border_color)>
            <div class="flex items-center gap-2">
                <span class="text-xs">{status_icon}</span>
                <span class="text-sm font-medium">{check.name.clone()}</span>
            </div>
            <div class="flex items-center gap-2">
                <span class="font-mono text-xs text-text-muted">{response_time}</span>
                {check.error_message.as_ref().map(|msg| {
                    view! {
                        <span class="cursor-help" title={msg.clone()}>
                            "ℹ️"
                        </span>
                    }
                })}
            </div>
        </div>
    }
}

#[component]
fn HealthSkeleton() -> impl IntoView {
    view! {
        <div class="p-4 animate-pulse">
            <div class="w-full h-9 bg-bg-elevated rounded mb-4"/>
            <div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2">
                {(0..6).map(|_| view! {
                    <div class="h-[60px] bg-bg-elevated rounded"/>
                }).collect_view()}
            </div>
        </div>
    }
}

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

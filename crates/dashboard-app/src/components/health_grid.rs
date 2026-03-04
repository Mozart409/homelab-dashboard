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
        <div class="card health-card">
            <div class="card-header">
                <h3 class="card-title">
                    <span class="service-icon">"🩺"</span>
                    "Service Health"
                </h3>
                <button
                    class="refresh-btn"
                    on:click=move |_| resource.refetch()
                    title="Refresh"
                >
                    <RefreshIcon/>
                </button>
            </div>

            <Suspense fallback=move || view! { <HealthSkeleton/> }>
                {move || {
                    resource.get().map(|result| {
                        match result {
                            Ok(overview) => view! { <HealthContent overview=overview/> }.into_any(),
                            Err(e) => view! {
                                <div class="card-error">
                                    <span class="error-icon">"⚠️"</span>
                                    <span>{e.to_string()}</span>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn HealthContent(overview: HealthOverview) -> impl IntoView {
    let all_healthy = overview.unhealthy_count == 0;

    view! {
        <div class="health-content">
            // Summary bar
            <div class=move || format!("health-summary {}", if all_healthy { "all-healthy" } else { "has-issues" })>
                <Show
                    when=move || all_healthy
                    fallback=move || view! {
                        <span class="summary-icon">"⚠️"</span>
                        <span class="summary-text">
                            {overview.unhealthy_count} " service(s) unhealthy"
                        </span>
                    }
                >
                    <span class="summary-icon">"✅"</span>
                    <span class="summary-text">"All systems operational"</span>
                </Show>
            </div>

            // Health check grid
            <div class="health-grid">
                <For
                    each=move || overview.checks.clone()
                    key=|check| check.id
                    children=move |check| {
                        view! { <HealthCheckItem check=check/> }
                    }
                />
            </div>

            <div class="service-updated">
                "Checked: " {overview.fetched_at.format("%H:%M:%S").to_string()}
            </div>
        </div>
    }
}

#[component]
fn HealthCheckItem(check: HealthCheck) -> impl IntoView {
    let (status_class, status_icon) = match check.status {
        HealthStatus::Healthy => ("status-healthy", "🟢"),
        HealthStatus::Degraded => ("status-degraded", "🟡"),
        HealthStatus::Unhealthy => ("status-unhealthy", "🔴"),
        HealthStatus::Unknown => ("status-unknown", "⚪"),
    };

    let response_time = check
        .response_time_ms
        .map_or_else(|| "—".to_string(), |ms| format!("{ms}ms"));

    view! {
        <div class=format!("health-item {}", status_class)>
            <div class="health-item-header">
                <span class="status-indicator">{status_icon}</span>
                <span class="service-name">{check.name.clone()}</span>
            </div>
            <div class="health-item-details">
                <span class="response-time">{response_time}</span>
                {move || {
                    let error_msg = check.error_message.clone();
                    if error_msg.is_some() {
                        view! {
                            <span class="error-hint" title={error_msg.unwrap_or_default()}>
                                "ℹ️"
                            </span>
                        }.into_any()
                    } else {
                        ().into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn HealthSkeleton() -> impl IntoView {
    view! {
        <div class="health-content skeleton">
            <div class="skeleton-summary"/>
            <div class="health-grid">
                {(0..6).map(|_| view! {
                    <div class="skeleton-health-item"/>
                }).collect_view()}
            </div>
        </div>
    }
}

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

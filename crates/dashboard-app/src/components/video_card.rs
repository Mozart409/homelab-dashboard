//! Pinchflat recent videos card component.

use leptos::prelude::*;
use crate::server::get_pinchflat_status;
use crate::types::{PinchflatStatus, PinchflatVideo};

/// Displays recent downloads from Pinchflat.
#[component]
pub fn VideoCard() -> impl IntoView {
    let videos_resource = Resource::new(
        || (),
        |()| async move { get_pinchflat_status(Some(3)).await },
    );

    view! {
        <div class="card video-card">
            <div class="card-header">
                <h3 class="card-title">"Recent Downloads"</h3>
                <button
                    class="refresh-btn"
                    on:click=move |_| videos_resource.refetch()
                    title="Refresh"
                >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
                        <path d="M3 3v5h5"/>
                        <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/>
                        <path d="M16 21h5v-5"/>
                    </svg>
                </button>
            </div>

            <Suspense fallback=move || view! { <VideoSkeleton/> }>
                {move || {
                    videos_resource.get().map(|result| {
                        match result {
                            Ok(status) => view! { <VideoContent status=status/> }.into_any(),
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
fn VideoContent(status: PinchflatStatus) -> impl IntoView {
    view! {
        <div class="video-content">
            <Show when=move || status.is_downloading>
                <div class="download-indicator">
                    <span class="download-spinner"/>
                    <span>"Downloading..."</span>
                </div>
            </Show>

            <ul class="video-list">
                <For
                    each=move || status.videos.clone()
                    key=|video| video.id
                    children=move |video| {
                        view! { <VideoItem video=video/> }
                    }
                />
            </ul>

            <div class="video-stats">
                <span class="stat-label">"Total downloads:"</span>
                <span class="stat-value">{status.total_downloads}</span>
            </div>
        </div>
    }
}

#[component]
fn VideoItem(video: PinchflatVideo) -> impl IntoView {
    let duration = format_duration(video.duration_seconds);
    let downloaded = video.downloaded_at.format("%b %d, %H:%M").to_string();

    view! {
        <li class="video-item">
            <div class="video-thumbnail">
                {match &video.thumbnail_url {
                    Some(url) => view! {
                        <img src={url.clone()} alt="" loading="lazy"/>
                    }.into_any(),
                    None => view! {
                        <div class="thumbnail-placeholder">
                            <svg viewBox="0 0 24 24" fill="currentColor">
                                <path d="M8 5v14l11-7z"/>
                            </svg>
                        </div>
                    }.into_any(),
                }}
                <span class="video-duration">{duration}</span>
            </div>
            <div class="video-info">
                <div class="video-title" title={video.title.clone()}>
                    {video.title.clone()}
                </div>
                <div class="video-channel">{video.channel.clone()}</div>
                <div class="video-downloaded">{downloaded}</div>
            </div>
        </li>
    }
}

#[component]
fn VideoSkeleton() -> impl IntoView {
    view! {
        <div class="video-content skeleton">
            <ul class="video-list">
                {(0..3).map(|_| view! {
                    <li class="video-item skeleton">
                        <div class="skeleton-thumbnail"/>
                        <div class="video-info">
                            <div class="skeleton-title"/>
                            <div class="skeleton-channel"/>
                            <div class="skeleton-date"/>
                        </div>
                    </li>
                }).collect_view()}
            </ul>
        </div>
    }
}

fn format_duration(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

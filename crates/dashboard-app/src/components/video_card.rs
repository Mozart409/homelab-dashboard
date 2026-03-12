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
        <div class="bg-bg-card border border-border rounded-xl overflow-hidden">
            <div class="flex items-center justify-between p-4 border-b border-border-subtle bg-bg-secondary">
                <h3 class="font-mono text-sm font-semibold text-text-primary">"Recent Downloads"</h3>
                <button
                    class="flex items-center justify-center w-7 h-7 bg-transparent border border-border rounded text-text-muted cursor-pointer transition-all duration-150 hover:text-text-primary hover:border-text-muted"
                    on:click=move |_| videos_resource.refetch()
                    title="Refresh"
                >
                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
                        <path d="M3 3v5h5"/>
                        <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/>
                        <path d="M16 21h5v-5"/>
                    </svg>
                </button>
            </div>

            <Suspense fallback=move || view! { <VideoSkeleton/> }>
                {move || Suspend::new(async move {
                    match videos_resource.await {
                        Ok(status) => view! { <VideoContent status=status/> }.into_any(),
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
fn VideoContent(status: PinchflatStatus) -> impl IntoView {
    view! {
        <div class="p-4">
            <Show when=move || status.is_downloading>
                <div class="flex items-center gap-2 px-4 py-2 mb-4 bg-accent-blue/10 rounded text-accent-blue text-sm">
                    <span class="w-3.5 h-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"/>
                    <span>"Downloading..."</span>
                </div>
            </Show>

            <ul class="flex flex-col gap-4">
                <For
                    each=move || status.videos.clone()
                    key=|video| video.id
                    children=move |video| {
                        view! { <VideoItem video=video/> }
                    }
                />
            </ul>

            <div class="flex justify-between pt-4 border-t border-border-subtle text-xs">
                <span class="text-text-muted">"Total downloads:"</span>
                <span class="text-text-secondary font-mono">{status.total_downloads}</span>
            </div>
        </div>
    }
}

#[component]
fn VideoItem(video: PinchflatVideo) -> impl IntoView {
    let duration = format_duration(video.duration_seconds);
    let downloaded = video.downloaded_at.format("%b %d, %H:%M").to_string();

    view! {
        <li class="flex gap-4">
            <div class="relative shrink-0 w-[120px] h-[68px] bg-bg-elevated rounded overflow-hidden">
                {match &video.thumbnail_url {
                    Some(url) => view! {
                        <img class="w-full h-full object-cover" src={url.clone()} alt="" loading="lazy"/>
                    }.into_any(),
                    None => view! {
                        <div class="flex items-center justify-center w-full h-full text-text-muted">
                            <svg class="w-6 h-6" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M8 5v14l11-7z"/>
                            </svg>
                        </div>
                    }.into_any(),
                }}
                <span class="absolute bottom-1 right-1 px-1 py-0.5 bg-black/80 rounded-sm font-mono text-[0.7rem] text-white">{duration}</span>
            </div>
            <div class="flex-1 min-w-0">
                <div class="text-sm font-medium text-text-primary truncate mb-1" title={video.title.clone()}>
                    {video.title.clone()}
                </div>
                <div class="text-xs text-text-secondary mb-1">{video.channel.clone()}</div>
                <div class="text-[0.7rem] text-text-muted font-mono">{downloaded}</div>
            </div>
        </li>
    }
}

#[component]
fn VideoSkeleton() -> impl IntoView {
    view! {
        <div class="p-4 animate-pulse">
            <ul class="flex flex-col gap-4">
                {(0..3).map(|_| view! {
                    <li class="flex gap-4">
                        <div class="w-[120px] h-[68px] bg-bg-elevated rounded"/>
                        <div class="flex-1">
                            <div class="w-4/5 h-3.5 bg-bg-elevated rounded mb-1"/>
                            <div class="w-3/5 h-3 bg-bg-elevated rounded mb-1"/>
                            <div class="w-2/5 h-2.5 bg-bg-elevated rounded"/>
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

//! `SearXNG` search box component.
//!
//! This component performs searches directly from the browser to your
//! self-hosted `SearXNG` instance, avoiding the need for a backend proxy.

use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::types::{SearchResponse, SearchResult};

/// A search box that queries `SearXNG` directly from the browser.
#[component]
#[allow(clippy::too_many_lines)]
pub fn SearchBox(searxng_url: Signal<String>) -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let (results, set_results) = signal(Vec::<SearchResult>::new());
    let (is_searching, set_is_searching) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (show_results, set_show_results) = signal(false);

    // Perform search when user submits
    let on_search = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        
        let q = query.get_untracked();
        if q.trim().is_empty() {
            return;
        }

        let url = searxng_url.get_untracked();
        set_is_searching.set(true);
        set_error.set(None);
        set_show_results.set(true);

        spawn_local(async move {
            match fetch_search_results(&url, &q).await {
                Ok(response) => {
                    set_results.set(response.results);
                }
                Err(e) => {
                    set_error.set(Some(e));
                }
            }
            set_is_searching.set(false);
        });
    };

    // Close results when clicking outside
    let on_blur = move |_| {
        // Delay to allow click on results
        set_timeout(
            move || {
                if !is_searching.get_untracked() {
                    set_show_results.set(false);
                }
            },
            std::time::Duration::from_millis(200),
        );
    };

    view! {
        <div class="relative">
            <form on:submit=on_search class="w-full">
                <div class="relative flex items-center">
                    <svg class="absolute left-4 w-[18px] h-[18px] text-text-muted pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="11" cy="11" r="8"/>
                        <path d="m21 21-4.35-4.35"/>
                    </svg>
                    <input
                        type="text"
                        class="w-full py-2 px-4 pl-[calc(1rem+24px)] bg-bg-secondary border border-border rounded-lg text-text-primary font-mono text-sm transition-all duration-150 placeholder:text-text-muted focus:outline-none focus:border-accent-blue focus:ring-[3px] focus:ring-accent-blue/15"
                        placeholder="Search the web..."
                        prop:value=query
                        on:input=move |ev| set_query.set(event_target_value(&ev))
                        on:focus=move |_| {
                            if !results.get_untracked().is_empty() {
                                set_show_results.set(true);
                            }
                        }
                        on:blur=on_blur
                    />
                    <Show when=move || is_searching.with_untracked(|s| *s)>
                        <div class="absolute right-4 w-4 h-4 border-2 border-border border-t-accent-blue rounded-full animate-spin"/>
                    </Show>
                </div>
            </form>

            <Show when=move || show_results.with_untracked(|s| *s)>
                <div class="absolute top-[calc(100%+0.25rem)] left-0 right-0 bg-bg-card border border-border rounded-lg shadow-lg max-h-[400px] overflow-y-auto z-[100]">
                    <Show
                        when=move || error.with_untracked(|e| e.is_none())
                        fallback=move || view! {
                            <div class="p-6 text-center text-accent-red">
                                {move || error.get().unwrap_or_default()}
                            </div>
                        }
                    >
                        <Show
                            when=move || results.with_untracked(|r| !r.is_empty())
                            fallback=move || view! {
                                <Show when=move || is_searching.with_untracked(|s| !s)>
                                    <div class="p-6 text-center text-text-muted">"No results found"</div>
                                </Show>
                            }
                        >
                            <ul class="list-none">
                                <For
                                    each=move || results.get()
                                    key=|result| result.url.clone()
                                    children=move |result| {
                                        let content = result.content.clone();
                                        view! {
                                            <li class="border-b border-border-subtle last:border-b-0">
                                                <a href={result.url.clone()} target="_blank" rel="noopener" class="block p-4 no-underline transition-colors duration-150 hover:bg-bg-elevated">
                                                    <div class="text-accent-blue font-medium mb-1">{result.title.clone()}</div>
                                                    <div class="text-text-muted text-xs font-mono mb-1 truncate">{result.url.clone()}</div>
                                                    {move || {
                                                        if let Some(ref c) = content {
                                                            view! {
                                                                <div class="text-text-secondary text-sm leading-snug line-clamp-2">{c.clone()}</div>
                                                            }.into_any()
                                                        } else {
                                                            ().into_any()
                                                        }
                                                    }}
                                                </a>
                                            </li>
                                        }
                                    }
                                />
                            </ul>
                        </Show>
                    </Show>
                </div>
            </Show>
        </div>
    }
}

/// Fetch search results from SearXNG.
/// This runs in the browser via WASM.
#[cfg(feature = "hydrate")]
async fn fetch_search_results(base_url: &str, query: &str) -> Result<SearchResponse, String> {
    use gloo_net::http::Request;

    let url = format!("{}/search?q={}&format=json", base_url, urlencoding::encode(query));

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.ok() {
        return Err(format!("Search failed: HTTP {}", response.status()));
    }

    response
        .json::<SearchResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

#[cfg(not(feature = "hydrate"))]
#[allow(clippy::unused_async)]
async fn fetch_search_results(_base_url: &str, _query: &str) -> Result<SearchResponse, String> {
    Err("Search only available in hydrate mode".to_string())
}

// Helper for setTimeout
fn set_timeout<F>(f: F, duration: std::time::Duration)
where
    F: FnOnce() + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        
        let closure = Closure::once(f);
        let window = web_sys::window().unwrap();
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                duration.as_millis() as i32,
            )
            .unwrap();
        closure.forget();
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (f, duration);
    }
}

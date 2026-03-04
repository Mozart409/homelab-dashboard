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
        
        let q = query.get();
        if q.trim().is_empty() {
            return;
        }

        let url = searxng_url.get();
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
                if !is_searching.get() {
                    set_show_results.set(false);
                }
            },
            std::time::Duration::from_millis(200),
        );
    };

    view! {
        <div class="search-container">
            <form on:submit=on_search class="search-form">
                <div class="search-input-wrapper">
                    <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="11" cy="11" r="8"/>
                        <path d="m21 21-4.35-4.35"/>
                    </svg>
                    <input
                        type="text"
                        class="search-input"
                        placeholder="Search the web..."
                        prop:value=query
                        on:input=move |ev| set_query.set(event_target_value(&ev))
                        on:focus=move |_| {
                            if !results.get().is_empty() {
                                set_show_results.set(true);
                            }
                        }
                        on:blur=on_blur
                    />
                    <Show when=move || is_searching.get()>
                        <div class="search-spinner"/>
                    </Show>
                </div>
            </form>

            <Show when=move || show_results.get()>
                <div class="search-results">
                    <Show
                        when=move || error.get().is_none()
                        fallback=move || view! {
                            <div class="search-error">
                                {move || error.get().unwrap_or_default()}
                            </div>
                        }
                    >
                        <Show
                            when=move || !results.get().is_empty()
                            fallback=move || view! {
                                <Show when=move || !is_searching.get()>
                                    <div class="search-empty">"No results found"</div>
                                </Show>
                            }
                        >
                            <ul class="search-results-list">
                                <For
                                    each=move || results.get()
                                    key=|result| result.url.clone()
                                    children=move |result| {
                                        let content = result.content.clone();
                                        view! {
                                            <li class="search-result-item">
                                                <a href={result.url.clone()} target="_blank" rel="noopener">
                                                    <div class="result-title">{result.title.clone()}</div>
                                                    <div class="result-url">{result.url.clone()}</div>
                                                    {move || {
                                                        if let Some(ref c) = content {
                                                            view! {
                                                                <div class="result-snippet">{c.clone()}</div>
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

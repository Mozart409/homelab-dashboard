//! Search results dropdown partial (rendered from `/search`).

use maud::{Markup, html};

use crate::types::SearchResult;

/// The dropdown panel wrapper shared by every result state.
#[allow(clippy::needless_pass_by_value)]
fn panel(inner: Markup) -> Markup {
    html! {
        div class="bg-bg-card border border-border rounded-lg shadow-lg max-h-[400px] overflow-y-auto" {
            (inner)
        }
    }
}

/// Render search results (or an empty / error state) for the dropdown.
///
/// An empty query returns nothing so the dropdown collapses.
#[must_use]
pub fn search_results(query: &str, results: &[SearchResult]) -> Markup {
    if query.trim().is_empty() {
        return html! {};
    }

    if results.is_empty() {
        return panel(html! {
            div class="p-6 text-center text-text-muted" { "No results found" }
        });
    }

    panel(html! {
        ul class="list-none" {
            @for result in results {
                li class="border-b border-border-subtle last:border-b-0" {
                    a href=(result.url) target="_blank" rel="noopener"
                        class="block p-4 no-underline transition-colors duration-150 hover:bg-bg-elevated" {
                        div class="text-accent-blue font-medium mb-1" { (result.title) }
                        div class="text-text-muted text-xs font-mono mb-1 truncate" { (result.url) }
                        @if let Some(content) = &result.content {
                            div class="text-text-secondary text-sm leading-snug line-clamp-2" { (content) }
                        }
                    }
                }
            }
        }
    })
}

/// Render a failed search (e.g. `SearXNG` unreachable or unconfigured).
#[must_use]
pub fn search_error(message: &str) -> Markup {
    panel(html! {
        div class="p-6 text-center text-accent-red" { (message) }
    })
}

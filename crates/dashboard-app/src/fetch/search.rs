//! `SearXNG` search proxy.
//!
//! With the htmx/SSE architecture the browser no longer talks to `SearXNG`
//! directly; the query is proxied through the server so the instance URL stays
//! server-side and CORS is avoided.

use crate::types::{SearchResponse, SearchResult};
use color_eyre::eyre::{Result, WrapErr, eyre};

/// Query a `SearXNG` instance and return parsed results.
pub async fn search(base_url: Option<&str>, query: &str) -> Result<SearchResponse> {
    use serde::Deserialize;

    /// Raw result from `SearXNG`'s JSON API.
    #[derive(Debug, Deserialize)]
    struct ApiResult {
        title: String,
        url: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        engine: String,
    }

    #[derive(Debug, Deserialize)]
    struct ApiResponse {
        #[serde(default)]
        results: Vec<ApiResult>,
        #[serde(default)]
        suggestions: Vec<String>,
    }

    let base_url = base_url.ok_or_else(|| eyre!("SearXNG URL not configured"))?;

    let url = format!(
        "{}/search?q={}&format=json",
        base_url.trim_end_matches('/'),
        urlencoding::encode(query)
    );

    let client = reqwest::Client::new();
    let response: ApiResponse = client
        .get(&url)
        .send()
        .await
        .wrap_err("Search request failed")?
        .json()
        .await
        .wrap_err("Failed to parse search response")?;

    let results = response
        .results
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.url,
            content: r.content,
            engine: r.engine,
        })
        .collect();

    Ok(SearchResponse {
        query: query.to_string(),
        results,
        suggestions: response.suggestions,
    })
}

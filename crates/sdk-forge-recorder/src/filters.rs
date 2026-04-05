//! Network traffic filtering heuristics.
//!
//! Separates API-relevant exchanges from noise (static assets,
//! analytics beacons, tracking pixels, etc.).

use sdk_forge_session::Exchange;

/// Known analytics and tracking URL patterns to filter out.
const NOISE_PATTERNS: &[&str] = &[
    "analytics",
    "telemetry",
    "tracking",
    "pixel",
    "doubleclick",
    "googletag",
    "facebook.com/tr",
    "hotjar",
    "segment.io",
    "mixpanel",
];

/// Static asset file extensions to filter out.
const STATIC_EXTENSIONS: &[&str] = &[
    ".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".woff", ".woff2", ".ttf", ".ico",
    ".map",
];

/// Determine whether a captured exchange is likely an API call
/// rather than noise (static assets, analytics, etc.).
pub fn is_api_relevant(exchange: &Exchange) -> bool {
    let url = &exchange.request.url;

    // Drop static assets
    for ext in STATIC_EXTENSIONS {
        if url.contains(ext) {
            return false;
        }
    }

    // Drop known analytics/tracking
    for pattern in NOISE_PATTERNS {
        if url.contains(pattern) {
            return false;
        }
    }

    // Keep if JSON content type
    if let Some(ct) = &exchange.response.content_type
        && (ct.contains("application/json") || ct.contains("application/graphql"))
    {
        return true;
    }

    // Keep if XHR/fetch initiated
    matches!(
        exchange.initiator,
        sdk_forge_session::Initiator::Xhr | sdk_forge_session::Initiator::Fetch
    )
}

#[cfg(test)]
mod tests {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use sdk_forge_session::{CapturedRequest, CapturedResponse, Exchange, Initiator, Timing};
    use std::collections::HashMap;

    /// Helper to build a minimal exchange for testing filters.
    fn make_exchange(url: &str, content_type: Option<&str>, initiator: Initiator) -> Exchange {
        Exchange {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            request: CapturedRequest {
                method: "GET".to_owned(),
                url: url.to_owned(),
                headers: HashMap::new(),
                query_params: HashMap::new(),
                body: None,
            },
            response: CapturedResponse {
                status: 200,
                headers: HashMap::new(),
                body: None,
                content_type: content_type.map(ToOwned::to_owned),
            },
            initiator,
            timing: Timing {
                start_ms: 0,
                ttfb_ms: None,
                duration_ms: 100,
            },
            classification: None,
        }
    }

    #[test]
    fn test_filters_out_static_assets() {
        let exchange = make_exchange(
            "https://example.com/bundle.js",
            Some("application/javascript"),
            Initiator::Script,
        );
        assert!(
            !is_api_relevant(&exchange),
            "JS files should be filtered out"
        );
    }

    #[test]
    fn test_keeps_json_api_calls() {
        let exchange = make_exchange(
            "https://api.example.com/users",
            Some("application/json"),
            Initiator::Fetch,
        );
        assert!(is_api_relevant(&exchange), "JSON API calls should be kept");
    }

    #[test]
    fn test_filters_out_analytics() {
        let exchange = make_exchange(
            "https://analytics.example.com/collect",
            Some("application/json"),
            Initiator::Xhr,
        );
        assert!(
            !is_api_relevant(&exchange),
            "analytics calls should be filtered out"
        );
    }

    #[test]
    fn test_keeps_xhr_without_content_type() {
        let exchange = make_exchange("https://api.example.com/data", None, Initiator::Xhr);
        assert!(
            is_api_relevant(&exchange),
            "XHR requests should be kept even without content type"
        );
    }
}

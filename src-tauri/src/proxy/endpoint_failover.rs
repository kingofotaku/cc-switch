use crate::proxy::error::ProxyError;
use std::collections::HashSet;

pub(crate) const CURRENT_ENDPOINT_ATTEMPTS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EndpointAttempt {
    pub(crate) url: String,
}

fn normalize_endpoint_url(url: &str) -> Option<String> {
    let normalized = url.trim().trim_end_matches('/').to_string();
    (!normalized.is_empty()).then_some(normalized)
}

pub(crate) fn build_endpoint_attempt_plan<I, S>(
    current_endpoint: &str,
    endpoints: I,
) -> Vec<EndpointAttempt>
where
    I: IntoIterator<Item = (S, i64)>,
    S: AsRef<str>,
{
    let Some(current_endpoint) = normalize_endpoint_url(current_endpoint) else {
        return Vec::new();
    };

    let mut seen = HashSet::from([current_endpoint.clone()]);
    let mut fallbacks = endpoints
        .into_iter()
        .filter_map(|(url, added_at)| {
            normalize_endpoint_url(url.as_ref()).map(|url| (url, added_at))
        })
        .filter(|(url, _)| seen.insert(url.clone()))
        .collect::<Vec<_>>();
    fallbacks.sort_by(|(left_url, left_added), (right_url, right_added)| {
        left_added
            .cmp(right_added)
            .then_with(|| left_url.cmp(right_url))
    });

    let mut plan = Vec::with_capacity(CURRENT_ENDPOINT_ATTEMPTS + fallbacks.len());
    for _ in 0..CURRENT_ENDPOINT_ATTEMPTS {
        plan.push(EndpointAttempt {
            url: current_endpoint.clone(),
        });
    }
    plan.extend(
        fallbacks
            .into_iter()
            .map(|(url, _)| EndpointAttempt { url }),
    );
    plan
}

pub(crate) fn is_endpoint_failover_error(error: &ProxyError) -> bool {
    match error {
        ProxyError::ForwardFailed(_) | ProxyError::Timeout(_) => true,
        ProxyError::UpstreamError { status, .. } => matches!(status, 502..=504),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_endpoint_attempt_plan, is_endpoint_failover_error};
    use crate::codex_config::update_provider_settings_base_url;
    use crate::proxy::error::ProxyError;
    use serde_json::json;

    #[test]
    fn current_endpoint_gets_three_attempts_then_each_fallback_gets_one() {
        let plan = build_endpoint_attempt_plan(
            "https://api.example.com/",
            [
                ("https://speed.example.com", 30),
                ("https://api.example.com", 10),
                ("https://cf.example.com/", 20),
            ],
        );

        let urls: Vec<_> = plan.iter().map(|attempt| attempt.url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://api.example.com",
                "https://api.example.com",
                "https://api.example.com",
                "https://cf.example.com",
                "https://speed.example.com",
            ]
        );
    }

    #[test]
    fn endpoint_plan_normalizes_and_deduplicates_urls() {
        let plan = build_endpoint_attempt_plan(
            "https://api.example.com///",
            [
                ("https://api.example.com/", 10),
                ("https://cf.example.com", 20),
                ("https://cf.example.com///", 30),
                ("   ", 40),
            ],
        );

        let urls: Vec<_> = plan.iter().map(|attempt| attempt.url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://api.example.com",
                "https://api.example.com",
                "https://api.example.com",
                "https://cf.example.com",
            ]
        );
    }

    #[test]
    fn only_network_timeout_and_gateway_errors_trigger_endpoint_failover() {
        assert!(is_endpoint_failover_error(&ProxyError::ForwardFailed(
            "connection reset".to_string()
        )));
        assert!(is_endpoint_failover_error(&ProxyError::Timeout(
            "first byte".to_string()
        )));
        for status in [502, 503, 504] {
            assert!(is_endpoint_failover_error(&ProxyError::UpstreamError {
                status,
                body: None,
            }));
        }

        for status in [400, 401, 403, 429, 500] {
            assert!(!is_endpoint_failover_error(&ProxyError::UpstreamError {
                status,
                body: None,
            }));
        }
        assert!(!is_endpoint_failover_error(&ProxyError::StreamIdleTimeout(
            30
        )));
    }

    #[test]
    fn updates_toml_backed_codex_provider_endpoint_without_touching_other_fields() {
        let mut settings = json!({
            "config": r#"model = "gpt-test"
model_provider = "custom"

[model_providers.custom]
name = "Custom"
base_url = "https://old.example.com/v1"
wire_api = "responses"
"#,
            "auth": {"OPENAI_API_KEY": "keep-me"}
        });

        let previous =
            update_provider_settings_base_url(&mut settings, "https://new.example.com/v1/")
                .expect("update endpoint");

        assert_eq!(previous.as_deref(), Some("https://old.example.com/v1"));
        let config = settings["config"].as_str().expect("config string");
        assert!(config.contains("base_url = \"https://new.example.com/v1\""));
        assert!(config.contains("model = \"gpt-test\""));
        assert_eq!(settings["auth"]["OPENAI_API_KEY"], "keep-me");
    }

    #[test]
    fn updates_direct_and_object_backed_codex_provider_endpoints() {
        let mut direct = json!({"base_url": "https://old.example.com/"});
        let previous = update_provider_settings_base_url(&mut direct, "https://new.example.com/")
            .expect("update direct endpoint");
        assert_eq!(previous.as_deref(), Some("https://old.example.com"));
        assert_eq!(direct["base_url"], "https://new.example.com");

        let mut object = json!({
            "config": {
                "base_url": "https://old-object.example.com/v1",
                "wire_api": "responses"
            }
        });
        let previous =
            update_provider_settings_base_url(&mut object, "https://new-object.example.com/v1")
                .expect("update object endpoint");
        assert_eq!(
            previous.as_deref(),
            Some("https://old-object.example.com/v1")
        );
        assert_eq!(
            object["config"]["base_url"],
            "https://new-object.example.com/v1"
        );
        assert_eq!(object["config"]["wire_api"], "responses");
    }
}

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipSourceType {
    Http,
    Ldap,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipAttributeRequest {
    pub attribute_name: String,
    pub source_type: PipSourceType,
    pub source_config: PipSourceConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipSourceConfig {
    pub uri: String,
    pub timeout_seconds: u64,
    pub cache_ttl_seconds: u64,
    #[serde(default)]
    pub tls_ca_path: Option<String>,
    #[serde(default)]
    pub auth_config: Option<PipAuthConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipAuthConfig {
    #[serde(default)]
    pub bearer_token: Option<String>,
    #[serde(default)]
    pub basic_username: Option<String>,
    #[serde(default)]
    pub basic_password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipRequest {
    pub subject: Option<Uuid>,
    pub resource: Uuid,
    pub attributes_requested: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipResponse {
    pub attributes: BTreeMap<String, PipAttributeValue>,
    pub source_status: BTreeMap<String, PipSourceStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipAttributeValue {
    pub value: String,
    pub source: String,
    pub cached: bool,
    pub retrieved_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipSourceStatus {
    Success,
    Timeout,
    Error,
    Unavailable,
    Cached,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipHealthCheckResponse {
    pub sources: BTreeMap<String, PipSourceHealth>,
    pub overall_status: PipOverallHealth,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipSourceHealth {
    pub source_type: PipSourceType,
    pub uri: String,
    pub status: PipSourceStatus,
    pub last_check: u64,
    pub latency_ms: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipOverallHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipCacheEntry {
    pub key: PipCacheKey,
    pub value: String,
    pub source: String,
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipCacheKey {
    pub subject: Option<Uuid>,
    pub resource: Uuid,
    pub attribute_name: String,
}

impl PipCacheEntry {
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time > self.cached_at + self.ttl_seconds
    }
}

impl PipRequest {
    pub fn new(subject: Option<Uuid>, resource: Uuid, attributes: Vec<String>) -> Self {
        Self {
            subject,
            resource,
            attributes_requested: attributes,
        }
    }
}

impl PipResponse {
    pub fn new() -> Self {
        Self {
            attributes: BTreeMap::new(),
            source_status: BTreeMap::new(),
        }
    }

    pub fn with_attribute(mut self, name: String, value: PipAttributeValue) -> Self {
        self.attributes.insert(name, value);
        self
    }

    pub fn with_source_status(mut self, source: String, status: PipSourceStatus) -> Self {
        self.source_status.insert(source, status);
        self
    }
}

impl Default for PipResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_request_serialization() {
        let req = PipRequest::new(
            Some(Uuid::nil()),
            Uuid::nil(),
            vec!["department".to_string()],
        );
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("department"));
    }

    #[test]
    fn test_pip_cache_entry_expiration() {
        let entry = PipCacheEntry {
            key: PipCacheKey {
                subject: None,
                resource: Uuid::nil(),
                attribute_name: "test".to_string(),
            },
            value: "value".to_string(),
            source: "http".to_string(),
            cached_at: 1000,
            ttl_seconds: 60,
        };

        assert!(!entry.is_expired(1050));
        assert!(entry.is_expired(1100));
    }

    #[test]
    fn test_pip_response_builder() {
        let response = PipResponse::new()
            .with_attribute(
                "department".to_string(),
                PipAttributeValue {
                    value: "engineering".to_string(),
                    source: "hr-api".to_string(),
                    cached: false,
                    retrieved_at: 1000,
                },
            )
            .with_source_status("hr-api".to_string(), PipSourceStatus::Success);

        assert_eq!(response.attributes.len(), 1);
        assert_eq!(response.source_status.len(), 1);
    }

    #[test]
    fn test_pip_request_deserialization() {
        let json = r#"{
            "subject": "00000000-0000-0000-0000-000000000000",
            "resource": "00000000-0000-0000-0000-000000000001",
            "attributesRequested": ["department", "role"]
        }"#;

        let req: PipRequest = serde_json::from_str(json).unwrap();
        assert!(req.subject.is_some());
        assert_eq!(req.attributes_requested.len(), 2);
        assert!(req.attributes_requested.contains(&"department".to_string()));
        assert!(req.attributes_requested.contains(&"role".to_string()));
    }

    #[test]
    fn test_pip_request_empty_attributes() {
        let req = PipRequest::new(None, Uuid::nil(), vec![]);
        assert!(req.attributes_requested.is_empty());
    }

    #[test]
    fn test_pip_response_empty() {
        let response = PipResponse::new();
        assert!(response.attributes.is_empty());
        assert!(response.source_status.is_empty());
    }

    #[test]
    fn test_pip_response_with_multiple_attributes() {
        let response = PipResponse::new()
            .with_attribute(
                "department".to_string(),
                PipAttributeValue {
                    value: "engineering".to_string(),
                    source: "hr-api".to_string(),
                    cached: false,
                    retrieved_at: 1000,
                },
            )
            .with_attribute(
                "role".to_string(),
                PipAttributeValue {
                    value: "admin".to_string(),
                    source: "hr-api".to_string(),
                    cached: true,
                    retrieved_at: 900,
                },
            )
            .with_source_status("hr-api".to_string(), PipSourceStatus::Success);

        assert_eq!(response.attributes.len(), 2);
        assert!(response.attributes.contains_key(&"department".to_string()));
        assert!(response.attributes.contains_key(&"role".to_string()));
    }

    #[test]
    fn test_pip_source_status_serde() {
        for status in [
            PipSourceStatus::Success,
            PipSourceStatus::Timeout,
            PipSourceStatus::Error,
            PipSourceStatus::Unavailable,
            PipSourceStatus::Cached,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: PipSourceStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_pip_overall_health_serde() {
        for health in [
            PipOverallHealth::Healthy,
            PipOverallHealth::Degraded,
            PipOverallHealth::Unhealthy,
        ] {
            let json = serde_json::to_string(&health).unwrap();
            let parsed: PipOverallHealth = serde_json::from_str(&json).unwrap();
            assert_eq!(health, parsed);
        }
    }

    #[test]
    fn test_pip_source_type_serde() {
        for source_type in [PipSourceType::Http, PipSourceType::Ldap] {
            let json = serde_json::to_string(&source_type).unwrap();
            let parsed: PipSourceType = serde_json::from_str(&json).unwrap();
            assert_eq!(source_type, parsed);
        }
    }

    #[test]
    fn test_pip_attribute_value_serialization() {
        let attr_value = PipAttributeValue {
            value: "test_value".to_string(),
            source: "test_source".to_string(),
            cached: true,
            retrieved_at: 1234567890,
        };

        let json = serde_json::to_string(&attr_value).unwrap();
        assert!(json.contains("test_value"));
        assert!(json.contains("test_source"));
        assert!(json.contains("cached"));

        let parsed: PipAttributeValue = serde_json::from_str(&json).unwrap();
        assert_eq!(attr_value.value, parsed.value);
        assert_eq!(attr_value.source, parsed.source);
        assert_eq!(attr_value.cached, parsed.cached);
    }

    #[test]
    fn test_pip_cache_key_serialization() {
        let key = PipCacheKey {
            subject: Some(Uuid::nil()),
            resource: Uuid::nil(),
            attribute_name: "test_attr".to_string(),
        };

        let json = serde_json::to_string(&key).unwrap();
        let parsed: PipCacheKey = serde_json::from_str(&json).unwrap();

        assert_eq!(key.subject, parsed.subject);
        assert_eq!(key.resource, parsed.resource);
        assert_eq!(key.attribute_name, parsed.attribute_name);
    }

    #[test]
    fn test_pip_cache_entry_full_serialization() {
        let entry = PipCacheEntry {
            key: PipCacheKey {
                subject: None,
                resource: Uuid::nil(),
                attribute_name: "department".to_string(),
            },
            value: "engineering".to_string(),
            source: "http-source".to_string(),
            cached_at: 1000,
            ttl_seconds: 300,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: PipCacheEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.value, parsed.value);
        assert_eq!(entry.source, parsed.source);
        assert_eq!(entry.cached_at, parsed.cached_at);
        assert_eq!(entry.ttl_seconds, parsed.ttl_seconds);
    }

    #[test]
    fn test_pip_health_check_response() {
        let response = PipHealthCheckResponse {
            sources: BTreeMap::new(),
            overall_status: PipOverallHealth::Healthy,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: PipHealthCheckResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.overall_status, parsed.overall_status);
    }

    #[test]
    fn test_pip_health_check_with_sources() {
        let mut sources = BTreeMap::new();
        sources.insert(
            "hr-api".to_string(),
            PipSourceHealth {
                source_type: PipSourceType::Http,
                uri: "https://hr.example.com/api".to_string(),
                status: PipSourceStatus::Success,
                last_check: 1234567890,
                latency_ms: Some(50),
                error_message: None,
            },
        );

        let response = PipHealthCheckResponse {
            sources,
            overall_status: PipOverallHealth::Healthy,
        };

        assert_eq!(response.sources.len(), 1);
        assert!(response.sources.contains_key(&"hr-api".to_string()));
    }

    #[test]
    fn test_pip_source_health_serialization() {
        let health = PipSourceHealth {
            source_type: PipSourceType::Http,
            uri: "https://example.com".to_string(),
            status: PipSourceStatus::Error,
            last_check: 1000,
            latency_ms: None,
            error_message: Some("Connection timeout".to_string()),
        };

        let json = serde_json::to_string(&health).unwrap();
        let parsed: PipSourceHealth = serde_json::from_str(&json).unwrap();

        assert_eq!(health.status, parsed.status);
        assert!(parsed.error_message.is_some());
        assert!(parsed.latency_ms.is_none());
    }

    #[test]
    fn test_pip_auth_config_serialization() {
        let config = PipAuthConfig {
            bearer_token: Some("test_token".to_string()),
            basic_username: None,
            basic_password: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PipAuthConfig = serde_json::from_str(&json).unwrap();

        assert!(parsed.bearer_token.is_some());
        assert!(parsed.basic_username.is_none());
    }

    #[test]
    fn test_pip_auth_config_basic_auth() {
        let config = PipAuthConfig {
            bearer_token: None,
            basic_username: Some("user".to_string()),
            basic_password: Some("password".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PipAuthConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.basic_username.unwrap(), "user");
        assert_eq!(parsed.basic_password.unwrap(), "password");
    }

    #[test]
    fn test_pip_source_config_serialization() {
        let config = PipSourceConfig {
            uri: "https://example.com/api".to_string(),
            timeout_seconds: 30,
            cache_ttl_seconds: 60,
            tls_ca_path: Some("/path/to/ca.pem".to_string()),
            auth_config: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PipSourceConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.uri, "https://example.com/api");
        assert_eq!(parsed.timeout_seconds, 30);
        assert_eq!(parsed.cache_ttl_seconds, 60);
    }

    #[test]
    fn test_pip_attribute_request_serialization() {
        let request = PipAttributeRequest {
            attribute_name: "department".to_string(),
            source_type: PipSourceType::Http,
            source_config: PipSourceConfig {
                uri: "https://hr.example.com".to_string(),
                timeout_seconds: 10,
                cache_ttl_seconds: 30,
                tls_ca_path: None,
                auth_config: Some(PipAuthConfig {
                    bearer_token: Some("token".to_string()),
                    basic_username: None,
                    basic_password: None,
                }),
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: PipAttributeRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.attribute_name, "department");
        assert_eq!(parsed.source_type, PipSourceType::Http);
        assert!(parsed.source_config.auth_config.is_some());
    }

    #[test]
    fn test_pip_cache_entry_edge_times() {
        let entry = PipCacheEntry {
            key: PipCacheKey {
                subject: None,
                resource: Uuid::nil(),
                attribute_name: "test".to_string(),
            },
            value: "value".to_string(),
            source: "http".to_string(),
            cached_at: 1000,
            ttl_seconds: 60,
        };

        assert!(!entry.is_expired(1059));
        assert!(entry.is_expired(1061));
        assert!(!entry.is_expired(1000));
        assert!(!entry.is_expired(1060));
    }

    #[test]
    fn test_pip_request_null_subject() {
        let json = r#"{
            "subject": null,
            "resource": "00000000-0000-0000-0000-000000000000",
            "attributesRequested": ["department"]
        }"#;

        let req: PipRequest = serde_json::from_str(json).unwrap();
        assert!(req.subject.is_none());
    }

    #[test]
    fn test_pip_response_multiple_sources() {
        let response = PipResponse::new()
            .with_attribute(
                "department".to_string(),
                PipAttributeValue {
                    value: "engineering".to_string(),
                    source: "hr-api".to_string(),
                    cached: false,
                    retrieved_at: 1000,
                },
            )
            .with_attribute(
                "security_level".to_string(),
                PipAttributeValue {
                    value: "high".to_string(),
                    source: "security-api".to_string(),
                    cached: true,
                    retrieved_at: 900,
                },
            )
            .with_source_status("hr-api".to_string(), PipSourceStatus::Success)
            .with_source_status("security-api".to_string(), PipSourceStatus::Cached);

        assert_eq!(response.attributes.len(), 2);
        assert_eq!(response.source_status.len(), 2);
        assert_eq!(
            response.source_status.get("hr-api"),
            Some(&PipSourceStatus::Success)
        );
        assert_eq!(
            response.source_status.get("security-api"),
            Some(&PipSourceStatus::Cached)
        );
    }
}

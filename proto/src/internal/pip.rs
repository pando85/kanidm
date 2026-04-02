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
}

//! HTTP Policy Information Point Implementation
//!
//! This module provides a PIP implementation that retrieves attributes from
//! HTTP REST endpoints.

#![allow(dead_code)]

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::cache::PipAttributeCache;
use super::config::{HttpMethod, HttpPipConfig, PipFallbackBehavior};
use super::health::PipHealthState;
use super::{
    PipAttributeName, PipAttributeSet, PipAttributeValue, PipHealthCheck, PipHealthStatus, PipId,
    PipResult, PipSubject, PolicyInformationPoint,
};
use crate::prelude::*;

/// HTTP PIP implementation
pub struct HttpPip {
    id: PipId,
    config: HttpPipConfig,
    client: Client,
    cache: Arc<RwLock<PipAttributeCache>>,
    health_status: Arc<RwLock<PipHealthState>>,
    provided_attributes: Vec<String>,
}

impl HttpPip {
    /// Create a new HTTP PIP from configuration
    pub fn new(config: HttpPipConfig) -> Result<Self, String> {
        let client = Self::build_client(&config)?;
        let cache = Arc::new(RwLock::new(PipAttributeCache::with_settings(
            Duration::from_secs(config.cache_ttl_secs),
            1000,
        )));

        let provided_attributes = if config.provided_attributes.is_empty() {
            Self::extract_provided_attributes_from_endpoint(&config)?
        } else {
            config.provided_attributes.clone()
        };

        Ok(HttpPip {
            id: PipId::new(&config.id),
            config,
            client,
            cache,
            health_status: Arc::new(RwLock::new(PipHealthState::new())),
            provided_attributes,
        })
    }

    fn build_client(config: &HttpPipConfig) -> Result<Client, String> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(10));

        if config.tls.allow_insecure {
            admin_warn!(
                "PIP '{}' is using insecure TLS - this should only be used for development!",
                config.id
            );
        }

        if !config.tls.verify_server || config.tls.allow_insecure {
            builder = builder.danger_accept_invalid_certs(config.tls.allow_insecure);
        }

        builder
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))
    }

    fn extract_provided_attributes_from_endpoint(
        _config: &HttpPipConfig,
    ) -> Result<Vec<String>, String> {
        Ok(vec!["attributes".to_string()])
    }

    fn build_url(&self, subject: &PipSubject) -> String {
        let mut path = self.config.endpoint_path.clone();

        path = path.replace("{uuid}", &subject.uuid.to_string());
        if let Some(username) = &subject.username {
            path = path.replace("{username}", username);
        }
        if let Some(email) = &subject.email {
            path = path.replace("{email}", email);
        }
        for (key, value) in &subject.context {
            path = path.replace(&format!("{{{}}}", key), value);
        }

        format!("{}{}", self.config.base_url, path)
    }

    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut builder = match self.config.method {
            HttpMethod::Get => self.client.get(url),
            HttpMethod::Post => self.client.post(url),
        };

        for (key, value) in &self.config.headers {
            builder = builder.header(key, value);
        }

        builder = self.apply_auth(builder);

        builder
    }

    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(auth) = &self.config.auth {
            match auth {
                super::config::PipAuthConfig::None => builder,
                super::config::PipAuthConfig::Basic { username, password } => {
                    builder.basic_auth(username, Some(password))
                }
                super::config::PipAuthConfig::Bearer { token } => builder.bearer_auth(token),
                super::config::PipAuthConfig::ApiKey {
                    key_name,
                    key_value,
                    location,
                } => match location {
                    super::config::ApiKeyLocation::Header => builder.header(key_name, key_value),
                    super::config::ApiKeyLocation::QueryParam => {
                        builder.query(&[(key_name, key_value)])
                    }
                },
                super::config::PipAuthConfig::OAuth2ClientCredentials { .. } => {
                    admin_warn!("OAuth2 client credentials auth not yet implemented for PIP");
                    builder
                }
                super::config::PipAuthConfig::MutualTls { .. } => {
                    admin_warn!("Mutual TLS auth not yet implemented for PIP");
                    builder
                }
            }
        } else {
            builder
        }
    }

    fn parse_response(&self, json: Value) -> PipAttributeSet {
        let mut attrs = PipAttributeSet::new();

        let attr_map = match json {
            Value::Object(map) => map,
            Value::Array(arr) if arr.len() == 1 && arr.first().is_some_and(|v| v.is_object()) => {
                arr.first()
                    .and_then(|v| v.as_object().cloned())
                    .unwrap_or_default()
            }
            _ => {
                admin_debug!("PIP response is not a JSON object, wrapping in 'response' attribute");
                attrs.insert(
                    PipAttributeName::new(&self.id, "response"),
                    Self::json_to_attribute_value(json),
                );
                return attrs;
            }
        };

        for (key, value) in attr_map {
            let attr_name = PipAttributeName::new(&self.id, &key);
            let attr_value = Self::json_to_attribute_value(value);
            attrs.insert(attr_name, attr_value);
        }

        attrs
    }

    fn json_to_attribute_value(value: Value) -> PipAttributeValue {
        match value {
            Value::Null => PipAttributeValue::Json(Value::Null),
            Value::Bool(b) => PipAttributeValue::Boolean(b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    PipAttributeValue::Integer(i)
                } else {
                    PipAttributeValue::Json(Value::Number(n))
                }
            }
            Value::String(s) => PipAttributeValue::String(s),
            Value::Array(arr) => PipAttributeValue::Array(
                arr.into_iter().map(Self::json_to_attribute_value).collect(),
            ),
            Value::Object(obj) => PipAttributeValue::Json(Value::Object(obj)),
        }
    }

    fn apply_fallback(&self) -> PipAttributeSet {
        let mut attrs = PipAttributeSet::new();

        for (key, value) in &self.config.fallback_values {
            let attr_name = PipAttributeName::new(&self.id, key);
            attrs.insert(attr_name, value.clone());
        }

        attrs
    }

    async fn update_health_success(&self) {
        let mut state = self.health_status.write().await;
        state.record_success(&self.config.health_check);
    }

    async fn update_health_failure(&self, error: String) {
        let mut state = self.health_status.write().await;
        state.record_failure(&self.config.health_check, error);
    }
}

#[async_trait]
impl PolicyInformationPoint for HttpPip {
    fn id(&self) -> &PipId {
        &self.id
    }

    async fn retrieve_attributes(&self, subject: &PipSubject) -> PipResult {
        {
            let mut cache = self.cache.write().await;
            if let Some(cached_attrs) = cache.get(&self.id, subject) {
                return PipResult::Success(cached_attrs);
            }
        }

        if !self.cached_health_status().can_retrieve() {
            match self.config.fallback_behavior {
                PipFallbackBehavior::UseFallback => {
                    return PipResult::Success(self.apply_fallback());
                }
                PipFallbackBehavior::Deny => {
                    return PipResult::Unavailable {
                        reason: "PIP source is unhealthy".to_string(),
                        fallback_used: false,
                    };
                }
                PipFallbackBehavior::Ignore => {
                    return PipResult::Unavailable {
                        reason: "PIP source is unhealthy".to_string(),
                        fallback_used: false,
                    };
                }
            }
        }

        let url = self.build_url(subject);
        let request = self.build_request(&url);

        let result = request.send().await;

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    let json_result = response.json::<Value>().await;

                    match json_result {
                        Ok(json) => {
                            self.update_health_success().await;

                            let attrs = self.parse_response(json);

                            {
                                let mut cache = self.cache.write().await;
                                cache.put(&self.id, subject, attrs.clone());
                            }

                            PipResult::Success(attrs)
                        }
                        Err(e) => {
                            let error = format!("Failed to parse response JSON: {}", e);
                            self.update_health_failure(error.clone()).await;

                            match self.config.fallback_behavior {
                                PipFallbackBehavior::UseFallback => PipResult::Error {
                                    error,
                                    fallback_used: true,
                                },
                                PipFallbackBehavior::Deny => PipResult::Error {
                                    error,
                                    fallback_used: false,
                                },
                                PipFallbackBehavior::Ignore => PipResult::Error {
                                    error,
                                    fallback_used: false,
                                },
                            }
                        }
                    }
                } else {
                    let error = format!("HTTP error: {}", response.status());
                    self.update_health_failure(error.clone()).await;

                    match self.config.fallback_behavior {
                        PipFallbackBehavior::UseFallback => PipResult::Error {
                            error,
                            fallback_used: true,
                        },
                        PipFallbackBehavior::Deny => PipResult::Error {
                            error,
                            fallback_used: false,
                        },
                        PipFallbackBehavior::Ignore => PipResult::Error {
                            error,
                            fallback_used: false,
                        },
                    }
                }
            }
            Err(e) => {
                let is_timeout = e.is_timeout();
                let error = format!("Request failed: {}", e);

                self.update_health_failure(error.clone()).await;

                match self.config.fallback_behavior {
                    PipFallbackBehavior::UseFallback => {
                        if is_timeout {
                            PipResult::Timeout {
                                fallback_used: true,
                            }
                        } else {
                            PipResult::Error {
                                error,
                                fallback_used: true,
                            }
                        }
                    }
                    PipFallbackBehavior::Deny => {
                        if is_timeout {
                            PipResult::Timeout {
                                fallback_used: false,
                            }
                        } else {
                            PipResult::Error {
                                error,
                                fallback_used: false,
                            }
                        }
                    }
                    PipFallbackBehavior::Ignore => {
                        if is_timeout {
                            PipResult::Timeout {
                                fallback_used: false,
                            }
                        } else {
                            PipResult::Error {
                                error,
                                fallback_used: false,
                            }
                        }
                    }
                }
            }
        }
    }

    async fn retrieve_named_attributes(
        &self,
        subject: &PipSubject,
        attribute_names: &[String],
    ) -> PipResult {
        let all_result = self.retrieve_attributes(subject).await;

        if let Some(all_attrs) = all_result.attributes() {
            let mut filtered_attrs = PipAttributeSet::new();

            for name in attribute_names {
                let full_name = PipAttributeName::new(&self.id, name);
                if let Some(value) = all_attrs.get(&full_name) {
                    filtered_attrs.insert(full_name, value.clone());
                }
            }

            PipResult::Success(filtered_attrs)
        } else {
            all_result
        }
    }

    async fn health_check(&self) -> PipHealthCheck {
        let start = Instant::now();

        let health_url = if let Some(path) = &self.config.health_check_path {
            format!("{}{}", self.config.base_url, path)
        } else {
            self.config.base_url.to_string()
        };

        let result = self
            .client
            .get(&health_url)
            .timeout(Duration::from_secs(self.config.health_check.timeout_secs))
            .send()
            .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    self.update_health_success().await;

                    PipHealthCheck {
                        pip_id: self.id.clone(),
                        status: PipHealthStatus::Healthy,
                        last_check: Instant::now(),
                        latency_ms: Some(latency_ms),
                        error_message: None,
                    }
                } else {
                    let error = format!("Health check returned: {}", response.status());
                    self.update_health_failure(error.clone()).await;

                    PipHealthCheck {
                        pip_id: self.id.clone(),
                        status: PipHealthStatus::Degraded,
                        last_check: Instant::now(),
                        latency_ms: Some(latency_ms),
                        error_message: Some(error),
                    }
                }
            }
            Err(e) => {
                let error = format!("Health check failed: {}", e);
                self.update_health_failure(error.clone()).await;

                PipHealthCheck {
                    pip_id: self.id.clone(),
                    status: PipHealthStatus::Unhealthy,
                    last_check: Instant::now(),
                    latency_ms: None,
                    error_message: Some(error),
                }
            }
        }
    }

    fn cached_health_status(&self) -> PipHealthStatus {
        let state = self.health_status.blocking_read();
        state.status
    }

    fn provided_attributes(&self) -> &[String] {
        &self.provided_attributes
    }

    async fn clear_cache(&self, subject: &PipSubject) {
        let mut cache = self.cache.write().await;
        cache.remove(&self.id, subject);
    }

    async fn clear_all_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::super::config::PipTlsConfig;
    use super::*;
    use std::collections::BTreeMap;
    use url::Url;

    fn create_test_config() -> HttpPipConfig {
        HttpPipConfig {
            id: "test_http_pip".to_string(),
            base_url: Url::parse("https://example.com/api").unwrap(),
            endpoint_path: "/users/{username}".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: Some("/health".to_string()),
            provided_attributes: vec!["department".to_string(), "role".to_string()],
        }
    }

    #[test]
    fn test_http_pip_creation() {
        let config = create_test_config();
        let pip = HttpPip::new(config);
        assert!(pip.is_ok());
    }

    #[test]
    fn test_url_building_with_username() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        let subject = PipSubject::from_uuid(uuid::Uuid::new_v4()).with_username("testuser");

        let url = pip.build_url(&subject);
        assert!(url.contains("/users/testuser"));
    }

    #[test]
    fn test_url_building_with_uuid() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/users/{uuid}".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config).unwrap();

        let uuid = uuid::Uuid::new_v4();
        let subject = PipSubject::from_uuid(uuid);

        let url = pip.build_url(&subject);
        assert!(url.contains(&uuid.to_string()));
    }

    #[test]
    fn test_url_building_with_email() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/lookup/{email}".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config).unwrap();

        let subject = PipSubject::from_uuid(uuid::Uuid::new_v4()).with_email("test@example.com");

        let url = pip.build_url(&subject);
        assert!(url.contains("/lookup/test@example.com"));
    }

    #[test]
    fn test_url_building_with_context() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/users/{username}/devices/{device_id}".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config).unwrap();

        let subject = PipSubject::from_uuid(uuid::Uuid::new_v4())
            .with_username("testuser")
            .with_context("device_id", "device123");

        let url = pip.build_url(&subject);
        assert!(url.contains("/users/testuser/devices/device123"));
    }

    #[test]
    fn test_json_to_attribute_value_string() {
        let string_val = HttpPip::json_to_attribute_value(Value::String("test".to_string()));
        assert_eq!(string_val.as_str(), Some("test"));
    }

    #[test]
    fn test_json_to_attribute_value_integer() {
        let int_val = HttpPip::json_to_attribute_value(Value::Number(serde_json::Number::from(42)));
        assert_eq!(int_val.as_int(), Some(42));
    }

    #[test]
    fn test_json_to_attribute_value_boolean() {
        let bool_val = HttpPip::json_to_attribute_value(Value::Bool(true));
        assert_eq!(bool_val.as_bool(), Some(true));
    }

    #[test]
    fn test_json_to_attribute_value_null() {
        let null_val = HttpPip::json_to_attribute_value(Value::Null);
        assert!(matches!(null_val, PipAttributeValue::Json(Value::Null)));
    }

    #[test]
    fn test_json_to_attribute_value_array() {
        let arr = Value::Array(vec![
            Value::String("a".to_string()),
            Value::Number(serde_json::Number::from(1)),
        ]);
        let array_val = HttpPip::json_to_attribute_value(arr);
        assert_eq!(array_val.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_json_to_attribute_value_object() {
        let obj = Value::Object(serde_json::Map::from_iter(vec![(
            "key".to_string(),
            Value::String("value".to_string()),
        )]));
        let obj_val = HttpPip::json_to_attribute_value(obj);
        assert!(matches!(obj_val, PipAttributeValue::Json(_)));
    }

    #[test]
    fn test_provided_attributes() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        assert_eq!(pip.provided_attributes().len(), 2);
        assert!(pip
            .provided_attributes()
            .contains(&"department".to_string()));
    }

    #[test]
    fn test_provided_attributes_empty() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/test".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec![],
        };
        let pip = HttpPip::new(config).unwrap();

        assert_eq!(pip.provided_attributes().len(), 1);
        assert!(pip.provided_attributes().contains(&"attributes".to_string()));
    }

    #[test]
    fn test_parse_response_object() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        let json = serde_json::json!({
            "department": "Engineering",
            "role": "Developer"
        });

        let attrs = pip.parse_response(json);
        assert_eq!(attrs.len(), 2);
    }

    #[test]
    fn test_parse_response_array() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        let json = serde_json::json!([{
            "department": "Engineering"
        }]);

        let attrs = pip.parse_response(json);
        assert_eq!(attrs.len(), 1);
    }

    #[test]
    fn test_parse_response_empty_array() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        let json = serde_json::json!([]);

        let attrs = pip.parse_response(json);
        assert_eq!(attrs.len(), 1);
        assert!(attrs.get(&PipAttributeName::new(&pip.id, "response")).is_some());
    }

    #[test]
    fn test_parse_response_primitive() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        let json = serde_json::json!("simple_value");

        let attrs = pip.parse_response(json);
        assert_eq!(attrs.len(), 1);
    }

    #[test]
    fn test_apply_fallback_empty() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        let attrs = pip.apply_fallback();
        assert!(attrs.is_empty());
    }

    #[test]
    fn test_apply_fallback_with_values() {
        let mut fallback = BTreeMap::new();
        fallback.insert(
            "department".to_string(),
            PipAttributeValue::String("Unknown".to_string()),
        );
        fallback.insert(
            "level".to_string(),
            PipAttributeValue::Integer(0),
        );

        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/test".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: fallback,
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config).unwrap();

        let attrs = pip.apply_fallback();
        assert_eq!(attrs.len(), 2);
    }

    #[test]
    fn test_pip_id() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        assert_eq!(pip.id().as_str(), "test_http_pip");
    }

    #[test]
    fn test_cached_health_status_initial() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        let status = pip.cached_health_status();
        assert_eq!(status, PipHealthStatus::Unknown);
    }

    #[test]
    fn test_http_pip_with_post_method() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/users".to_string(),
            method: HttpMethod::Post,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config);
        assert!(pip.is_ok());
    }

    #[test]
    fn test_http_pip_with_headers() {
        let mut headers = BTreeMap::new();
        headers.insert("X-API-Key".to_string(), "secret".to_string());
        headers.insert("X-Custom".to_string(), "value".to_string());

        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/test".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers,
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config);
        assert!(pip.is_ok());
    }

    #[test]
    fn test_http_pip_with_insecure_tls() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/test".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig {
                ca_path: None,
                verify_server: false,
                allow_insecure: true,
            },
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config);
        assert!(pip.is_ok());
    }

    #[test]
    fn test_fallback_behavior_deny() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/test".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::Deny,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config);
        assert!(pip.is_ok());
    }

    #[test]
    fn test_fallback_behavior_ignore() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/test".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::Ignore,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config);
        assert!(pip.is_ok());
    }

    #[test]
    fn test_health_check_url_without_custom_path() {
        let config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com/api").unwrap(),
            endpoint_path: "/test".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["attr".to_string()],
        };
        let pip = HttpPip::new(config).unwrap();

        assert!(pip.config.health_check_path.is_none());
    }

    #[test]
    fn test_health_check_url_with_custom_path() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        assert_eq!(pip.config.health_check_path, Some("/health".to_string()));
    }
}

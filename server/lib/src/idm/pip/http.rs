//! HTTP Policy Information Point Implementation
//!
//! This module provides a PIP implementation that retrieves attributes from
//! HTTP REST endpoints.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::prelude::*;
use super::{
    PipAttributeName, PipAttributeSet, PipAttributeValue, PipHealthCheck, PipHealthStatus,
    PipId, PipResult, PipSubject, PolicyInformationPoint,
};
use super::cache::PipAttributeCache;
use super::config::{HttpPipConfig, HttpMethod, PipFallbackBehavior};

/// HTTP PIP implementation
pub struct HttpPip {
    id: PipId,
    config: HttpPipConfig,
    client: Client,
    cache: Arc<RwLock<PipAttributeCache>>,
    health_status: Arc<RwLock<PipHealthState>>,
    provided_attributes: Vec<String>,
}

/// Internal health state tracking
#[derive(Debug, Clone)]
struct PipHealthState {
    status: PipHealthStatus,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_check: Option<Instant>,
    last_error: Option<String>,
}

impl PipHealthState {
    fn new() -> Self {
        PipHealthState {
            status: PipHealthStatus::Unknown,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_check: None,
            last_error: None,
        }
    }

    fn record_success(&mut self, config: &HttpPipConfig) {
        self.consecutive_failures = 0;
        self.consecutive_successes += 1;

        if self.consecutive_successes >= config.health_check.success_threshold {
            self.status = PipHealthStatus::Healthy;
        }

        self.last_check = Some(Instant::now());
        self.last_error = None;
    }

    fn record_failure(&mut self, config: &HttpPipConfig, error: String) {
        self.consecutive_successes = 0;
        self.consecutive_failures += 1;

        if self.consecutive_failures >= config.health_check.failure_threshold {
            self.status = PipHealthStatus::Unhealthy;
        } else if self.consecutive_failures > 0 {
            self.status = PipHealthStatus::Degraded;
        }

        self.last_check = Some(Instant::now());
        self.last_error = Some(error);
    }
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

        builder.build().map_err(|e| format!("Failed to create HTTP client: {}", e))
    }

    fn extract_provided_attributes_from_endpoint(_config: &HttpPipConfig) -> Result<Vec<String>, String> {
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
                super::config::PipAuthConfig::Bearer { token } => {
                    builder.bearer_auth(token)
                }
                super::config::PipAuthConfig::ApiKey { key_name, key_value, location } => {
                    match location {
                        super::config::ApiKeyLocation::Header => {
                            builder.header(key_name, key_value)
                        }
                        super::config::ApiKeyLocation::QueryParam => {
                            builder.query(&[(key_name, key_value)])
                        }
                    }
                }
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
            Value::Array(arr) if arr.len() == 1 && arr[0].is_object() => {
                arr[0].as_object().unwrap().clone()
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
            Value::Array(arr) => {
                PipAttributeValue::Array(
                    arr.into_iter()
                        .map(Self::json_to_attribute_value)
                        .collect()
                )
            }
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
        state.record_success(&self.config);
    }

    async fn update_health_failure(&self, error: String) {
        let mut state = self.health_status.write().await;
        state.record_failure(&self.config, error);
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
                                PipFallbackBehavior::UseFallback => {
                                    PipResult::Error {
                                        error,
                                        fallback_used: true,
                                    }
                                }
                                PipFallbackBehavior::Deny => {
                                    PipResult::Error {
                                        error,
                                        fallback_used: false,
                                    }
                                }
                                PipFallbackBehavior::Ignore => {
                                    PipResult::Error {
                                        error,
                                        fallback_used: false,
                                    }
                                }
                            }
                        }
                    }
                } else {
                    let error = format!("HTTP error: {}", response.status());
                    self.update_health_failure(error.clone()).await;

                    match self.config.fallback_behavior {
                        PipFallbackBehavior::UseFallback => {
                            PipResult::Error {
                                error,
                                fallback_used: true,
                            }
                        }
                        PipFallbackBehavior::Deny => {
                            PipResult::Error {
                                error,
                                fallback_used: false,
                            }
                        }
                        PipFallbackBehavior::Ignore => {
                            PipResult::Error {
                                error,
                                fallback_used: false,
                            }
                        }
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
                            PipResult::Timeout { fallback_used: true }
                        } else {
                            PipResult::Error {
                                error,
                                fallback_used: true,
                            }
                        }
                    }
                    PipFallbackBehavior::Deny => {
                        if is_timeout {
                            PipResult::Timeout { fallback_used: false }
                        } else {
                            PipResult::Error {
                                error,
                                fallback_used: false,
                            }
                        }
                    }
                    PipFallbackBehavior::Ignore => {
                        if is_timeout {
                            PipResult::Timeout { fallback_used: false }
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
        state.status.clone()
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
    use super::*;

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
    fn test_url_building() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        let subject = PipSubject::from_uuid(uuid::Uuid::new_v4())
            .with_username("testuser");

        let url = pip.build_url(&subject);
        assert!(url.contains("/users/testuser"));
    }

    #[test]
    fn test_json_to_attribute_value() {
        let string_val = HttpPip::json_to_attribute_value(Value::String("test".to_string()));
        assert_eq!(string_val.as_str(), Some("test"));

        let int_val = HttpPip::json_to_attribute_value(Value::Number(serde_json::Number::from(42)));
        assert_eq!(int_val.as_int(), Some(42));

        let bool_val = HttpPip::json_to_attribute_value(Value::Bool(true));
        assert_eq!(bool_val.as_bool(), Some(true));
    }

    #[test]
    fn test_provided_attributes() {
        let config = create_test_config();
        let pip = HttpPip::new(config).unwrap();

        assert_eq!(pip.provided_attributes().len(), 2);
        assert!(pip.provided_attributes().contains(&"department".to_string()));
    }
}
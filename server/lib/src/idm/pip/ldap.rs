//! LDAP Policy Information Point Implementation
//!
//! This module provides a PIP implementation that retrieves attributes from
//! LDAP servers.

#![allow(dead_code)]

use async_trait::async_trait;
use ldap3_client::{
    LdapClient, LdapClientBuilder, LdapEntry, proto::LdapFilter, proto::LdapSearchScope,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::prelude::*;
use super::{
    PipAttributeName, PipAttributeSet, PipAttributeValue, PipHealthCheck, PipHealthStatus,
    PipId, PipResult, PipSubject, PolicyInformationPoint,
};
use super::cache::PipAttributeCache;
use super::health::PipHealthState;
use super::config::{LdapPipConfig, PipFallbackBehavior};

/// LDAP PIP implementation
pub struct LdapPip {
    id: PipId,
    config: LdapPipConfig,
    cache: Arc<RwLock<PipAttributeCache>>,
    health_status: Arc<RwLock<PipHealthState>>,
    provided_attributes: Vec<String>,
    attribute_mapping: BTreeMap<String, String>,
}

impl LdapPip {
    /// Create a new LDAP PIP from configuration
    pub fn new(config: LdapPipConfig) -> Result<Self, String> {
        let cache = Arc::new(RwLock::new(PipAttributeCache::with_settings(
            Duration::from_secs(config.cache_ttl_secs),
            1000,
        )));

        let provided_attributes = if config.attributes.is_empty() {
            config.attribute_mapping.keys().cloned().collect()
        } else {
            config.attributes.clone()
        };

        let attribute_mapping = config.attribute_mapping.clone();

        Ok(LdapPip {
            id: PipId::new(&config.id),
            config,
            cache,
            health_status: Arc::new(RwLock::new(PipHealthState::new())),
            provided_attributes,
            attribute_mapping,
        })
    }

    /// Build LDAP client with configuration
    async fn build_client(&self) -> Result<LdapClient, String> {
        let builder = LdapClientBuilder::new(self.config.url.clone());

        let builder = if self.config.tls.allow_insecure {
            admin_warn!(
                "LDAP PIP '{}' is using insecure TLS - this should only be used for development!",
                self.config.id
            );
            builder.danger_accept_invalid_certs()
        } else {
            builder
        };

        builder
            .build()
            .await
            .map_err(|e| format!("Failed to create LDAP client: {:?}", e))
    }

    /// Build search filter from subject
    fn build_search_filter(&self, subject: &PipSubject) -> String {
        let mut filter = self.config.search_filter.clone();

        filter = filter.replace("{uuid}", &subject.uuid.to_string());
        if let Some(username) = &subject.username {
            filter = filter.replace("{username}", username);
        }
        if let Some(email) = &subject.email {
            filter = filter.replace("{email}", email);
        }
        for (key, value) in &subject.context {
            filter = filter.replace(&format!("{{{}}}", key), value);
        }

        filter
    }

    /// Parse LDAP entry attributes into PipAttributeSet
    fn parse_ldap_entry(&self, entry: &LdapEntry) -> PipAttributeSet {
        let mut attrs = PipAttributeSet::new();

        for (ldap_attr_name, values) in &entry.attrs {
            let pip_attr_name = self.attribute_mapping
                .get(ldap_attr_name)
                .cloned()
                .unwrap_or_else(|| ldap_attr_name.clone());

            let full_attr_name = PipAttributeName::new(&self.id, &pip_attr_name);

            if values.len() == 1 {
                if let Some(bs) = values.iter().next() {
                    let value_bytes: &[u8] = bs.as_ref();
                    let value_str = String::from_utf8_lossy(value_bytes).to_string();
                    attrs.insert(full_attr_name, PipAttributeValue::String(value_str));
                }
            } else if values.len() > 1 {
                let pip_values: Vec<PipAttributeValue> = values
                    .iter()
                    .map(|bs| {
                        let value_str = String::from_utf8_lossy(bs.as_ref()).to_string();
                        PipAttributeValue::String(value_str)
                    })
                    .collect();
                attrs.insert(full_attr_name, PipAttributeValue::Array(pip_values));
            }
        }

        attrs
    }

    /// Apply fallback values
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
impl PolicyInformationPoint for LdapPip {
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
                        reason: "LDAP PIP source is unhealthy".to_string(),
                        fallback_used: false,
                    };
                }
                PipFallbackBehavior::Ignore => {
                    return PipResult::Unavailable {
                        reason: "LDAP PIP source is unhealthy".to_string(),
                        fallback_used: false,
                    };
                }
            }
        }

        let client_result = self.build_client().await;

        match client_result {
            Ok(mut client) => {
                let bind_result = client
                    .bind(self.config.bind_dn.clone(), self.config.bind_password.clone())
                    .await;

                match bind_result {
                    Ok(_bind_response) => {
                        let filter_str = self.build_search_filter(subject);
                        let filter = parse_ldap_filter(&filter_str);

                        match filter {
                            Ok(ldap_filter) => {
                                let search_result = client
                                    .search(
                                        self.config.base_dn.clone(),
                                        ldap_filter,
                                    )
                                    .scope(LdapSearchScope::Subtree)
                                    .attrs(self.config.attributes.clone())
                                    .send()
                                    .await;

                                match search_result {
                                    Ok(entries) => {
                                        self.update_health_success().await;

                                        if entries.entries.is_empty() {
                                            match self.config.fallback_behavior {
                                                PipFallbackBehavior::UseFallback => {
                                                    PipResult::Success(self.apply_fallback())
                                                }
                                                PipFallbackBehavior::Deny => {
                                                    PipResult::Error {
                                                        error: "No LDAP entries found for subject".to_string(),
                                                        fallback_used: false,
                                                    }
                                                }
                                                PipFallbackBehavior::Ignore => {
                                                    PipResult::Success(PipAttributeSet::new())
                                                }
                                            }
                                        } else if let Some(entry) = entries.entries.first() {
                                            let attrs = self.parse_ldap_entry(entry);

                                            {
                                                let mut cache = self.cache.write().await;
                                                cache.put(&self.id, subject, attrs.clone());
                                            }

                                            PipResult::Success(attrs)
                                        } else {
                                            PipResult::Success(PipAttributeSet::new())
                                        }
                                    }
                                    Err(e) => {
                                        let error = format!("LDAP search failed: {:?}", e);
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
                            }
                            Err(e) => {
                                let error = format!("Failed to parse LDAP filter: {}", e);
                                self.update_health_failure(error.clone()).await;

                                PipResult::Error {
                                    error,
                                    fallback_used: false,
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let error = format!("LDAP bind failed: {:?}", e);
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
            }
            Err(e) => {
                self.update_health_failure(e.clone()).await;

                match self.config.fallback_behavior {
                    PipFallbackBehavior::UseFallback => {
                        PipResult::Unavailable {
                            reason: e,
                            fallback_used: true,
                        }
                    }
                    PipFallbackBehavior::Deny => {
                        PipResult::Unavailable {
                            reason: e,
                            fallback_used: false,
                        }
                    }
                    PipFallbackBehavior::Ignore => {
                        PipResult::Unavailable {
                            reason: e,
                            fallback_used: false,
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

        let client_result = self.build_client().await;

        match client_result {
            Ok(mut client) => {
                let bind_result = client
                    .bind(self.config.bind_dn.clone(), self.config.bind_password.clone())
                    .await;

                let latency_ms = start.elapsed().as_millis() as u64;

                match bind_result {
                    Ok(_response) => {
                        self.update_health_success().await;

                        PipHealthCheck {
                            pip_id: self.id.clone(),
                            status: PipHealthStatus::Healthy,
                            last_check: Instant::now(),
                            latency_ms: Some(latency_ms),
                            error_message: None,
                        }
                    }
                    Err(e) => {
                        let error = format!("LDAP health check bind failed: {:?}", e);
                        self.update_health_failure(error.clone()).await;

                        PipHealthCheck {
                            pip_id: self.id.clone(),
                            status: PipHealthStatus::Unhealthy,
                            last_check: Instant::now(),
                            latency_ms: Some(latency_ms),
                            error_message: Some(error),
                        }
                    }
                }
            }
            Err(e) => {
                self.update_health_failure(e.clone()).await;

                PipHealthCheck {
                    pip_id: self.id.clone(),
                    status: PipHealthStatus::Unhealthy,
                    last_check: Instant::now(),
                    latency_ms: None,
                    error_message: Some(e),
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

/// Parse LDAP filter string into LdapFilter
fn parse_ldap_filter(filter_str: &str) -> Result<LdapFilter, String> {
    ldap3_proto::filter::parse_ldap_filter_str(filter_str)
        .map_err(|e| format!("Failed to parse filter '{}': {:?}", filter_str, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use url::Url;

    fn create_test_config() -> LdapPipConfig {
        LdapPipConfig {
            id: "test_ldap_pip".to_string(),
            url: Url::parse("ldap://localhost:389").unwrap(),
            base_dn: "dc=example,dc=com".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret".to_string(),
            search_filter: "(uid={username})".to_string(),
            attributes: vec!["department".to_string(), "manager".to_string()],
            tls: super::super::config::PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            fallback_behavior: PipFallbackBehavior::UseFallback,
            fallback_values: BTreeMap::new(),
            health_check: super::super::config::PipHealthCheckConfig::default(),
            attribute_mapping: BTreeMap::new(),
        }
    }

    #[test]
    fn test_ldap_pip_creation() {
        let config = create_test_config();
        let pip = LdapPip::new(config);
        assert!(pip.is_ok());
    }

    #[test]
    fn test_search_filter_building() {
        let config = create_test_config();
        let pip = LdapPip::new(config).unwrap();

        let subject = PipSubject::from_uuid(uuid::Uuid::new_v4())
            .with_username("testuser");

        let filter = pip.build_search_filter(&subject);
        assert_eq!(filter, "(uid=testuser)");
    }

    #[test]
    fn test_provided_attributes() {
        let config = create_test_config();
        let pip = LdapPip::new(config).unwrap();

        assert_eq!(pip.provided_attributes().len(), 2);
        assert!(pip.provided_attributes().contains(&"department".to_string()));
    }

    #[test]
    fn test_parse_ldap_filter() {
        let result = parse_ldap_filter("(uid=testuser)");
        assert!(result.is_ok());

        let result = parse_ldap_filter("(&(objectClass=person)(uid=testuser))");
        assert!(result.is_ok());

        let result = parse_ldap_filter("invalid filter");
        assert!(result.is_err());
    }
}
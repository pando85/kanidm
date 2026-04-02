//! Policy Information Point (PIP) module for external attribute retrieval.
//!
//! This module provides a framework for querying external systems (HTTP REST APIs,
//! LDAP servers) for attribute values during authorization decisions. This enables
//! Kanidm to incorporate external context into policy decisions.

pub mod config;
pub mod http_client;
pub mod ldap_client;

use crate::prelude::*;
use kanidm_proto::internal::{
    PipAttributeValue, PipCacheEntry, PipCacheKey, PipHealthCheckResponse, PipOverallHealth,
    PipRequest, PipResponse, PipSourceHealth, PipSourceStatus, PipSourceType,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

pub use config::{PipConfig, PipSourceDefinition};
pub use http_client::HttpPipClient;
pub use ldap_client::LdapPipClient;

const DEFAULT_CACHE_TTL_SECONDS: u64 = 60;

pub trait PolicyInformationPoint: Send + Sync + std::fmt::Debug {
    fn source_type(&self) -> PipSourceType;
    fn source_name(&self) -> &str;
    fn retrieve_attributes(
        &self,
        request: &PipRequest,
        attributes: &[String],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<BTreeMap<String, String>, OperationError>> + Send + '_>,
    >;
    fn health_check(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = PipSourceStatus> + Send + '_>>;
}

pub enum PipSource {
    Http(HttpPipClient),
    Ldap(LdapPipClient),
}

impl PolicyInformationPoint for PipSource {
    fn source_type(&self) -> PipSourceType {
        match self {
            PipSource::Http(client) => client.source_type(),
            PipSource::Ldap(client) => client.source_type(),
        }
    }

    fn source_name(&self) -> &str {
        match self {
            PipSource::Http(client) => client.source_name(),
            PipSource::Ldap(client) => client.source_name(),
        }
    }

    fn retrieve_attributes(
        &self,
        request: &PipRequest,
        attributes: &[String],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<BTreeMap<String, String>, OperationError>> + Send + '_>,
    > {
        match self {
            PipSource::Http(client) => client.retrieve_attributes(request, attributes),
            PipSource::Ldap(client) => client.retrieve_attributes(request, attributes),
        }
    }

    fn health_check(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = PipSourceStatus> + Send + '_>> {
        match self {
            PipSource::Http(client) => client.health_check(),
            PipSource::Ldap(client) => client.health_check(),
        }
    }
}

impl std::fmt::Debug for PipSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipSource::Http(client) => client.fmt(f),
            PipSource::Ldap(client) => client.fmt(f),
        }
    }
}

pub struct PipCoordinator {
    sources: Vec<PipSource>,
    cache: Arc<RwLock<BTreeMap<PipCacheKey, PipCacheEntry>>>,
    config: PipConfig,
}

impl PipCoordinator {
    pub fn new(config: PipConfig) -> Self {
        let sources: Vec<PipSource> = config
            .sources
            .iter()
            .map(|source_def| {
                match source_def.source_type {
                    PipSourceType::Http => PipSource::Http(HttpPipClient::new(source_def.clone())),
                    PipSourceType::Ldap => PipSource::Ldap(LdapPipClient::new(source_def.clone())),
                }
            })
            .collect();

        Self {
            sources,
            cache: Arc::new(RwLock::new(BTreeMap::new())),
            config,
        }
    }

    pub async fn retrieve_attributes(&self, request: &PipRequest) -> PipResponse {
        let mut response = PipResponse::new();
        let current_time = duration_from_epoch_now().as_secs();

        for attribute_name in &request.attributes_requested {
            let cache_key = PipCacheKey {
                subject: request.subject,
                resource: request.resource,
                attribute_name: attribute_name.clone(),
            };

            if let Some(cached_value) = self.get_cached_value(&cache_key, current_time).await {
                response = response.with_attribute(
                    attribute_name.clone(),
                    PipAttributeValue {
                        value: cached_value.value.clone(),
                        source: cached_value.source.clone(),
                        cached: true,
                        retrieved_at: cached_value.cached_at,
                    },
                );
                continue;
            }

            let (value, source_name, status) =
                self.fetch_from_sources(request, attribute_name).await;

            if let Some(v) = value {
                let cache_entry = PipCacheEntry {
                    key: cache_key.clone(),
                    value: v.clone(),
                    source: source_name.clone(),
                    cached_at: current_time,
                    ttl_seconds: DEFAULT_CACHE_TTL_SECONDS,
                };
                self.cache_value(cache_key, cache_entry).await;

                response = response.with_attribute(
                    attribute_name.clone(),
                    PipAttributeValue {
                        value: v,
                        source: source_name.clone(),
                        cached: false,
                        retrieved_at: current_time,
                    },
                );
            }

            response = response.with_source_status(source_name, status);
        }

        response
    }

    async fn get_cached_value(
        &self,
        key: &PipCacheKey,
        current_time: u64,
    ) -> Option<PipCacheEntry> {
        let cache = self.cache.read().await;
        cache.get(key).and_then(|entry| {
            if !entry.is_expired(current_time) {
                Some(entry.clone())
            } else {
                None
            }
        })
    }

    async fn cache_value(&self, key: PipCacheKey, entry: PipCacheEntry) {
        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }

    async fn fetch_from_sources(
        &self,
        request: &PipRequest,
        attribute_name: &str,
    ) -> (Option<String>, String, PipSourceStatus) {
        for source in &self.sources {
            let result = source
                .retrieve_attributes(request, &[attribute_name.to_string()])
                .await;

            match result {
                Ok(attrs) => {
                    if let Some(value) = attrs.get(attribute_name) {
                        return (
                            Some(value.clone()),
                            source.source_name().to_string(),
                            PipSourceStatus::Success,
                        );
                    }
                }
                Err(OperationError::KG001TaskTimeout) => {
                    return (
                        None,
                        source.source_name().to_string(),
                        PipSourceStatus::Timeout,
                    );
                }
                Err(_) => {
                    return (
                        None,
                        source.source_name().to_string(),
                        PipSourceStatus::Error,
                    );
                }
            }
        }

        (None, "unknown".to_string(), PipSourceStatus::Unavailable)
    }

    pub async fn health_check(&self) -> PipHealthCheckResponse {
        let current_time = duration_from_epoch_now().as_secs();
        let mut sources_health = BTreeMap::new();
        let mut healthy_count = 0;
        let mut _degraded_count = 0;

        for source in &self.sources {
            let start = Instant::now();
            let status = source.health_check().await;
            let latency_ms = start.elapsed().as_millis() as u64;

            if status == PipSourceStatus::Success || status == PipSourceStatus::Cached {
                healthy_count += 1;
            } else if status == PipSourceStatus::Timeout {
                _degraded_count += 1;
            }

            sources_health.insert(
                source.source_name().to_string(),
                PipSourceHealth {
                    source_type: source.source_type(),
                    uri: self.get_source_uri(source.source_name()),
                    status,
                    last_check: current_time,
                    latency_ms: Some(latency_ms),
                    error_message: None,
                },
            );
        }

        let overall_status = if healthy_count == self.sources.len() {
            PipOverallHealth::Healthy
        } else if healthy_count > 0 {
            PipOverallHealth::Degraded
        } else {
            PipOverallHealth::Unhealthy
        };

        PipHealthCheckResponse {
            sources: sources_health,
            overall_status,
        }
    }

    fn get_source_uri(&self, source_name: &str) -> String {
        self.config
            .sources
            .iter()
            .find(|s| s.name == source_name)
            .map(|s| s.uri.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

impl std::fmt::Debug for PipCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipCoordinator")
            .field("sources_count", &self.sources.len())
            .field("config", &self.config)
            .finish()
    }
}
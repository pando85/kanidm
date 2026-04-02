//! Policy Information Point (PIP) Framework
//!
//! This module provides a framework for retrieving external attributes from various
//! sources (HTTP REST APIs, LDAP servers, etc.) during authorization decisions.
//!
//! The PIP framework allows Kanidm to incorporate attributes from external systems
//! like HR databases, CRM systems, asset management tools, and risk signals from
//! SIEM/UEBA systems into access control decisions.
//!
//! NOTE: This framework is implemented but awaiting integration into the authorization
//! flow. Dead code warnings are suppressed until integration is complete.

#![allow(dead_code)]

use crate::prelude::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub mod cache;
pub mod config;
pub mod health;
pub mod http;
pub mod ldap;

pub use cache::PipAttributeCache;
#[allow(unused_imports)]
pub use config::{PipConfig, PipFallbackBehavior, PipSourceConfig};
#[allow(unused_imports)]
pub use http::HttpPip;
#[allow(unused_imports)]
pub use ldap::LdapPip;

/// Unique identifier for a PIP instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct PipId(String);

impl PipId {
    pub fn new(id: impl Into<String>) -> Self {
        PipId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for PipId {
    fn from(s: String) -> Self {
        PipId::new(s)
    }
}

impl From<&str> for PipId {
    fn from(s: &str) -> Self {
        PipId::new(s)
    }
}

/// External attribute name with namespace prefix to prevent collisions
/// with internal Kanidm attributes.
///
/// Format: `pip:<source_id>:<attribute_name>`
/// Example: `pip:hr_system:department`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct PipAttributeName(String);

impl PipAttributeName {
    const PREFIX: &'static str = "pip:";

    pub fn new(source_id: &PipId, attribute_name: &str) -> Self {
        PipAttributeName(format!(
            "{}{}:{}",
            Self::PREFIX,
            source_id.as_str(),
            attribute_name
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_pip_attribute(name: &str) -> bool {
        name.starts_with(Self::PREFIX)
    }

    pub fn parse(name: &str) -> Option<(PipId, String)> {
        if !Self::is_pip_attribute(name) {
            return None;
        }
        let rest = name.strip_prefix(Self::PREFIX)?;
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        let (first, second) = parts.first().zip(parts.get(1))?;
        Some((PipId::new(*first), (*second).to_string()))
    }
}

/// A single attribute value retrieved from a PIP
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PipAttributeValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<PipAttributeValue>),
    Json(serde_json::Value),
}

impl PipAttributeValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PipAttributeValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            PipAttributeValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PipAttributeValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[PipAttributeValue]> {
        match self {
            PipAttributeValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            PipAttributeValue::Json(v) => Some(v),
            _ => None,
        }
    }
}

/// A set of attributes retrieved from a PIP for a specific subject
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipAttributeSet {
    attributes: BTreeMap<PipAttributeName, PipAttributeValue>,
}

impl PipAttributeSet {
    pub fn new() -> Self {
        PipAttributeSet {
            attributes: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: PipAttributeName, value: PipAttributeValue) {
        self.attributes.insert(name, value);
    }

    pub fn get(&self, name: &PipAttributeName) -> Option<&PipAttributeValue> {
        self.attributes.get(name)
    }

    pub fn contains(&self, name: &PipAttributeName) -> bool {
        self.attributes.contains_key(name)
    }

    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PipAttributeName, &PipAttributeValue)> {
        self.attributes.iter()
    }

    pub fn merge(&mut self, other: PipAttributeSet) {
        for (name, value) in other.attributes {
            self.attributes.insert(name, value);
        }
    }
}

/// Subject identifier for PIP attribute queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipSubject {
    /// UUID of the user/entry being queried
    pub uuid: Uuid,
    /// Optional username
    pub username: Option<String>,
    /// Optional email
    pub email: Option<String>,
    /// Additional context attributes for the query
    pub context: BTreeMap<String, String>,
}

impl PipSubject {
    pub fn from_uuid(uuid: Uuid) -> Self {
        PipSubject {
            uuid,
            username: None,
            email: None,
            context: BTreeMap::new(),
        }
    }

    pub fn with_username(self, username: impl Into<String>) -> Self {
        PipSubject {
            username: Some(username.into()),
            ..self
        }
    }

    pub fn with_email(self, email: impl Into<String>) -> Self {
        PipSubject {
            email: Some(email.into()),
            ..self
        }
    }

    pub fn with_context(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut context = self.context;
        context.insert(key.into(), value.into());
        PipSubject { context, ..self }
    }
}

/// Result of a PIP attribute retrieval operation
#[derive(Debug)]
pub enum PipResult {
    /// Successfully retrieved attributes
    Success(PipAttributeSet),
    /// PIP source is unavailable/unhealthy
    Unavailable { reason: String, fallback_used: bool },
    /// Timeout occurred during retrieval
    Timeout { fallback_used: bool },
    /// Error occurred during retrieval
    Error { error: String, fallback_used: bool },
}

impl PipResult {
    pub fn is_success(&self) -> bool {
        matches!(self, PipResult::Success(_))
    }

    pub fn attributes(&self) -> Option<&PipAttributeSet> {
        match self {
            PipResult::Success(attrs) => Some(attrs),
            _ => None,
        }
    }

    pub fn fallback_used(&self) -> bool {
        match self {
            PipResult::Unavailable { fallback_used, .. } => *fallback_used,
            PipResult::Timeout { fallback_used } => *fallback_used,
            PipResult::Error { fallback_used, .. } => *fallback_used,
            PipResult::Success(_) => false,
        }
    }
}

/// Health status of a PIP source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl PipHealthStatus {
    pub fn is_healthy(self) -> bool {
        matches!(self, PipHealthStatus::Healthy)
    }

    pub fn can_retrieve(self) -> bool {
        matches!(self, PipHealthStatus::Healthy | PipHealthStatus::Degraded)
    }
}

/// Health check result for a PIP
#[derive(Debug, Clone)]
pub struct PipHealthCheck {
    pub pip_id: PipId,
    pub status: PipHealthStatus,
    pub last_check: Instant,
    pub latency_ms: Option<u64>,
    pub error_message: Option<String>,
}

/// The Policy Information Point trait that all PIP implementations must implement.
///
/// This trait defines the interface for retrieving external attributes during
/// authorization decisions.
#[async_trait]
pub trait PolicyInformationPoint: Send + Sync {
    /// Unique identifier for this PIP instance
    fn id(&self) -> &PipId;

    /// Retrieve attributes for the given subject.
    ///
    /// This method should:
    /// - Query the external source for attributes
    /// - Handle timeouts appropriately
    /// - Apply fallback values if configured
    /// - Return cached values if available and within TTL
    async fn retrieve_attributes(&self, subject: &PipSubject) -> PipResult;

    /// Retrieve specific named attributes for the given subject.
    ///
    /// If the PIP supports selective attribute retrieval, this can be more efficient
    /// than retrieving all attributes.
    async fn retrieve_named_attributes(
        &self,
        subject: &PipSubject,
        attribute_names: &[String],
    ) -> PipResult;

    /// Perform a health check on the PIP source.
    ///
    /// This should verify connectivity and basic functionality without
    /// performing a full attribute retrieval.
    async fn health_check(&self) -> PipHealthCheck;

    /// Get the current cached health status without performing a check.
    fn cached_health_status(&self) -> PipHealthStatus;

    /// Get the list of attribute names this PIP can provide.
    fn provided_attributes(&self) -> &[String];

    /// Clear any cached attributes for the given subject.
    async fn clear_cache(&self, subject: &PipSubject);

    /// Clear all cached attributes.
    async fn clear_all_cache(&self);
}

/// Manager for multiple PIP instances
pub struct PipManager {
    pips: BTreeMap<PipId, Arc<dyn PolicyInformationPoint>>,
    cache: Arc<RwLock<PipAttributeCache>>,
}

impl PipManager {
    pub fn new(cache_ttl: Duration) -> Self {
        PipManager {
            pips: BTreeMap::new(),
            cache: Arc::new(RwLock::new(PipAttributeCache::new(cache_ttl))),
        }
    }

    pub fn register(&mut self, pip: Arc<dyn PolicyInformationPoint>) {
        self.pips.insert(pip.id().clone(), pip);
    }

    pub fn get(&self, id: &PipId) -> Option<Arc<dyn PolicyInformationPoint>> {
        self.pips.get(id).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PipId, Arc<dyn PolicyInformationPoint>)> {
        self.pips.iter().map(|(id, pip)| (id, pip.clone()))
    }

    /// Retrieve attributes from all registered PIPs for the given subject.
    ///
    /// Attributes from all healthy PIPs are merged into a single set.
    pub async fn retrieve_all_attributes(&self, subject: &PipSubject) -> PipAttributeSet {
        let mut all_attributes = PipAttributeSet::new();

        for pip in self.pips.values() {
            if !pip.cached_health_status().can_retrieve() {
                continue;
            }

            let result = pip.retrieve_attributes(subject).await;
            if let Some(attrs) = result.attributes() {
                all_attributes.merge(attrs.clone());
            }
        }

        all_attributes
    }

    /// Retrieve specific attributes from the appropriate PIP(s).
    ///
    /// Parses the attribute names to determine which PIP(s) to query.
    pub async fn retrieve_named_attributes(
        &self,
        subject: &PipSubject,
        attribute_names: &[String],
    ) -> PipAttributeSet {
        let mut all_attributes = PipAttributeSet::new();

        for attr_name in attribute_names {
            if let Some((pip_id, internal_name)) = PipAttributeName::parse(attr_name) {
                if let Some(pip) = self.pips.get(&pip_id) {
                    if pip.cached_health_status().can_retrieve() {
                        let result = pip
                            .retrieve_named_attributes(
                                subject,
                                std::slice::from_ref(&internal_name),
                            )
                            .await;
                        if let Some(attrs) = result.attributes() {
                            all_attributes.merge(attrs.clone());
                        }
                    }
                }
            }
        }

        all_attributes
    }

    /// Perform health checks on all registered PIPs.
    pub async fn health_check_all(&self) -> Vec<PipHealthCheck> {
        let mut checks = Vec::new();
        for pip in self.pips.values() {
            let check = pip.health_check().await;
            checks.push(check);
        }
        checks
    }

    /// Get health status summary for all PIPs
    pub fn health_summary(&self) -> BTreeMap<PipId, PipHealthStatus> {
        self.pips
            .iter()
            .map(|(id, pip)| (id.clone(), pip.cached_health_status()))
            .collect()
    }

    /// Clear all cached attributes across all PIPs
    pub async fn clear_all_cache(&self) {
        for pip in self.pips.values() {
            pip.clear_all_cache().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_attribute_name_format() {
        let pip_id = PipId::new("hr_system");
        let attr_name = PipAttributeName::new(&pip_id, "department");

        assert_eq!(attr_name.as_str(), "pip:hr_system:department");
        assert!(PipAttributeName::is_pip_attribute(
            "pip:hr_system:department"
        ));
        assert!(!PipAttributeName::is_pip_attribute("department"));
    }

    #[test]
    fn test_pip_attribute_name_parse() {
        let parsed = PipAttributeName::parse("pip:hr_system:department");
        assert!(parsed.is_some());

        let (pip_id, internal_name) = parsed.unwrap();
        assert_eq!(pip_id.as_str(), "hr_system");
        assert_eq!(internal_name, "department");

        let invalid = PipAttributeName::parse("department");
        assert!(invalid.is_none());
    }

    #[test]
    fn test_pip_attribute_set() {
        let mut set = PipAttributeSet::new();
        let pip_id = PipId::new("hr_system");

        let dept_attr = PipAttributeName::new(&pip_id, "department");
        set.insert(
            dept_attr.clone(),
            PipAttributeValue::String("Engineering".to_string()),
        );

        assert!(set.contains(&dept_attr));
        assert_eq!(set.len(), 1);
        assert_eq!(set.get(&dept_attr).unwrap().as_str(), Some("Engineering"));
    }

    #[test]
    fn test_pip_attribute_value_types() {
        let string_val = PipAttributeValue::String("test".to_string());
        assert_eq!(string_val.as_str(), Some("test"));

        let int_val = PipAttributeValue::Integer(42);
        assert_eq!(int_val.as_int(), Some(42));

        let bool_val = PipAttributeValue::Boolean(true);
        assert_eq!(bool_val.as_bool(), Some(true));

        let array_val = PipAttributeValue::Array(vec![
            PipAttributeValue::String("a".to_string()),
            PipAttributeValue::String("b".to_string()),
        ]);
        assert_eq!(array_val.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_pip_subject() {
        let subject = PipSubject::from_uuid(Uuid::new_v4())
            .with_username("testuser")
            .with_email("test@example.com")
            .with_context("device_id", "device123");

        assert!(subject.username.is_some());
        assert!(subject.email.is_some());
        assert!(subject.context.contains_key("device_id"));
    }

    #[test]
    fn test_pip_id_from_string() {
        let pip_id = PipId::from("test_pip".to_string());
        assert_eq!(pip_id.as_str(), "test_pip");

        let pip_id_ref: PipId = "another_pip".into();
        assert_eq!(pip_id_ref.as_str(), "another_pip");
    }

    #[test]
    fn test_pip_attribute_name_edge_cases() {
        let pip_id = PipId::new("source_with_underscores");
        let attr_name = PipAttributeName::new(&pip_id, "attr_with_special_chars_123");
        assert_eq!(
            attr_name.as_str(),
            "pip:source_with_underscores:attr_with_special_chars_123"
        );

        let parsed = PipAttributeName::parse(attr_name.as_str());
        assert!(parsed.is_some());
    }

    #[test]
    fn test_pip_attribute_name_invalid_parse() {
        assert!(PipAttributeName::parse("pip:").is_none());
        assert!(PipAttributeName::parse("pip:source").is_none());
        assert!(PipAttributeName::parse("no_prefix_here").is_none());
        assert!(PipAttributeName::parse("").is_none());
    }

    #[test]
    fn test_pip_attribute_set_operations() {
        let mut set = PipAttributeSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);

        let pip_id = PipId::new("source1");
        let attr1 = PipAttributeName::new(&pip_id, "attr1");
        let attr2 = PipAttributeName::new(&pip_id, "attr2");

        set.insert(attr1.clone(), PipAttributeValue::String("value1".to_string()));
        set.insert(attr2.clone(), PipAttributeValue::String("value2".to_string()));

        assert_eq!(set.len(), 2);
        assert!(set.contains(&attr1));
        assert!(set.contains(&attr2));

        let iter_count = set.iter().count();
        assert_eq!(iter_count, 2);
    }

    #[test]
    fn test_pip_attribute_set_merge() {
        let mut set1 = PipAttributeSet::new();
        let pip_id1 = PipId::new("source1");
        let attr1 = PipAttributeName::new(&pip_id1, "attr1");
        set1.insert(attr1.clone(), PipAttributeValue::String("value1".to_string()));

        let mut set2 = PipAttributeSet::new();
        let pip_id2 = PipId::new("source2");
        let attr2 = PipAttributeName::new(&pip_id2, "attr2");
        set2.insert(attr2.clone(), PipAttributeValue::String("value2".to_string()));

        set1.merge(set2);

        assert_eq!(set1.len(), 2);
        assert!(set1.contains(&attr1));
        assert!(set1.contains(&attr2));
    }

    #[test]
    fn test_pip_attribute_set_merge_conflict() {
        let mut set1 = PipAttributeSet::new();
        let pip_id = PipId::new("source");
        let attr = PipAttributeName::new(&pip_id, "same_attr");
        set1.insert(attr.clone(), PipAttributeValue::String("value1".to_string()));

        let mut set2 = PipAttributeSet::new();
        set2.insert(attr.clone(), PipAttributeValue::String("value2".to_string()));

        set1.merge(set2);

        assert_eq!(set1.len(), 1);
        assert_eq!(set1.get(&attr).unwrap().as_str(), Some("value2"));
    }

    #[test]
    fn test_pip_attribute_value_json() {
        let json_val = PipAttributeValue::Json(serde_json::json!({"key": "value"}));
        assert!(json_val.as_json().is_some());
        let json = json_val.as_json().unwrap();
        assert_eq!(json.get("key").unwrap().as_str(), Some("value"));

        let string_val = PipAttributeValue::String("test".to_string());
        assert!(string_val.as_json().is_none());
    }

    #[test]
    fn test_pip_attribute_value_array_operations() {
        let array_val = PipAttributeValue::Array(vec![
            PipAttributeValue::Integer(1),
            PipAttributeValue::Integer(2),
            PipAttributeValue::Integer(3),
        ]);

        let arr = array_val.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_int(), Some(1));
        assert_eq!(arr[2].as_int(), Some(3));
    }

    #[test]
    fn test_pip_result_success() {
        let mut set = PipAttributeSet::new();
        let pip_id = PipId::new("test");
        let attr = PipAttributeName::new(&pip_id, "test_attr");
        set.insert(attr, PipAttributeValue::String("value".to_string()));

        let result = PipResult::Success(set);
        assert!(result.is_success());
        assert!(result.attributes().is_some());
        assert!(!result.fallback_used());
    }

    #[test]
    fn test_pip_result_unavailable() {
        let result = PipResult::Unavailable {
            reason: "Connection timeout".to_string(),
            fallback_used: true,
        };

        assert!(!result.is_success());
        assert!(result.fallback_used());
        assert!(result.attributes().is_none());
    }

    #[test]
    fn test_pip_result_timeout() {
        let result = PipResult::Timeout { fallback_used: false };
        assert!(!result.is_success());
        assert!(!result.fallback_used());
    }

    #[test]
    fn test_pip_result_error() {
        let result = PipResult::Error {
            error: "Invalid response format".to_string(),
            fallback_used: false,
        };

        assert!(!result.is_success());
        assert!(!result.fallback_used());
    }

    #[test]
    fn test_pip_health_status() {
        assert!(PipHealthStatus::Healthy.is_healthy());
        assert!(PipHealthStatus::Healthy.can_retrieve());
        assert!(PipHealthStatus::Degraded.can_retrieve());
        assert!(!PipHealthStatus::Degraded.is_healthy());
        assert!(!PipHealthStatus::Unhealthy.can_retrieve());
        assert!(!PipHealthStatus::Unhealthy.is_healthy());
        assert!(!PipHealthStatus::Unknown.is_healthy());
        assert!(!PipHealthStatus::Unknown.can_retrieve());
    }

    #[test]
    fn test_pip_subject_builder() {
        let uuid = Uuid::new_v4();
        let subject = PipSubject::from_uuid(uuid)
            .with_username("admin")
            .with_email("admin@example.org")
            .with_context("tenant", "tenant_a")
            .with_context("ip", "192.168.1.1");

        assert_eq!(subject.uuid, uuid);
        assert_eq!(subject.username.as_ref().unwrap(), "admin");
        assert_eq!(subject.email.as_ref().unwrap(), "admin@example.org");
        assert_eq!(subject.context.len(), 2);
    }

    #[test]
    fn test_pip_health_check_struct() {
        let health_check = PipHealthCheck {
            pip_id: PipId::new("test_pip"),
            status: PipHealthStatus::Healthy,
            last_check: Instant::now(),
            latency_ms: Some(50),
            error_message: None,
        };

        assert_eq!(health_check.pip_id.as_str(), "test_pip");
        assert!(health_check.status.is_healthy());
        assert!(health_check.latency_ms.is_some());
    }

    #[test]
    fn test_pip_manager_basic() {
        let manager = PipManager::new(std::time::Duration::from_secs(60));
        assert!(manager.get(&PipId::new("nonexistent")).is_none());
    }

    #[test]
    fn test_pip_attribute_name_with_empty_components() {
        let pip_id = PipId::new("source");
        let _attr = PipAttributeName::new(&pip_id, "attribute");
        let parsed = PipAttributeName::parse("pip::attribute");
        assert!(parsed.is_some());
        let (empty_id, attr) = parsed.unwrap();
        assert_eq!(empty_id.as_str(), "");
        assert_eq!(attr, "attribute");
    }

    #[test]
    fn test_pip_attribute_set_get_missing() {
        let set = PipAttributeSet::new();
        let pip_id = PipId::new("source");
        let attr = PipAttributeName::new(&pip_id, "missing_attr");
        assert!(set.get(&attr).is_none());
    }

    #[test]
    fn test_pip_subject_default_context() {
        let subject = PipSubject::from_uuid(Uuid::new_v4());
        assert!(subject.username.is_none());
        assert!(subject.email.is_none());
        assert!(subject.context.is_empty());
    }
}

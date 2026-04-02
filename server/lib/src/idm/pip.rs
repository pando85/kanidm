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
    fn test_pip_id_creation() {
        let id = PipId::new("test_pip");
        assert_eq!(id.as_str(), "test_pip");

        let id_from_str: PipId = "another_pip".into();
        assert_eq!(id_from_str.as_str(), "another_pip");

        let id_from_string: PipId = String::from("string_pip").into();
        assert_eq!(id_from_string.as_str(), "string_pip");
    }

    #[test]
    fn test_pip_id_equality_and_ordering() {
        let id1 = PipId::new("alpha");
        let id2 = PipId::new("beta");
        let id3 = PipId::new("alpha");

        assert_eq!(id1, id3);
        assert_ne!(id1, id2);
        assert!(id1 < id2);
    }

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
    fn test_pip_attribute_name_parse_valid() {
        let parsed = PipAttributeName::parse("pip:hr_system:department");
        assert!(parsed.is_some());

        let (pip_id, internal_name) = parsed.unwrap();
        assert_eq!(pip_id.as_str(), "hr_system");
        assert_eq!(internal_name, "department");
    }

    #[test]
    fn test_pip_attribute_name_parse_invalid() {
        assert!(PipAttributeName::parse("department").is_none());
        assert!(PipAttributeName::parse("pip:").is_none());
        assert!(PipAttributeName::parse("pip:only_source").is_none());
        assert!(PipAttributeName::parse("").is_none());
    }

    #[test]
    fn test_pip_attribute_name_special_characters() {
        let pip_id = PipId::new("source-with-dashes");
        let attr_name = PipAttributeName::new(&pip_id, "attr_with_underscores");
        assert_eq!(
            attr_name.as_str(),
            "pip:source-with-dashes:attr_with_underscores"
        );

        let parsed = PipAttributeName::parse("pip:source-with-dashes:attr_with_underscores");
        assert!(parsed.is_some());
    }

    #[test]
    fn test_pip_attribute_set_insert_and_get() {
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
    fn test_pip_attribute_set_multiple_attributes() {
        let mut set = PipAttributeSet::new();
        let pip_id = PipId::new("hr_system");

        set.insert(
            PipAttributeName::new(&pip_id, "department"),
            PipAttributeValue::String("Engineering".to_string()),
        );
        set.insert(
            PipAttributeName::new(&pip_id, "role"),
            PipAttributeValue::String("Developer".to_string()),
        );
        set.insert(
            PipAttributeName::new(&pip_id, "level"),
            PipAttributeValue::Integer(5),
        );

        assert_eq!(set.len(), 3);
        assert!(set.contains(&PipAttributeName::new(&pip_id, "department")));
        assert!(set.contains(&PipAttributeName::new(&pip_id, "role")));
        assert!(set.contains(&PipAttributeName::new(&pip_id, "level")));
    }

    #[test]
    fn test_pip_attribute_set_merge() {
        let mut set1 = PipAttributeSet::new();
        let pip_id = PipId::new("source1");
        set1.insert(
            PipAttributeName::new(&pip_id, "attr1"),
            PipAttributeValue::String("value1".to_string()),
        );

        let mut set2 = PipAttributeSet::new();
        let pip_id2 = PipId::new("source2");
        set2.insert(
            PipAttributeName::new(&pip_id2, "attr2"),
            PipAttributeValue::String("value2".to_string()),
        );

        set1.merge(set2);
        assert_eq!(set1.len(), 2);
        assert!(set1.contains(&PipAttributeName::new(&pip_id, "attr1")));
        assert!(set1.contains(&PipAttributeName::new(&pip_id2, "attr2")));
    }

    #[test]
    fn test_pip_attribute_set_empty() {
        let set = PipAttributeSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_pip_attribute_set_default() {
        let set = PipAttributeSet::default();
        assert!(set.is_empty());
    }

    #[test]
    fn test_pip_attribute_value_string() {
        let val = PipAttributeValue::String("test".to_string());
        assert_eq!(val.as_str(), Some("test"));
        assert_eq!(val.as_int(), None);
        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_array(), None);
        assert_eq!(val.as_json(), None);
    }

    #[test]
    fn test_pip_attribute_value_integer() {
        let val = PipAttributeValue::Integer(42);
        assert_eq!(val.as_int(), Some(42));
        assert_eq!(val.as_str(), None);
        assert_eq!(val.as_bool(), None);
    }

    #[test]
    fn test_pip_attribute_value_boolean() {
        let val_true = PipAttributeValue::Boolean(true);
        assert_eq!(val_true.as_bool(), Some(true));

        let val_false = PipAttributeValue::Boolean(false);
        assert_eq!(val_false.as_bool(), Some(false));
    }

    #[test]
    fn test_pip_attribute_value_array() {
        let arr = vec![
            PipAttributeValue::String("a".to_string()),
            PipAttributeValue::Integer(1),
            PipAttributeValue::Boolean(true),
        ];
        let val = PipAttributeValue::Array(arr);
        assert_eq!(val.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_pip_attribute_value_json() {
        let json = serde_json::json!({"key": "value"});
        let val = PipAttributeValue::Json(json.clone());
        assert_eq!(val.as_json(), Some(&json));
    }

    #[test]
    fn test_pip_attribute_value_equality() {
        assert_eq!(
            PipAttributeValue::String("test".to_string()),
            PipAttributeValue::String("test".to_string())
        );
        assert_ne!(
            PipAttributeValue::String("test".to_string()),
            PipAttributeValue::String("other".to_string())
        );
        assert_eq!(
            PipAttributeValue::Integer(42),
            PipAttributeValue::Integer(42)
        );
        assert_eq!(
            PipAttributeValue::Boolean(true),
            PipAttributeValue::Boolean(true)
        );
    }

    #[test]
    fn test_pip_subject_from_uuid() {
        let uuid = Uuid::new_v4();
        let subject = PipSubject::from_uuid(uuid);
        assert_eq!(subject.uuid, uuid);
        assert!(subject.username.is_none());
        assert!(subject.email.is_none());
        assert!(subject.context.is_empty());
    }

    #[test]
    fn test_pip_subject_with_username() {
        let subject = PipSubject::from_uuid(Uuid::new_v4()).with_username("testuser");
        assert_eq!(subject.username, Some("testuser".to_string()));
    }

    #[test]
    fn test_pip_subject_with_email() {
        let subject = PipSubject::from_uuid(Uuid::new_v4()).with_email("test@example.com");
        assert_eq!(subject.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_pip_subject_with_context() {
        let subject = PipSubject::from_uuid(Uuid::new_v4())
            .with_context("device_id", "device123")
            .with_context("location", "office");
        assert_eq!(
            subject.context.get("device_id"),
            Some(&"device123".to_string())
        );
        assert_eq!(subject.context.get("location"), Some(&"office".to_string()));
    }

    #[test]
    fn test_pip_subject_full() {
        let subject = PipSubject::from_uuid(Uuid::new_v4())
            .with_username("testuser")
            .with_email("test@example.com")
            .with_context("device_id", "device123");

        assert!(subject.username.is_some());
        assert!(subject.email.is_some());
        assert!(subject.context.contains_key("device_id"));
    }

    #[test]
    fn test_pip_result_success() {
        let attrs = PipAttributeSet::new();
        let result = PipResult::Success(attrs);
        assert!(result.is_success());
        assert!(result.attributes().is_some());
        assert!(!result.fallback_used());
    }

    #[test]
    fn test_pip_result_unavailable() {
        let result = PipResult::Unavailable {
            reason: "source down".to_string(),
            fallback_used: true,
        };
        assert!(!result.is_success());
        assert!(result.attributes().is_none());
        assert!(result.fallback_used());
    }

    #[test]
    fn test_pip_result_timeout() {
        let result = PipResult::Timeout {
            fallback_used: false,
        };
        assert!(!result.is_success());
        assert!(result.attributes().is_none());
        assert!(!result.fallback_used());
    }

    #[test]
    fn test_pip_result_error() {
        let result = PipResult::Error {
            error: "connection failed".to_string(),
            fallback_used: true,
        };
        assert!(!result.is_success());
        assert!(result.attributes().is_none());
        assert!(result.fallback_used());
    }

    #[test]
    fn test_pip_health_status_healthy() {
        let status = PipHealthStatus::Healthy;
        assert!(status.is_healthy());
        assert!(status.can_retrieve());
    }

    #[test]
    fn test_pip_health_status_degraded() {
        let status = PipHealthStatus::Degraded;
        assert!(!status.is_healthy());
        assert!(status.can_retrieve());
    }

    #[test]
    fn test_pip_health_status_unhealthy() {
        let status = PipHealthStatus::Unhealthy;
        assert!(!status.is_healthy());
        assert!(!status.can_retrieve());
    }

    #[test]
    fn test_pip_health_status_unknown() {
        let status = PipHealthStatus::Unknown;
        assert!(!status.is_healthy());
        assert!(!status.can_retrieve());
    }

    #[test]
    fn test_pip_health_check_fields() {
        let check = PipHealthCheck {
            pip_id: PipId::new("test"),
            status: PipHealthStatus::Healthy,
            last_check: Instant::now(),
            latency_ms: Some(50),
            error_message: None,
        };
        assert_eq!(check.pip_id.as_str(), "test");
        assert!(check.status.is_healthy());
        assert!(check.latency_ms.is_some());
        assert!(check.error_message.is_none());
    }

    #[test]
    fn test_pip_health_check_with_error() {
        let check = PipHealthCheck {
            pip_id: PipId::new("test"),
            status: PipHealthStatus::Unhealthy,
            last_check: Instant::now(),
            latency_ms: None,
            error_message: Some("connection timeout".to_string()),
        };
        assert!(check.error_message.is_some());
        assert!(check.latency_ms.is_none());
    }

    #[test]
    fn test_pip_manager_new() {
        let manager = PipManager::new(Duration::from_secs(300));
        assert!(manager.health_summary().is_empty());
    }

    #[test]
    fn test_pip_manager_register_and_get() {
        let mut manager = PipManager::new(Duration::from_secs(300));
        let mock_pip = Arc::new(MockPip::new(PipId::new("test_pip")));
        manager.register(mock_pip.clone());

        assert!(manager.get(&PipId::new("test_pip")).is_some());
        assert!(manager.get(&PipId::new("nonexistent")).is_none());
    }

    #[test]
    fn test_pip_manager_iter() {
        let mut manager = PipManager::new(Duration::from_secs(300));
        manager.register(Arc::new(MockPip::new(PipId::new("pip1"))));
        manager.register(Arc::new(MockPip::new(PipId::new("pip2"))));

        let count = manager.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_pip_manager_health_summary() {
        let mut manager = PipManager::new(Duration::from_secs(300));
        manager.register(Arc::new(MockPip::new(PipId::new("healthy_pip"))));

        let summary = manager.health_summary();
        assert_eq!(summary.len(), 1);
        assert_eq!(
            summary.get(&PipId::new("healthy_pip")),
            Some(&PipHealthStatus::Unknown)
        );
    }

    struct MockPip {
        id: PipId,
    }

    impl MockPip {
        fn new(id: PipId) -> Self {
            MockPip { id }
        }
    }

    #[async_trait]
    impl PolicyInformationPoint for MockPip {
        fn id(&self) -> &PipId {
            &self.id
        }

        async fn retrieve_attributes(&self, _subject: &PipSubject) -> PipResult {
            PipResult::Success(PipAttributeSet::new())
        }

        async fn retrieve_named_attributes(
            &self,
            _subject: &PipSubject,
            _attribute_names: &[String],
        ) -> PipResult {
            PipResult::Success(PipAttributeSet::new())
        }

        async fn health_check(&self) -> PipHealthCheck {
            PipHealthCheck {
                pip_id: self.id.clone(),
                status: PipHealthStatus::Healthy,
                last_check: Instant::now(),
                latency_ms: Some(10),
                error_message: None,
            }
        }

        fn cached_health_status(&self) -> PipHealthStatus {
            PipHealthStatus::Unknown
        }

        fn provided_attributes(&self) -> &[String] {
            &[]
        }

        async fn clear_cache(&self, _subject: &PipSubject) {}

        async fn clear_all_cache(&self) {}
    }
}

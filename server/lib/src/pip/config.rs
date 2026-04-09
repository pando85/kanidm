//! PIP Configuration module for defining external attribute sources.

use kubidm_proto::internal::{PipAuthConfig, PipSourceType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipConfig {
    pub enabled: bool,
    pub sources: Vec<PipSourceDefinition>,
    #[serde(default = "default_timeout")]
    pub default_timeout_seconds: u64,
    #[serde(default = "default_cache_ttl")]
    pub default_cache_ttl_seconds: u64,
    #[serde(default)]
    pub attribute_mappings: BTreeMap<String, String>,
}

fn default_timeout() -> u64 {
    10
}

fn default_cache_ttl() -> u64 {
    60
}

impl Default for PipConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sources: Vec::new(),
            default_timeout_seconds: default_timeout(),
            default_cache_ttl_seconds: default_cache_ttl(),
            attribute_mappings: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipSourceDefinition {
    pub name: String,
    pub source_type: PipSourceType,
    pub uri: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
    #[serde(default)]
    pub tls_ca_path: Option<String>,
    #[serde(default)]
    pub auth_config: Option<PipAuthConfig>,
    #[serde(default)]
    pub attributes_supported: Vec<String>,
    #[serde(default)]
    pub query_template: Option<String>,
}

impl PipSourceDefinition {
    pub fn new_http(name: &str, uri: &str) -> Self {
        Self {
            name: name.to_string(),
            source_type: PipSourceType::Http,
            uri: uri.to_string(),
            timeout_seconds: default_timeout(),
            cache_ttl_seconds: default_cache_ttl(),
            tls_ca_path: None,
            auth_config: None,
            attributes_supported: Vec::new(),
            query_template: None,
        }
    }

    pub fn new_ldap(name: &str, uri: &str) -> Self {
        Self {
            name: name.to_string(),
            source_type: PipSourceType::Ldap,
            uri: uri.to_string(),
            timeout_seconds: default_timeout(),
            cache_ttl_seconds: default_cache_ttl(),
            tls_ca_path: None,
            auth_config: None,
            attributes_supported: Vec::new(),
            query_template: None,
        }
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_cache_ttl(mut self, cache_ttl_seconds: u64) -> Self {
        self.cache_ttl_seconds = cache_ttl_seconds;
        self
    }

    pub fn with_tls_ca(mut self, path: &str) -> Self {
        self.tls_ca_path = Some(path.to_string());
        self
    }

    pub fn with_bearer_token(mut self, token: &str) -> Self {
        self.auth_config = Some(PipAuthConfig {
            bearer_token: Some(token.to_string()),
            basic_username: None,
            basic_password: None,
        });
        self
    }

    pub fn with_basic_auth(mut self, username: &str, password: &str) -> Self {
        self.auth_config = Some(PipAuthConfig {
            bearer_token: None,
            basic_username: Some(username.to_string()),
            basic_password: Some(password.to_string()),
        });
        self
    }

    pub fn with_attributes(mut self, attributes: Vec<String>) -> Self {
        self.attributes_supported = attributes;
        self
    }

    pub fn with_query_template(mut self, template: &str) -> Self {
        self.query_template = Some(template.to_string());
        self
    }

    pub fn supports_attribute(&self, attribute: &str) -> bool {
        self.attributes_supported.is_empty()
            || self.attributes_supported.contains(&attribute.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_config_default() {
        let config = PipConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.default_timeout_seconds, 10);
        assert_eq!(config.default_cache_ttl_seconds, 60);
    }

    #[test]
    fn test_pip_source_definition_http() {
        let source = PipSourceDefinition::new_http("hr-api", "https://hr.example.com/api")
            .with_timeout(5)
            .with_bearer_token("secret-token")
            .with_attributes(vec!["department".to_string(), "manager".to_string()]);

        assert_eq!(source.name, "hr-api");
        assert_eq!(source.source_type, PipSourceType::Http);
        assert_eq!(source.timeout_seconds, 5);
        assert!(source.supports_attribute("department"));
        assert!(source.supports_attribute("manager"));
        assert!(!source.supports_attribute("location"));
    }

    #[test]
    fn test_pip_source_definition_ldap() {
        let source = PipSourceDefinition::new_ldap("ldap-corp", "ldap://corp.example.com")
            .with_tls_ca("/etc/ssl/certs/ca.pem");

        assert_eq!(source.name, "ldap-corp");
        assert_eq!(source.source_type, PipSourceType::Ldap);
        assert!(source.tls_ca_path.is_some());
    }

    #[test]
    fn test_attribute_support_empty_list() {
        let source = PipSourceDefinition::new_http("generic", "https://api.example.com");
        assert!(source.supports_attribute("any-attribute"));
    }

    #[test]
    fn test_pip_config_serialization() {
        let config = PipConfig {
            enabled: true,
            sources: vec![
                PipSourceDefinition::new_http("hr", "https://hr.example.com")
                    .with_attributes(vec!["department".to_string()]),
            ],
            default_timeout_seconds: 15,
            default_cache_ttl_seconds: 120,
            attribute_mappings: BTreeMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PipConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_pip_config_deserialization_defaults() {
        let json = r#"{"enabled": true, "sources": []}"#;
        let config: PipConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(config.sources.is_empty());
        assert_eq!(config.default_timeout_seconds, 10);
        assert_eq!(config.default_cache_ttl_seconds, 60);
        assert!(config.attribute_mappings.is_empty());
    }

    #[test]
    fn test_pip_config_empty_sources() {
        let config = PipConfig::default();
        assert!(config.sources.is_empty());
        assert!(!config.enabled);
    }

    #[test]
    fn test_pip_config_with_attribute_mappings() {
        let mut mappings = BTreeMap::new();
        mappings.insert("dept".to_string(), "department".to_string());
        mappings.insert("mgr".to_string(), "manager".to_string());

        let config = PipConfig {
            enabled: true,
            sources: vec![],
            default_timeout_seconds: 10,
            default_cache_ttl_seconds: 60,
            attribute_mappings: mappings.clone(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PipConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.attribute_mappings, mappings);
    }

    #[test]
    fn test_pip_source_definition_serialization_roundtrip() {
        let source = PipSourceDefinition::new_http("test", "https://test.example.com")
            .with_timeout(30)
            .with_cache_ttl(120)
            .with_bearer_token("token123")
            .with_attributes(vec!["a".to_string(), "b".to_string()])
            .with_query_template("/api/{subject}/{attribute}");

        let json = serde_json::to_string(&source).unwrap();
        let parsed: PipSourceDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(source, parsed);
    }

    #[test]
    fn test_pip_source_definition_deserialization_minimal() {
        let json = r#"{
            "name": "minimal",
            "source_type": "http",
            "uri": "https://example.com"
        }"#;
        let source: PipSourceDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(source.name, "minimal");
        assert_eq!(source.source_type, PipSourceType::Http);
        assert_eq!(source.uri, "https://example.com");
        assert_eq!(source.timeout_seconds, 10);
        assert_eq!(source.cache_ttl_seconds, 60);
        assert!(source.tls_ca_path.is_none());
        assert!(source.auth_config.is_none());
        assert!(source.attributes_supported.is_empty());
        assert!(source.query_template.is_none());
    }

    #[test]
    fn test_pip_source_definition_with_tls_ca() {
        let source = PipSourceDefinition::new_ldap("ldaps", "ldaps://example.com")
            .with_tls_ca("/certs/ca.pem");
        assert_eq!(source.tls_ca_path.as_deref(), Some("/certs/ca.pem"));
    }

    #[test]
    fn test_pip_source_definition_basic_auth() {
        let source = PipSourceDefinition::new_http("svc", "https://svc.example.com")
            .with_basic_auth("user", "pass");

        let auth = source.auth_config.as_ref().unwrap();
        assert_eq!(auth.basic_username.as_deref(), Some("user"));
        assert_eq!(auth.basic_password.as_deref(), Some("pass"));
        assert!(auth.bearer_token.is_none());
    }

    #[test]
    fn test_pip_source_definition_bearer_auth() {
        let source = PipSourceDefinition::new_http("svc", "https://svc.example.com")
            .with_bearer_token("my-token");

        let auth = source.auth_config.as_ref().unwrap();
        assert_eq!(auth.bearer_token.as_deref(), Some("my-token"));
        assert!(auth.basic_username.is_none());
        assert!(auth.basic_password.is_none());
    }

    #[test]
    fn test_pip_source_definition_with_query_template() {
        let source = PipSourceDefinition::new_http("svc", "https://svc.example.com")
            .with_query_template("/api/v1/users/{subject}/attrs/{attribute}");

        assert_eq!(
            source.query_template.as_deref(),
            Some("/api/v1/users/{subject}/attrs/{attribute}")
        );
    }

    #[test]
    fn test_pip_source_definition_builder_chaining() {
        let source = PipSourceDefinition::new_ldap("ldap", "ldap://example.com")
            .with_timeout(20)
            .with_cache_ttl(180)
            .with_tls_ca("/ca.pem")
            .with_basic_auth("cn=admin", "secret")
            .with_attributes(vec!["department".to_string()])
            .with_query_template("(&(objectClass=user)(uid={subject}))");

        assert_eq!(source.name, "ldap");
        assert_eq!(source.source_type, PipSourceType::Ldap);
        assert_eq!(source.uri, "ldap://example.com");
        assert_eq!(source.timeout_seconds, 20);
        assert_eq!(source.cache_ttl_seconds, 180);
        assert!(source.tls_ca_path.is_some());
        assert!(source.auth_config.is_some());
        assert_eq!(source.attributes_supported.len(), 1);
        assert!(source.query_template.is_some());
    }

    #[test]
    fn test_supports_attribute_specific_list() {
        let source = PipSourceDefinition::new_http("test", "https://test.example.com")
            .with_attributes(vec!["department".to_string(), "manager".to_string()]);

        assert!(source.supports_attribute("department"));
        assert!(source.supports_attribute("manager"));
        assert!(!source.supports_attribute("salary"));
        assert!(!source.supports_attribute("location"));
    }

    #[test]
    fn test_pip_config_multiple_sources() {
        let config = PipConfig {
            enabled: true,
            sources: vec![
                PipSourceDefinition::new_http("hr", "https://hr.example.com"),
                PipSourceDefinition::new_ldap("ldap", "ldap://corp.example.com"),
            ],
            default_timeout_seconds: 10,
            default_cache_ttl_seconds: 60,
            attribute_mappings: BTreeMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PipConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sources.len(), 2);
        assert_eq!(parsed.sources[0].source_type, PipSourceType::Http);
        assert_eq!(parsed.sources[1].source_type, PipSourceType::Ldap);
    }

    #[test]
    fn test_pip_source_definition_equality() {
        let a = PipSourceDefinition::new_http("test", "https://test.example.com");
        let b = PipSourceDefinition::new_http("test", "https://test.example.com");
        assert_eq!(a, b);

        let c = PipSourceDefinition::new_http("test", "https://other.example.com");
        assert_ne!(a, c);
    }

    #[test]
    fn test_pip_source_definition_ldap_type() {
        let source = PipSourceDefinition::new_ldap("corp", "ldap://corp.example.com");
        assert_eq!(source.source_type, PipSourceType::Ldap);
        assert_eq!(source.name, "corp");
        assert_eq!(source.uri, "ldap://corp.example.com");
        assert_eq!(source.timeout_seconds, 10);
        assert_eq!(source.cache_ttl_seconds, 60);
    }
}

//! PIP Configuration module for defining external attribute sources.

use kanidm_proto::internal::{PipAuthConfig, PipSourceType};
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
}

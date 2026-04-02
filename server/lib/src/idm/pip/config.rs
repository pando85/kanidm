//! PIP Configuration
//!
//! This module defines the configuration schema for Policy Information Points,
//! including HTTP and LDAP source configurations.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

use super::PipAttributeValue;

/// Behavior when PIP fails to retrieve attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PipFallbackBehavior {
    /// Use configured fallback values
    #[default]
    UseFallback,
    /// Deny the authorization request
    Deny,
    /// Continue without the attributes (may affect policy evaluation)
    Ignore,
}

/// Authentication method for PIP sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PipAuthConfig {
    /// No authentication
    None,

    /// Basic authentication
    Basic { username: String, password: String },

    /// Bearer token authentication
    Bearer { token: String },

    /// API key authentication (header or query param)
    ApiKey {
        key_name: String,
        key_value: String,
        location: ApiKeyLocation,
    },

    /// OAuth2 client credentials flow
    OAuth2ClientCredentials {
        token_url: Url,
        client_id: String,
        client_secret: String,
        scope: Option<String>,
    },

    /// Mutual TLS authentication
    MutualTls {
        cert_path: PathBuf,
        key_path: PathBuf,
    },
}

/// Where to place the API key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    #[default]
    Header,
    QueryParam,
}

/// TLS configuration for PIP sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipTlsConfig {
    /// Path to CA certificate for server verification
    pub ca_path: Option<PathBuf>,
    /// Whether to verify server certificate
    #[serde(default = "default_verify_server")]
    pub verify_server: bool,
    /// Allow insecure connections (for development only)
    #[serde(default)]
    pub allow_insecure: bool,
}

fn default_verify_server() -> bool {
    true
}

impl Default for PipTlsConfig {
    fn default() -> Self {
        PipTlsConfig {
            ca_path: None,
            verify_server: true,
            allow_insecure: false,
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipHealthCheckConfig {
    /// How often to check health (in seconds)
    #[serde(default = "default_health_check_interval")]
    pub interval_secs: u64,

    /// Number of consecutive failures before marking unhealthy
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,

    /// Number of consecutive successes to mark healthy after being unhealthy
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,

    /// Timeout for health check requests (in seconds)
    #[serde(default = "default_health_check_timeout")]
    pub timeout_secs: u64,
}

fn default_health_check_interval() -> u64 {
    60
}

fn default_failure_threshold() -> u32 {
    3
}

fn default_success_threshold() -> u32 {
    2
}

fn default_health_check_timeout() -> u64 {
    5
}

impl Default for PipHealthCheckConfig {
    fn default() -> Self {
        PipHealthCheckConfig {
            interval_secs: default_health_check_interval(),
            failure_threshold: default_failure_threshold(),
            success_threshold: default_success_threshold(),
            timeout_secs: default_health_check_timeout(),
        }
    }
}

/// HTTP PIP source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpPipConfig {
    /// Unique identifier for this PIP source
    pub id: String,

    /// Base URL for the HTTP endpoint
    pub base_url: Url,

    /// Endpoint path template (supports variables like {uuid}, {username})
    pub endpoint_path: String,

    /// HTTP method to use
    #[serde(default = "default_http_method")]
    pub method: HttpMethod,

    /// Authentication configuration
    #[serde(default)]
    pub auth: Option<PipAuthConfig>,

    /// TLS configuration
    #[serde(default)]
    pub tls: PipTlsConfig,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,

    /// Headers to include in requests
    #[serde(default)]
    pub headers: BTreeMap<String, String>,

    /// Fallback behavior when source is unavailable
    #[serde(default)]
    pub fallback_behavior: PipFallbackBehavior,

    /// Fallback attribute values
    #[serde(default)]
    pub fallback_values: BTreeMap<String, PipAttributeValue>,

    /// Health check configuration
    #[serde(default)]
    pub health_check: PipHealthCheckConfig,

    /// Health check endpoint path (relative to base_url)
    pub health_check_path: Option<String>,

    /// Attributes that this source provides
    #[serde(default)]
    pub provided_attributes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
}

fn default_timeout() -> u64 {
    30
}

fn default_cache_ttl() -> u64 {
    300
}

fn default_http_method() -> HttpMethod {
    HttpMethod::Get
}

/// LDAP PIP source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapPipConfig {
    /// Unique identifier for this PIP source
    pub id: String,

    /// LDAP server URL (ldap:// or ldaps://)
    pub url: Url,

    /// Base DN for searches
    pub base_dn: String,

    /// Bind DN for authentication
    pub bind_dn: String,

    /// Bind password
    pub bind_password: String,

    /// Search filter template (supports variables like {uuid}, {username})
    pub search_filter: String,

    /// Attributes to retrieve from LDAP entries
    #[serde(default)]
    pub attributes: Vec<String>,

    /// TLS configuration (for ldaps://)
    #[serde(default)]
    pub tls: PipTlsConfig,

    /// Connection timeout in seconds
    #[serde(default = "default_ldap_timeout")]
    pub timeout_secs: u64,

    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,

    /// Fallback behavior when source is unavailable
    #[serde(default)]
    pub fallback_behavior: PipFallbackBehavior,

    /// Fallback attribute values
    #[serde(default)]
    pub fallback_values: BTreeMap<String, PipAttributeValue>,

    /// Health check configuration
    #[serde(default)]
    pub health_check: PipHealthCheckConfig,

    /// Attribute mapping: LDAP attribute name -> PIP attribute name
    #[serde(default)]
    pub attribute_mapping: BTreeMap<String, String>,
}

fn default_ldap_timeout() -> u64 {
    30
}

/// PIP source configuration (enum to support multiple source types)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PipSourceConfig {
    Http(HttpPipConfig),
    Ldap(LdapPipConfig),
}

/// Global PIP configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipConfig {
    /// List of PIP sources
    #[serde(default)]
    pub sources: Vec<PipSourceConfig>,

    /// Global default cache TTL in seconds
    #[serde(default = "default_global_cache_ttl")]
    pub default_cache_ttl_secs: u64,

    /// Maximum cache entries
    #[serde(default = "default_max_cache_entries")]
    pub max_cache_entries: usize,

    /// Enable/disable PIP globally
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_global_cache_ttl() -> u64 {
    300
}

fn default_max_cache_entries() -> usize {
    10000
}

fn default_enabled() -> bool {
    true
}

impl PipSourceConfig {
    pub fn id(&self) -> &str {
        match self {
            PipSourceConfig::Http(config) => &config.id,
            PipSourceConfig::Ldap(config) => &config.id,
        }
    }

    pub fn cache_ttl(&self) -> Duration {
        match self {
            PipSourceConfig::Http(config) => Duration::from_secs(config.cache_ttl_secs),
            PipSourceConfig::Ldap(config) => Duration::from_secs(config.cache_ttl_secs),
        }
    }

    pub fn timeout(&self) -> Duration {
        match self {
            PipSourceConfig::Http(config) => Duration::from_secs(config.timeout_secs),
            PipSourceConfig::Ldap(config) => Duration::from_secs(config.timeout_secs),
        }
    }

    pub fn fallback_behavior(&self) -> PipFallbackBehavior {
        match self {
            PipSourceConfig::Http(config) => config.fallback_behavior,
            PipSourceConfig::Ldap(config) => config.fallback_behavior,
        }
    }

    pub fn fallback_values(&self) -> &BTreeMap<String, PipAttributeValue> {
        match self {
            PipSourceConfig::Http(config) => &config.fallback_values,
            PipSourceConfig::Ldap(config) => &config.fallback_values,
        }
    }
}

impl PipConfig {
    pub fn default_cache_ttl(&self) -> Duration {
        Duration::from_secs(self.default_cache_ttl_secs)
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_config_deserialization() {
        let config_str = r#"
sources = []

default_cache_ttl_secs = 300
max_cache_entries = 5000
enabled = true
"#;

        let config: PipConfig = toml::from_str(config_str).expect("Failed to parse config");
        assert!(config.enabled);
        assert_eq!(config.default_cache_ttl_secs, 300);
        assert_eq!(config.max_cache_entries, 5000);
    }

    #[test]
    fn test_http_pip_config() {
        let config_str = r#"
type = "http"
id = "hr_system"
base_url = "https://hr.example.com/api"
endpoint_path = "/employees/{username}"
timeout_secs = 30
cache_ttl_secs = 60
"#;

        let config: PipSourceConfig = toml::from_str(config_str).expect("Failed to parse config");
        assert!(matches!(config, PipSourceConfig::Http(_)));

        if let PipSourceConfig::Http(http_config) = config {
            assert_eq!(http_config.id, "hr_system");
            assert_eq!(http_config.base_url.as_str(), "https://hr.example.com/api");
            assert_eq!(http_config.endpoint_path, "/employees/{username}");
        }
    }

    #[test]
    fn test_ldap_pip_config() {
        let config_str = r#"
type = "ldap"
id = "company_ldap"
url = "ldaps://ldap.example.com"
base_dn = "dc=example,dc=com"
bind_dn = "cn=admin,dc=example,dc=com"
bind_password = "secret"
search_filter = "(uid={username})"
attributes = ["department", "manager", "location"]
"#;

        let config: PipSourceConfig = toml::from_str(config_str).expect("Failed to parse config");
        assert!(matches!(config, PipSourceConfig::Ldap(_)));

        if let PipSourceConfig::Ldap(ldap_config) = config {
            assert_eq!(ldap_config.id, "company_ldap");
            assert_eq!(ldap_config.attributes.len(), 3);
        }
    }

    #[test]
    fn test_fallback_behavior_defaults() {
        let http_config = HttpPipConfig {
            id: "test".to_string(),
            base_url: Url::parse("https://example.com").unwrap(),
            endpoint_path: "/test".to_string(),
            method: HttpMethod::Get,
            auth: None,
            tls: PipTlsConfig::default(),
            timeout_secs: 30,
            cache_ttl_secs: 60,
            headers: BTreeMap::new(),
            fallback_behavior: PipFallbackBehavior::default(),
            fallback_values: BTreeMap::new(),
            health_check: PipHealthCheckConfig::default(),
            health_check_path: None,
            provided_attributes: vec!["test_attr".to_string()],
        };

        assert_eq!(
            http_config.fallback_behavior,
            PipFallbackBehavior::UseFallback
        );
    }
}

//! LDAP PIP client for retrieving attributes from LDAP servers.
//!
//! Note: This module provides a placeholder implementation. Full LDAP search
//! support requires the ldap3_client crate which is not currently a dependency
//! of the main server library. The health check will return Unavailable.

use crate::prelude::*;
use kanidm_proto::internal::{PipRequest, PipSourceStatus, PipSourceType};
use std::collections::BTreeMap;
use std::pin::Pin;

use super::config::PipSourceDefinition;
use super::PolicyInformationPoint;

pub struct LdapPipClient {
    config: PipSourceDefinition,
}

impl LdapPipClient {
    pub fn new(config: PipSourceDefinition) -> Self {
        Self { config }
    }

    pub fn build_ldap_filter(&self, request: &PipRequest) -> String {
        let subject_str = request.subject.map_or("*".to_string(), |u| u.to_string());
        if let Some(template) = &self.config.query_template {
            template
                .replace("{subject}", &subject_str)
                .replace("{resource}", &request.resource.to_string())
        } else {
            format!("(&(objectClass=*)(uuid={}))", subject_str)
        }
    }

    pub fn get_bind_dn(&self) -> Option<(String, String)> {
        self.config.auth_config.as_ref().and_then(|auth| {
            if let (Some(username), Some(password)) = (&auth.basic_username, &auth.basic_password) {
                Some((username.clone(), password.clone()))
            } else {
                None
            }
        })
    }
}

impl PolicyInformationPoint for LdapPipClient {
    fn source_type(&self) -> PipSourceType {
        PipSourceType::Ldap
    }

    fn source_name(&self) -> &str {
        &self.config.name
    }

    fn retrieve_attributes(
        &self,
        _request: &PipRequest,
        _attributes: &[String],
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<BTreeMap<String, String>, OperationError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            error!(
                "LDAP PIP source '{}' requested but LDAP client support is not yet available. \
                 Please use HTTP PIP sources for external attribute retrieval.",
                self.config.name
            );
            Err(OperationError::InvalidState)
        })
    }

    fn health_check(
        &self,
    ) -> Pin<Box<dyn std::future::Future<Output = PipSourceStatus> + Send + '_>> {
        Box::pin(async move { PipSourceStatus::Unavailable })
    }
}

impl std::fmt::Debug for LdapPipClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LdapPipClient")
            .field("name", &self.config.name)
            .field("uri", &self.config.uri)
            .field("timeout_seconds", &self.config.timeout_seconds)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_ldap_pip_client_creation() {
        let config = PipSourceDefinition::new_ldap("test-ldap", "ldap://ldap.example.com")
            .with_timeout(10)
            .with_basic_auth("cn=admin", "password");

        let client = LdapPipClient::new(config);
        assert_eq!(client.source_type(), PipSourceType::Ldap);
        assert_eq!(client.source_name(), "test-ldap");
    }

    #[test]
    fn test_build_ldap_filter_default() {
        let config = PipSourceDefinition::new_ldap("test", "ldap://ldap.example.com");
        let client = LdapPipClient::new(config);
        let request = PipRequest::new(
            Some(Uuid::nil()),
            Uuid::nil(),
            vec!["department".to_string()],
        );

        let filter = client.build_ldap_filter(&request);
        assert!(filter.contains("uuid="));
    }

    #[test]
    fn test_build_ldap_filter_with_template() {
        let config = PipSourceDefinition::new_ldap("test", "ldap://ldap.example.com")
            .with_query_template("(&(objectClass=user)(uuid={subject}))");

        let client = LdapPipClient::new(config);
        let request = PipRequest::new(
            Some(Uuid::nil()),
            Uuid::nil(),
            vec!["department".to_string()],
        );

        let filter = client.build_ldap_filter(&request);
        assert!(filter.contains("objectClass=user"));
        assert!(filter.contains("uuid="));
    }

    #[test]
    fn test_bind_dn_extraction() {
        let config = PipSourceDefinition::new_ldap("test", "ldap://ldap.example.com")
            .with_basic_auth("cn=admin", "secret");

        let client = LdapPipClient::new(config);
        let bind = client.get_bind_dn();
        assert!(bind.is_some());
        let (dn, pw) = bind.unwrap();
        assert_eq!(dn, "cn=admin");
        assert_eq!(pw, "secret");
    }
}

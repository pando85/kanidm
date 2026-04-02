//! HTTP PIP client for retrieving attributes from REST API endpoints.

use crate::prelude::*;
use kanidm_proto::internal::{PipRequest, PipSourceStatus, PipSourceType};
use std::collections::BTreeMap;
use std::pin::Pin;

use super::config::PipSourceDefinition;
use super::PolicyInformationPoint;

pub struct HttpPipClient {
    config: PipSourceDefinition,
    client: reqwest::Client,
}

impl HttpPipClient {
    pub fn new(config: PipSourceDefinition) -> Self {
        let mut client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .connect_timeout(std::time::Duration::from_secs(5));

        if let Some(ca_path) = &config.tls_ca_path {
            if let Ok(ca_content) = std::fs::read(ca_path) {
                let ca_cert = reqwest::Certificate::from_pem(&ca_content);
                if let Ok(cert) = ca_cert {
                    client_builder = client_builder.add_root_certificate(cert);
                }
            }
        }

        let client = client_builder
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { config, client }
    }
}

impl PolicyInformationPoint for HttpPipClient {
    fn source_type(&self) -> PipSourceType {
        PipSourceType::Http
    }

    fn source_name(&self) -> &str {
        &self.config.name
    }

    fn retrieve_attributes(
        &self,
        request: &PipRequest,
        attributes: &[String],
    ) -> Pin<Box<dyn std::future::Future<Output = Result<BTreeMap<String, String>, OperationError>> + Send + '_>>
    {
        let request = request.clone();
        let attributes = attributes.to_vec();
        let config = self.config.clone();
        let client = self.client.clone();

        Box::pin(async move {
            let mut results = BTreeMap::new();

            for attribute in attributes {
                if !config.supports_attribute(&attribute) {
                    continue;
                }

                let subject_str = request
                    .subject
                    .map_or("unknown".to_string(), |u| u.to_string());

                let url = if let Some(template) = &config.query_template {
                    template
                        .replace("{subject}", &subject_str)
                        .replace("{resource}", &request.resource.to_string())
                        .replace("{attribute}", &attribute)
                } else {
                    format!(
                        "{}?subject={}&resource={}&attribute={}",
                        config.uri.trim_end_matches('/'),
                        subject_str,
                        request.resource,
                        attribute
                    )
                };

                let request_builder = client.get(&url);

                let request_builder = if let Some(auth) = &config.auth_config {
                    if let Some(token) = &auth.bearer_token {
                        request_builder.bearer_auth(token)
                    } else if let (Some(username), Some(password)) =
                        (&auth.basic_username, &auth.basic_password)
                    {
                        request_builder.basic_auth(username, Some(password))
                    } else {
                        request_builder
                    }
                } else {
                    request_builder
                };

                let response = tokio::time::timeout(
                    Duration::from_secs(config.timeout_seconds),
                    request_builder.send(),
                )
                .await
                .map_err(|_| {
                    error!("HTTP PIP request timed out for source {}", config.name);
                    OperationError::KG001TaskTimeout
                })?
                .map_err(|e| {
                    error!(?e, "HTTP PIP request failed for source {}", config.name);
                    OperationError::InvalidState
                })?;

                if !response.status().is_success() {
                    error!(
                        "HTTP PIP returned status {} for source {}",
                        response.status(),
                        config.name
                    );
                    return Err(OperationError::InvalidState);
                }

                let json_value: serde_json::Value = response.json().await.map_err(|e| {
                    error!(?e, "Failed to parse HTTP PIP response JSON");
                    OperationError::InvalidState
                })?;

                if let Some(value) = json_value.get(&attribute).and_then(|v| v.as_str()) {
                    results.insert(attribute, value.to_string());
                }
            }

            Ok(results)
        })
    }

    fn health_check(&self) -> Pin<Box<dyn std::future::Future<Output = PipSourceStatus> + Send + '_>> {
        let config = self.config.clone();
        let client = self.client.clone();

        Box::pin(async move {
            let health_url = format!("{}{}", config.uri.trim_end_matches('/'), "/health");

            let request_builder = if let Some(auth) = &config.auth_config {
                if let Some(token) = &auth.bearer_token {
                    client.get(&health_url).bearer_auth(token)
                } else if let (Some(username), Some(password)) =
                    (&auth.basic_username, &auth.basic_password)
                {
                    client.get(&health_url).basic_auth(username, Some(password))
                } else {
                    client.get(&health_url)
                }
            } else {
                client.get(&health_url)
            };

            let result =
                tokio::time::timeout(Duration::from_secs(5), request_builder.send()).await;

            match result {
                Ok(Ok(response)) => {
                    if response.status().is_success() {
                        PipSourceStatus::Success
                    } else {
                        PipSourceStatus::Error
                    }
                }
                Ok(Err(_)) => PipSourceStatus::Error,
                Err(_) => PipSourceStatus::Timeout,
            }
        })
    }
}

impl std::fmt::Debug for HttpPipClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpPipClient")
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
    fn test_http_pip_client_creation() {
        let config = PipSourceDefinition::new_http("test-api", "https://api.example.com")
            .with_timeout(5)
            .with_bearer_token("test-token");

        let client = HttpPipClient::new(config);
        assert_eq!(client.source_type(), PipSourceType::Http);
        assert_eq!(client.source_name(), "test-api");
    }
}
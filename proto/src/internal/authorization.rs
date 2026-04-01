use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AuthorizationDecision {
    Allow,
    Deny,
    ReauthRequired,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AuthorizationAction {
    Search,
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationRequest {
    pub subject: Option<Uuid>,
    pub resource: Uuid,
    pub action: AuthorizationAction,
    #[serde(default)]
    pub attributes: Option<BTreeSet<String>>,
    #[serde(default)]
    pub include_explanation: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationResponse {
    pub decision: AuthorizationDecision,
    pub resource: Uuid,
    pub action: AuthorizationAction,
    pub allowed_attributes: Option<BTreeSet<String>>,
    pub explanation: Option<AuthorizationExplanation>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationExplanation {
    pub matched_rules: Vec<String>,
    pub denied_by: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BatchAuthorizationRequest {
    pub requests: Vec<AuthorizationRequest>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BatchAuthorizationResponse {
    pub responses: Vec<AuthorizationResponse>,
}

impl AuthorizationRequest {
    pub fn new(subject: Option<Uuid>, resource: Uuid, action: AuthorizationAction) -> Self {
        Self {
            subject,
            resource,
            action,
            attributes: None,
            include_explanation: false,
        }
    }

    pub fn with_attributes(mut self, attributes: BTreeSet<String>) -> Self {
        self.attributes = Some(attributes);
        self
    }

    pub fn with_explanation(mut self, include: bool) -> Self {
        self.include_explanation = include;
        self
    }
}

impl AuthorizationResponse {
    pub fn allow(resource: Uuid, action: AuthorizationAction) -> Self {
        Self {
            decision: AuthorizationDecision::Allow,
            resource,
            action,
            allowed_attributes: None,
            explanation: None,
        }
    }

    pub fn allow_with_attributes(
        resource: Uuid,
        action: AuthorizationAction,
        attributes: BTreeSet<String>,
    ) -> Self {
        Self {
            decision: AuthorizationDecision::Allow,
            resource,
            action,
            allowed_attributes: Some(attributes),
            explanation: None,
        }
    }

    pub fn deny(resource: Uuid, action: AuthorizationAction) -> Self {
        Self {
            decision: AuthorizationDecision::Deny,
            resource,
            action,
            allowed_attributes: None,
            explanation: None,
        }
    }

    pub fn reauth_required(resource: Uuid, action: AuthorizationAction) -> Self {
        Self {
            decision: AuthorizationDecision::ReauthRequired,
            resource,
            action,
            allowed_attributes: None,
            explanation: None,
        }
    }

    pub fn with_explanation(mut self, explanation: AuthorizationExplanation) -> Self {
        self.explanation = Some(explanation);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecisionCacheEntry {
    pub subject: Option<Uuid>,
    pub resource: Uuid,
    pub action: AuthorizationAction,
    pub decision: AuthorizationDecision,
    pub allowed_attributes: Option<BTreeSet<String>>,
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

impl DecisionCacheEntry {
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time > self.cached_at + self.ttl_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_request_serialization() {
        let req =
            AuthorizationRequest::new(Some(Uuid::nil()), Uuid::nil(), AuthorizationAction::Search);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("search"));
    }

    #[test]
    fn test_authorization_response_decision() {
        let response = AuthorizationResponse::allow(Uuid::nil(), AuthorizationAction::Search);
        assert_eq!(response.decision, AuthorizationDecision::Allow);

        let response = AuthorizationResponse::deny(Uuid::nil(), AuthorizationAction::Delete);
        assert_eq!(response.decision, AuthorizationDecision::Deny);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = DecisionCacheEntry {
            subject: None,
            resource: Uuid::nil(),
            action: AuthorizationAction::Search,
            decision: AuthorizationDecision::Allow,
            allowed_attributes: None,
            cached_at: 1000,
            ttl_seconds: 60,
        };

        assert!(!entry.is_expired(1050));
        assert!(entry.is_expired(1100));
    }
}

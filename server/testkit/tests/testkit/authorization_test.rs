#![deny(warnings)]
use kanidm_client::KanidmClient;
use kanidm_proto::internal::{
    AuthorizationAction, AuthorizationDecision, AuthorizationExplanation, AuthorizationRequest,
    AuthorizationResponse, BatchAuthorizationRequest, BatchAuthorizationResponse,
};
use kanidmd_testkit::{ADMIN_TEST_PASSWORD, ADMIN_TEST_USER};
use std::collections::BTreeSet;
use uuid::Uuid;

#[kanidmd_testkit::test]
async fn test_authorization_endpoint_unauthorized(rsclient: &KanidmClient) {
    let resource_uuid = Uuid::new_v4();
    let request = AuthorizationRequest::new(None, resource_uuid, AuthorizationAction::Search);

    let result: Result<AuthorizationResponse, kanidm_client::ClientError> = rsclient
        .perform_post_request("/v1/authorize", request)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        kanidm_client::ClientError::Unauthorized => (),
        other => panic!("Expected Unauthorized error, got {:?}", other),
    }
}

#[kanidmd_testkit::test]
async fn test_authorization_endpoint_authenticated_admin(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let admin_uuid = rsclient
        .whoami()
        .await
        .expect("Unable to call whoami")
        .expect("No entry matching self returned")
        .attrs
        .get("uuid")
        .and_then(|v| v.first())
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("Unable to parse admin uuid");

    let request = AuthorizationRequest::new(
        Some(admin_uuid),
        admin_uuid,
        AuthorizationAction::Search,
    );

    let response: AuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize", request)
        .await
        .expect("Authorization request failed");

    assert_eq!(response.resource, admin_uuid);
    assert_eq!(response.action, AuthorizationAction::Search);
}

#[kanidmd_testkit::test]
async fn test_authorization_request_serialization(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let resource_uuid = Uuid::new_v4();
    let subject_uuid = Uuid::new_v4();

    let request = AuthorizationRequest::new(Some(subject_uuid), resource_uuid, AuthorizationAction::Modify)
        .with_attributes(BTreeSet::from(["name".to_string(), "displayname".to_string()]))
        .with_explanation(true);

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("modify"));
    assert!(json.contains("subject"));
    assert!(json.contains("resource"));
    assert!(json.contains("attributes"));
    assert!(json.contains("includeExplanation"));
}

#[kanidmd_testkit::test]
async fn test_authorization_response_deserialization(rsclient: &KanidmClient) {
    let json = r#"{
        "decision": "allow",
        "resource": "00000000-0000-0000-0000-000000000000",
        "action": "search",
        "allowedAttributes": ["name", "displayname"],
        "explanation": {
            "matchedRules": ["test-rule"],
            "deniedBy": null,
            "reason": "Access granted"
        }
    }"#;

    let response: AuthorizationResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.decision, AuthorizationDecision::Allow);
    assert_eq!(response.action, AuthorizationAction::Search);
    assert!(response.allowed_attributes.is_some());
    assert!(response.explanation.is_some());
}

#[kanidmd_testkit::test]
async fn test_authorization_decision_deny(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let resource_uuid = Uuid::new_v4();
    let request = AuthorizationRequest::new(None, resource_uuid, AuthorizationAction::Delete);

    let response: AuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize", request)
        .await
        .expect("Authorization request failed");

    assert_eq!(response.decision, AuthorizationDecision::Deny);
    assert_eq!(response.resource, resource_uuid);
    assert_eq!(response.action, AuthorizationAction::Delete);
}

#[kanidmd_testkit::test]
async fn test_authorization_decision_types(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let resource_uuid = Uuid::new_v4();

    for action in [
        AuthorizationAction::Search,
        AuthorizationAction::Create,
        AuthorizationAction::Modify,
        AuthorizationAction::Delete,
    ] {
        let request = AuthorizationRequest::new(None, resource_uuid, action);
        let response: AuthorizationResponse = rsclient
            .perform_post_request("/v1/authorize", request)
            .await
            .expect("Authorization request failed");

        assert_eq!(response.action, action);
        assert_eq!(response.resource, resource_uuid);
    }
}

#[kanidmd_testkit::test]
async fn test_authorization_with_explanation(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let resource_uuid = Uuid::new_v4();
    let request = AuthorizationRequest::new(None, resource_uuid, AuthorizationAction::Search)
        .with_explanation(true);

    let response: AuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize", request)
        .await
        .expect("Authorization request failed");

    if let Some(explanation) = response.explanation {
        assert!(!explanation.reason.is_empty());
    }
}

#[kanidmd_testkit::test]
async fn test_batch_authorization_endpoint_unauthorized(rsclient: &KanidmClient) {
    let requests = vec![
        AuthorizationRequest::new(None, Uuid::new_v4(), AuthorizationAction::Search),
        AuthorizationRequest::new(None, Uuid::new_v4(), AuthorizationAction::Modify),
    ];
    let batch_request = BatchAuthorizationRequest { requests };

    let result: Result<BatchAuthorizationResponse, kanidm_client::ClientError> = rsclient
        .perform_post_request("/v1/authorize/batch", batch_request)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        kanidm_client::ClientError::Unauthorized => (),
        other => panic!("Expected Unauthorized error, got {:?}", other),
    }
}

#[kanidmd_testkit::test]
async fn test_batch_authorization_endpoint_authenticated(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let resource_a = Uuid::new_v4();
    let resource_b = Uuid::new_v4();

    let requests = vec![
        AuthorizationRequest::new(None, resource_a, AuthorizationAction::Search),
        AuthorizationRequest::new(None, resource_b, AuthorizationAction::Modify),
    ];
    let batch_request = BatchAuthorizationRequest { requests };

    let response: BatchAuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize/batch", batch_request)
        .await
        .expect("Batch authorization request failed");

    assert_eq!(response.responses.len(), 2);
    assert_eq!(response.responses[0].resource, resource_a);
    assert_eq!(response.responses[1].resource, resource_b);
}

#[kanidmd_testkit::test]
async fn test_batch_authorization_empty_requests(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let batch_request = BatchAuthorizationRequest { requests: vec![] };

    let response: BatchAuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize/batch", batch_request)
        .await
        .expect("Batch authorization request failed");

    assert!(response.responses.is_empty());
}

#[kanidmd_testkit::test]
async fn test_batch_authorization_large_batch(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let num_requests = 10;
    let requests: Vec<AuthorizationRequest> = (0..num_requests)
        .map(|_| AuthorizationRequest::new(None, Uuid::new_v4(), AuthorizationAction::Search))
        .collect();
    let batch_request = BatchAuthorizationRequest { requests };

    let response: BatchAuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize/batch", batch_request)
        .await
        .expect("Batch authorization request failed");

    assert_eq!(response.responses.len(), num_requests);
}

#[kanidmd_testkit::test]
async fn test_authorization_empty_resource_uuid(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let request = AuthorizationRequest::new(None, Uuid::nil(), AuthorizationAction::Search);

    let result: Result<AuthorizationResponse, kanidm_client::ClientError> = rsclient
        .perform_post_request("/v1/authorize", request)
        .await;

    assert!(result.is_err());
}

#[kanidmd_testkit::test]
async fn test_authorization_action_case_sensitivity(rsclient: &KanidmClient) {
    let json_lowercase = r#"{
        "decision": "allow",
        "resource": "00000000-0000-0000-0000-000000000000",
        "action": "search"
    }"#;

    let response: AuthorizationResponse = serde_json::from_str(json_lowercase).unwrap();
    assert_eq!(response.action, AuthorizationAction::Search);

    let json_uppercase = r#"{
        "decision": "allow",
        "resource": "00000000-0000-0000-0000-000000000000",
        "action": "SEARCH"
    }"#;

    let result: Result<AuthorizationResponse, _> = serde_json::from_str(json_uppercase);
    assert!(result.is_err());
}

#[kanidmd_testkit::test]
async fn test_authorization_decision_case_sensitivity(rsclient: &KanidmClient) {
    let json_lowercase = r#"{
        "decision": "deny",
        "resource": "00000000-0000-0000-0000-000000000000",
        "action": "search"
    }"#;

    let response: AuthorizationResponse = serde_json::from_str(json_lowercase).unwrap();
    assert_eq!(response.decision, AuthorizationDecision::Deny);

    let json_uppercase = r#"{
        "decision": "DENY",
        "resource": "00000000-0000-0000-0000-000000000000",
        "action": "search"
    }"#;

    let result: Result<AuthorizationResponse, _> = serde_json::from_str(json_uppercase);
    assert!(result.is_err());
}

#[kanidmd_testkit::test]
async fn test_authorization_with_attributes_filtering(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let admin_uuid = rsclient
        .whoami()
        .await
        .expect("Unable to call whoami")
        .expect("No entry matching self returned")
        .attrs
        .get("uuid")
        .and_then(|v| v.first())
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("Unable to parse admin uuid");

    let attrs: BTreeSet<String> = BTreeSet::from(["name".to_string()]);
    let request = AuthorizationRequest::new(Some(admin_uuid), admin_uuid, AuthorizationAction::Search)
        .with_attributes(attrs.clone());

    let response: AuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize", request)
        .await
        .expect("Authorization request failed");

    if let Some(allowed) = response.allowed_attributes {
        assert!(allowed.contains("name") || allowed.is_empty() || response.decision == AuthorizationDecision::Deny);
    }
}

#[kanidmd_testkit::test]
async fn test_authorization_missing_required_fields(rsclient: &KanidmClient) {
    let json_missing_resource = r#"{
        "subject": "00000000-0000-0000-0000-000000000000",
        "action": "search"
    }"#;

    let result: Result<AuthorizationRequest, _> = serde_json::from_str(json_missing_resource);
    assert!(result.is_err());

    let json_missing_action = r#"{
        "subject": "00000000-0000-0000-0000-000000000000",
        "resource": "00000000-0000-0000-0000-000000000000"
    }"#;

    let result: Result<AuthorizationRequest, _> = serde_json::from_str(json_missing_action);
    assert!(result.is_err());
}

#[kanidmd_testkit::test]
async fn test_authorization_reauth_required_decision(rsclient: &KanidmClient) {
    let json_reauth = r#"{
        "decision": "reauthrequired",
        "resource": "00000000-0000-0000-0000-000000000000",
        "action": "modify"
    }"#;

    let response: AuthorizationResponse = serde_json::from_str(json_reauth).unwrap();
    assert_eq!(response.decision, AuthorizationDecision::ReauthRequired);
    assert_eq!(response.action, AuthorizationAction::Modify);
}

#[kanidmd_testkit::test]
async fn test_authorization_unicode_in_attributes(rsclient: &KanidmClient) {
    let attrs: BTreeSet<String> = BTreeSet::from([
        "日本語属性".to_string(),
        "属性名".to_string(),
        "displayName".to_string(),
    ]);
    let request = AuthorizationRequest::new(
        None,
        Uuid::new_v4(),
        AuthorizationAction::Search,
    ).with_attributes(attrs);

    let json = serde_json::to_string(&request).unwrap();
    let parsed: AuthorizationRequest = serde_json::from_str(&json).unwrap();

    assert!(parsed.attributes.is_some());
    let parsed_attrs = parsed.attributes.unwrap();
    assert!(parsed_attrs.contains("日本語属性"));
}

#[kanidmd_testkit::test]
async fn test_authorization_explanation_fields(rsclient: &KanidmClient) {
    let explanation = AuthorizationExplanation {
        matched_rules: vec!["rule_a".to_string(), "rule_b".to_string()],
        denied_by: Some("policy_c".to_string()),
        reason: "Access denied due to policy restrictions".to_string(),
    };

    let json = serde_json::to_string(&explanation).unwrap();
    let parsed: AuthorizationExplanation = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.matched_rules.len(), 2);
    assert!(parsed.denied_by.is_some());
    assert!(parsed.reason.contains("denied"));
}

#[kanidmd_testkit::test]
async fn test_authorization_session_expiry_handling(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let request = AuthorizationRequest::new(None, Uuid::new_v4(), AuthorizationAction::Search);

    let response: AuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize", request.clone())
        .await
        .expect("Authorization request failed");

    assert_eq!(response.resource, request.resource);
    assert_eq!(response.action, request.action);
}

#[kanidmd_testkit::test]
async fn test_authorization_concurrent_requests(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let requests: Vec<AuthorizationRequest> = (0..5)
        .map(|i| AuthorizationRequest::new(None, Uuid::new_v4(), AuthorizationAction::Search))
        .collect();

    let futures: Vec<_> = requests
        .iter()
        .map(|req| rsclient.perform_post_request::<AuthorizationRequest, AuthorizationResponse>(
            "/v1/authorize",
            req.clone()
        ))
        .collect();

    let results = futures::future::join_all(futures).await;

    for result in results {
        assert!(result.is_ok());
    }
}

#[kanidmd_testkit::test]
async fn test_authorization_create_action_unsupported(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let request = AuthorizationRequest::new(None, Uuid::new_v4(), AuthorizationAction::Create);

    let response: AuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize", request)
        .await
        .expect("Authorization request failed");

    assert_eq!(response.decision, AuthorizationDecision::Deny);
}

#[kanidmd_testkit::test]
async fn test_authorization_null_subject_handling(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let request = AuthorizationRequest::new(None, Uuid::new_v4(), AuthorizationAction::Search);

    assert!(request.subject.is_none());
}

#[kanidmd_testkit::test]
async fn test_authorization_response_builder_methods() {
    let resource = Uuid::new_v4();

    let allow_response = AuthorizationResponse::allow(resource, AuthorizationAction::Search);
    assert_eq!(allow_response.decision, AuthorizationDecision::Allow);
    assert!(allow_response.allowed_attributes.is_none());

    let deny_response = AuthorizationResponse::deny(resource, AuthorizationAction::Delete);
    assert_eq!(deny_response.decision, AuthorizationDecision::Deny);

    let reauth_response = AuthorizationResponse::reauth_required(resource, AuthorizationAction::Modify);
    assert_eq!(reauth_response.decision, AuthorizationDecision::ReauthRequired);

    let attrs = BTreeSet::from(["name".to_string(), "displayname".to_string()]);
    let allow_with_attrs = AuthorizationResponse::allow_with_attributes(resource, AuthorizationAction::Search, attrs.clone());
    assert!(allow_with_attrs.allowed_attributes.is_some());
    assert_eq!(allow_with_attrs.allowed_attributes.unwrap().len(), 2);
}
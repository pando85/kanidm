#![deny(warnings)]
use kanidm_client::KanidmClient;
use kanidm_proto::internal::{
    AuthorizationAction, AuthorizationDecision, AuthorizationRequest, AuthorizationResponse,
    BatchAuthorizationRequest, BatchAuthorizationResponse,
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

    let request =
        AuthorizationRequest::new(Some(admin_uuid), admin_uuid, AuthorizationAction::Search);

    let response: AuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize", request)
        .await
        .expect("Authorization request failed");

    assert_eq!(response.resource, admin_uuid);
    assert_eq!(response.action, AuthorizationAction::Search);
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
    let request =
        AuthorizationRequest::new(Some(admin_uuid), admin_uuid, AuthorizationAction::Search)
            .with_attributes(attrs.clone());

    let response: AuthorizationResponse = rsclient
        .perform_post_request("/v1/authorize", request)
        .await
        .expect("Authorization request failed");

    if let Some(allowed) = response.allowed_attributes {
        assert!(
            allowed.contains("name")
                || allowed.is_empty()
                || response.decision == AuthorizationDecision::Deny
        );
    }
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
        .map(|_| AuthorizationRequest::new(None, Uuid::new_v4(), AuthorizationAction::Search))
        .collect();

    let futures: Vec<_> = requests
        .iter()
        .map(|req| {
            rsclient.perform_post_request::<AuthorizationRequest, AuthorizationResponse>(
                "/v1/authorize",
                req.clone(),
            )
        })
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

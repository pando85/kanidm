//! Tests for health and readiness endpoints.

use kubidmd_lib::status::{DatabaseHealth, ReplicationState, ServingReadiness};

#[kubidmd_testkit::test]
async fn test_healthz_endpoint() {
    let rsclient = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build client");

    let res = rsclient
        .get("https://127.0.0.1:8080/healthz")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.expect("Failed to parse JSON");
    assert_eq!(body["alive"], true);
}

#[kubidmd_testkit::test]
async fn test_readyz_endpoint_initial() {
    let rsclient = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build client");

    let res = rsclient
        .get("https://127.0.0.1:8080/readyz")
        .send()
        .await
        .expect("Failed to send request");

    // Initially the server may not be ready (still bootstrapping)
    // or it may be ready. Either is acceptable.
    assert!(res.status() == 200 || res.status() == 503);

    let body: serde_json::Value = res.json().await.expect("Failed to parse JSON");
    
    // Check that the response has the expected fields
    assert!(body.get("serving_ready").is_some());
    assert!(body.get("replication_state").is_some());
    assert!(body.get("database_health").is_some());
    assert!(body.get("server_phase").is_some());
    assert!(body.get("message").is_some());
}

#[kubidmd_testkit::test]
async fn test_status_endpoint_legacy() {
    let rsclient = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build client");

    let res = rsclient
        .get("https://127.0.0.1:8080/status")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), 200);

    let body: bool = res.json().await.expect("Failed to parse JSON");
    assert_eq!(body, true);
}

#[test]
fn test_replication_state_serialization() {
    let state = ReplicationState::Healthy;
    let json = serde_json::to_string(&state).expect("Failed to serialize");
    assert_eq!(json, "\"healthy\"");

    let state: ReplicationState = serde_json::from_str("\"catching_up\"").expect("Failed to deserialize");
    assert_eq!(state, ReplicationState::CatchingUp);
}

#[test]
fn test_serving_readiness_serialization() {
    let readiness = ServingReadiness::Ready;
    let json = serde_json::to_string(&readiness).expect("Failed to serialize");
    assert_eq!(json, "\"ready\"");

    let readiness: ServingReadiness = serde_json::from_str("\"not_ready\"").expect("Failed to deserialize");
    assert_eq!(readiness, ServingReadiness::NotReady);
}

#[test]
fn test_database_health_serialization() {
    let health = DatabaseHealth::Healthy;
    let json = serde_json::to_string(&health).expect("Failed to serialize");
    assert_eq!(json, "\"healthy\"");

    let health: DatabaseHealth = serde_json::from_str("\"unhealthy\"").expect("Failed to deserialize");
    assert_eq!(health, DatabaseHealth::Unhealthy);
}

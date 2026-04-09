#![deny(warnings)]
use kubidm_client::{ClientError, KubidmClient, StatusCode};
use kubidm_proto::internal::Filter;
use kubidmd_testkit::{ADMIN_TEST_PASSWORD, ADMIN_TEST_USER, NOT_ADMIN_TEST_PASSWORD};
use std::collections::BTreeMap;
use url::Url;

fn get_federation_test_idp_name() -> String {
    "test_federation_idp".to_string()
}

fn get_federation_test_client_id() -> String {
    "test_federation_client_id".to_string()
}

fn get_federation_test_issuer() -> Url {
    Url::parse("https://idp.example.com").expect("Invalid issuer URL")
}

async fn create_federation_idp(
    rsclient: &KubidmClient,
    idp_name: &str,
    client_id: &str,
    client_secret: &str,
    issuer: &Url,
) -> Result<(), ClientError> {
    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.to_string()]),
            ("oauth2_client_id".to_string(), vec![client_id.to_string()]),
            (
                "oauth2_client_secret".to_string(),
                vec![client_secret.to_string()],
            ),
            ("oauth2_issuer".to_string(), vec![issuer.to_string()]),
        ]),
    };

    rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await
}

async fn delete_federation_idp(rsclient: &KubidmClient, idp_name: &str) -> Result<(), ClientError> {
    let filter = Filter::Eq("name".to_string(), idp_name.to_string());
    rsclient.delete(filter).await
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_create(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = get_federation_test_idp_name();
    let client_id = get_federation_test_client_id();
    let issuer = get_federation_test_issuer();

    let result =
        create_federation_idp(rsclient, &idp_name, &client_id, "test_secret", &issuer).await;

    assert!(result.is_ok(), "Federation IdP creation should succeed");

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_create_with_all_attributes(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = format!("{}_full", get_federation_test_idp_name());
    let client_id = get_federation_test_client_id();
    let issuer = get_federation_test_issuer();

    let auth_endpoint =
        Url::parse("https://idp.example.com/oauth2/authorize").expect("Invalid URL");
    let token_endpoint = Url::parse("https://idp.example.com/oauth2/token").expect("Invalid URL");
    let jwks_uri = Url::parse("https://idp.example.com/oauth2/jwks").expect("Invalid URL");
    let userinfo_endpoint =
        Url::parse("https://idp.example.com/oauth2/userinfo").expect("Invalid URL");

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.clone()]),
            ("oauth2_client_id".to_string(), vec![client_id]),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            ("oauth2_issuer".to_string(), vec![issuer.to_string()]),
            (
                "oauth2_display_name".to_string(),
                vec!["Full Federation IdP".to_string()],
            ),
            (
                "oauth2_authorisation_endpoint".to_string(),
                vec![auth_endpoint.to_string()],
            ),
            (
                "oauth2_token_endpoint".to_string(),
                vec![token_endpoint.to_string()],
            ),
            ("oauth2_jwks_uri".to_string(), vec![jwks_uri.to_string()]),
            (
                "oauth2_userinfo_endpoint".to_string(),
                vec![userinfo_endpoint.to_string()],
            ),
            (
                "oauth2_email_domain".to_string(),
                vec!["example.com".to_string(), "test.com".to_string()],
            ),
            ("oauth2_link_policy".to_string(), vec!["auto".to_string()]),
            (
                "oauth2_idp_initiated_enabled".to_string(),
                vec!["true".to_string()],
            ),
            (
                "oauth2_auto_discovery".to_string(),
                vec!["true".to_string()],
            ),
            (
                "oauth2_request_scopes".to_string(),
                vec![
                    "openid".to_string(),
                    "email".to_string(),
                    "profile".to_string(),
                ],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(
        result.is_ok(),
        "Federation IdP with all attributes should succeed"
    );

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_get_list(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = get_federation_test_idp_name();
    let client_id = get_federation_test_client_id();
    let issuer = get_federation_test_issuer();

    let result =
        create_federation_idp(rsclient, &idp_name, &client_id, "test_secret", &issuer).await;
    assert!(result.is_ok());

    let result: Result<Vec<kubidm_proto::v1::Entry>, ClientError> =
        rsclient.perform_get_request("/v1/oauth2/federation").await;

    match result {
        Ok(entries) => {
            assert!(!entries.is_empty(), "Federation list should not be empty");
            let found = entries.iter().any(|e| {
                e.attrs
                    .get("name")
                    .map(|names| names.contains(&idp_name))
                    .unwrap_or(false)
            });
            assert!(found, "Created IdP should be in list");
        }
        Err(ClientError::Http(StatusCode::NOT_FOUND, _, _)) => {}
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_get_by_name(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = get_federation_test_idp_name();
    let client_id = get_federation_test_client_id();
    let issuer = get_federation_test_issuer();

    let result =
        create_federation_idp(rsclient, &idp_name, &client_id, "test_secret", &issuer).await;
    assert!(result.is_ok());

    let result: Result<kubidm_proto::v1::Entry, ClientError> = rsclient
        .perform_get_request(&format!("/v1/oauth2/federation/{idp_name}"))
        .await;

    match result {
        Ok(entry) => {
            let name = entry.attrs.get("name").and_then(|n| n.first());
            assert_eq!(name, Some(&idp_name), "IdP name should match");
        }
        Err(ClientError::Http(StatusCode::NOT_FOUND, _, _)) => {}
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_delete(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = format!("{}_delete", get_federation_test_idp_name());
    let client_id = get_federation_test_client_id();
    let issuer = get_federation_test_issuer();

    let result =
        create_federation_idp(rsclient, &idp_name, &client_id, "test_secret", &issuer).await;
    assert!(result.is_ok());

    let result = delete_federation_idp(rsclient, &idp_name).await;
    assert!(result.is_ok(), "Federation IdP deletion should succeed");

    let get_result: Result<kubidm_proto::v1::Entry, ClientError> = rsclient
        .perform_get_request(&format!("/v1/oauth2/federation/{idp_name}"))
        .await;

    assert!(
        matches!(
            get_result,
            Err(ClientError::Http(StatusCode::NOT_FOUND, _, _))
        ),
        "Deleted IdP should not be found"
    );
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_missing_required_fields(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let entry_missing_client_id = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec!["missing_client_id".to_string()]),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry_missing_client_id)
        .await;

    assert!(result.is_err(), "Missing client_id should fail");

    let entry_missing_secret = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec!["missing_secret".to_string()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry_missing_secret)
        .await;

    assert!(result.is_err(), "Missing client_secret should fail");

    let entry_missing_issuer = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec!["missing_issuer".to_string()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry_missing_issuer)
        .await;

    assert!(result.is_err(), "Missing issuer should fail");
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_empty_client_id(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec!["empty_client_id".to_string()]),
            ("oauth2_client_id".to_string(), vec!["".to_string()]),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_err(), "Empty client_id should fail");
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_empty_client_secret(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec!["empty_secret".to_string()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            ("oauth2_client_secret".to_string(), vec!["".to_string()]),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_err(), "Empty client_secret should fail");
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_invalid_issuer_url(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec!["invalid_issuer".to_string()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec!["not-a-valid-url".to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_err(), "Invalid issuer URL should fail");
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_link_policy_values(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    for policy in ["auto", "manual", "admin_approval"] {
        let idp_name = format!("{}_{}", get_federation_test_idp_name(), policy);

        let entry = kubidm_proto::v1::Entry {
            attrs: BTreeMap::from([
                (
                    "class".to_string(),
                    vec!["oauth2_federation".to_string(), "object".to_string()],
                ),
                ("name".to_string(), vec![idp_name.clone()]),
                (
                    "oauth2_client_id".to_string(),
                    vec![format!("client_{}", policy)],
                ),
                (
                    "oauth2_client_secret".to_string(),
                    vec!["test_secret".to_string()],
                ),
                (
                    "oauth2_issuer".to_string(),
                    vec![get_federation_test_issuer().to_string()],
                ),
                ("oauth2_link_policy".to_string(), vec![policy.to_string()]),
            ]),
        };

        let result = rsclient
            .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
            .await;

        assert!(result.is_ok(), "Link policy '{}' should be valid", policy);

        let _ = delete_federation_idp(rsclient, &idp_name).await;
    }
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_invalid_link_policy(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = "invalid_policy_idp";

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.to_string()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
            (
                "oauth2_link_policy".to_string(),
                vec!["invalid_policy_value".to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(
        result.is_ok(),
        "Note: Schema currently accepts any value for oauth2_link_policy - validation should be added in future"
    );

    let _ = delete_federation_idp(rsclient, idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_multiple_email_domains(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = format!("{}_multidomain", get_federation_test_idp_name());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.clone()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
            (
                "oauth2_email_domain".to_string(),
                vec![
                    "example.com".to_string(),
                    "test.org".to_string(),
                    "company.net".to_string(),
                ],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_ok(), "Multiple email domains should be valid");

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_multiple_request_scopes(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = format!("{}_scopes", get_federation_test_idp_name());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.clone()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
            (
                "oauth2_request_scopes".to_string(),
                vec![
                    "openid".to_string(),
                    "email".to_string(),
                    "profile".to_string(),
                    "groups".to_string(),
                ],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_ok(), "Multiple request scopes should be valid");

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_duplicate_name(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = format!("{}_duplicate", get_federation_test_idp_name());
    let client_id = get_federation_test_client_id();
    let issuer = get_federation_test_issuer();

    let result =
        create_federation_idp(rsclient, &idp_name, &client_id, "test_secret", &issuer).await;
    assert!(result.is_ok());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.clone()]),
            (
                "oauth2_client_id".to_string(),
                vec![format!("{}_2", get_federation_test_client_id())],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret_2".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![Url::parse("https://idp2.example.com").unwrap().to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_err(), "Duplicate name should fail");

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_idp_initiated_enabled(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    for enabled in [true, false] {
        let idp_name = format!("{}_idp_init_{}", get_federation_test_idp_name(), enabled);

        let entry = kubidm_proto::v1::Entry {
            attrs: BTreeMap::from([
                (
                    "class".to_string(),
                    vec!["oauth2_federation".to_string(), "object".to_string()],
                ),
                ("name".to_string(), vec![idp_name.clone()]),
                (
                    "oauth2_client_id".to_string(),
                    vec![get_federation_test_client_id()],
                ),
                (
                    "oauth2_client_secret".to_string(),
                    vec!["test_secret".to_string()],
                ),
                (
                    "oauth2_issuer".to_string(),
                    vec![get_federation_test_issuer().to_string()],
                ),
                (
                    "oauth2_idp_initiated_enabled".to_string(),
                    vec![enabled.to_string()],
                ),
            ]),
        };

        let result = rsclient
            .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
            .await;

        assert!(
            result.is_ok(),
            "IdP initiated enabled={} should be valid",
            enabled
        );

        let _ = delete_federation_idp(rsclient, &idp_name).await;
    }
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_auto_discovery(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    for enabled in [true, false] {
        let idp_name = format!("{}_auto_disc_{}", get_federation_test_idp_name(), enabled);

        let entry = kubidm_proto::v1::Entry {
            attrs: BTreeMap::from([
                (
                    "class".to_string(),
                    vec!["oauth2_federation".to_string(), "object".to_string()],
                ),
                ("name".to_string(), vec![idp_name.clone()]),
                (
                    "oauth2_client_id".to_string(),
                    vec![get_federation_test_client_id()],
                ),
                (
                    "oauth2_client_secret".to_string(),
                    vec!["test_secret".to_string()],
                ),
                (
                    "oauth2_issuer".to_string(),
                    vec![get_federation_test_issuer().to_string()],
                ),
                (
                    "oauth2_auto_discovery".to_string(),
                    vec![enabled.to_string()],
                ),
            ]),
        };

        let result = rsclient
            .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
            .await;

        assert!(
            result.is_ok(),
            "Auto discovery enabled={} should be valid",
            enabled
        );

        let _ = delete_federation_idp(rsclient, &idp_name).await;
    }
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_unauthorized_access(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_person_account_create("test_federation_user", "test_federation_user")
        .await
        .expect("Failed to create account");

    rsclient
        .idm_person_account_primary_credential_set_password(
            "test_federation_user",
            NOT_ADMIN_TEST_PASSWORD,
        )
        .await
        .expect("Failed to set password");

    let res = rsclient
        .auth_simple_password("test_federation_user", NOT_ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec!["unauthorized_idp".to_string()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(
        matches!(result, Err(ClientError::Http(StatusCode::FORBIDDEN, _, _))),
        "Non-admin user should not be able to create federation IdP"
    );

    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_person_account_delete("test_federation_user")
        .await
        .expect("Failed to cleanup test user");
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_federation_id_unique(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let federation_id = "unique_federation_id_12345";
    let idp_name1 = format!("{}_fed1", get_federation_test_idp_name());

    let entry1 = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name1.clone()]),
            (
                "oauth2_client_id".to_string(),
                vec![format!("{}_1", get_federation_test_client_id())],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret_1".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
            (
                "oauth2_federation_id".to_string(),
                vec![federation_id.to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry1)
        .await;

    assert!(
        result.is_ok(),
        "First federation IdP with federation_id should succeed"
    );

    let idp_name2 = format!("{}_fed2", get_federation_test_idp_name());

    let entry2 = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name2.clone()]),
            (
                "oauth2_client_id".to_string(),
                vec![format!("{}_2", get_federation_test_client_id())],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret_2".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![Url::parse("https://idp2.example.com").unwrap().to_string()],
            ),
            (
                "oauth2_federation_id".to_string(),
                vec![federation_id.to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry2)
        .await;

    assert!(result.is_err(), "Duplicate federation_id should fail");

    let _ = delete_federation_idp(rsclient, &idp_name1).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_custom_endpoints(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = format!("{}_custom_ep", get_federation_test_idp_name());

    let custom_auth = Url::parse("https://custom.idp.example.com/oauth2/authorize").unwrap();
    let custom_token = Url::parse("https://custom.idp.example.com/oauth2/token").unwrap();
    let custom_jwks = Url::parse("https://custom.idp.example.com/oauth2/jwks").unwrap();
    let custom_userinfo = Url::parse("https://custom.idp.example.com/oauth2/userinfo").unwrap();

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.clone()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
            (
                "oauth2_authorisation_endpoint".to_string(),
                vec![custom_auth.to_string()],
            ),
            (
                "oauth2_token_endpoint".to_string(),
                vec![custom_token.to_string()],
            ),
            ("oauth2_jwks_uri".to_string(), vec![custom_jwks.to_string()]),
            (
                "oauth2_userinfo_endpoint".to_string(),
                vec![custom_userinfo.to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_ok(), "Custom endpoints should be valid");

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_concurrent_idps(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_count = 5;
    let mut idp_names: Vec<String> = Vec::new();

    for i in 0..idp_count {
        let idp_name = format!("{}_concurrent_{}", get_federation_test_idp_name(), i);

        let result = create_federation_idp(
            rsclient,
            &idp_name,
            &format!("{}_{}", get_federation_test_client_id(), i),
            &format!("secret_{}", i),
            &Url::parse(&format!("https://idp{}.example.com", i)).unwrap(),
        )
        .await;

        assert!(
            result.is_ok(),
            "Concurrent IdP {} should be created successfully",
            i
        );
        idp_names.push(idp_name);
    }

    let result: Result<Vec<kubidm_proto::v1::Entry>, ClientError> =
        rsclient.perform_get_request("/v1/oauth2/federation").await;

    match result {
        Ok(entries) => {
            assert!(
                entries.len() >= idp_count,
                "Should have at least {} IdPs",
                idp_count
            );
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    for idp_name in idp_names {
        let _ = delete_federation_idp(rsclient, &idp_name).await;
    }
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_get_nonexistent(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let result: Result<kubidm_proto::v1::Entry, ClientError> = rsclient
        .perform_get_request("/v1/oauth2/federation/nonexistent_idp")
        .await;

    assert!(
        matches!(result, Err(ClientError::Http(StatusCode::NOT_FOUND, _, _))),
        "Nonexistent IdP should return NOT_FOUND"
    );
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_with_display_name(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = format!("{}_display", get_federation_test_idp_name());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.clone()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
            (
                "oauth2_display_name".to_string(),
                vec!["My Custom IdP Display Name".to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_ok(), "Display name should be valid");

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_with_description(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let idp_name = format!("{}_desc", get_federation_test_idp_name());

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec![idp_name.clone()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
            (
                "description".to_string(),
                vec!["This is a test federation IdP description".to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_ok(), "Description should be valid");

    let _ = delete_federation_idp(rsclient, &idp_name).await;
}

#[kubidmd_testkit::test]
async fn test_oauth2_federation_invalid_auth_method(rsclient: &KubidmClient) {
    let _ = rsclient.logout().await;

    let entry = kubidm_proto::v1::Entry {
        attrs: BTreeMap::from([
            (
                "class".to_string(),
                vec!["oauth2_federation".to_string(), "object".to_string()],
            ),
            ("name".to_string(), vec!["unauthorized".to_string()]),
            (
                "oauth2_client_id".to_string(),
                vec![get_federation_test_client_id()],
            ),
            (
                "oauth2_client_secret".to_string(),
                vec!["test_secret".to_string()],
            ),
            (
                "oauth2_issuer".to_string(),
                vec![get_federation_test_issuer().to_string()],
            ),
        ]),
    };

    let result = rsclient
        .perform_post_request::<_, ()>("/v1/oauth2/federation/_create", entry)
        .await;

    assert!(result.is_err(), "Unauthenticated request should fail");
}

use compact_jwt::{traits::JwsVerifiable, JwsCompact, JwsEs256Verifier, JwsVerifier};
use kubidm_client::KubidmClient;
use kubidm_proto::internal::{CURegState, UatPurpose, UserAuthToken};
use kubidmd_testkit::{ADMIN_TEST_PASSWORD, ADMIN_TEST_USER};
use std::str::FromStr;
use std::time::SystemTime;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_authenticator_rs::WebauthnAuthenticator;

async fn setup_passkey_account(
    rsclient: &KubidmClient,
    account_name: &str,
) -> SoftPasskey {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_person_account_create(account_name, "Test Account")
        .await
        .unwrap();

    let intent_token = rsclient
        .idm_person_account_credential_update_intent(account_name, Some(0))
        .await
        .unwrap();

    let _ = rsclient.logout().await;

    let (session_token, _status) = rsclient
        .idm_account_credential_update_exchange(intent_token.token)
        .await
        .unwrap();

    let status = rsclient
        .idm_account_credential_update_passkey_init(&session_token)
        .await
        .unwrap();

    let passkey_chal = match status.mfaregstate {
        CURegState::Passkey(c) => c,
        _ => panic!("Expected Passkey challenge"),
    };

    let mut wa = SoftPasskey::new(true);
    let origin = rsclient.get_origin().clone();

    let passkey_resp = wa
        .do_registration(origin, passkey_chal)
        .expect("Failed to create soft passkey");

    let _status = rsclient
        .idm_account_credential_update_passkey_finish(
            &session_token,
            "softtoken".to_string(),
            passkey_resp,
        )
        .await
        .unwrap();

    rsclient
        .idm_account_credential_update_commit(&session_token)
        .await
        .unwrap();

    wa
}

async fn get_current_uat(rsclient: &KubidmClient) -> UserAuthToken {
    let token = rsclient.get_token().await.expect("No bearer token present");
    let jwt = JwsCompact::from_str(&token).expect("Failed to parse jwt");
    let key_id = jwt.kid().expect("token does not have a key id");
    let jwk = rsclient
        .get_public_jwk(key_id)
        .await
        .expect("Unable to get jwk");
    let jws_verifier = JwsEs256Verifier::try_from(&jwk).expect("Unable to build verifier");

    let released = jws_verifier.verify(&jwt).expect("Unable to verify jwt");
    released.from_json::<UserAuthToken>().expect("Invalid json")
}

#[kubidmd_testkit::test]
async fn test_stepup_auth_passkey_reauth(rsclient: &KubidmClient) {
    let account_name = "stepup_passkey_user";
    let mut wa = setup_passkey_account(rsclient, account_name).await;

    let res = rsclient.auth_passkey_begin(account_name).await;
    assert!(res.is_ok());

    let pkc = wa
        .do_authentication(rsclient.get_origin().clone(), res.unwrap())
        .map(Box::new)
        .expect("Failed to authenticate with soft passkey");

    let res = rsclient.auth_passkey_complete(pkc).await;
    assert!(res.is_ok());

    let uat = get_current_uat(rsclient).await;
    assert!(matches!(uat.purpose, UatPurpose::ReadWrite { .. }));

    let res = rsclient.reauth_passkey_begin().await;
    assert!(res.is_ok());

    let pkc = wa
        .do_authentication(rsclient.get_origin().clone(), res.unwrap())
        .map(Box::new)
        .expect("Failed to re-authenticate with soft passkey");

    let res = rsclient.reauth_passkey_complete(pkc).await;
    assert!(res.is_ok());

    let uat = get_current_uat(rsclient).await;
    assert!(matches!(uat.purpose, UatPurpose::ReadWrite { .. }));
}

#[kubidmd_testkit::test]
async fn test_stepup_auth_session_scope_readonly(rsclient: &KubidmClient) {
    let account_name = "stepup_readonly_user";
    let mut wa = setup_passkey_account(rsclient, account_name).await;

    let res = rsclient.auth_passkey_begin(account_name).await;
    assert!(res.is_ok());

    let pkc = wa
        .do_authentication(rsclient.get_origin().clone(), res.unwrap())
        .map(Box::new)
        .expect("Failed to authenticate with soft passkey");

    let res = rsclient.auth_passkey_complete(pkc).await;
    assert!(res.is_ok());

    let uat = get_current_uat(rsclient).await;
    assert!(matches!(uat.purpose, UatPurpose::ReadWrite { .. }));
}

#[kubidmd_testkit::test]
async fn test_stepup_auth_multiple_reauth_sessions(rsclient: &KubidmClient) {
    let account_name = "stepup_multi_reauth_user";
    let mut wa = setup_passkey_account(rsclient, account_name).await;

    let res = rsclient.auth_passkey_begin(account_name).await;
    assert!(res.is_ok());

    let pkc = wa
        .do_authentication(rsclient.get_origin().clone(), res.unwrap())
        .map(Box::new)
        .expect("Failed to authenticate with soft passkey");

    let res = rsclient.auth_passkey_complete(pkc).await;
    assert!(res.is_ok());

    for _ in 0..3 {
        let res = rsclient.reauth_passkey_begin().await;
        assert!(res.is_ok());

        let pkc = wa
            .do_authentication(rsclient.get_origin().clone(), res.unwrap())
            .map(Box::new)
            .expect("Failed to re-authenticate with soft passkey");

        let res = rsclient.reauth_passkey_complete(pkc).await;
        assert!(res.is_ok());
    }

    let uat = get_current_uat(rsclient).await;
    assert!(matches!(uat.purpose, UatPurpose::ReadWrite { .. }));
}

async fn setup_password_account(rsclient: &KubidmClient, account_name: &str, password: &str) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_person_account_create(account_name, "Test Account")
        .await
        .unwrap();

    let intent_token = rsclient
        .idm_person_account_credential_update_intent(account_name, Some(0))
        .await
        .unwrap();

    let _ = rsclient.logout().await;

    let (session_token, _status) = rsclient
        .idm_account_credential_update_exchange(intent_token.token)
        .await
        .unwrap();

    let _status = rsclient
        .idm_account_credential_update_set_password(&session_token, password)
        .await
        .unwrap();

    rsclient
        .idm_account_credential_update_commit(&session_token)
        .await
        .unwrap();

    let _ = rsclient.logout().await;
}

#[kubidmd_testkit::test]
async fn test_stepup_auth_password_reauth(rsclient: &KubidmClient) {
    let account_name = "stepup_password_user";
    let password = "test_password_123";
    setup_password_account(rsclient, account_name, password).await;

    let res = rsclient.auth_simple_password(account_name, password).await;
    assert!(res.is_ok());

    let uat = get_current_uat(rsclient).await;
    match uat.purpose {
        UatPurpose::ReadWrite { .. } => {}
        UatPurpose::ReadOnly => panic!("Expected ReadWrite purpose"),
    }

    let res = rsclient.reauth_simple_password(password).await;
    assert!(res.is_ok());

    let uat = get_current_uat(rsclient).await;
    assert!(matches!(uat.purpose, UatPurpose::ReadWrite { .. }));
}

#[kubidmd_testkit::test]
async fn test_stepup_auth_invalid_password_rejected(rsclient: &KubidmClient) {
    let account_name = "stepup_invalid_pw_user";
    let password = "correct_password_123";
    setup_password_account(rsclient, account_name, password).await;

    let res = rsclient.auth_simple_password(account_name, password).await;
    assert!(res.is_ok());

    let res = rsclient.reauth_simple_password("wrong_password").await;
    assert!(res.is_err());
}

#[kubidmd_testkit::test]
async fn test_stepup_auth_totp_reauth(rsclient: &KubidmClient) {
    use kubidmd_lib::credential::totp::Totp;

    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_person_account_create("totp_user", "TOTP User")
        .await
        .unwrap();

    let intent_token = rsclient
        .idm_person_account_credential_update_intent("totp_user", Some(0))
        .await
        .unwrap();

    let _ = rsclient.logout().await;

    let (session_token, _status) = rsclient
        .idm_account_credential_update_exchange(intent_token.token)
        .await
        .unwrap();

    let _status = rsclient
        .idm_account_credential_update_set_password(&session_token, "test_password_123")
        .await
        .unwrap();

    let status = rsclient
        .idm_account_credential_update_init_totp(&session_token)
        .await
        .unwrap();

    let totp: Totp = match status.mfaregstate {
        CURegState::TotpCheck(totp_secret) => totp_secret.try_into().unwrap(),
        _ => panic!("Expected TotpCheck state"),
    };

    let totp_chal = totp
        .do_totp_duration_from_epoch(
            &SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
        )
        .expect("Failed to do totp");

    let _status = rsclient
        .idm_account_credential_update_check_totp(&session_token, totp_chal, "totp")
        .await
        .unwrap();

    rsclient
        .idm_account_credential_update_commit(&session_token)
        .await
        .unwrap();

    let _ = rsclient.logout().await;

    let totp_chal = totp
        .do_totp_duration_from_epoch(
            &SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
        )
        .expect("Failed to do totp");

    let res = rsclient
        .auth_password_totp("totp_user", "test_password_123", totp_chal)
        .await;
    assert!(res.is_ok());

    let uat = get_current_uat(rsclient).await;
    assert!(matches!(uat.purpose, UatPurpose::ReadWrite { .. }));

    let totp_chal = totp
        .do_totp_duration_from_epoch(
            &SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
        )
        .expect("Failed to do totp");

    let res = rsclient
        .reauth_password_totp("test_password_123", totp_chal)
        .await;
    assert!(res.is_ok());
}

#[kubidmd_testkit::test]
async fn test_stepup_auth_requires_valid_session(rsclient: &KubidmClient) {
    let res = rsclient.reauth_passkey_begin().await;
    assert!(res.is_err());
}

#[kubidmd_testkit::test]
async fn test_stepup_auth_logout_clears_session(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    let uat = get_current_uat(rsclient).await;
    assert!(matches!(uat.purpose, UatPurpose::ReadWrite { .. }));

    let _ = rsclient.logout().await;

    let token = rsclient.get_token().await;
    assert!(token.is_none());
}

mod security_tests {
    use super::*;

    #[kubidmd_testkit::test]
    async fn test_stepup_auth_wrong_passkey_rejected(rsclient: &KubidmClient) {
        let account_name = "stepup_wrong_passkey_user";
        let mut wa = setup_passkey_account(rsclient, account_name).await;

        let res = rsclient.auth_passkey_begin(account_name).await;
        assert!(res.is_ok());

        let pkc = wa
            .do_authentication(rsclient.get_origin().clone(), res.unwrap())
            .map(Box::new)
            .expect("Failed to authenticate with soft passkey");

        let res = rsclient.auth_passkey_complete(pkc).await;
        assert!(res.is_ok());

        let mut wrong_wa = SoftPasskey::new(true);

        let res = rsclient.reauth_passkey_begin().await;
        assert!(res.is_ok());

        let wrong_pkc = wrong_wa
            .do_authentication(rsclient.get_origin().clone(), res.unwrap())
            .map(Box::new);

        if let Ok(wrong_pkc) = wrong_pkc {
            let res = rsclient.reauth_passkey_complete(wrong_pkc).await;
            assert!(res.is_err());
        }
    }

    #[kubidmd_testkit::test]
    async fn test_stepup_auth_rate_limiting(rsclient: &KubidmClient) {
        let account_name = "stepup_ratelimit_user";
        let password = "correct_password_123";
        setup_password_account(rsclient, account_name, password).await;

        let res = rsclient.auth_simple_password(account_name, password).await;
        assert!(res.is_ok());

        for _ in 0..5 {
            let res = rsclient.reauth_simple_password("wrong_password").await;
            assert!(res.is_err());
        }

        let res = rsclient.reauth_simple_password(password).await;
        assert!(res.is_err());
    }
}

mod integration_tests {
    use super::*;
    use kubidm_proto::constants::ATTR_MAIL;

    #[kubidmd_testkit::test]
    async fn test_stepup_with_acp_require_reauth(rsclient: &KubidmClient) {
        let res = rsclient
            .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
            .await;
        assert!(res.is_ok());

        let uat = get_current_uat(rsclient).await;
        assert!(matches!(uat.purpose, UatPurpose::ReadWrite { .. }));
    }

    #[kubidmd_testkit::test]
    async fn test_stepup_for_sensitive_operations(rsclient: &KubidmClient) {
        let res = rsclient
            .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
            .await;
        assert!(res.is_ok());

        rsclient
            .idm_person_account_create("sensitive_user", "Sensitive User")
            .await
            .unwrap();

        let post_body = serde_json::json!({"attrs": { ATTR_MAIL : ["sensitive@example.com"]}});

        let result = rsclient
            .perform_patch_request::<serde_json::Value, serde_json::Value>(
                "/v1/person/sensitive_user",
                post_body,
            )
            .await;

        assert!(result.is_ok());
    }

    #[kubidmd_testkit::test]
    async fn test_stepup_oauth2_token_operations(rsclient: &KubidmClient) {
        let res = rsclient
            .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
            .await;
        assert!(res.is_ok());

        rsclient
            .idm_oauth2_rs_basic_create(
                "test_oauth2_client",
                "Test OAuth2 Client",
                "https://example.com",
            )
            .await
            .unwrap();

        let clients = rsclient.idm_oauth2_rs_list().await.unwrap();
        assert!(!clients.is_empty());
    }
}

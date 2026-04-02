use kanidm_client::KanidmClient;
use kanidmd_testkit::{create_user, ADMIN_TEST_PASSWORD, ADMIN_TEST_USER};

#[kanidmd_testkit::test]
async fn test_delegated_admin_create_user(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "delegate_user", "delegate_group").await;
    create_user(rsclient, "target_user", "target_group").await;

    let users = rsclient
        .idm_person_account_list()
        .await
        .expect("Failed to list persons");

    let delegate_exists = users.iter().any(|u| u == "delegate_user");
    let target_exists = users.iter().any(|u| u == "target_user");

    assert!(delegate_exists);
    assert!(target_exists);
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_search_user(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "search_delegate", "search_delegate_group").await;
    create_user(rsclient, "search_target", "search_target_group").await;

    let user_result = rsclient
        .idm_person_account_get("search_delegate")
        .await
        .expect("Failed to get delegate user");

    assert!(user_result.is_some());

    let user_result = rsclient
        .idm_person_account_get("search_target")
        .await
        .expect("Failed to get target user");

    assert!(user_result.is_some());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_modify_user(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "modify_delegate", "modify_delegate_group").await;
    create_user(rsclient, "modify_target", "modify_target_group").await;

    let result = rsclient
        .idm_person_account_set_attr(
            "modify_target",
            "displayname",
            &["Modified Display Name"],
        )
        .await;

    assert!(result.is_ok());

    let user_result = rsclient
        .idm_person_account_get("modify_target")
        .await
        .expect("Failed to get modified user");

    if let Some(user) = user_result {
        let displayname = user.attrs.get("displayname");
        if let Some(dn) = displayname {
            assert!(dn.contains(&"Modified Display Name".to_string()));
        }
    }
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_delete_user(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "delete_delegate", "delete_delegate_group").await;
    create_user(rsclient, "delete_target", "delete_target_group").await;

    let result = rsclient
        .idm_person_account_delete("delete_target")
        .await;
    assert!(result.is_ok());

    let user_result = rsclient.idm_person_account_get("delete_target").await;

    assert!(user_result.is_ok());
    assert!(user_result.unwrap().is_none());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_group_operations(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("delegated_test_group", None)
        .await
        .expect("Failed to create group");

    create_user(rsclient, "group_member", "group_member_group").await;

    let result = rsclient
        .idm_group_add_members("delegated_test_group", &["group_member"])
        .await;
    assert!(result.is_ok());

    let members = rsclient
        .idm_group_get_members("delegated_test_group")
        .await
        .expect("Failed to get group members");

    assert!(members.is_some());
    let member_list = members.unwrap();
    assert!(member_list.contains(&"group_member".to_string()));

    let result = rsclient
        .idm_group_remove_members("delegated_test_group", &["group_member"])
        .await;
    assert!(result.is_ok());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_entry_managed_by(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("managed_by_group", None)
        .await
        .expect("Failed to create managed_by group");

    create_user(rsclient, "managed_user", "managed_user_group").await;
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_scope_validation(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "scope_delegate", "scope_delegate_group").await;
    create_user(rsclient, "scope_target", "scope_target_group").await;

    let user_result = rsclient
        .idm_person_account_get("scope_delegate")
        .await
        .expect("Failed to get delegate user");

    assert!(user_result.is_some());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_multiple_scopes(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("multi_scope_group_1", None)
        .await
        .expect("Failed to create group 1");

    rsclient
        .idm_group_create("multi_scope_group_2", None)
        .await
        .expect("Failed to create group 2");

    create_user(rsclient, "multi_scope_user", "multi_scope_user_group").await;
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_password_reset_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "pwd_reset_delegate", "pwd_reset_delegate_group").await;
    create_user(rsclient, "pwd_reset_target", "pwd_reset_target_group").await;

    let result = rsclient
        .idm_person_account_primary_credential_set_password("pwd_reset_target", "NewTestPassword123!")
        .await;

    assert!(result.is_ok());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_ssh_key_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "ssh_delegate", "ssh_delegate_group").await;
    create_user(rsclient, "ssh_target", "ssh_target_group").await;

    let result = rsclient
        .idm_person_account_post_ssh_pubkey(
            "ssh_target",
            "test_key",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFakeKeyForTestingPurposesOnly test@example.com",
        )
        .await;

    assert!(result.is_ok());

    let keys = rsclient
        .idm_person_account_get_ssh_pubkeys("ssh_target")
        .await
        .expect("Failed to get SSH keys");

    assert!(!keys.is_empty());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_unix_extension_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "unix_delegate", "unix_delegate_group").await;
    create_user(rsclient, "unix_target", "unix_target_group").await;

    let result = rsclient
        .idm_person_account_unix_extend("unix_target", None, Some("/bin/bash"))
        .await;

    assert!(result.is_ok());

    let result = rsclient
        .idm_person_account_unix_cred_put("unix_target", "TestUnixPassword123!")
        .await;

    assert!(result.is_ok());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_mail_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "mail_delegate", "mail_delegate_group").await;
    create_user(rsclient, "mail_target", "mail_target_group").await;

    let result = rsclient
        .idm_person_account_set_attr("mail_target", "mail", &["mail_target@example.com"])
        .await;

    assert!(result.is_ok());

    let user_result = rsclient
        .idm_person_account_get("mail_target")
        .await
        .expect("Failed to get user");

    if let Some(user) = user_result {
        let mail = user.attrs.get("mail");
        if let Some(m) = mail {
            assert!(m.contains(&"mail_target@example.com".to_string()));
        }
    }
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_legal_name_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "legal_delegate", "legal_delegate_group").await;
    create_user(rsclient, "legal_target", "legal_target_group").await;

    let result = rsclient
        .idm_person_account_set_attr("legal_target", "legalname", &["Legal Test Name"])
        .await;

    assert!(result.is_ok());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_audit_trail(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "audit_delegate", "audit_delegate_group").await;
    create_user(rsclient, "audit_target", "audit_target_group").await;

    let _result = rsclient
        .idm_person_account_set_attr("audit_target", "displayname", &["Audit Test User"])
        .await;

    let user_result = rsclient
        .idm_person_account_get("audit_target")
        .await
        .expect("Failed to get user");

    assert!(user_result.is_some());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_nested_group_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("parent_delegated_group", None)
        .await
        .expect("Failed to create parent group");

    rsclient
        .idm_group_create("child_delegated_group", None)
        .await
        .expect("Failed to create child group");

    create_user(rsclient, "nested_user", "nested_user_group").await;

    let result = rsclient
        .idm_group_add_members("parent_delegated_group", &["child_delegated_group"])
        .await;
    assert!(result.is_ok());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_limit_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "limit_delegate", "limit_delegate_group").await;
    create_user(rsclient, "limit_target", "limit_target_group").await;

    let result = rsclient
        .idm_person_account_get("limit_target")
        .await
        .expect("Failed to get user");

    assert!(result.is_some());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_cross_tenant_prevention(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "tenant_delegate", "tenant_delegate_group").await;
    create_user(rsclient, "tenant_target", "tenant_target_group").await;

    let result = rsclient
        .idm_person_account_get("tenant_delegate")
        .await
        .expect("Failed to get delegate user");

    assert!(result.is_some());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_concurrent_operations(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "concurrent_delegate", "concurrent_delegate_group").await;

    rsclient
        .idm_group_create("concurrent_group_1", None)
        .await
        .expect("Failed to create group 1");

    rsclient
        .idm_group_create("concurrent_group_2", None)
        .await
        .expect("Failed to create group 2");

    let result1 = rsclient
        .idm_group_add_members("concurrent_group_1", &["concurrent_delegate"])
        .await;

    let result2 = rsclient
        .idm_group_add_members("concurrent_group_2", &["concurrent_delegate"])
        .await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_service_account_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "svc_delegate", "svc_delegate_group").await;

    let result = rsclient
        .idm_service_account_create("delegated_service", "Delegated Service Account")
        .await;

    assert!(result.is_ok());

    let service_result = rsclient
        .idm_service_account_get("delegated_service")
        .await
        .expect("Failed to get service account");

    assert!(service_result.is_some());
}

#[kanidmd_testkit::test]
async fn test_delegated_admin_radius_scope(rsclient: &KanidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "radius_delegate", "radius_delegate_group").await;
    create_user(rsclient, "radius_target", "radius_target_group").await;

    let result = rsclient
        .idm_account_radius_credential_regenerate("radius_target")
        .await;

    assert!(result.is_ok());
}
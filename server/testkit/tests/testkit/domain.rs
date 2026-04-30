use kubidm_client::KubidmClient;
use kubidm_proto::constants::ATTR_DOMAIN_DISPLAY_NAME;
use kubidmd_testkit::{ADMIN_TEST_PASSWORD, ADMIN_TEST_USER};

#[kubidmd_testkit::test]
async fn test_idm_set_ldap_allow_unix_password_bind(rsclient: &KubidmClient) {
    rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await
        .expect("Failed to login as admin");
    rsclient
        .idm_set_ldap_allow_unix_password_bind(true)
        .await
        .expect("Failed to set LDAP allow unix password bind to true");
}

#[kubidmd_testkit::test]
async fn test_idm_domain_set_ldap_basedn(rsclient: &KubidmClient) {
    rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await
        .expect("Failed to login as admin");

    rsclient
        .idm_domain_set_ldap_basedn("dc=example,dc=com")
        .await
        .expect("Failed to set idm_domain_set_ldap_basedn");
}

#[kubidmd_testkit::test]
async fn test_idm_domain_set_ldap_max_queryable_attrs(rsclient: &KubidmClient) {
    rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await
        .expect("Failed to login as admin");

    rsclient
        .idm_domain_set_ldap_max_queryable_attrs(30)
        .await
        .expect("Failed to set idm_domain_set_ldap_max_queryable_attrs");
}

#[kubidmd_testkit::test]
async fn test_idm_domain_set_display_name(rsclient: &KubidmClient) {
    rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await
        .expect("Failed to login as admin");

    let new_domain_display_name = "hello kubidm 12345667";

    rsclient
        .idm_domain_set_display_name(new_domain_display_name)
        .await
        .expect("Failed to set idm_domain_set_display_name");

    let domain_after = rsclient
        .idm_domain_get()
        .await
        .expect("Failed to idm_domain_get");

    assert_eq!(
        domain_after.attrs.get(ATTR_DOMAIN_DISPLAY_NAME),
        Some(&vec![new_domain_display_name.to_string()])
    );
}

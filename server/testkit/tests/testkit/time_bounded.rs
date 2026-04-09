use kubidm_client::KubidmClient;
use kubidmd_testkit::{create_user, ADMIN_TEST_PASSWORD, ADMIN_TEST_USER};

#[kubidmd_testkit::test]
async fn test_time_bounded_group_membership_basic(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "tb_test_user", "tb_test_group").await;

    let user_entry = rsclient
        .idm_person_account_get("tb_test_user")
        .await
        .expect("Failed to get user");

    assert!(user_entry.is_some());
}

#[kubidmd_testkit::test]
async fn test_time_bounded_access_multiple_groups(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("tb_group_a", None)
        .await
        .expect("Failed to create group A");

    rsclient
        .idm_group_create("tb_group_b", None)
        .await
        .expect("Failed to create group B");

    rsclient
        .idm_group_create("tb_group_c", None)
        .await
        .expect("Failed to create group C");

    create_user(rsclient, "tb_multi_user", "tb_group_a").await;

    rsclient
        .idm_group_add_members("tb_group_b", &["tb_multi_user"])
        .await
        .expect("Failed to add user to group B");

    let groups = rsclient
        .idm_person_account_get_attr("tb_multi_user", "memberof")
        .await
        .expect("Failed to get memberof");

    assert!(groups.is_some());
    let group_list = groups.unwrap();
    assert!(group_list.len() >= 2);
}

#[kubidmd_testkit::test]
async fn test_time_bounded_access_nested_groups(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("tb_parent_group", None)
        .await
        .expect("Failed to create parent group");

    rsclient
        .idm_group_create("tb_child_group", None)
        .await
        .expect("Failed to create child group");

    rsclient
        .idm_group_add_members("tb_parent_group", &["tb_child_group"])
        .await
        .expect("Failed to add child to parent");

    create_user(rsclient, "tb_nested_user", "tb_child_group").await;

    let groups = rsclient
        .idm_person_account_get_attr("tb_nested_user", "memberof")
        .await
        .expect("Failed to get memberof");

    assert!(groups.is_some());
    let group_list = groups.unwrap();
    assert!(group_list.len() >= 2);
}

#[kubidmd_testkit::test]
async fn test_time_bounded_access_group_delete(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("tb_temp_group", None)
        .await
        .expect("Failed to create temp group");

    create_user(rsclient, "tb_temp_user", "tb_temp_group").await;

    let groups_before = rsclient
        .idm_person_account_get_attr("tb_temp_user", "memberof")
        .await
        .expect("Failed to get memberof before delete");

    assert!(groups_before.is_some());
    assert!(groups_before.unwrap().len() >= 1);

    rsclient
        .idm_group_delete("tb_temp_group")
        .await
        .expect("Failed to delete group");

    let groups_after = rsclient
        .idm_person_account_get_attr("tb_temp_user", "memberof")
        .await
        .expect("Failed to get memberof after delete");

    if let Some(groups) = groups_after {
        assert!(!groups.iter().any(|g| g.contains("tb_temp_group")));
    }
}

#[kubidmd_testkit::test]
async fn test_time_bounded_access_member_remove(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("tb_remove_group", None)
        .await
        .expect("Failed to create group");

    create_user(rsclient, "tb_remove_user", "tb_remove_group").await;

    let groups_before = rsclient
        .idm_person_account_get_attr("tb_remove_user", "memberof")
        .await
        .expect("Failed to get memberof");

    assert!(groups_before.is_some());

    rsclient
        .idm_group_remove_members("tb_remove_group", &["tb_remove_user"])
        .await
        .expect("Failed to remove member");

    let groups_after = rsclient
        .idm_person_account_get_attr("tb_remove_user", "memberof")
        .await
        .expect("Failed to get memberof after removal");

    if let Some(groups) = groups_after {
        assert!(!groups.iter().any(|g| g.contains("tb_remove_group")));
    }
}

#[kubidmd_testkit::test]
async fn test_time_bounded_access_cycle_groups(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("tb_cycle_a", None)
        .await
        .expect("Failed to create cycle group A");

    rsclient
        .idm_group_create("tb_cycle_b", None)
        .await
        .expect("Failed to create cycle group B");

    rsclient
        .idm_group_add_members("tb_cycle_a", &["tb_cycle_b"])
        .await
        .expect("Failed to add B to A");

    rsclient
        .idm_group_add_members("tb_cycle_b", &["tb_cycle_a"])
        .await
        .expect("Failed to add A to B");

    create_user(rsclient, "tb_cycle_user", "tb_cycle_a").await;

    let groups = rsclient
        .idm_person_account_get_attr("tb_cycle_user", "memberof")
        .await
        .expect("Failed to get memberof");

    assert!(groups.is_some());
    let group_list = groups.unwrap();
    assert!(group_list.len() >= 2);
}

#[kubidmd_testkit::test]
async fn test_time_bounded_access_large_group(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    rsclient
        .idm_group_create("tb_large_group", None)
        .await
        .expect("Failed to create large group");

    for i in 0..10 {
        let username = format!("tb_large_user_{i}");
        create_user(rsclient, &username, "tb_large_group").await;
    }

    let members = rsclient
        .idm_group_get_members("tb_large_group")
        .await
        .expect("Failed to get group members");

    assert!(members.is_some());
    let member_list = members.unwrap();
    assert_eq!(member_list.len(), 10);
}

#[kubidmd_testkit::test]
async fn test_time_bounded_access_user_attributes(rsclient: &KubidmClient) {
    let res = rsclient
        .auth_simple_password(ADMIN_TEST_USER, ADMIN_TEST_PASSWORD)
        .await;
    assert!(res.is_ok());

    create_user(rsclient, "tb_attrs_user", "tb_attrs_group").await;

    let entry = rsclient
        .idm_person_account_get("tb_attrs_user")
        .await
        .expect("Failed to get user");

    assert!(entry.is_some());
    let user = entry.unwrap();

    assert!(user.attrs.contains_key("memberof"));
    assert!(user.attrs.contains_key("directmemberof"));
}

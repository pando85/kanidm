use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use actix::prelude::*;
use kanidm::config::{Configuration, IntegrationTestConfig};
use kanidm::core::create_server_core;

use kanidm_unix_common::cache::CacheLayer;
use tokio::runtime::Runtime;

use kanidm_client::asynchronous::KanidmAsyncClient;
use kanidm_client::{KanidmClient, KanidmClientBuilder};

static PORT_ALLOC: AtomicUsize = AtomicUsize::new(18080);
const ADMIN_TEST_PASSWORD: &str = "integration test admin password";
const TESTACCOUNT1_PASSWORD_A: &str = "password a for account1 test";
const TESTACCOUNT1_PASSWORD_B: &str = "password b for account1 test";
const TESTACCOUNT1_PASSWORD_INC: &str = "never going to work";

fn run_test(fix_fn: fn(&KanidmClient) -> (), test_fn: fn(CacheLayer, KanidmAsyncClient) -> ()) {
    // ::std::env::set_var("RUST_LOG", "actix_web=debug,kanidm=debug");
    let _ = env_logger::builder().is_test(true).try_init();
    let (tx, rx) = mpsc::channel();
    let port = PORT_ALLOC.fetch_add(1, Ordering::SeqCst);

    let int_config = Box::new(IntegrationTestConfig {
        admin_password: ADMIN_TEST_PASSWORD.to_string(),
    });

    let mut config = Configuration::new();
    config.address = format!("127.0.0.1:{}", port);
    config.secure_cookies = false;
    config.integration_test_config = Some(int_config);
    // Setup the config ...

    thread::spawn(move || {
        // Spawn a thread for the test runner, this should have a unique
        // port....
        System::run(move || {
            create_server_core(config);
            let _ = tx.send(System::current());
        })
        .expect("Failed to start system");
    });
    let sys = rx.recv().unwrap();
    System::set_current(sys.clone());

    // Setup the client, and the address we selected.
    let addr = format!("http://127.0.0.1:{}", port);

    // Run fixtures
    let adminclient = KanidmClientBuilder::new()
        .address(addr.clone())
        .build()
        .expect("Failed to build sync client");
    fix_fn(&adminclient);

    let client = KanidmClientBuilder::new()
        .address(addr.clone())
        .build_async()
        .expect("Failed to build async admin client");

    let rsclient = KanidmClientBuilder::new()
        .address(addr)
        .build_async()
        .expect("Failed to build client");

    let cachelayer = CacheLayer::new(
        "", // The sqlite db path, this is in memory.
        300,
        rsclient,
        vec!["allowed_group".to_string()],
    )
    .expect("Failed to build cache layer.");

    test_fn(cachelayer, client);

    // We DO NOT need teardown, as sqlite is in mem
    // let the tables hit the floor
    sys.stop();
}

fn test_fixture(rsclient: &KanidmClient) -> () {
    let res = rsclient.auth_simple_password("admin", ADMIN_TEST_PASSWORD);
    assert!(res.is_ok());
    // Not recommended in production!
    rsclient
        .idm_group_add_members("idm_admins", vec!["admin"])
        .unwrap();

    // Create a new account
    rsclient
        .idm_account_create("testaccount1", "Posix Demo Account")
        .unwrap();

    // Extend the account with posix attrs.
    rsclient
        .idm_account_unix_extend("testaccount1", Some(20000), None)
        .unwrap();
    // Assign an ssh public key.
    rsclient
        .idm_account_post_ssh_pubkey("testaccount1", "tk",
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAeGW1P6Pc2rPq0XqbRaDKBcXZUPRklo0L1EyR30CwoP william@amethyst")
        .unwrap();
    // Set a posix password
    rsclient
        .idm_account_unix_cred_put("testaccount1", TESTACCOUNT1_PASSWORD_A)
        .unwrap();

    // Setup a group
    rsclient.idm_group_create("testgroup1").unwrap();
    rsclient
        .idm_group_add_members("testgroup1", vec!["testaccount1"])
        .unwrap();
    rsclient
        .idm_group_unix_extend("testgroup1", Some(20001))
        .unwrap();

    // Setup the allowed group
    rsclient.idm_group_create("allowed_group").unwrap();
    rsclient
        .idm_group_unix_extend("allowed_group", Some(20002))
        .unwrap();
}

#[test]
fn test_cache_sshkey() {
    run_test(test_fixture, |cachelayer, _adminclient| {
        let mut rt = Runtime::new().expect("Failed to start tokio");
        let fut = async move {
            // Force offline. Show we have no keys.
            cachelayer.mark_offline().await;

            let sk = cachelayer
                .get_sshkeys("testaccount1")
                .await
                .expect("Failed to get from cache.");
            assert!(sk.len() == 0);

            // Bring ourselves online.
            cachelayer.attempt_online().await;
            assert!(cachelayer.test_connection().await);

            let sk = cachelayer
                .get_sshkeys("testaccount1")
                .await
                .expect("Failed to get from cache.");
            assert!(sk.len() == 1);

            // Go offline, and get from cache.
            cachelayer.mark_offline().await;
            let sk = cachelayer
                .get_sshkeys("testaccount1")
                .await
                .expect("Failed to get from cache.");
            assert!(sk.len() == 1);
        };
        rt.block_on(fut);
    })
}

#[test]
fn test_cache_account() {
    run_test(test_fixture, |cachelayer, _adminclient| {
        let mut rt = Runtime::new().expect("Failed to start tokio");
        let fut = async move {
            // Force offline. Show we have no account
            cachelayer.mark_offline().await;

            let ut = cachelayer
                .get_nssaccount_name("testaccount1")
                .await
                .expect("Failed to get from cache");
            assert!(ut.is_none());

            // go online
            cachelayer.attempt_online().await;
            assert!(cachelayer.test_connection().await);

            // get the account
            let ut = cachelayer
                .get_nssaccount_name("testaccount1")
                .await
                .expect("Failed to get from cache");
            assert!(ut.is_some());

            // go offline
            cachelayer.mark_offline().await;

            // can still get account
            let ut = cachelayer
                .get_nssaccount_name("testaccount1")
                .await
                .expect("Failed to get from cache");
            assert!(ut.is_some());

            // Finally, check we have "all accounts" in the list.
            let us = cachelayer
                .get_nssaccounts()
                .expect("failed to list all accounts");
            assert!(us.len() == 1);
        };
        rt.block_on(fut);
    })
}

#[test]
fn test_cache_group() {
    run_test(test_fixture, |cachelayer, _adminclient| {
        let mut rt = Runtime::new().expect("Failed to start tokio");
        let fut = async move {
            // Force offline. Show we have no groups.
            cachelayer.mark_offline().await;
            let gt = cachelayer
                .get_nssgroup_name("testgroup1")
                .await
                .expect("Failed to get from cache");
            assert!(gt.is_none());

            // go online. Get the group
            cachelayer.attempt_online().await;
            assert!(cachelayer.test_connection().await);
            let gt = cachelayer
                .get_nssgroup_name("testgroup1")
                .await
                .expect("Failed to get from cache");
            assert!(gt.is_some());

            // go offline. still works
            cachelayer.mark_offline().await;
            let gt = cachelayer
                .get_nssgroup_name("testgroup1")
                .await
                .expect("Failed to get from cache");
            assert!(gt.is_some());
            // And check we have no members in the group. Members are an artifact of
            // user lookups!
            assert!(gt.unwrap().members.len() == 0);

            // clear cache, go online
            assert!(cachelayer.invalidate().is_ok());
            cachelayer.attempt_online().await;
            assert!(cachelayer.test_connection().await);

            // get an account with the group
            // DO NOT get the group yet.
            let ut = cachelayer
                .get_nssaccount_name("testaccount1")
                .await
                .expect("Failed to get from cache");
            assert!(ut.is_some());

            // go offline.
            cachelayer.mark_offline().await;

            // show we have the group despite no direct calls
            let gt = cachelayer
                .get_nssgroup_name("testgroup1")
                .await
                .expect("Failed to get from cache");
            assert!(gt.is_some());
            // And check we have members in the group, since we came from a userlook up
            assert!(gt.unwrap().members.len() == 1);

            // Finally, check we have "all groups" in the list.
            let gs = cachelayer
                .get_nssgroups()
                .expect("failed to list all groups");
            assert!(gs.len() == 2);
        };
        rt.block_on(fut);
    })
}

#[test]
fn test_cache_group_delete() {
    run_test(test_fixture, |cachelayer, adminclient| {
        let mut rt = Runtime::new().expect("Failed to start tokio");
        let fut = async move {
            // get the group
            cachelayer.attempt_online().await;
            assert!(cachelayer.test_connection().await);
            let gt = cachelayer
                .get_nssgroup_name("testgroup1")
                .await
                .expect("Failed to get from cache");
            assert!(gt.is_some());

            // delete it.
            adminclient
                .auth_simple_password("admin", ADMIN_TEST_PASSWORD)
                .await
                .expect("failed to auth as admin");
            adminclient
                .idm_group_delete("testgroup1")
                .await
                .expect("failed to delete");

            // invalidate cache
            assert!(cachelayer.invalidate().is_ok());

            // "get it"
            // should be empty.
            let gt = cachelayer
                .get_nssgroup_name("testgroup1")
                .await
                .expect("Failed to get from cache");
            assert!(gt.is_none());
        };
        rt.block_on(fut);
    })
}

#[test]
fn test_cache_account_delete() {
    run_test(test_fixture, |cachelayer, adminclient| {
        let mut rt = Runtime::new().expect("Failed to start tokio");
        let fut = async move {
            // get the account
            cachelayer.attempt_online().await;
            assert!(cachelayer.test_connection().await);
            let ut = cachelayer
                .get_nssaccount_name("testaccount1")
                .await
                .expect("Failed to get from cache");
            assert!(ut.is_some());

            // delete it.
            adminclient
                .auth_simple_password("admin", ADMIN_TEST_PASSWORD)
                .await
                .expect("failed to auth as admin");
            adminclient
                .idm_account_delete("testaccount1")
                .await
                .expect("failed to delete");

            // invalidate cache
            assert!(cachelayer.invalidate().is_ok());

            // "get it"
            let ut = cachelayer
                .get_nssaccount_name("testaccount1")
                .await
                .expect("Failed to get from cache");
            // should be empty.
            assert!(ut.is_none());

            // The group should be removed too.
            let gt = cachelayer
                .get_nssgroup_name("testaccount1")
                .await
                .expect("Failed to get from cache");
            assert!(gt.is_none());
        };
        rt.block_on(fut);
    })
}

#[test]
fn test_cache_account_password() {
    run_test(test_fixture, |cachelayer, adminclient| {
        let mut rt = Runtime::new().expect("Failed to start tokio");
        let fut = async move {
            cachelayer.attempt_online().await;
            // Test authentication failure.
            let a1 = cachelayer
                .pam_account_authenticate("testaccount1", TESTACCOUNT1_PASSWORD_INC)
                .await
                .expect("failed to authenticate");
            assert!(a1 == Some(false));

            // Test authentication success.
            let a2 = cachelayer
                .pam_account_authenticate("testaccount1", TESTACCOUNT1_PASSWORD_A)
                .await
                .expect("failed to authenticate");
            assert!(a2 == Some(true));

            // change pw
            adminclient
                .auth_simple_password("admin", ADMIN_TEST_PASSWORD)
                .await
                .expect("failed to auth as admin");
            adminclient
                .idm_account_unix_cred_put("testaccount1", TESTACCOUNT1_PASSWORD_B)
                .await
                .expect("Failed to change password");

            // test auth (old pw) fail
            let a3 = cachelayer
                .pam_account_authenticate("testaccount1", TESTACCOUNT1_PASSWORD_A)
                .await
                .expect("failed to authenticate");
            assert!(a3 == Some(false));

            // test auth (new pw) success
            let a4 = cachelayer
                .pam_account_authenticate("testaccount1", TESTACCOUNT1_PASSWORD_B)
                .await
                .expect("failed to authenticate");
            assert!(a4 == Some(true));

            // Go offline.
            cachelayer.mark_offline().await;

            // Test auth success
            let a5 = cachelayer
                .pam_account_authenticate("testaccount1", TESTACCOUNT1_PASSWORD_B)
                .await
                .expect("failed to authenticate");
            assert!(a5 == Some(true));

            // Test auth failure.
            let a6 = cachelayer
                .pam_account_authenticate("testaccount1", TESTACCOUNT1_PASSWORD_INC)
                .await
                .expect("failed to authenticate");
            assert!(a6 == Some(false));

            // clear cache
            cachelayer.clear_cache().expect("failed to clear cache");

            // test auth good (fail)
            let a7 = cachelayer
                .pam_account_authenticate("testaccount1", TESTACCOUNT1_PASSWORD_B)
                .await
                .expect("failed to authenticate");
            assert!(a7 == None);

            // go online
            cachelayer.attempt_online().await;
            assert!(cachelayer.test_connection().await);

            // test auth success
            let a8 = cachelayer
                .pam_account_authenticate("testaccount1", TESTACCOUNT1_PASSWORD_B)
                .await
                .expect("failed to authenticate");
            assert!(a8 == Some(true));
        };
        rt.block_on(fut);
    })
}

#[test]
fn test_cache_account_pam_allowed() {
    run_test(test_fixture, |cachelayer, adminclient| {
        let mut rt = Runtime::new().expect("Failed to start tokio");
        let fut = async move {
            cachelayer.attempt_online().await;

            // Should fail
            let a1 = cachelayer
                .pam_account_allowed("testaccount1")
                .await
                .expect("failed to authenticate");
            assert!(a1 == Some(false));

            adminclient
                .auth_simple_password("admin", ADMIN_TEST_PASSWORD)
                .await
                .expect("failed to auth as admin");
            adminclient
                .idm_group_add_members("allowed_group", vec!["testaccount1"])
                .await
                .unwrap();

            // Invalidate cache to force a refresh
            assert!(cachelayer.invalidate().is_ok());

            // Should pass
            let a2 = cachelayer
                .pam_account_allowed("testaccount1")
                .await
                .expect("failed to authenticate");
            assert!(a2 == Some(true));
        };
        rt.block_on(fut);
    })
}

#[test]
fn test_cache_account_pam_nonexist() {
    run_test(test_fixture, |cachelayer, _adminclient| {
        let mut rt = Runtime::new().expect("Failed to start tokio");
        let fut = async move {
            cachelayer.attempt_online().await;

            let a1 = cachelayer
                .pam_account_allowed("NO_SUCH_ACCOUNT")
                .await
                .expect("failed to authenticate");
            assert!(a1 == None);

            let a2 = cachelayer
                .pam_account_authenticate("NO_SUCH_ACCOUNT", TESTACCOUNT1_PASSWORD_B)
                .await
                .expect("failed to authenticate");
            assert!(a2 == None);

            cachelayer.mark_offline().await;

            let a1 = cachelayer
                .pam_account_allowed("NO_SUCH_ACCOUNT")
                .await
                .expect("failed to authenticate");
            assert!(a1 == None);

            let a2 = cachelayer
                .pam_account_authenticate("NO_SUCH_ACCOUNT", TESTACCOUNT1_PASSWORD_B)
                .await
                .expect("failed to authenticate");
            assert!(a2 == None);
        };
        rt.block_on(fut);
    })
}

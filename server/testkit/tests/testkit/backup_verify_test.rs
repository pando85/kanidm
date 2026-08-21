use kubidm_client::{KubidmClient, KubidmClientBuilder};
use kubidm_proto::backup::BackupCompression;
use kubidm_proto::internal::Filter;
use kubidmd_core::config::{Configuration, IntegrationTestConfig};
use kubidmd_core::{create_server_core, verify_backup_server_core, CoreHandle};
use kubidmd_lib::prelude::{Attribute, NAME_SYSTEM_ADMINS};
use kubidmd_testkit::{
    login_put_admin_idm_admins, ADMIN_TEST_PASSWORD, ADMIN_TEST_USER, NOT_ADMIN_TEST_PASSWORD,
    NOT_ADMIN_TEST_USERNAME, PORT_ALLOC, TEST_INTEGRATION_RS_DISPLAY, TEST_INTEGRATION_RS_ID,
    TEST_INTEGRATION_RS_URL,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::atomic::Ordering;
use tokio::task;
use url::Url;

fn is_free_port(port: u16) -> bool {
    std::net::TcpStream::connect(("0.0.0.0", port)).is_err()
}

fn port_loop() -> u16 {
    let mut counter = 0;
    loop {
        let possible_port = PORT_ALLOC.fetch_add(1, Ordering::SeqCst);
        if is_free_port(possible_port) {
            break possible_port;
        }
        counter += 1;
        #[allow(clippy::panic)]
        if counter >= 5 {
            tracing::error!("Unable to allocate port!");
            panic!();
        }
    }
}

async fn setup_test_server(db_path: Option<std::path::PathBuf>) -> (KubidmClient, CoreHandle, Url) {
    sketching::test_init();

    let port = port_loop();

    let int_config = Box::new(IntegrationTestConfig {
        admin_user: ADMIN_TEST_USER.to_string(),
        admin_password: ADMIN_TEST_PASSWORD.to_string(),
        idm_admin_user: "idm_admin".to_string(),
        idm_admin_password: "integration idm admin password".to_string(),
    });

    #[allow(clippy::expect_used)]
    let addr =
        Url::from_str(&format!("http://localhost:{port}")).expect("Failed to parse origin URL");

    let http_sock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);

    let mut config = Configuration::new_for_test();
    config.address = vec![http_sock_addr.to_string()];
    config.integration_test_config = Some(int_config);
    config.domain = "localhost".to_string();
    config.origin.clone_from(&addr);
    config.db_path = db_path;

    let core_handle = match create_server_core(config, false).await {
        Ok(val) => val,
        #[allow(clippy::panic)]
        Err(_) => panic!("failed to start server core"),
    };
    task::yield_now().await;

    #[allow(clippy::panic)]
    let rsclient = match KubidmClientBuilder::new()
        .address(addr.to_string())
        .enable_native_ca_roots(false)
        .no_proxy()
        .build()
    {
        Ok(val) => val,
        Err(_) => panic!("failed to build client"),
    };

    (rsclient, core_handle, addr)
}

async fn populate_test_data(rsclient: &KubidmClient) {
    login_put_admin_idm_admins(rsclient).await;

    rsclient
        .idm_person_account_create(NOT_ADMIN_TEST_USERNAME, NOT_ADMIN_TEST_USERNAME)
        .await
        .expect("Failed to create test user");

    rsclient
        .idm_person_account_primary_credential_set_password(
            NOT_ADMIN_TEST_USERNAME,
            NOT_ADMIN_TEST_PASSWORD,
        )
        .await
        .expect("Failed to set test user password");

    rsclient
        .idm_group_create("backup_test_group", None)
        .await
        .expect("Failed to create test group");

    rsclient
        .idm_group_add_members("backup_test_group", &[NOT_ADMIN_TEST_USERNAME])
        .await
        .expect("Failed to add member to group");

    rsclient
        .idm_person_account_create("backup_user_alice", "Alice")
        .await
        .expect("Failed to create alice");

    rsclient
        .idm_person_account_create("backup_user_bob", "Bob")
        .await
        .expect("Failed to create bob");

    rsclient
        .idm_group_create("backup_engineers", None)
        .await
        .expect("Failed to create engineers group");

    rsclient
        .idm_group_add_members("backup_engineers", &["backup_user_alice", "backup_user_bob"])
        .await
        .expect("Failed to add members to engineers group");

    rsclient
        .idm_oauth2_rs_basic_create(
            TEST_INTEGRATION_RS_ID,
            TEST_INTEGRATION_RS_DISPLAY,
            TEST_INTEGRATION_RS_URL,
        )
        .await
        .expect("Failed to create oauth2 config");

    rsclient
        .idm_oauth2_client_add_origin(
            TEST_INTEGRATION_RS_ID,
            &url::Url::parse("https://demo.example.com/oauth2/flow")
                .expect("Invalid redirect URL"),
        )
        .await
        .expect("Failed to add oauth2 origin");

    rsclient
        .idm_oauth2_rs_update(TEST_INTEGRATION_RS_ID, None, None, None, true)
        .await
        .expect("Failed to update oauth2 config");

    rsclient
        .idm_group_create("recycle_test_group", None)
        .await
        .expect("Failed to create recycle test group");

    rsclient
        .idm_group_delete("recycle_test_group")
        .await
        .expect("Failed to delete recycle test group");
}

async fn create_backup_from_server(
    db_path: &std::path::Path,
    backup_path: &std::path::Path,
) {
    let schema = kubidmd_lib::schema::Schema::new().expect("Failed to create schema");
    let schema_txn = schema.write();
    let idxmeta = schema_txn.reload_idxmeta();

    let cfg = kubidmd_lib::be::BackendConfig::new(
        Some(db_path),
        1,
        kubidm_proto::internal::FsType::Generic,
        None,
    );

    let be = kubidmd_lib::be::Backend::new(cfg, idxmeta, false)
        .expect("Failed to create backend");

    let mut be_ro_txn = be.read().expect("Failed to get read txn");

    let output = std::fs::File::create(backup_path).expect("Failed to create backup file");

    be_ro_txn
        .backup(output, BackupCompression::NoCompression)
        .expect("Failed to create backup");
}

#[test]
fn test_backup_verify_structural_valid() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backup_path = temp_dir.path().join("backup.json");

        let (rsclient, core_handle, _addr) =
            setup_test_server(Some(db_path.clone())).await;

        populate_test_data(&rsclient).await;

        create_backup_from_server(&db_path, &backup_path).await;

        assert!(backup_path.exists(), "Backup file should exist");
        assert!(
            std::fs::metadata(&backup_path)
                .expect("Failed to read backup metadata")
                .len()
                > 0,
            "Backup file should not be empty"
        );

        let mut verify_config = Configuration::new_for_test();
        verify_config.db_path = Some(temp_dir.path().join("verify.db"));

        let result = verify_backup_server_core(&verify_config, &backup_path, false).await;
        assert!(result, "Structural verification should pass for valid backup");

        core_handle.shutdown().await;
    });
}

#[test]
fn test_backup_verify_full_restore() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let backup_path = temp_dir.path().join("backup.json");

        let (rsclient, core_handle, _addr) =
            setup_test_server(Some(db_path.clone())).await;

        populate_test_data(&rsclient).await;

        create_backup_from_server(&db_path, &backup_path).await;

        core_handle.shutdown().await;

        let restore_dir = tempfile::tempdir().expect("Failed to create restore temp dir");
        let restore_db_path = restore_dir.path().join("restored.db");

        let mut verify_config = Configuration::new_for_test();
        verify_config.db_path = Some(restore_db_path.clone());

        let result = verify_backup_server_core(&verify_config, &backup_path, true).await;
        assert!(result, "Full restore verification should pass for valid backup");

        assert!(restore_db_path.exists(), "Restored database should exist");
    });
}

#[test]
fn test_backup_verify_detects_corrupted_backup() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let corrupted_path = temp_dir.path().join("corrupted.json");

        std::fs::write(&corrupted_path, b"this is not valid json backup data")
            .expect("Failed to write corrupted backup");

        let mut verify_config = Configuration::new_for_test();
        verify_config.db_path = Some(temp_dir.path().join("verify.db"));

        let result = verify_backup_server_core(&verify_config, &corrupted_path, false).await;
        assert!(
            !result,
            "Structural verification should fail for corrupted backup"
        );

        let result_full = verify_backup_server_core(&verify_config, &corrupted_path, true).await;
        assert!(
            !result_full,
            "Full verification should fail for corrupted backup"
        );
    });
}

#[test]
fn test_backup_verify_restored_server_is_functional() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("original.db");
        let backup_path = temp_dir.path().join("backup.json");

        let (rsclient, core_handle, _addr) =
            setup_test_server(Some(db_path.clone())).await;

        populate_test_data(&rsclient).await;

        create_backup_from_server(&db_path, &backup_path).await;

        core_handle.shutdown().await;

        let restore_dir = tempfile::tempdir().expect("Failed to create restore temp dir");
        let restore_db_path = restore_dir.path().join("restored.db");

        let mut verify_config = Configuration::new_for_test();
        verify_config.db_path = Some(restore_db_path.clone());

        let result = verify_backup_server_core(&verify_config, &backup_path, true).await;
        assert!(result, "Full restore verification should pass");

        let (restored_client, restored_handle, _restored_addr) =
            setup_test_server(Some(restore_db_path.clone())).await;

        login_put_admin_idm_admins(&restored_client).await;

        let user = restored_client
            .idm_person_account_get(NOT_ADMIN_TEST_USERNAME)
            .await
            .expect("Failed to get restored user");
        assert!(user.is_some(), "Restored user should exist");

        let group = restored_client
            .idm_group_get("backup_test_group")
            .await
            .expect("Failed to get restored group");
        assert!(group.is_some(), "Restored group should exist");

        let engineers = restored_client
            .idm_group_get("backup_engineers")
            .await
            .expect("Failed to get restored engineers group");
        assert!(engineers.is_some(), "Restored engineers group should exist");

        let oauth2 = restored_client
            .idm_oauth2_rs_get(TEST_INTEGRATION_RS_ID)
            .await
            .expect("Failed to get restored oauth2 config");
        assert!(oauth2.is_some(), "Restored OAuth2 config should exist");

        restored_handle.shutdown().await;
    });
}

#[test]
fn test_backup_verify_nonexistent_file() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let nonexistent = temp_dir.path().join("does_not_exist.json");

        let mut verify_config = Configuration::new_for_test();
        verify_config.db_path = Some(temp_dir.path().join("verify.db"));

        let result = verify_backup_server_core(&verify_config, &nonexistent, false).await;
        assert!(
            !result,
            "Verification should fail for nonexistent backup file"
        );
    });
}

#[test]
fn test_backup_verify_empty_backup_detection() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let empty_path = temp_dir.path().join("empty.json");

        std::fs::write(&empty_path, b"[]").expect("Failed to write empty backup");

        let mut verify_config = Configuration::new_for_test();
        verify_config.db_path = Some(temp_dir.path().join("verify.db"));

        let result = verify_backup_server_core(&verify_config, &empty_path, true).await;
        assert!(
            !result,
            "Full verification should fail for empty/invalid backup"
        );
    });
}

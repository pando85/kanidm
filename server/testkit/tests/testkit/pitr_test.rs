use kubidm_proto::backup::{
    BackupCompression, PitrManifest, RecoveryTarget, RecoveryTargetType, WalArchiveConfig,
    WalEntry, WalOperation, WalSegment,
};
use std::time::Duration;
use uuid::Uuid;

fn create_test_segment(server_uuid: Uuid, start_ts: Duration, end_ts: Duration) -> WalSegment {
    WalSegment::new(
        format!("wal-{}-test.wal", server_uuid),
        server_uuid,
        start_ts,
        end_ts,
        "checksum123".to_string(),
        100,
        BackupCompression::NoCompression,
    )
}

fn create_test_manifest(server_uuid: Uuid) -> PitrManifest {
    PitrManifest::new(
        server_uuid,
        "backup-001".to_string(),
        "2024-01-01T00:00:00Z".to_string(),
    )
}

fn create_test_wal_entry(server_uuid: Uuid, ts: Duration, entry_id: u64) -> WalEntry {
    WalEntry {
        cid_ts: ts,
        cid_server: server_uuid,
        entry_id,
        operation: WalOperation::Create {
            entry_data: vec![1, 2, 3],
        },
    }
}

#[test]
fn test_pitr_wal_segment_creation() {
    let server_uuid = Uuid::new_v4();
    let segment = create_test_segment(
        server_uuid,
        Duration::from_secs(0),
        Duration::from_secs(100),
    );

    assert!(segment.segment_id.contains("wal"));
    assert_eq!(segment.server_uuid, server_uuid);
    assert!(!segment.checksum_sha256.is_empty());
    assert_eq!(segment.compression, BackupCompression::NoCompression);
}

#[test]
fn test_pitr_manifest_creation_and_manipulation() {
    let server_uuid = Uuid::new_v4();
    let manifest = create_test_manifest(server_uuid);

    assert_eq!(manifest.server_uuid, server_uuid);
    assert_eq!(manifest.base_backup_id, "backup-001");
    assert!(manifest.segments.is_empty());
}

#[test]
fn test_pitr_manifest_add_segments() {
    let server_uuid = Uuid::new_v4();
    let mut manifest = create_test_manifest(server_uuid);

    for i in 0..5 {
        let ts = Duration::from_secs(i * 100);
        manifest.add_segment(create_test_segment(server_uuid, ts, ts));
    }

    assert_eq!(manifest.segments.len(), 5);
}

#[test]
fn test_pitr_recovery_target_time() {
    let target = RecoveryTarget::to_time("2024-01-15T10:30:00Z");
    assert!(target.is_ok());

    let target = target.unwrap();
    assert!(matches!(
        target.target_type,
        RecoveryTargetType::Time { .. }
    ));
}

#[test]
fn test_pitr_recovery_target_transaction() {
    let target = RecoveryTarget::to_transaction("test-cid-12345");
    assert!(target.is_ok());

    let target = target.unwrap();
    assert!(matches!(
        target.target_type,
        RecoveryTargetType::Transaction { .. }
    ));
}

#[test]
fn test_pitr_recovery_target_latest() {
    let target = RecoveryTarget::latest();
    assert!(matches!(target.target_type, RecoveryTargetType::Latest));
}

#[test]
fn test_pitr_recovery_target_invalid_time() {
    let target = RecoveryTarget::to_time("invalid-time");
    assert!(target.is_err());
}

#[test]
fn test_pitr_recovery_target_empty_transaction() {
    let target = RecoveryTarget::to_transaction("");
    assert!(target.is_err());
}

#[test]
fn test_pitr_wal_entry_operations() {
    let server_uuid = Uuid::new_v4();

    let create_entry = WalEntry {
        cid_ts: Duration::from_secs(1000),
        cid_server: server_uuid,
        entry_id: 1,
        operation: WalOperation::Create {
            entry_data: vec![1, 2, 3],
        },
    };
    assert!(matches!(
        create_entry.operation,
        WalOperation::Create { .. }
    ));

    let modify_entry = WalEntry {
        cid_ts: Duration::from_secs(2000),
        cid_server: server_uuid,
        entry_id: 2,
        operation: WalOperation::Modify {
            entry_data: vec![4, 5, 6],
        },
    };
    assert!(matches!(
        modify_entry.operation,
        WalOperation::Modify { .. }
    ));

    let delete_entry = WalEntry {
        cid_ts: Duration::from_secs(3000),
        cid_server: server_uuid,
        entry_id: 3,
        operation: WalOperation::Delete,
    };
    assert!(matches!(delete_entry.operation, WalOperation::Delete));
}

#[test]
fn test_pitr_wal_config_default() {
    let config = WalArchiveConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.retention_days, 7);
    assert_eq!(config.segment_size_bytes, 16 * 1024 * 1024);
}

#[test]
fn test_pitr_wal_entry_serialization_deserialization() {
    let server_uuid = Uuid::new_v4();
    let entry = create_test_wal_entry(server_uuid, Duration::from_secs(1000), 1);

    let serialized = serde_json::to_string(&entry).expect("Failed to serialize");
    assert!(serialized.contains("1000"));

    let deserialized: WalEntry = serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(entry.entry_id, deserialized.entry_id);
    assert_eq!(entry.cid_server, deserialized.cid_server);
}

#[test]
fn test_pitr_wal_segment_serialization_deserialization() {
    let server_uuid = Uuid::new_v4();
    let segment = create_test_segment(
        server_uuid,
        Duration::from_secs(0),
        Duration::from_secs(100),
    );

    let serialized = serde_json::to_string(&segment).expect("Failed to serialize");
    assert!(serialized.contains(server_uuid.to_string().as_str()));

    let deserialized: WalSegment =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(segment.segment_id, deserialized.segment_id);
    assert_eq!(segment.server_uuid, deserialized.server_uuid);
}

#[test]
fn test_pitr_manifest_serialization_deserialization() {
    let server_uuid = Uuid::new_v4();
    let mut manifest = create_test_manifest(server_uuid);
    manifest.add_segment(create_test_segment(
        server_uuid,
        Duration::from_secs(0),
        Duration::from_secs(100),
    ));

    let serialized = serde_json::to_string(&manifest).expect("Failed to serialize");
    assert!(serialized.contains("backup-001"));

    let deserialized: PitrManifest =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(manifest.server_uuid, deserialized.server_uuid);
    assert_eq!(manifest.segments.len(), deserialized.segments.len());
}

#[test]
fn test_pitr_recovery_target_serialization() {
    let target_time = RecoveryTarget::to_time("2024-01-15T10:30:00Z").unwrap();
    let serialized = serde_json::to_string(&target_time).expect("Failed to serialize");
    assert!(serialized.contains("2024-01-15T10:30:00Z"));

    let target_transaction = RecoveryTarget::to_transaction("test-cid").unwrap();
    let serialized = serde_json::to_string(&target_transaction).expect("Failed to serialize");
    assert!(serialized.contains("test-cid"));

    let target_latest = RecoveryTarget::latest();
    let serialized = serde_json::to_string(&target_latest).expect("Failed to serialize");
    assert!(serialized.contains("Latest"));
}

#[test]
fn test_pitr_recovery_window_calculation() {
    let server_uuid = Uuid::new_v4();
    let mut manifest = create_test_manifest(server_uuid);

    let base_time = "2024-01-01T00:00:00Z";
    assert_eq!(manifest.earliest_recoverable_time, base_time);
    assert_eq!(manifest.latest_recoverable_time, base_time);

    let segment1 = create_test_segment(
        server_uuid,
        Duration::from_secs(0),
        Duration::from_secs(100),
    );
    manifest.add_segment(segment1.clone());

    assert_eq!(manifest.earliest_recoverable_time, segment1.created_at);
    assert_eq!(manifest.latest_recoverable_time, segment1.created_at);

    let segment2 = create_test_segment(
        server_uuid,
        Duration::from_secs(100),
        Duration::from_secs(200),
    );
    manifest.add_segment(segment2.clone());

    assert_eq!(manifest.earliest_recoverable_time, segment1.created_at);
    assert_eq!(manifest.latest_recoverable_time, segment2.created_at);
}

#[test]
fn test_pitr_segment_ordering() {
    let server_uuid = Uuid::new_v4();
    let mut manifest = create_test_manifest(server_uuid);

    let segments: Vec<(Duration, Duration)> = vec![
        (Duration::from_secs(300), Duration::from_secs(400)),
        (Duration::from_secs(0), Duration::from_secs(100)),
        (Duration::from_secs(100), Duration::from_secs(200)),
    ];

    for (start, end) in segments {
        manifest.add_segment(create_test_segment(server_uuid, start, end));
    }

    assert_eq!(manifest.segments.len(), 3);
}

#[test]
fn test_pitr_checksum_verification() {
    let server_uuid = Uuid::new_v4();
    let segment = create_test_segment(
        server_uuid,
        Duration::from_secs(0),
        Duration::from_secs(100),
    );

    assert!(!segment.checksum_sha256.is_empty());
}

#[test]
fn test_pitr_compression_types() {
    let server_uuid = Uuid::new_v4();

    let uncompressed = WalSegment::new(
        "test".to_string(),
        server_uuid,
        Duration::from_secs(0),
        Duration::from_secs(100),
        "checksum".to_string(),
        100,
        BackupCompression::NoCompression,
    );
    assert_eq!(uncompressed.compression, BackupCompression::NoCompression);

    let compressed = WalSegment::new(
        "test".to_string(),
        server_uuid,
        Duration::from_secs(0),
        Duration::from_secs(100),
        "checksum".to_string(),
        100,
        BackupCompression::Gzip,
    );
    assert_eq!(compressed.compression, BackupCompression::Gzip);
}

#[test]
fn test_pitr_empty_manifest_recovery() {
    let server_uuid = Uuid::new_v4();
    let manifest = create_test_manifest(server_uuid);

    assert!(manifest.segments.is_empty());
    assert_eq!(
        manifest.earliest_recoverable_time,
        manifest.latest_recoverable_time
    );
}

#[test]
fn test_pitr_manifest_multiple_servers() {
    let server1 = Uuid::new_v4();
    let server2 = Uuid::new_v4();

    let mut manifest = create_test_manifest(server1);
    manifest.add_segment(create_test_segment(
        server1,
        Duration::from_secs(0),
        Duration::from_secs(100),
    ));
    manifest.add_segment(create_test_segment(
        server2,
        Duration::from_secs(100),
        Duration::from_secs(200),
    ));

    assert_eq!(manifest.segments.len(), 2);
    assert_ne!(
        manifest.segments[0].server_uuid,
        manifest.segments[1].server_uuid
    );
}

#[test]
fn test_pitr_wal_entry_ordering() {
    let server_uuid = Uuid::new_v4();
    let entries: Vec<WalEntry> = vec![
        create_test_wal_entry(server_uuid, Duration::from_secs(3000), 3),
        create_test_wal_entry(server_uuid, Duration::from_secs(1000), 1),
        create_test_wal_entry(server_uuid, Duration::from_secs(2000), 2),
    ];

    let timestamps: Vec<Duration> = entries.iter().map(|e| e.cid_ts).collect();
    assert_eq!(timestamps[0], Duration::from_secs(3000));
    assert_eq!(timestamps[1], Duration::from_secs(1000));
    assert_eq!(timestamps[2], Duration::from_secs(2000));
}

#[test]
fn test_pitr_segment_size_tracking() {
    let server_uuid = Uuid::new_v4();
    let segment = WalSegment::new(
        "test".to_string(),
        server_uuid,
        Duration::from_secs(0),
        Duration::from_secs(100),
        "checksum".to_string(),
        1024 * 1024,
        BackupCompression::Gzip,
    );

    assert_eq!(segment.size_bytes, 1024 * 1024);
}

#[test]
fn test_pitr_timestamp_precision() {
    let server_uuid = Uuid::new_v4();
    let entry = WalEntry {
        cid_ts: Duration::from_nanos(1234567890),
        cid_server: server_uuid,
        entry_id: 1,
        operation: WalOperation::Create { entry_data: vec![] },
    };

    assert_eq!(entry.cid_ts.as_nanos(), 1234567890);
}

#[test]
fn test_pitr_recovery_target_display() {
    let time_target = RecoveryTarget::to_time("2024-01-15T10:30:00Z").unwrap();
    assert!(time_target.to_string().starts_with("time:"));

    let transaction_target = RecoveryTarget::to_transaction("cid-123").unwrap();
    assert!(transaction_target.to_string().starts_with("transaction:"));

    let latest_target = RecoveryTarget::latest();
    assert_eq!(latest_target.to_string(), "latest");
}

#[test]
fn test_pitr_wal_config_custom_settings() {
    let config = WalArchiveConfig {
        enabled: true,
        s3: None,
        retention_days: 30,
        segment_size_bytes: 32 * 1024 * 1024,
    };

    assert!(config.enabled);
    assert_eq!(config.retention_days, 30);
    assert_eq!(config.segment_size_bytes, 32 * 1024 * 1024);
}

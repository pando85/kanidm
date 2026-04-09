//! Write-Ahead Log (WAL) archiving for Point-in-Time Recovery (PITR)
//!
//! This module provides WAL archiving capabilities that allow recovery to
//! any point in time within the configured retention window.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use kubidm_proto::backup::{BackupCompression, PitrManifest, WalArchiveConfig, WalSegment};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;
use uuid::Uuid;

use crate::repl::cid::Cid;

#[allow(dead_code)]
pub const WAL_SEGMENT_PREFIX: &str = "wal";
#[allow(dead_code)]
pub const WAL_MANIFEST_FILE: &str = "pitr-manifest.json";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WalSegmentData {
    pub segment_id: String,
    pub server_uuid: Uuid,
    pub entries: Vec<WalEntryRecord>,
    pub start_ts: Duration,
    pub end_ts: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntryRecord {
    pub cid_ts: u64,
    pub cid_server: Uuid,
    pub entry_id: u64,
    pub operation: WalOperationRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalOperationRecord {
    Create { entry_data: Vec<u8> },
    Modify { entry_data: Vec<u8> },
    Delete,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum WalError {
    IoError(std::io::Error),
    SerializationError(String),
    InvalidSegment(String),
    RetentionError(String),
    ConfigError(String),
}

impl std::fmt::Display for WalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalError::IoError(e) => write!(f, "WAL IO error: {}", e),
            WalError::SerializationError(msg) => write!(f, "WAL serialization error: {}", msg),
            WalError::InvalidSegment(msg) => write!(f, "Invalid WAL segment: {}", msg),
            WalError::RetentionError(msg) => write!(f, "WAL retention error: {}", msg),
            WalError::ConfigError(msg) => write!(f, "WAL config error: {}", msg),
        }
    }
}

impl std::error::Error for WalError {}

impl From<std::io::Error> for WalError {
    fn from(e: std::io::Error) -> Self {
        WalError::IoError(e)
    }
}

impl From<serde_json::Error> for WalError {
    fn from(e: serde_json::Error) -> Self {
        WalError::SerializationError(e.to_string())
    }
}

#[allow(dead_code)]
pub struct WalArchiver {
    config: WalArchiveConfig,
    server_uuid: Uuid,
    current_segment: Option<WalSegmentBuilder>,
    segments_path: PathBuf,
}

#[allow(dead_code)]
struct WalSegmentBuilder {
    entries: Vec<WalEntryRecord>,
    start_ts: Duration,
    current_size: u64,
    segment_id: String,
}

#[allow(dead_code)]
impl WalArchiver {
    pub fn new(config: WalArchiveConfig, server_uuid: Uuid, base_path: &Path) -> Self {
        let segments_path = base_path.join("wal");
        Self {
            config,
            server_uuid,
            current_segment: None,
            segments_path,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn record_create(
        &mut self,
        cid: &Cid,
        entry_id: u64,
        entry_data: Vec<u8>,
    ) -> Result<(), WalError> {
        if !self.is_enabled() {
            return Ok(());
        }

        let record = WalEntryRecord {
            cid_ts: cid.ts.as_nanos() as u64,
            cid_server: cid.s_uuid,
            entry_id,
            operation: WalOperationRecord::Create { entry_data },
        };

        self.add_record(record)
    }

    pub fn record_modify(
        &mut self,
        cid: &Cid,
        entry_id: u64,
        entry_data: Vec<u8>,
    ) -> Result<(), WalError> {
        if !self.is_enabled() {
            return Ok(());
        }

        let record = WalEntryRecord {
            cid_ts: cid.ts.as_nanos() as u64,
            cid_server: cid.s_uuid,
            entry_id,
            operation: WalOperationRecord::Modify { entry_data },
        };

        self.add_record(record)
    }

    pub fn record_delete(&mut self, cid: &Cid, entry_id: u64) -> Result<(), WalError> {
        if !self.is_enabled() {
            return Ok(());
        }

        let record = WalEntryRecord {
            cid_ts: cid.ts.as_nanos() as u64,
            cid_server: cid.s_uuid,
            entry_id,
            operation: WalOperationRecord::Delete,
        };

        self.add_record(record)
    }

    fn add_record(&mut self, record: WalEntryRecord) -> Result<(), WalError> {
        let record_size = self.estimate_record_size(&record);

        if self.current_segment.is_none() {
            self.start_new_segment(Duration::from_nanos(record.cid_ts))?;
        }

        if let Some(segment) = self.current_segment.as_mut() {
            segment.entries.push(record);
            segment.current_size += record_size;

            if segment.current_size >= self.config.segment_size_bytes {
                self.flush_current_segment()?;
            }
        }

        Ok(())
    }

    fn estimate_record_size(&self, record: &WalEntryRecord) -> u64 {
        let base_size = std::mem::size_of::<WalEntryRecord>() as u64;
        match &record.operation {
            WalOperationRecord::Create { entry_data }
            | WalOperationRecord::Modify { entry_data } => base_size + entry_data.len() as u64,
            WalOperationRecord::Delete => base_size,
        }
    }

    fn start_new_segment(&mut self, start_ts: Duration) -> Result<(), WalError> {
        let segment_id = format!(
            "{}-{}-{}.wal",
            WAL_SEGMENT_PREFIX,
            self.server_uuid,
            chrono::Utc::now().format("%Y%m%d%H%M%S%3f")
        );

        self.current_segment = Some(WalSegmentBuilder {
            entries: Vec::new(),
            start_ts,
            current_size: 0,
            segment_id,
        });

        Ok(())
    }

    pub fn flush_current_segment(&mut self) -> Result<Option<WalSegment>, WalError> {
        let Some(segment_builder) = self.current_segment.take() else {
            return Ok(None);
        };

        if segment_builder.entries.is_empty() {
            return Ok(None);
        }

        let end_ts = segment_builder
            .entries
            .last()
            .map(|e| Duration::from_nanos(e.cid_ts))
            .unwrap_or(segment_builder.start_ts);

        let segment_data = WalSegmentData {
            segment_id: segment_builder.segment_id.clone(),
            server_uuid: self.server_uuid,
            entries: segment_builder.entries,
            start_ts: segment_builder.start_ts,
            end_ts,
        };

        let (segment, _) = self.serialize_and_save_segment(segment_data)?;

        Ok(Some(segment))
    }

    fn serialize_and_save_segment(
        &self,
        segment_data: WalSegmentData,
    ) -> Result<(WalSegment, Vec<u8>), WalError> {
        let serialized = serde_json::to_vec(&segment_data.entries)?;

        let (compressed, compression) = match self.compress_segment(&serialized)? {
            (data, BackupCompression::Gzip) => (data, BackupCompression::Gzip),
            (data, BackupCompression::NoCompression) => (data, BackupCompression::NoCompression),
        };

        let checksum = hex::encode(Sha256::digest(&compressed));
        let size = compressed.len() as u64;

        let segment = WalSegment::new(
            segment_data.segment_id,
            segment_data.server_uuid,
            segment_data.start_ts,
            segment_data.end_ts,
            checksum,
            size,
            compression,
        );

        Ok((segment, compressed))
    }

    fn compress_segment(&self, data: &[u8]) -> Result<(Vec<u8>, BackupCompression), WalError> {
        match self.config.s3.as_ref() {
            Some(_) => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                let compressed = encoder.finish()?;
                Ok((compressed, BackupCompression::Gzip))
            }
            None => Ok((data.to_vec(), BackupCompression::NoCompression)),
        }
    }

    pub fn apply_retention_policy(
        &mut self,
        manifest: &mut PitrManifest,
    ) -> Result<Vec<String>, WalError> {
        let retention_duration =
            Duration::from_secs(self.config.retention_days as u64 * 24 * 60 * 60);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();

        let cutoff_time = now - retention_duration;
        let cutoff_ts = Duration::from_secs(cutoff_time.as_secs());

        let mut deleted_segments = Vec::new();
        let mut retained_segments = Vec::new();

        for segment in manifest.segments.drain(..) {
            if segment.end_ts < cutoff_ts {
                deleted_segments.push(segment.segment_id.clone());
                info!("Deleting expired WAL segment: {}", segment.segment_id);
            } else {
                retained_segments.push(segment);
            }
        }

        manifest.segments = retained_segments;

        if !manifest.segments.is_empty() {
            manifest.earliest_recoverable_time = manifest
                .segments
                .first()
                .map(|s| s.created_at.clone())
                .unwrap_or_else(|| manifest.base_backup_timestamp.clone());
            manifest.latest_recoverable_time = manifest
                .segments
                .last()
                .map(|s| s.created_at.clone())
                .unwrap_or_else(|| manifest.base_backup_timestamp.clone());
        }

        Ok(deleted_segments)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RecoveryState {
    pub target: kubidm_proto::backup::RecoveryTarget,
    pub available_segments: Vec<WalSegment>,
    pub base_backup_id: String,
    pub base_backup_timestamp: String,
}

#[allow(dead_code)]
impl RecoveryState {
    pub fn validate_target(&self) -> Result<(), WalError> {
        match &self.target.target_type {
            kubidm_proto::backup::RecoveryTargetType::Time { timestamp } => {
                let target_time = chrono::DateTime::parse_from_rfc3339(timestamp)
                    .map_err(|e| WalError::InvalidSegment(format!("Invalid timestamp: {}", e)))?;

                let earliest = chrono::DateTime::parse_from_rfc3339(&self.base_backup_timestamp)
                    .map_err(|e| {
                        WalError::InvalidSegment(format!("Invalid base backup time: {}", e))
                    })?;

                let latest = chrono::DateTime::parse_from_rfc3339(
                    self.available_segments
                        .last()
                        .map(|s| s.created_at.as_str())
                        .unwrap_or(&self.base_backup_timestamp),
                )
                .map_err(|e| WalError::InvalidSegment(format!("Invalid segment time: {}", e)))?;

                if target_time < earliest {
                    return Err(WalError::InvalidSegment(format!(
                        "Target time {} is before earliest recoverable time {}",
                        timestamp, self.base_backup_timestamp
                    )));
                }

                if target_time > latest {
                    return Err(WalError::InvalidSegment(format!(
                        "Target time {} is after latest recoverable time {}",
                        timestamp,
                        latest.to_rfc3339()
                    )));
                }
            }
            kubidm_proto::backup::RecoveryTargetType::Transaction { cid } => {
                let cid_found = self.available_segments.iter().any(|s| {
                    s.segment_id.contains(cid)
                        || format!("{}-{}", s.server_uuid, s.start_ts.as_nanos()).contains(cid)
                });

                if !cid_found {
                    return Err(WalError::InvalidSegment(format!(
                        "Transaction CID {} not found in available segments",
                        cid
                    )));
                }
            }
            kubidm_proto::backup::RecoveryTargetType::Latest => {}
        }

        Ok(())
    }

    pub fn get_segments_for_recovery(&self) -> Vec<WalSegment> {
        let mut segments = self.available_segments.clone();
        segments.sort_by_key(|s| s.start_ts);
        segments
    }
}

#[allow(dead_code)]
pub struct WalReplayer {
    server_uuid: Uuid,
}

#[allow(dead_code)]
impl WalReplayer {
    pub fn new(server_uuid: Uuid) -> Self {
        Self { server_uuid }
    }

    pub fn load_segment(
        &self,
        data: &[u8],
        compression: BackupCompression,
    ) -> Result<Vec<WalEntryRecord>, WalError> {
        let decompressed = match compression {
            BackupCompression::Gzip => {
                let mut decoder = flate2::read::GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                decompressed
            }
            BackupCompression::NoCompression => data.to_vec(),
        };

        let entries: Vec<WalEntryRecord> = serde_json::from_slice(&decompressed)?;
        Ok(entries)
    }

    pub fn replay_until<'a>(
        &self,
        entries: &'a [WalEntryRecord],
        target_ts: Option<Duration>,
        target_cid: Option<&Cid>,
    ) -> Vec<&'a WalEntryRecord> {
        entries
            .iter()
            .filter(|entry| {
                if let Some(ts) = target_ts {
                    if Duration::from_nanos(entry.cid_ts) > ts {
                        return false;
                    }
                }
                if let Some(cid) = target_cid {
                    if entry.cid_ts > cid.ts.as_nanos() as u64 || entry.cid_server != cid.s_uuid {
                        return false;
                    }
                }
                true
            })
            .collect()
    }
}

#[allow(dead_code)]
pub fn parse_recovery_target_time(timestamp: &str) -> Result<Duration, WalError> {
    let dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| WalError::InvalidSegment(format!("Invalid timestamp format: {}", e)))?;

    let unix_ts = dt.timestamp();
    let nanos = dt.timestamp_subsec_nanos();

    Ok(Duration::new(unix_ts as u64, nanos))
}

#[allow(dead_code)]
pub fn parse_recovery_target_cid(cid_str: &str) -> Result<Cid, WalError> {
    let Some((ts_str, uuid_str)) = cid_str.split_once('-') else {
        return Err(WalError::InvalidSegment(format!(
            "Invalid CID format: {}",
            cid_str
        )));
    };

    let ts_nanos: u64 = ts_str
        .parse()
        .map_err(|_| WalError::InvalidSegment("Invalid timestamp in CID".to_string()))?;

    let s_uuid = Uuid::parse_str(uuid_str)
        .map_err(|_| WalError::InvalidSegment("Invalid UUID in CID".to_string()))?;

    Ok(Cid {
        ts: Duration::from_nanos(ts_nanos),
        s_uuid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kubidm_proto::backup::{PitrManifest, RecoveryTarget, RecoveryTargetType};
    use std::time::SystemTime;

    fn create_test_config() -> WalArchiveConfig {
        WalArchiveConfig {
            enabled: true,
            s3: None,
            retention_days: 7,
            segment_size_bytes: 1024,
        }
    }

    fn create_test_cid(ts_nanos: u64) -> Cid {
        Cid {
            ts: Duration::from_nanos(ts_nanos),
            s_uuid: Uuid::new_v4(),
        }
    }

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

    #[test]
    fn test_wal_segment_builder() {
        let server_uuid = Uuid::new_v4();
        let config = WalArchiveConfig::default();
        let base_path = std::env::temp_dir();

        let archiver = WalArchiver::new(config, server_uuid, &base_path);

        assert!(!archiver.is_enabled());
    }

    #[test]
    fn test_wal_archiver_enabled() {
        let server_uuid = Uuid::new_v4();
        let config = create_test_config();
        let base_path = std::env::temp_dir();

        let archiver = WalArchiver::new(config, server_uuid, &base_path);

        assert!(archiver.is_enabled());
    }

    #[test]
    fn test_wal_entry_record_create() {
        let cid = create_test_cid(1000);
        let entry_id = 1u64;
        let entry_data = vec![1, 2, 3, 4];

        let record = WalEntryRecord {
            cid_ts: cid.ts.as_nanos() as u64,
            cid_server: cid.s_uuid,
            entry_id,
            operation: WalOperationRecord::Create { entry_data },
        };

        assert_eq!(record.cid_ts, 1000);
        assert_eq!(record.entry_id, 1);
        assert!(matches!(
            record.operation,
            WalOperationRecord::Create { .. }
        ));
    }

    #[test]
    fn test_wal_entry_record_modify() {
        let cid = create_test_cid(2000);
        let entry_id = 2u64;
        let entry_data = vec![5, 6, 7, 8];

        let record = WalEntryRecord {
            cid_ts: cid.ts.as_nanos() as u64,
            cid_server: cid.s_uuid,
            entry_id,
            operation: WalOperationRecord::Modify { entry_data },
        };

        assert_eq!(record.cid_ts, 2000);
        assert_eq!(record.entry_id, 2);
        assert!(matches!(
            record.operation,
            WalOperationRecord::Modify { .. }
        ));
    }

    #[test]
    fn test_wal_entry_record_delete() {
        let cid = create_test_cid(3000);
        let entry_id = 3u64;

        let record = WalEntryRecord {
            cid_ts: cid.ts.as_nanos() as u64,
            cid_server: cid.s_uuid,
            entry_id,
            operation: WalOperationRecord::Delete,
        };

        assert_eq!(record.cid_ts, 3000);
        assert_eq!(record.entry_id, 3);
        assert!(matches!(record.operation, WalOperationRecord::Delete));
    }

    #[test]
    fn test_wal_entry_serialization() {
        let record = WalEntryRecord {
            cid_ts: 1000,
            cid_server: Uuid::new_v4(),
            entry_id: 1,
            operation: WalOperationRecord::Create {
                entry_data: vec![1, 2, 3],
            },
        };

        let serialized = serde_json::to_vec(&record);
        assert!(serialized.is_ok());

        let deserialized: WalEntryRecord = serde_json::from_slice(&serialized.unwrap()).unwrap();
        assert_eq!(record.cid_ts, deserialized.cid_ts);
        assert_eq!(record.entry_id, deserialized.entry_id);
    }

    #[test]
    fn test_wal_entry_deserialization() {
        let json = r#"{"cid_ts":1000,"cid_server":"00000000-0000-0000-0000-000000000001","entry_id":1,"operation":{"Create":{"entry_data":[1,2,3]}}}"#;
        let record: WalEntryRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.cid_ts, 1000);
        assert_eq!(record.entry_id, 1);
    }

    #[test]
    fn test_wal_entry_ordering() {
        let records: Vec<WalEntryRecord> = vec![
            WalEntryRecord {
                cid_ts: 3000,
                cid_server: Uuid::new_v4(),
                entry_id: 3,
                operation: WalOperationRecord::Delete,
            },
            WalEntryRecord {
                cid_ts: 1000,
                cid_server: Uuid::new_v4(),
                entry_id: 1,
                operation: WalOperationRecord::Create {
                    entry_data: vec![1],
                },
            },
            WalEntryRecord {
                cid_ts: 2000,
                cid_server: Uuid::new_v4(),
                entry_id: 2,
                operation: WalOperationRecord::Modify {
                    entry_data: vec![2],
                },
            },
        ];

        let sorted: Vec<u64> = records.iter().map(|r| r.cid_ts).collect();
        assert_eq!(sorted, vec![3000, 1000, 2000]);
    }

    #[test]
    fn test_parse_recovery_target_time() {
        let timestamp = "2024-01-15T10:30:00Z";
        let result = parse_recovery_target_time(timestamp);
        assert!(result.is_ok());
        let duration = result.unwrap();
        assert!(duration.as_secs() > 0);
    }

    #[test]
    fn test_parse_recovery_target_time_invalid() {
        let timestamp = "not-a-timestamp";
        let result = parse_recovery_target_time(timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_recovery_target_time_with_timezone() {
        let timestamp = "2024-01-15T10:30:00+05:00";
        let result = parse_recovery_target_time(timestamp);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_recovery_target_time_microsecond_precision() {
        let timestamp = "2024-01-15T10:30:00.123456Z";
        let result = parse_recovery_target_time(timestamp);
        assert!(result.is_ok());
        let duration = result.unwrap();
        assert!(duration.subsec_nanos() > 0);
    }

    #[test]
    fn test_timestamp_range_query() {
        let start_ts = Duration::from_secs(1704067200);
        let end_ts = Duration::from_secs(1704153600);
        let test_ts = Duration::from_secs(1704100000);

        assert!(test_ts >= start_ts);
        assert!(test_ts <= end_ts);
    }

    #[test]
    fn test_timestamp_comparison() {
        let ts1 = Duration::from_nanos(1000);
        let ts2 = Duration::from_nanos(2000);
        let ts3 = Duration::from_nanos(1000);

        assert!(ts1 < ts2);
        assert!(ts1 == ts3);
        assert!(ts2 > ts1);
    }

    #[test]
    fn test_recovery_state_creation() {
        let server_uuid = Uuid::new_v4();
        let target = RecoveryTarget::latest();
        let segments = vec![create_test_segment(
            server_uuid,
            Duration::from_secs(0),
            Duration::from_secs(100),
        )];

        let state = RecoveryState {
            target,
            available_segments: segments,
            base_backup_id: "backup-001".to_string(),
            base_backup_timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(state.available_segments.len(), 1);
        assert_eq!(state.base_backup_id, "backup-001");
    }

    #[test]
    fn test_recovery_state_validate_target_latest() {
        let server_uuid = Uuid::new_v4();
        let target = RecoveryTarget::latest();
        let segments = vec![create_test_segment(
            server_uuid,
            Duration::from_secs(0),
            Duration::from_secs(100),
        )];

        let state = RecoveryState {
            target,
            available_segments: segments,
            base_backup_id: "backup-001".to_string(),
            base_backup_timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(state.validate_target().is_ok());
    }

    #[test]
    fn test_recovery_state_validate_target_time_valid() {
        let server_uuid = Uuid::new_v4();
        let target = RecoveryTarget::to_time("2024-01-01T01:00:00Z").unwrap();
        let segments = vec![create_test_segment(
            server_uuid,
            Duration::from_secs(0),
            Duration::from_secs(3600),
        )];

        let state = RecoveryState {
            target,
            available_segments: segments,
            base_backup_id: "backup-001".to_string(),
            base_backup_timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(state.validate_target().is_ok());
    }

    #[test]
    fn test_recovery_state_validate_target_time_before_backup() {
        let server_uuid = Uuid::new_v4();
        let target = RecoveryTarget::to_time("2023-12-31T00:00:00Z").unwrap();
        let segments = vec![create_test_segment(
            server_uuid,
            Duration::from_secs(0),
            Duration::from_secs(3600),
        )];

        let state = RecoveryState {
            target,
            available_segments: segments,
            base_backup_id: "backup-001".to_string(),
            base_backup_timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(state.validate_target().is_err());
    }

    #[test]
    fn test_recovery_state_validate_target_time_after_latest() {
        let server_uuid = Uuid::new_v4();
        let target = RecoveryTarget::to_time("2030-01-01T00:00:00Z").unwrap();
        let segments = vec![create_test_segment(
            server_uuid,
            Duration::from_secs(0),
            Duration::from_secs(3600),
        )];

        let state = RecoveryState {
            target,
            available_segments: segments,
            base_backup_id: "backup-001".to_string(),
            base_backup_timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(state.validate_target().is_err());
    }

    #[test]
    fn test_recovery_state_get_segments_for_recovery() {
        let server_uuid = Uuid::new_v4();
        let target = RecoveryTarget::latest();
        let segments = vec![
            create_test_segment(
                server_uuid,
                Duration::from_secs(200),
                Duration::from_secs(300),
            ),
            create_test_segment(
                server_uuid,
                Duration::from_secs(0),
                Duration::from_secs(100),
            ),
            create_test_segment(
                server_uuid,
                Duration::from_secs(100),
                Duration::from_secs(200),
            ),
        ];

        let state = RecoveryState {
            target,
            available_segments: segments,
            base_backup_id: "backup-001".to_string(),
            base_backup_timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let sorted_segments = state.get_segments_for_recovery();
        assert_eq!(sorted_segments.len(), 3);
        assert!(sorted_segments[0].start_ts <= sorted_segments[1].start_ts);
        assert!(sorted_segments[1].start_ts <= sorted_segments[2].start_ts);
    }

    #[test]
    fn test_wal_segment_creation() {
        let server_uuid = Uuid::new_v4();
        let segment = create_test_segment(
            server_uuid,
            Duration::from_secs(0),
            Duration::from_secs(100),
        );

        assert!(segment.segment_id.contains("wal"));
        assert_eq!(segment.server_uuid, server_uuid);
        assert_eq!(segment.start_ts, Duration::from_secs(0));
        assert_eq!(segment.end_ts, Duration::from_secs(100));
    }

    #[test]
    fn test_wal_segment_display() {
        let server_uuid = Uuid::new_v4();
        let segment = create_test_segment(
            server_uuid,
            Duration::from_secs(0),
            Duration::from_secs(100),
        );
        let display = format!("{}", segment);
        assert!(display.contains("WalSegment"));
        assert!(display.contains(server_uuid.to_string().as_str()));
    }

    #[test]
    fn test_wal_archiver_record_create() {
        let server_uuid = Uuid::new_v4();
        let config = create_test_config();
        let base_path = std::env::temp_dir();
        let mut archiver = WalArchiver::new(config, server_uuid, &base_path);

        let cid = create_test_cid(1000);
        let result = archiver.record_create(&cid, 1, vec![1, 2, 3]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wal_archiver_record_modify() {
        let server_uuid = Uuid::new_v4();
        let config = create_test_config();
        let base_path = std::env::temp_dir();
        let mut archiver = WalArchiver::new(config, server_uuid, &base_path);

        let cid = create_test_cid(1000);
        let result = archiver.record_modify(&cid, 1, vec![4, 5, 6]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wal_archiver_record_delete() {
        let server_uuid = Uuid::new_v4();
        let config = create_test_config();
        let base_path = std::env::temp_dir();
        let mut archiver = WalArchiver::new(config, server_uuid, &base_path);

        let cid = create_test_cid(1000);
        let result = archiver.record_delete(&cid, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wal_archiver_disabled_operations() {
        let server_uuid = Uuid::new_v4();
        let config = WalArchiveConfig::default();
        let base_path = std::env::temp_dir();
        let mut archiver = WalArchiver::new(config, server_uuid, &base_path);

        let cid = create_test_cid(1000);
        let create_result = archiver.record_create(&cid, 1, vec![1, 2, 3]);
        let modify_result = archiver.record_modify(&cid, 1, vec![4, 5, 6]);
        let delete_result = archiver.record_delete(&cid, 1);

        assert!(create_result.is_ok());
        assert!(modify_result.is_ok());
        assert!(delete_result.is_ok());
    }

    #[test]
    fn test_wal_archiver_flush_empty_segment() {
        let server_uuid = Uuid::new_v4();
        let config = create_test_config();
        let base_path = std::env::temp_dir();
        let mut archiver = WalArchiver::new(config, server_uuid, &base_path);

        let result = archiver.flush_current_segment();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_wal_replayer_load_segment_uncompressed() {
        let server_uuid = Uuid::new_v4();
        let replayer = WalReplayer::new(server_uuid);

        let entries = vec![WalEntryRecord {
            cid_ts: 1000,
            cid_server: server_uuid,
            entry_id: 1,
            operation: WalOperationRecord::Create {
                entry_data: vec![1, 2, 3],
            },
        }];

        let data = serde_json::to_vec(&entries).unwrap();
        let result = replayer.load_segment(&data, BackupCompression::NoCompression);
        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_wal_replayer_replay_until_time() {
        let server_uuid = Uuid::new_v4();
        let replayer = WalReplayer::new(server_uuid);

        let entries = vec![
            WalEntryRecord {
                cid_ts: 1000,
                cid_server: server_uuid,
                entry_id: 1,
                operation: WalOperationRecord::Create {
                    entry_data: vec![1],
                },
            },
            WalEntryRecord {
                cid_ts: 2000,
                cid_server: server_uuid,
                entry_id: 2,
                operation: WalOperationRecord::Create {
                    entry_data: vec![2],
                },
            },
            WalEntryRecord {
                cid_ts: 3000,
                cid_server: server_uuid,
                entry_id: 3,
                operation: WalOperationRecord::Create {
                    entry_data: vec![3],
                },
            },
        ];

        let target_ts = Some(Duration::from_nanos(2500));
        let filtered = replayer.replay_until(&entries, target_ts, None);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_wal_replayer_replay_all() {
        let server_uuid = Uuid::new_v4();
        let replayer = WalReplayer::new(server_uuid);

        let entries = vec![
            WalEntryRecord {
                cid_ts: 1000,
                cid_server: server_uuid,
                entry_id: 1,
                operation: WalOperationRecord::Create {
                    entry_data: vec![1],
                },
            },
            WalEntryRecord {
                cid_ts: 2000,
                cid_server: server_uuid,
                entry_id: 2,
                operation: WalOperationRecord::Create {
                    entry_data: vec![2],
                },
            },
        ];

        let filtered = replayer.replay_until(&entries, None, None);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_wal_replayer_replay_until_cid() {
        let server_uuid = Uuid::new_v4();
        let replayer = WalReplayer::new(server_uuid);

        let target_cid = Cid {
            ts: Duration::from_nanos(1500),
            s_uuid: server_uuid,
        };

        let entries = vec![
            WalEntryRecord {
                cid_ts: 1000,
                cid_server: server_uuid,
                entry_id: 1,
                operation: WalOperationRecord::Create {
                    entry_data: vec![1],
                },
            },
            WalEntryRecord {
                cid_ts: 2000,
                cid_server: server_uuid,
                entry_id: 2,
                operation: WalOperationRecord::Create {
                    entry_data: vec![2],
                },
            },
        ];

        let filtered = replayer.replay_until(&entries, None, Some(&target_cid));
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_parse_recovery_target_cid_valid() {
        let uuid = Uuid::new_v4();
        let cid_str = format!("{:032}-{}", 1000u64, uuid);
        let result = parse_recovery_target_cid(&cid_str);
        assert!(result.is_ok());
        let cid = result.unwrap();
        assert_eq!(cid.ts, Duration::from_nanos(1000));
        assert_eq!(cid.s_uuid, uuid);
    }

    #[test]
    fn test_parse_recovery_target_cid_invalid_format() {
        let cid_str = "invalid-cid";
        let result = parse_recovery_target_cid(cid_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_recovery_target_cid_missing_uuid() {
        let cid_str = "1000";
        let result = parse_recovery_target_cid(cid_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_retention_policy() {
        let server_uuid = Uuid::new_v4();
        let config = WalArchiveConfig {
            enabled: true,
            s3: None,
            retention_days: 1,
            segment_size_bytes: 1024,
        };
        let base_path = std::env::temp_dir();
        let mut archiver = WalArchiver::new(config, server_uuid, &base_path);

        let mut manifest = create_test_manifest(server_uuid);

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        let old_ts = Duration::from_secs(now.as_secs() - 86400 * 2);
        let new_ts = Duration::from_secs(now.as_secs() - 3600);

        manifest.add_segment(create_test_segment(server_uuid, old_ts, old_ts));
        manifest.add_segment(create_test_segment(server_uuid, new_ts, new_ts));

        let deleted = archiver.apply_retention_policy(&mut manifest).unwrap();
        assert!(!deleted.is_empty());
        assert!(manifest.segments.len() < 2);
    }

    #[test]
    fn test_apply_retention_policy_empty_manifest() {
        let server_uuid = Uuid::new_v4();
        let config = create_test_config();
        let base_path = std::env::temp_dir();
        let mut archiver = WalArchiver::new(config, server_uuid, &base_path);

        let mut manifest = create_test_manifest(server_uuid);
        assert!(manifest.segments.is_empty());

        let deleted = archiver.apply_retention_policy(&mut manifest).unwrap();
        assert!(deleted.is_empty());
    }

    #[test]
    fn test_wal_error_display() {
        let error = WalError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
        assert!(error.to_string().contains("IO error"));

        let error = WalError::SerializationError("test".to_string());
        assert!(error.to_string().contains("serialization error"));

        let error = WalError::InvalidSegment("test".to_string());
        assert!(error.to_string().contains("Invalid WAL segment"));

        let error = WalError::RetentionError("test".to_string());
        assert!(error.to_string().contains("retention error"));

        let error = WalError::ConfigError("test".to_string());
        assert!(error.to_string().contains("config error"));
    }

    #[test]
    fn test_wal_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let wal_error: WalError = io_error.into();
        assert!(matches!(wal_error, WalError::IoError(_)));
    }

    #[test]
    fn test_wal_error_from_serde_json() {
        let serde_error = serde_json::from_str::<WalEntryRecord>("invalid json").unwrap_err();
        let wal_error: WalError = serde_error.into();
        assert!(matches!(wal_error, WalError::SerializationError(_)));
    }

    #[test]
    fn test_empty_transaction_log_recovery() {
        let server_uuid = Uuid::new_v4();
        let replayer = WalReplayer::new(server_uuid);

        let entries: Vec<WalEntryRecord> = vec![];
        let filtered = replayer.replay_until(&entries, None, None);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_wal_segment_data_creation() {
        let server_uuid = Uuid::new_v4();
        let entries = vec![WalEntryRecord {
            cid_ts: 1000,
            cid_server: server_uuid,
            entry_id: 1,
            operation: WalOperationRecord::Create {
                entry_data: vec![1, 2, 3],
            },
        }];

        let segment_data = WalSegmentData {
            segment_id: "test-segment".to_string(),
            server_uuid,
            entries,
            start_ts: Duration::from_secs(0),
            end_ts: Duration::from_secs(100),
        };

        assert_eq!(segment_data.entries.len(), 1);
        assert_eq!(segment_data.segment_id, "test-segment");
    }

    #[test]
    fn test_wal_config_default() {
        let config = WalArchiveConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.retention_days, 7);
        assert_eq!(config.segment_size_bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn test_estimate_record_size_create() {
        let server_uuid = Uuid::new_v4();
        let config = create_test_config();
        let base_path = std::env::temp_dir();
        let archiver = WalArchiver::new(config, server_uuid, &base_path);

        let record = WalEntryRecord {
            cid_ts: 1000,
            cid_server: server_uuid,
            entry_id: 1,
            operation: WalOperationRecord::Create {
                entry_data: vec![1; 100],
            },
        };

        let size = archiver.estimate_record_size(&record);
        assert!(size > 100);
    }

    #[test]
    fn test_estimate_record_size_delete() {
        let server_uuid = Uuid::new_v4();
        let config = create_test_config();
        let base_path = std::env::temp_dir();
        let archiver = WalArchiver::new(config, server_uuid, &base_path);

        let record = WalEntryRecord {
            cid_ts: 1000,
            cid_server: server_uuid,
            entry_id: 1,
            operation: WalOperationRecord::Delete,
        };

        let size = archiver.estimate_record_size(&record);
        let base_size = std::mem::size_of::<WalEntryRecord>() as u64;
        assert_eq!(size, base_size);
    }

    #[test]
    fn test_multiple_recovery_points() {
        let server_uuid = Uuid::new_v4();
        let mut manifest = create_test_manifest(server_uuid);

        for i in 0..5 {
            let ts = Duration::from_secs(i * 100);
            manifest.add_segment(create_test_segment(server_uuid, ts, ts));
        }

        assert_eq!(manifest.segments.len(), 5);
        assert!(manifest.earliest_recoverable_time != manifest.latest_recoverable_time);
    }

    #[test]
    fn test_pitr_manifest_add_segment_updates_times() {
        let server_uuid = Uuid::new_v4();
        let mut manifest = create_test_manifest(server_uuid);

        let segment1 = create_test_segment(
            server_uuid,
            Duration::from_secs(0),
            Duration::from_secs(100),
        );
        let segment2 = create_test_segment(
            server_uuid,
            Duration::from_secs(100),
            Duration::from_secs(200),
        );

        manifest.add_segment(segment1.clone());
        assert_eq!(manifest.earliest_recoverable_time, segment1.created_at);
        assert_eq!(manifest.latest_recoverable_time, segment1.created_at);

        manifest.add_segment(segment2.clone());
        assert_eq!(manifest.earliest_recoverable_time, segment1.created_at);
        assert_eq!(manifest.latest_recoverable_time, segment2.created_at);
    }

    #[test]
    fn test_timestamp_future() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let future_ts = Duration::from_secs(now.as_secs() + 86400 * 365);

        let server_uuid = Uuid::new_v4();
        let segment = create_test_segment(server_uuid, future_ts, future_ts);
        assert!(segment.start_ts > now);
    }

    #[test]
    fn test_timestamp_before_database_creation() {
        let old_ts = Duration::from_secs(0);
        let server_uuid = Uuid::new_v4();
        let segment = create_test_segment(server_uuid, old_ts, old_ts);
        assert_eq!(segment.start_ts, Duration::from_secs(0));
    }

    #[test]
    fn test_wal_archiver_segment_rotation() {
        let server_uuid = Uuid::new_v4();
        let config = WalArchiveConfig {
            enabled: true,
            s3: None,
            retention_days: 7,
            segment_size_bytes: 1,
        };
        let base_path = std::env::temp_dir();
        let mut archiver = WalArchiver::new(config, server_uuid, &base_path);

        let cid1 = create_test_cid(1000);
        let result = archiver.record_create(&cid1, 1, vec![1; 10]);
        assert!(result.is_ok());

        let cid2 = create_test_cid(2000);
        let result = archiver.record_create(&cid2, 2, vec![2; 10]);
        assert!(result.is_ok());

        let result = archiver.flush_current_segment();
        assert!(result.is_ok());
    }

    #[test]
    fn test_recovery_target_time_creation() {
        let target = RecoveryTarget::to_time("2024-01-15T10:30:00Z");
        assert!(target.is_ok());
        let target = target.unwrap();
        assert!(matches!(
            target.target_type,
            RecoveryTargetType::Time { .. }
        ));
    }

    #[test]
    fn test_recovery_target_transaction_creation() {
        let target = RecoveryTarget::to_transaction("1000-uuid");
        assert!(target.is_ok());
        let target = target.unwrap();
        assert!(matches!(
            target.target_type,
            RecoveryTargetType::Transaction { .. }
        ));
    }

    #[test]
    fn test_recovery_target_transaction_empty() {
        let target = RecoveryTarget::to_transaction("");
        assert!(target.is_err());
    }

    #[test]
    fn test_recovery_target_latest() {
        let target = RecoveryTarget::latest();
        assert!(matches!(target.target_type, RecoveryTargetType::Latest));
    }

    #[test]
    fn test_recovery_target_display() {
        let time_target = RecoveryTarget::to_time("2024-01-15T10:30:00Z").unwrap();
        assert!(time_target.to_string().starts_with("time:"));

        let cid_target = RecoveryTarget::to_transaction("test-cid").unwrap();
        assert!(cid_target.to_string().starts_with("transaction:"));

        let latest_target = RecoveryTarget::latest();
        assert_eq!(latest_target.to_string(), "latest");
    }
}

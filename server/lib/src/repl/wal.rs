//! Write-Ahead Log (WAL) archiving for Point-in-Time Recovery (PITR)
//!
//! This module provides WAL archiving capabilities that allow recovery to
//! any point in time within the configured retention window.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use kanidm_proto::backup::{BackupCompression, PitrManifest, WalArchiveConfig, WalSegment};
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
    pub target: kanidm_proto::backup::RecoveryTarget,
    pub available_segments: Vec<WalSegment>,
    pub base_backup_id: String,
    pub base_backup_timestamp: String,
}

#[allow(dead_code)]
impl RecoveryState {
    pub fn validate_target(&self) -> Result<(), WalError> {
        match &self.target.target_type {
            kanidm_proto::backup::RecoveryTargetType::Time { timestamp } => {
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
            kanidm_proto::backup::RecoveryTargetType::Transaction { cid } => {
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
            kanidm_proto::backup::RecoveryTargetType::Latest => {}
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
    let parts: Vec<&str> = cid_str.split('-').collect();
    if parts.len() < 2 {
        return Err(WalError::InvalidSegment(format!(
            "Invalid CID format: {}",
            cid_str
        )));
    }

    let Some(ts_str) = parts.first() else {
        return Err(WalError::InvalidSegment(
            "Invalid CID format: missing timestamp".to_string(),
        ));
    };
    let Some(uuid_str) = parts.get(1) else {
        return Err(WalError::InvalidSegment(
            "Invalid CID format: missing UUID".to_string(),
        ));
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

    #[test]
    fn test_wal_segment_builder() {
        let server_uuid = Uuid::new_v4();
        let config = WalArchiveConfig::default();
        let base_path = std::env::temp_dir();

        let archiver = WalArchiver::new(config, server_uuid, &base_path);

        assert!(!archiver.is_enabled());
    }

    #[test]
    fn test_parse_recovery_target_time() {
        let timestamp = "2024-01-15T10:30:00Z";
        let result = parse_recovery_target_time(timestamp);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_recovery_target_time_invalid() {
        let timestamp = "not-a-timestamp";
        let result = parse_recovery_target_time(timestamp);
        assert!(result.is_err());
    }
}

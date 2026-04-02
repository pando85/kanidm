//! Relates to backup functionality in the Server
use std::{fmt::Display, path::Path, str::FromStr, time::Duration};

use serde::{Deserialize, Serialize};
use serde_with::DeserializeFromStr;
use sketching::tracing::warn;
use uuid::Uuid;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, DeserializeFromStr, Serialize)]
/// Compression types for backups, defaults to Gzip
pub enum BackupCompression {
    NoCompression,
    #[default]
    Gzip,
}

impl BackupCompression {
    pub fn suffix(&self) -> &'static str {
        match self {
            BackupCompression::NoCompression => "",
            BackupCompression::Gzip => ".gz",
        }
    }

    pub fn identify_file(filepath: &Path) -> Self {
        let filename = filepath.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if filename.ends_with(".gz") {
            BackupCompression::Gzip
        } else {
            BackupCompression::NoCompression
        }
    }
}

impl Display for BackupCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BackupCompression::NoCompression => write!(f, "No Compression"),
            BackupCompression::Gzip => write!(f, "Gzip"),
        }
    }
}

impl From<Option<String>> for BackupCompression {
    fn from(opt: Option<String>) -> Self {
        match opt {
            Some(s) => BackupCompression::from(s),
            None => BackupCompression::default(),
        }
    }
}

impl From<String> for BackupCompression {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "gzip" => BackupCompression::Gzip,
            "none" | "nocompression" => BackupCompression::NoCompression,
            _ => {
                warn!(
                    "Unknown compression type '{}', should be one of nocompression, gzip - defaulting to {}",
                    s,
                    BackupCompression::default()
                );
                BackupCompression::default()
            }
        }
    }
}

impl FromStr for BackupCompression {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_string().into())
    }
}

#[test]

fn test_backup_compression_identify() {
    let gzip_path = Path::new("/var/lib/kanidm/backups/backup-2024-01-01.tar.gz");
    let no_comp_path = Path::new("/var/lib/kanidm/backups/backup-2024-01-01.tar");

    assert_eq!(
        BackupCompression::identify_file(gzip_path),
        BackupCompression::Gzip
    );
    assert_eq!(
        BackupCompression::identify_file(no_comp_path),
        BackupCompression::NoCompression
    );

    for (input, expected) in [
        (vec!["gzip", "Gzip", "GzIp"], BackupCompression::Gzip),
        (
            vec!["none", "NoNe", "nocompression", "NoCompression"],
            BackupCompression::NoCompression,
        ),
    ] {
        for i in input {
            assert_eq!(
                BackupCompression::from_str(i).expect("Threw an error?"),
                expected
            );
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct S3Config {
    pub bucket: String,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub path_prefix: Option<String>,
    #[serde(default)]
    pub credentials: Option<S3Credentials>,
    #[serde(default)]
    pub server_side_encryption: Option<S3ServerSideEncryption>,
    #[serde(default = "default_s3_storage_class")]
    pub storage_class: String,
}

fn default_s3_storage_class() -> String {
    "STANDARD".to_string()
}

impl Display for S3Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "S3Config {{ bucket: {}, region: {:?}, endpoint: {:?} }}",
            self.bucket, self.region, self.endpoint
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct S3Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    #[serde(default)]
    pub session_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct S3ServerSideEncryption {
    #[serde(default)]
    pub algorithm: Option<S3EncryptionAlgorithm>,
    #[serde(default)]
    pub kms_key_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum S3EncryptionAlgorithm {
    #[serde(rename = "AES256")]
    Aes256,
    #[default]
    #[serde(rename = "aws:kms")]
    AwsKms,
}

impl Display for S3EncryptionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S3EncryptionAlgorithm::Aes256 => write!(f, "AES256"),
            S3EncryptionAlgorithm::AwsKms => write!(f, "aws:kms"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct S3BackupMetadata {
    pub checksum_sha256: String,
    pub timestamp: String,
    pub compression: BackupCompression,
    pub size_bytes: u64,
}

impl S3BackupMetadata {
    pub fn new(
        checksum_sha256: String,
        timestamp: String,
        compression: BackupCompression,
        size_bytes: u64,
    ) -> Self {
        Self {
            checksum_sha256,
            timestamp,
            compression,
            size_bytes,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WalArchiveConfig {
    #[serde(default = "default_wal_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub s3: Option<S3Config>,
    #[serde(default = "default_wal_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_wal_segment_size")]
    pub segment_size_bytes: u64,
}

fn default_wal_enabled() -> bool {
    false
}

fn default_wal_retention_days() -> u32 {
    7
}

fn default_wal_segment_size() -> u64 {
    16 * 1024 * 1024
}

impl Default for WalArchiveConfig {
    fn default() -> Self {
        Self {
            enabled: default_wal_enabled(),
            s3: None,
            retention_days: default_wal_retention_days(),
            segment_size_bytes: default_wal_segment_size(),
        }
    }
}

impl Display for WalArchiveConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WalArchiveConfig {{ enabled: {}, retention_days: {}, segment_size: {} }}",
            self.enabled, self.retention_days, self.segment_size_bytes
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WalSegment {
    pub segment_id: String,
    pub server_uuid: Uuid,
    pub start_ts: Duration,
    pub end_ts: Duration,
    pub checksum_sha256: String,
    pub size_bytes: u64,
    pub compression: BackupCompression,
    pub created_at: String,
}

impl WalSegment {
    pub fn new(
        segment_id: String,
        server_uuid: Uuid,
        start_ts: Duration,
        end_ts: Duration,
        checksum_sha256: String,
        size_bytes: u64,
        compression: BackupCompression,
    ) -> Self {
        Self {
            segment_id,
            server_uuid,
            start_ts,
            end_ts,
            checksum_sha256,
            size_bytes,
            compression,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl Display for WalSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WalSegment {{ id: {}, server: {}, range: {:?}-{:?}, size: {} }}",
            self.segment_id, self.server_uuid, self.start_ts, self.end_ts, self.size_bytes
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WalSegmentMetadata {
    pub segment: WalSegment,
    pub entry_count: u64,
    pub first_cid: String,
    pub last_cid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WalEntry {
    pub cid_ts: Duration,
    pub cid_server: Uuid,
    pub entry_id: u64,
    pub operation: WalOperation,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum WalOperation {
    Create { entry_data: Vec<u8> },
    Modify { entry_data: Vec<u8> },
    Delete,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecoveryTarget {
    pub target_type: RecoveryTargetType,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum RecoveryTargetType {
    Time { timestamp: String },
    Transaction { cid: String },
    Latest,
}

impl RecoveryTarget {
    pub fn to_time(target_time: &str) -> Result<Self, String> {
        let _ = chrono::DateTime::parse_from_rfc3339(target_time)
            .map_err(|e| format!("Invalid timestamp format: {}", e))?;
        Ok(Self {
            target_type: RecoveryTargetType::Time {
                timestamp: target_time.to_string(),
            },
        })
    }

    pub fn to_transaction(cid: &str) -> Result<Self, String> {
        if cid.is_empty() {
            return Err("Transaction CID cannot be empty".to_string());
        }
        Ok(Self {
            target_type: RecoveryTargetType::Transaction {
                cid: cid.to_string(),
            },
        })
    }

    pub fn latest() -> Self {
        Self {
            target_type: RecoveryTargetType::Latest,
        }
    }
}

impl Display for RecoveryTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.target_type {
            RecoveryTargetType::Time { timestamp } => write!(f, "time:{}", timestamp),
            RecoveryTargetType::Transaction { cid } => write!(f, "transaction:{}", cid),
            RecoveryTargetType::Latest => write!(f, "latest"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PitrManifest {
    pub server_uuid: Uuid,
    pub segments: Vec<WalSegment>,
    pub base_backup_id: String,
    pub base_backup_timestamp: String,
    pub earliest_recoverable_time: String,
    pub latest_recoverable_time: String,
}

impl PitrManifest {
    pub fn new(server_uuid: Uuid, base_backup_id: String, base_backup_timestamp: String) -> Self {
        Self {
            server_uuid,
            segments: Vec::new(),
            base_backup_id,
            base_backup_timestamp,
            earliest_recoverable_time: base_backup_timestamp.clone(),
            latest_recoverable_time: base_backup_timestamp,
        }
    }

    pub fn add_segment(&mut self, segment: WalSegment) {
        if self.segments.is_empty() {
            self.earliest_recoverable_time = segment.created_at.clone();
        }
        self.latest_recoverable_time = segment.created_at.clone();
        self.segments.push(segment);
    }
}

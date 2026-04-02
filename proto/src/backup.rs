//! Relates to backup functionality in the Server
use std::{fmt::Display, path::Path, str::FromStr, time::Duration};

use serde::{Deserialize, Serialize};
use serde_with::DeserializeFromStr;
use sketching::tracing::warn;
use uuid::Uuid;

pub const BACKUP_ENCRYPTION_MAGIC: &[u8] = b"KANIDM_ENC_BACKUP_V1";
pub const BACKUP_ENCRYPTION_KEY_LEN: usize = 32;
pub const BACKUP_ENCRYPTION_NONCE_LEN: usize = 12;
pub const BACKUP_ENCRYPTION_SALT_LEN: usize = 16;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, DeserializeFromStr, Serialize)]
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

#[test]
fn test_key_derivation_params_default() {
    let params = KeyDerivationParams::default();
    assert_eq!(params.m_cost, 19 * 1024);
    assert_eq!(params.t_cost, 2);
    assert_eq!(params.p_cost, 1);
}

#[test]
fn test_encryption_key_source_display() {
    assert_eq!(EncryptionKeySource::Passphrase.to_string(), "passphrase");
    assert_eq!(
        EncryptionKeySource::File {
            path: "/path/to/key".to_string()
        }
        .to_string(),
        "file:/path/to/key"
    );
    assert_eq!(
        EncryptionKeySource::HttpEndpoint {
            url: "https://vault.example.com/key".to_string()
        }
        .to_string(),
        "http:https://vault.example.com/key"
    );
}

#[test]
fn test_encryption_key_source_serialization() {
    let source_passphrase = EncryptionKeySource::Passphrase;
    let json = serde_json::to_string(&source_passphrase).unwrap();
    assert_eq!(json, "\"Passphrase\"");

    let source_file = EncryptionKeySource::File {
        path: "/path/to/key".to_string(),
    };
    let json = serde_json::to_string(&source_file).unwrap();
    let deserialized: EncryptionKeySource = serde_json::from_str(&json).unwrap();
    assert_eq!(source_file, deserialized);

    let source_http = EncryptionKeySource::HttpEndpoint {
        url: "https://vault.example.com/key".to_string(),
    };
    let json = serde_json::to_string(&source_http).unwrap();
    let deserialized: EncryptionKeySource = serde_json::from_str(&json).unwrap();
    assert_eq!(source_http, deserialized);
}

#[test]
fn test_backup_encryption_config_default() {
    let config = BackupEncryptionConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.key_source, EncryptionKeySource::Passphrase);
    assert!(config.key_identifier.is_none());
}

#[test]
fn test_backup_encryption_config_serialization() {
    let config = BackupEncryptionConfig {
        enabled: true,
        key_source: EncryptionKeySource::File {
            path: "/path/to/key".to_string(),
        },
        key_derivation: KeyDerivationParams::default(),
        key_identifier: Some("key-123".to_string()),
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: BackupEncryptionConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.enabled, deserialized.enabled);
    assert_eq!(config.key_source, deserialized.key_source);
    assert_eq!(config.key_identifier, deserialized.key_identifier);
}

#[test]
fn test_backup_encryption_config_display() {
    let config = BackupEncryptionConfig {
        enabled: true,
        key_source: EncryptionKeySource::Passphrase,
        key_derivation: KeyDerivationParams::default(),
        key_identifier: None,
    };
    assert!(config.to_string().contains("enabled: true"));
    assert!(config.to_string().contains("key_source: passphrase"));
}

#[test]
fn test_key_derivation_params_custom_values() {
    let params = KeyDerivationParams {
        m_cost: 32 * 1024,
        t_cost: 4,
        p_cost: 2,
    };
    assert_eq!(params.m_cost, 32 * 1024);
    assert_eq!(params.t_cost, 4);
    assert_eq!(params.p_cost, 2);
}

#[test]
fn test_key_derivation_params_serialization() {
    let params = KeyDerivationParams {
        m_cost: 16384,
        t_cost: 3,
        p_cost: 1,
    };

    let json = serde_json::to_string(&params).unwrap();
    let deserialized: KeyDerivationParams = serde_json::from_str(&json).unwrap();

    assert_eq!(params.m_cost, deserialized.m_cost);
    assert_eq!(params.t_cost, deserialized.t_cost);
    assert_eq!(params.p_cost, deserialized.p_cost);
}

#[test]
fn test_key_derivation_params_from_json_partial() {
    let json = "{}";
    let params: KeyDerivationParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.m_cost, 19 * 1024);
    assert_eq!(params.t_cost, 2);
    assert_eq!(params.p_cost, 1);
}

#[test]
fn test_backup_encryption_header_display() {
    let header = BackupEncryptionHeader::new(
        "test-key-id".to_string(),
        vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
        vec![0u8; BACKUP_ENCRYPTION_NONCE_LEN],
        KeyDerivationParams::default(),
        true,
    );

    let display = header.to_string();
    assert!(display.contains("test-key-id"));
    assert!(display.contains("compressed: true"));
}

#[test]
fn test_backup_encryption_magic_constant() {
    assert_eq!(BACKUP_ENCRYPTION_MAGIC.len(), 20);
    assert_eq!(BACKUP_ENCRYPTION_MAGIC, b"KANIDM_ENC_BACKUP_V1");
}

#[test]
fn test_backup_encryption_key_len_constant() {
    assert_eq!(BACKUP_ENCRYPTION_KEY_LEN, 32);
}

#[test]
fn test_backup_encryption_nonce_len_constant() {
    assert_eq!(BACKUP_ENCRYPTION_NONCE_LEN, 12);
}

#[test]
fn test_backup_encryption_salt_len_constant() {
    assert_eq!(BACKUP_ENCRYPTION_SALT_LEN, 16);
}

#[test]
fn test_s3_backup_metadata_not_encrypted() {
    let meta = S3BackupMetadata::new(
        "sha256".to_string(),
        "2024-01-01".to_string(),
        BackupCompression::Gzip,
        1024,
    );
    assert!(!meta.encrypted);
    assert!(meta.key_identifier.is_none());
}

#[test]
fn test_backup_encryption_header_validate_magic() {
    let header = BackupEncryptionHeader::new(
        "test-key".to_string(),
        vec![0u8; 16],
        vec![0u8; 12],
        KeyDerivationParams::default(),
        false,
    );
    assert!(header.validate_magic());
}

#[test]
fn test_s3_backup_metadata_encrypted() {
    let meta = S3BackupMetadata::new_encrypted(
        "sha256".to_string(),
        "2024-01-01".to_string(),
        BackupCompression::Gzip,
        1024,
        "key-123".to_string(),
    );
    assert!(meta.encrypted);
    assert_eq!(meta.key_identifier, Some("key-123".to_string()));
}

#[test]
fn test_replication_config_default() {
    let config = ReplicationConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.regions.len(), 0);
    assert_eq!(config.sync_interval_seconds, 300);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.retry_delay_seconds, 30);
}

#[test]
fn test_replication_status_display() {
    assert_eq!(
        ReplicationStatus::NotConfigured.to_string(),
        "Not Configured"
    );
    assert_eq!(ReplicationStatus::InProgress.to_string(), "In Progress");
    assert_eq!(
        ReplicationStatus::Failed {
            error: "network error".to_string()
        }
        .to_string(),
        "Failed: network error"
    );
}

#[test]
fn test_replication_region_status() {
    let status = ReplicationRegionStatus {
        region: "us-west-2".to_string(),
        bucket: "backup-bucket".to_string(),
        status: ReplicationStatus::Completed,
        last_sync_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
        last_sync_backup_id: Some("backup-123".to_string()),
        lag_seconds: Some(60),
        bytes_replicated: 1024,
        backups_replicated: 5,
        last_error: None,
    };
    assert!(status.last_error.is_none());
    assert_eq!(status.lag_seconds, Some(60));
}

#[test]
fn test_replication_health_check() {
    let check = ReplicationHealthCheck {
        overall_status: ReplicationStatus::Completed,
        regions: vec![],
        total_lag_seconds: 120,
        max_lag_seconds: 120,
        healthy_regions: 1,
        unhealthy_regions: 0,
        last_check_timestamp: "2024-01-01T00:00:00Z".to_string(),
    };
    assert_eq!(check.healthy_regions, 1);
    assert_eq!(check.unhealthy_regions, 0);
}

#[test]
fn test_replication_lag_metrics() {
    let metrics = ReplicationLagMetrics {
        region: "eu-west-1".to_string(),
        lag_seconds: 300,
        pending_backups: 2,
        last_backup_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
        replication_delay_seconds: 60,
    };
    assert_eq!(metrics.lag_seconds, 300);
    assert_eq!(metrics.pending_backups, 2);
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
    #[serde(default)]
    pub replication: Option<ReplicationConfig>,
}

fn default_s3_storage_class() -> String {
    "STANDARD".to_string()
}

impl Display for S3Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "S3Config {{ bucket: {}, region: {:?}, endpoint: {:?}, replication_enabled: {} }}",
            self.bucket,
            self.region,
            self.endpoint,
            self.replication
                .as_ref()
                .map(|r| r.enabled)
                .unwrap_or(false)
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
pub struct ReplicationRegionConfig {
    pub region: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    pub bucket: String,
    #[serde(default)]
    pub path_prefix: Option<String>,
    #[serde(default)]
    pub credentials: Option<S3Credentials>,
    #[serde(default)]
    pub server_side_encryption: Option<S3ServerSideEncryption>,
    #[serde(default = "default_s3_storage_class")]
    pub storage_class: String,
    #[serde(default)]
    pub kms_key_id: Option<String>,
}

impl Display for ReplicationRegionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReplicationRegionConfig {{ region: {}, bucket: {}, endpoint: {:?} }}",
            self.region, self.bucket, self.endpoint
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReplicationConfig {
    #[serde(default = "default_replication_enabled")]
    pub enabled: bool,
    pub regions: Vec<ReplicationRegionConfig>,
    #[serde(default = "default_replication_sync_interval")]
    pub sync_interval_seconds: u64,
    #[serde(default = "default_replication_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_replication_retry_delay")]
    pub retry_delay_seconds: u64,
}

fn default_replication_enabled() -> bool {
    false
}

fn default_replication_sync_interval() -> u64 {
    300
}

fn default_replication_max_retries() -> u32 {
    3
}

fn default_replication_retry_delay() -> u64 {
    30
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            enabled: default_replication_enabled(),
            regions: Vec::new(),
            sync_interval_seconds: default_replication_sync_interval(),
            max_retries: default_replication_max_retries(),
            retry_delay_seconds: default_replication_retry_delay(),
        }
    }
}

impl Display for ReplicationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReplicationConfig {{ enabled: {}, regions: {}, sync_interval: {}s }}",
            self.enabled,
            self.regions.len(),
            self.sync_interval_seconds
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ReplicationStatus {
    NotConfigured,
    Pending,
    InProgress,
    Completed,
    Failed { error: String },
    Degraded { message: String },
}

impl Display for ReplicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplicationStatus::NotConfigured => write!(f, "Not Configured"),
            ReplicationStatus::Pending => write!(f, "Pending"),
            ReplicationStatus::InProgress => write!(f, "In Progress"),
            ReplicationStatus::Completed => write!(f, "Completed"),
            ReplicationStatus::Failed { error } => write!(f, "Failed: {}", error),
            ReplicationStatus::Degraded { message } => write!(f, "Degraded: {}", message),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReplicationRegionStatus {
    pub region: String,
    pub bucket: String,
    pub status: ReplicationStatus,
    pub last_sync_timestamp: Option<String>,
    pub last_sync_backup_id: Option<String>,
    pub lag_seconds: Option<u64>,
    pub bytes_replicated: u64,
    pub backups_replicated: u64,
    pub last_error: Option<String>,
}

impl Display for ReplicationRegionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Region {} (bucket: {}): {} - lag: {}s, backups: {}, bytes: {}",
            self.region,
            self.bucket,
            self.status,
            self.lag_seconds.unwrap_or(0),
            self.backups_replicated,
            self.bytes_replicated
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReplicationHealthCheck {
    pub overall_status: ReplicationStatus,
    pub regions: Vec<ReplicationRegionStatus>,
    pub total_lag_seconds: u64,
    pub max_lag_seconds: u64,
    pub healthy_regions: usize,
    pub unhealthy_regions: usize,
    pub last_check_timestamp: String,
}

impl Display for ReplicationHealthCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReplicationHealth {{ overall: {}, healthy: {}, unhealthy: {}, max_lag: {}s }}",
            self.overall_status, self.healthy_regions, self.unhealthy_regions, self.max_lag_seconds
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReplicationLagMetrics {
    pub region: String,
    pub lag_seconds: u64,
    pub pending_backups: usize,
    pub last_backup_timestamp: Option<String>,
    pub replication_delay_seconds: u64,
}

impl Display for ReplicationLagMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReplicationLag {{ region: {}, lag: {}s, pending: {} }}",
            self.region, self.lag_seconds, self.pending_backups
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct KeyDerivationParams {
    #[serde(default = "default_argon2_m_cost")]
    pub m_cost: u32,
    #[serde(default = "default_argon2_t_cost")]
    pub t_cost: u32,
    #[serde(default = "default_argon2_p_cost")]
    pub p_cost: u32,
}

fn default_argon2_m_cost() -> u32 {
    19 * 1024
}

fn default_argon2_t_cost() -> u32 {
    2
}

fn default_argon2_p_cost() -> u32 {
    1
}

impl Default for KeyDerivationParams {
    fn default() -> Self {
        Self {
            m_cost: default_argon2_m_cost(),
            t_cost: default_argon2_t_cost(),
            p_cost: default_argon2_p_cost(),
        }
    }
}

impl Display for KeyDerivationParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KeyDerivationParams {{ m_cost: {}, t_cost: {}, p_cost: {} }}",
            self.m_cost, self.t_cost, self.p_cost
        )
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum EncryptionKeySource {
    #[default]
    Passphrase,
    File {
        path: String,
    },
    HttpEndpoint {
        url: String,
    },
}

impl Display for EncryptionKeySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionKeySource::Passphrase => write!(f, "passphrase"),
            EncryptionKeySource::File { path } => write!(f, "file:{}", path),
            EncryptionKeySource::HttpEndpoint { url } => write!(f, "http:{}", url),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BackupEncryptionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub key_source: EncryptionKeySource,
    #[serde(default)]
    pub key_derivation: KeyDerivationParams,
    #[serde(default)]
    pub key_identifier: Option<String>,
}

impl Default for BackupEncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        }
    }
}

impl Display for BackupEncryptionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BackupEncryptionConfig {{ enabled: {}, key_source: {}, key_derivation: {} }}",
            self.enabled, self.key_source, self.key_derivation
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupEncryptionHeader {
    pub magic: String,
    pub key_identifier: String,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_derivation: KeyDerivationParams,
    pub compressed: bool,
}

impl BackupEncryptionHeader {
    pub fn new(
        key_identifier: String,
        salt: Vec<u8>,
        nonce: Vec<u8>,
        key_derivation: KeyDerivationParams,
        compressed: bool,
    ) -> Self {
        Self {
            magic: String::from_utf8_lossy(BACKUP_ENCRYPTION_MAGIC).to_string(),
            key_identifier,
            salt,
            nonce,
            key_derivation,
            compressed,
        }
    }

    pub fn validate_magic(&self) -> bool {
        self.magic.as_bytes() == BACKUP_ENCRYPTION_MAGIC
    }
}

impl Display for BackupEncryptionHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BackupEncryptionHeader {{ key_identifier: {}, compressed: {} }}",
            self.key_identifier, self.compressed
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct S3BackupMetadata {
    pub checksum_sha256: String,
    pub timestamp: String,
    pub compression: BackupCompression,
    pub size_bytes: u64,
    #[serde(default)]
    pub encrypted: bool,
    #[serde(default)]
    pub key_identifier: Option<String>,
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
            encrypted: false,
            key_identifier: None,
        }
    }

    pub fn new_encrypted(
        checksum_sha256: String,
        timestamp: String,
        compression: BackupCompression,
        size_bytes: u64,
        key_identifier: String,
    ) -> Self {
        Self {
            checksum_sha256,
            timestamp,
            compression,
            size_bytes,
            encrypted: true,
            key_identifier: Some(key_identifier),
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
        let timestamp = base_backup_timestamp.clone();
        Self {
            server_uuid,
            segments: Vec::new(),
            base_backup_id,
            base_backup_timestamp,
            earliest_recoverable_time: timestamp.clone(),
            latest_recoverable_time: timestamp,
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

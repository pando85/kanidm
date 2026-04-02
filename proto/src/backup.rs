//! Relates to backup functionality in the Server
use std::{fmt::Display, path::Path, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_with::DeserializeFromStr;
use sketching::tracing::warn;

pub const BACKUP_ENCRYPTION_MAGIC: &[u8] = b"KANIDM_ENC_BACKUP_V1";
pub const BACKUP_ENCRYPTION_KEY_LEN: usize = 32;
pub const BACKUP_ENCRYPTION_NONCE_LEN: usize = 12;
pub const BACKUP_ENCRYPTION_SALT_LEN: usize = 16;

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

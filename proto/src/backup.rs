//! Relates to backup functionality in the Server
use std::{fmt::Display, path::Path, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_with::DeserializeFromStr;
use sketching::tracing::warn;

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

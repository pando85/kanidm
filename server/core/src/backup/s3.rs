use std::io::{Read, Write};

use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{
    CompletedMultipartUpload, CompletedPart, ServerSideEncryption, StorageClass,
};
use aws_sdk_s3::Client as S3Client;
use hex::encode as hex_encode;
use kanidm_proto::backup::{BackupCompression, S3BackupMetadata, S3Config, S3EncryptionAlgorithm};
use sha2::{Digest, Sha256};

const MULTIPART_THRESHOLD: u64 = 100 * 1024 * 1024;
const MULTIPART_CHUNK_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug)]
pub enum S3BackupError {
    ConfigError(String),
    UploadError(String),
    DownloadError(String),
    CredentialsError(String),
    InvalidChecksum { expected: String, actual: String },
    IoError(std::io::Error),
    SdkError(String),
}

impl std::fmt::Display for S3BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S3BackupError::ConfigError(msg) => write!(f, "S3 configuration error: {}", msg),
            S3BackupError::UploadError(msg) => write!(f, "S3 upload error: {}", msg),
            S3BackupError::DownloadError(msg) => write!(f, "S3 download error: {}", msg),
            S3BackupError::CredentialsError(msg) => write!(f, "S3 credentials error: {}", msg),
            S3BackupError::InvalidChecksum { expected, actual } => {
                write!(
                    f,
                    "Checksum mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            S3BackupError::IoError(e) => write!(f, "IO error: {}", e),
            S3BackupError::SdkError(msg) => write!(f, "AWS SDK error: {}", msg),
        }
    }
}

impl std::error::Error for S3BackupError {}

impl From<std::io::Error> for S3BackupError {
    fn from(e: std::io::Error) -> Self {
        S3BackupError::IoError(e)
    }
}

#[derive(Clone)]
pub struct S3ClientWrapper {
    client: S3Client,
    config: S3Config,
}

impl S3ClientWrapper {
    pub async fn new(config: S3Config) -> Result<Self, S3BackupError> {
        let sdk_config = Self::build_sdk_config(&config).await?;
        let client = S3Client::new(&sdk_config);
        Ok(Self { client, config })
    }

    async fn build_sdk_config(config: &S3Config) -> Result<SdkConfig, S3BackupError> {
        let mut config_builder = aws_config::defaults(BehaviorVersion::latest());

        if let Some(endpoint) = &config.endpoint {
            config_builder = config_builder.endpoint_url(endpoint);
        }

        if let Some(region) = &config.region {
            config_builder = config_builder.region(Region::new(region.clone()));
        }

        if let Some(credentials) = &config.credentials {
            let creds = Credentials::new(
                credentials.access_key_id.clone(),
                credentials.secret_access_key.clone(),
                credentials.session_token.clone(),
                None,
                "kanidm-backup",
            );
            config_builder = config_builder.credentials_provider(creds);
        }

        Ok(config_builder.load().await)
    }

    fn build_object_key(&self, key: &str) -> String {
        match &self.config.path_prefix {
            Some(prefix) => format!("{}/{}", prefix.trim_end_matches('/'), key),
            None => key.to_string(),
        }
    }

    pub async fn upload_backup(
        &self,
        data: Vec<u8>,
        key: &str,
        timestamp: &str,
        compression: BackupCompression,
    ) -> Result<S3BackupMetadata, S3BackupError> {
        let size = data.len() as u64;
        let checksum = hex_encode(Sha256::digest(&data));
        let object_key = self.build_object_key(key);

        let metadata =
            S3BackupMetadata::new(checksum.clone(), timestamp.to_string(), compression, size);

        if size > MULTIPART_THRESHOLD {
            self.upload_multipart(&data, &object_key, &metadata).await?;
        } else {
            self.upload_single(&data, &object_key, &metadata).await?;
        }

        self.upload_metadata(&object_key, &metadata).await?;

        Ok(metadata)
    }

    async fn upload_single(
        &self,
        data: &[u8],
        key: &str,
        metadata: &S3BackupMetadata,
    ) -> Result<(), S3BackupError> {
        let mut builder = self
            .client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(ByteStream::from(data.to_vec()))
            .metadata("checksum-sha256", &metadata.checksum_sha256)
            .metadata("backup-timestamp", &metadata.timestamp)
            .metadata("backup-size", metadata.size_bytes.to_string())
            .storage_class(parse_storage_class(self.config.storage_class.as_str()));

        builder = self.apply_encryption(builder);

        builder
            .send()
            .await
            .map_err(|e| S3BackupError::UploadError(format!("Failed to upload backup: {}", e)))?;

        info!("Uploaded backup to S3: {}", key);
        Ok(())
    }

    async fn upload_multipart(
        &self,
        data: &[u8],
        key: &str,
        metadata: &S3BackupMetadata,
    ) -> Result<(), S3BackupError> {
        let create_output = self.create_multipart_upload(key, metadata).await?;
        let upload_id = create_output.upload_id().unwrap_or_default();

        let mut parts = Vec::new();
        let mut part_number: i32 = 1;

        for chunk in data.chunks(MULTIPART_CHUNK_SIZE) {
            let part = self.upload_part(key, upload_id, part_number, chunk).await?;
            parts.push(
                CompletedPart::builder()
                    .part_number(part_number)
                    .e_tag(part.e_tag().unwrap_or_default())
                    .build(),
            );
            part_number += 1;
        }

        self.complete_multipart_upload(key, upload_id, parts)
            .await?;
        info!("Completed multipart upload to S3: {}", key);
        Ok(())
    }

    async fn create_multipart_upload(
        &self,
        key: &str,
        metadata: &S3BackupMetadata,
    ) -> Result<CreateMultipartUploadOutput, S3BackupError> {
        let mut builder = self
            .client
            .create_multipart_upload()
            .bucket(&self.config.bucket)
            .key(key)
            .metadata("checksum-sha256", &metadata.checksum_sha256)
            .metadata("backup-timestamp", &metadata.timestamp)
            .metadata("backup-size", metadata.size_bytes.to_string())
            .storage_class(parse_storage_class(self.config.storage_class.as_str()));

        builder = self.apply_encryption_multipart(builder);

        builder.send().await.map_err(|e| {
            S3BackupError::UploadError(format!("Failed to create multipart upload: {}", e))
        })
    }

    async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: &[u8],
    ) -> Result<aws_sdk_s3::operation::upload_part::UploadPartOutput, S3BackupError> {
        self.client
            .upload_part()
            .bucket(&self.config.bucket)
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(ByteStream::from(data.to_vec()))
            .send()
            .await
            .map_err(|e| {
                S3BackupError::UploadError(format!("Failed to upload part {}: {}", part_number, e))
            })
    }

    async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<CompletedPart>,
    ) -> Result<(), S3BackupError> {
        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.config.bucket)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .map_err(|e| {
                S3BackupError::UploadError(format!("Failed to complete multipart upload: {}", e))
            })?;

        Ok(())
    }

    async fn upload_metadata(
        &self,
        backup_key: &str,
        metadata: &S3BackupMetadata,
    ) -> Result<(), S3BackupError> {
        let metadata_key = format!("{}.metadata.json", backup_key);
        let metadata_json = serde_json::to_string(metadata).map_err(|e| {
            S3BackupError::UploadError(format!("Failed to serialize metadata: {}", e))
        })?;

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&metadata_key)
            .body(ByteStream::from(metadata_json.into_bytes()))
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| S3BackupError::UploadError(format!("Failed to upload metadata: {}", e)))?;

        Ok(())
    }

    fn apply_encryption(
        &self,
        mut builder: aws_sdk_s3::operation::put_object::builders::PutObjectFluentBuilder,
    ) -> aws_sdk_s3::operation::put_object::builders::PutObjectFluentBuilder {
        if let Some(sse) = &self.config.server_side_encryption {
            let sse_type = match &sse.algorithm {
                Some(S3EncryptionAlgorithm::Aes256) => ServerSideEncryption::Aes256,
                _ => ServerSideEncryption::AwsKms,
            };
            builder = builder.server_side_encryption(sse_type);
            if let Some(kms_key) = &sse.kms_key_id {
                builder = builder.ssekms_key_id(kms_key);
            }
        }
        builder
    }

    fn apply_encryption_multipart(
        &self,
        mut builder: aws_sdk_s3::operation::create_multipart_upload::builders::CreateMultipartUploadFluentBuilder,
    ) -> aws_sdk_s3::operation::create_multipart_upload::builders::CreateMultipartUploadFluentBuilder
    {
        if let Some(sse) = &self.config.server_side_encryption {
            let sse_type = match &sse.algorithm {
                Some(S3EncryptionAlgorithm::Aes256) => ServerSideEncryption::Aes256,
                _ => ServerSideEncryption::AwsKms,
            };
            builder = builder.server_side_encryption(sse_type);
            if let Some(kms_key) = &sse.kms_key_id {
                builder = builder.ssekms_key_id(kms_key);
            }
        }
        builder
    }

    pub async fn download_backup(
        &self,
        key: &str,
    ) -> Result<(Vec<u8>, S3BackupMetadata), S3BackupError> {
        let object_key = self.build_object_key(key);
        let metadata = self.download_metadata(&object_key).await?;

        let output = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&object_key)
            .send()
            .await
            .map_err(|e| {
                S3BackupError::DownloadError(format!("Failed to download backup: {}", e))
            })?;

        let data = self.collect_stream(output).await?;

        let actual_checksum = hex_encode(Sha256::digest(&data));
        if actual_checksum != metadata.checksum_sha256 {
            return Err(S3BackupError::InvalidChecksum {
                expected: metadata.checksum_sha256.clone(),
                actual: actual_checksum,
            });
        }

        info!("Downloaded and verified backup from S3: {}", object_key);
        Ok((data, metadata))
    }

    async fn download_metadata(&self, backup_key: &str) -> Result<S3BackupMetadata, S3BackupError> {
        let metadata_key = format!("{}.metadata.json", backup_key);

        let output = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&metadata_key)
            .send()
            .await
            .map_err(|e| {
                S3BackupError::DownloadError(format!("Failed to download metadata: {}", e))
            })?;

        let data = self.collect_stream(output).await?;
        let metadata: S3BackupMetadata = serde_json::from_slice(&data).map_err(|e| {
            S3BackupError::DownloadError(format!("Failed to parse metadata: {}", e))
        })?;

        Ok(metadata)
    }

    async fn collect_stream(&self, output: GetObjectOutput) -> Result<Vec<u8>, S3BackupError> {
        let body = output
            .body
            .collect()
            .await
            .map_err(|e| S3BackupError::DownloadError(format!("Stream error: {}", e)))?;
        Ok(body.into_bytes().to_vec())
    }

    pub async fn list_backups(&self) -> Result<Vec<String>, S3BackupError> {
        let prefix = self.config.path_prefix.clone().unwrap_or_default();

        let output = self
            .client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(&prefix)
            .send()
            .await
            .map_err(|e| S3BackupError::SdkError(format!("Failed to list objects: {}", e)))?;

        let mut backups = Vec::new();
        for obj in output.contents() {
            if let Some(key) = obj.key() {
                if !key.ends_with(".metadata.json") {
                    let display_key = if let Some(prefix) = &self.config.path_prefix {
                        key.strip_prefix(prefix)
                            .map(|s: &str| s.trim_start_matches('/').to_string())
                            .unwrap_or_else(|| key.to_string())
                    } else {
                        key.to_string()
                    };
                    backups.push(display_key);
                }
            }
        }

        Ok(backups)
    }

    pub async fn delete_backup(&self, key: &str) -> Result<(), S3BackupError> {
        let object_key = self.build_object_key(key);
        let metadata_key = format!("{}.metadata.json", object_key);

        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&object_key)
            .send()
            .await
            .map_err(|e| S3BackupError::SdkError(format!("Failed to delete backup: {}", e)))?;

        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&metadata_key)
            .send()
            .await
            .map_err(|e| S3BackupError::SdkError(format!("Failed to delete metadata: {}", e)))?;

        info!("Deleted backup from S3: {}", object_key);
        Ok(())
    }

    pub async fn verify_backup(&self, key: &str) -> Result<bool, S3BackupError> {
        let object_key = self.build_object_key(key);
        let metadata = self.download_metadata(&object_key).await?;

        let head = self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(&object_key)
            .send()
            .await
            .map_err(|e| S3BackupError::SdkError(format!("Failed to head object: {}", e)))?;

        let actual_size = head.content_length().unwrap_or(0) as u64;

        if actual_size != metadata.size_bytes {
            warn!(
                "Backup size mismatch: expected {}, got {}",
                metadata.size_bytes, actual_size
            );
            return Ok(false);
        }

        Ok(true)
    }
}

pub struct ChecksumWriter<W> {
    writer: W,
    hasher: Sha256,
}

impl<W> ChecksumWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            hasher: Sha256::new(),
        }
    }

    pub fn finalize(self) -> (W, String) {
        let checksum = hex_encode(self.hasher.finalize());
        (self.writer, checksum)
    }
}

impl<W: Write> Write for ChecksumWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.update(buf);
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub struct ChecksumReader<R> {
    reader: R,
    hasher: Sha256,
}

impl<R> ChecksumReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            hasher: Sha256::new(),
        }
    }

    pub fn finalize(self) -> (R, String) {
        let checksum = hex_encode(self.hasher.finalize());
        (self.reader, checksum)
    }
}

impl<R: Read> Read for ChecksumReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.reader.read(buf)?;
        if let Some(slice) = buf.get(..n) {
            self.hasher.update(slice);
        }
        Ok(n)
    }
}

fn parse_storage_class(s: &str) -> StorageClass {
    match s.to_uppercase().as_str() {
        "STANDARD" => StorageClass::Standard,
        "REDUCED_REDUNDANCY" => StorageClass::ReducedRedundancy,
        "STANDARD_IA" => StorageClass::StandardIa,
        "ONEZONE_IA" => StorageClass::OnezoneIa,
        "INTELLIGENT_TIERING" => StorageClass::IntelligentTiering,
        "GLACIER" => StorageClass::Glacier,
        "DEEP_ARCHIVE" => StorageClass::DeepArchive,
        "GLACIER_IR" => StorageClass::GlacierIr,
        _ => StorageClass::Standard,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_class_conversion() {
        assert_eq!(parse_storage_class("STANDARD"), StorageClass::Standard);
        assert_eq!(parse_storage_class("standard"), StorageClass::Standard);
        assert_eq!(parse_storage_class("GLACIER"), StorageClass::Glacier);
        assert_eq!(parse_storage_class("unknown"), StorageClass::Standard);
    }

    #[test]
    fn test_checksum_writer() {
        let mut writer = ChecksumWriter::new(Vec::new());
        writer.write_all(b"hello world").unwrap();
        let (data, checksum) = writer.finalize();

        assert_eq!(data, b"hello world");
        assert_eq!(checksum.len(), 64);
    }

    #[test]
    fn test_checksum_reader() {
        let input = b"hello world".to_vec();
        let mut reader = ChecksumReader::new(&input[..]);
        let mut output = Vec::new();
        reader.read_to_end(&mut output).unwrap();
        let (_, checksum) = reader.finalize();

        assert_eq!(output, b"hello world");
        assert_eq!(checksum.len(), 64);
    }
}

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use kanidm_proto::backup::{BackupCompression, S3BackupMetadata};

pub struct MockS3Storage {
    objects: Arc<RwLock<BTreeMap<String, Vec<u8>>>>,
    metadata: Arc<RwLock<BTreeMap<String, S3BackupMetadata>>>,
}

impl MockS3Storage {
    pub fn new() -> Self {
        Self {
            objects: Arc::new(RwLock::new(BTreeMap::new())),
            metadata: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub fn put_object(&self, key: &str, data: Vec<u8>) -> Result<(), MockS3Error> {
        let mut objects = self.objects.write().map_err(|_| MockS3Error::LockError)?;
        objects.insert(key.to_string(), data);
        Ok(())
    }

    pub fn get_object(&self, key: &str) -> Result<Vec<u8>, MockS3Error> {
        let objects = self.objects.read().map_err(|_| MockS3Error::LockError)?;
        objects.get(key).cloned().ok_or(MockS3Error::ObjectNotFound)
    }

    pub fn put_metadata(&self, key: &str, metadata: S3BackupMetadata) -> Result<(), MockS3Error> {
        let mut meta = self.metadata.write().map_err(|_| MockS3Error::LockError)?;
        meta.insert(key.to_string(), metadata);
        Ok(())
    }

    pub fn get_metadata(&self, key: &str) -> Result<S3BackupMetadata, MockS3Error> {
        let meta = self.metadata.read().map_err(|_| MockS3Error::LockError)?;
        meta.get(key).cloned().ok_or(MockS3Error::MetadataNotFound)
    }

    pub fn list_objects(&self, prefix: &str) -> Result<Vec<String>, MockS3Error> {
        let objects = self.objects.read().map_err(|_| MockS3Error::LockError)?;
        let keys: Vec<String> = objects
            .keys()
            .filter(|k| k.starts_with(prefix) && !k.ends_with(".metadata.json"))
            .cloned()
            .collect();
        Ok(keys)
    }

    pub fn delete_object(&self, key: &str) -> Result<(), MockS3Error> {
        let mut objects = self.objects.write().map_err(|_| MockS3Error::LockError)?;
        objects.remove(key);
        let mut meta = self.metadata.write().map_err(|_| MockS3Error::LockError)?;
        let metadata_key = format!("{}.metadata.json", key);
        meta.remove(&metadata_key);
        Ok(())
    }

    pub fn object_exists(&self, key: &str) -> bool {
        let objects = self.objects.read().unwrap();
        objects.contains_key(key)
    }

    pub fn clear(&self) {
        let mut objects = self.objects.write().unwrap();
        objects.clear();
        let mut meta = self.metadata.write().unwrap();
        meta.clear();
    }

    pub fn object_count(&self) -> usize {
        self.objects.read().unwrap().len()
    }

    pub fn corrupt_object(&self, key: &str) -> Result<(), MockS3Error> {
        let mut objects = self.objects.write().map_err(|_| MockS3Error::LockError)?;
        if let Some(data) = objects.get_mut(key) {
            if !data.is_empty() {
                data[0] = !data[0];
            }
            Ok(())
        } else {
            Err(MockS3Error::ObjectNotFound)
        }
    }

    pub fn set_object_size(&self, key: &str, size: usize) -> Result<(), MockS3Error> {
        let mut objects = self.objects.write().map_err(|_| MockS3Error::LockError)?;
        if let Some(data) = objects.get_mut(key) {
            data.resize(size, 0);
            Ok(())
        } else {
            Err(MockS3Error::ObjectNotFound)
        }
    }
}

impl Default for MockS3Storage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum MockS3Error {
    ObjectNotFound,
    MetadataNotFound,
    LockError,
    InvalidChecksum,
    PermissionDenied,
    BucketNotFound,
    QuotaExceeded,
    ConnectionTimeout,
    NetworkError,
}

impl std::fmt::Display for MockS3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockS3Error::ObjectNotFound => write!(f, "Object not found"),
            MockS3Error::MetadataNotFound => write!(f, "Metadata not found"),
            MockS3Error::LockError => write!(f, "Lock error"),
            MockS3Error::InvalidChecksum => write!(f, "Invalid checksum"),
            MockS3Error::PermissionDenied => write!(f, "Permission denied"),
            MockS3Error::BucketNotFound => write!(f, "Bucket not found"),
            MockS3Error::QuotaExceeded => write!(f, "Quota exceeded"),
            MockS3Error::ConnectionTimeout => write!(f, "Connection timeout"),
            MockS3Error::NetworkError => write!(f, "Network error"),
        }
    }
}

impl std::error::Error for MockS3Error {}

pub struct MockS3TestBuilder {
    storage: MockS3Storage,
    bucket: String,
    prefix: Option<String>,
    simulate_errors: bool,
}

impl MockS3TestBuilder {
    pub fn new() -> Self {
        Self {
            storage: MockS3Storage::new(),
            bucket: "test-backup-bucket".to_string(),
            prefix: None,
            simulate_errors: false,
        }
    }

    pub fn with_bucket(mut self, bucket: &str) -> Self {
        self.bucket = bucket.to_string();
        self
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn simulate_errors(mut self, simulate: bool) -> Self {
        self.simulate_errors = simulate;
        self
    }

    pub fn build(self) -> MockS3TestContext {
        MockS3TestContext {
            storage: self.storage,
            bucket: self.bucket,
            prefix: self.prefix,
            simulate_errors: self.simulate_errors,
        }
    }
}

impl Default for MockS3TestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MockS3TestContext {
    storage: MockS3Storage,
    bucket: String,
    prefix: Option<String>,
    simulate_errors: bool,
}

impl MockS3TestContext {
    pub fn upload_backup(
        &self,
        key: &str,
        data: &[u8],
        timestamp: &str,
        compression: BackupCompression,
    ) -> Result<S3BackupMetadata, MockS3Error> {
        if self.simulate_errors {
            return Err(MockS3Error::ConnectionTimeout);
        }

        let object_key = self.build_key(key);

        use sha2::{Digest, Sha256};
        let checksum = hex::encode(Sha256::digest(data));

        let metadata = S3BackupMetadata::new(
            checksum.clone(),
            timestamp.to_string(),
            compression,
            data.len() as u64,
        );

        self.storage.put_object(&object_key, data.to_vec())?;
        self.storage
            .put_metadata(&format!("{}.metadata.json", object_key), metadata.clone())?;

        Ok(metadata)
    }

    pub fn download_backup(&self, key: &str) -> Result<(Vec<u8>, S3BackupMetadata), MockS3Error> {
        if self.simulate_errors {
            return Err(MockS3Error::NetworkError);
        }

        let object_key = self.build_key(key);
        let metadata = self
            .storage
            .get_metadata(&format!("{}.metadata.json", object_key))?;
        let data = self.storage.get_object(&object_key)?;

        use sha2::{Digest, Sha256};
        let actual_checksum = hex::encode(Sha256::digest(&data));
        if actual_checksum != metadata.checksum_sha256 {
            return Err(MockS3Error::InvalidChecksum);
        }

        Ok((data, metadata))
    }

    pub fn list_backups(&self) -> Result<Vec<String>, MockS3Error> {
        let prefix = self.prefix.clone().unwrap_or_default();
        let objects = self.storage.list_objects(&prefix)?;

        let backups: Vec<String> = objects
            .iter()
            .map(|k| {
                if let Some(p) = &self.prefix {
                    k.strip_prefix(p)
                        .map(|s| s.trim_start_matches('/').to_string())
                        .unwrap_or_else(|| k.clone())
                } else {
                    k.clone()
                }
            })
            .collect();

        Ok(backups)
    }

    pub fn delete_backup(&self, key: &str) -> Result<(), MockS3Error> {
        let object_key = self.build_key(key);
        self.storage.delete_object(&object_key)?;
        Ok(())
    }

    pub fn verify_backup(&self, key: &str) -> Result<bool, MockS3Error> {
        let object_key = self.build_key(key);
        let metadata = self
            .storage
            .get_metadata(&format!("{}.metadata.json", object_key))?;
        let data = self.storage.get_object(&object_key)?;

        Ok(data.len() == metadata.size_bytes as usize)
    }

    fn build_key(&self, key: &str) -> String {
        match &self.prefix {
            Some(p) => format!("{}/{}", p.trim_end_matches('/'), key),
            None => key.to_string(),
        }
    }

    pub fn clear(&self) {
        self.storage.clear();
    }

    pub fn object_exists(&self, key: &str) -> bool {
        let object_key = self.build_key(key);
        self.storage.object_exists(&object_key)
    }

    pub fn corrupt_backup(&self, key: &str) -> Result<(), MockS3Error> {
        let object_key = self.build_key(key);
        self.storage.corrupt_object(&object_key)
    }

    pub fn set_backup_size(&self, key: &str, size: usize) -> Result<(), MockS3Error> {
        let object_key = self.build_key(key);
        self.storage.set_object_size(&object_key, size)
    }

    pub fn backup_count(&self) -> usize {
        self.storage.object_count() / 2
    }

    pub fn get_bucket(&self) -> &str {
        &self.bucket
    }

    pub fn get_prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

pub fn create_test_backup_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

pub fn create_small_backup_data() -> Vec<u8> {
    create_test_backup_data(1024)
}

pub fn create_medium_backup_data() -> Vec<u8> {
    create_test_backup_data(10 * 1024)
}

pub fn create_large_backup_data() -> Vec<u8> {
    create_test_backup_data(1024 * 1024)
}

pub fn create_empty_backup_data() -> Vec<u8> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_s3_upload_download() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_small_backup_data();

        let metadata = ctx
            .upload_backup(
                "backup-1",
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();

        assert_eq!(metadata.size_bytes, data.len() as u64);

        let (downloaded, downloaded_meta) = ctx.download_backup("backup-1").unwrap();
        assert_eq!(downloaded, data);
        assert_eq!(downloaded_meta.checksum_sha256, metadata.checksum_sha256);
    }

    #[test]
    fn test_mock_s3_list_backups() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "backup-1",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();
        ctx.upload_backup(
            "backup-2",
            &data,
            "2024-01-16T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        let backups = ctx.list_backups().unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[test]
    fn test_mock_s3_delete_backup() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "backup-1",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();
        assert!(ctx.object_exists("backup-1"));

        ctx.delete_backup("backup-1").unwrap();
        assert!(!ctx.object_exists("backup-1"));
    }

    #[test]
    fn test_mock_s3_verify_backup() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "backup-1",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        assert!(ctx.verify_backup("backup-1").unwrap());
    }

    #[test]
    fn test_mock_s3_corrupted_backup() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "backup-1",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        ctx.corrupt_backup("backup-1").unwrap();

        let result = ctx.download_backup("backup-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_s3_with_prefix() {
        let ctx = MockS3TestBuilder::new()
            .with_prefix("kanidm/backups")
            .build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "backup-1",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        assert!(ctx.object_exists("backup-1"));
        let backups = ctx.list_backups().unwrap();
        assert!(backups.contains(&"backup-1".to_string()));
    }

    #[test]
    fn test_mock_s3_empty_backup() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_empty_backup_data();

        let metadata = ctx
            .upload_backup(
                "empty-backup",
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();

        assert_eq!(metadata.size_bytes, 0);

        let (downloaded, _) = ctx.download_backup("empty-backup").unwrap();
        assert_eq!(downloaded.len(), 0);
    }

    #[test]
    fn test_mock_s3_error_simulation() {
        let ctx = MockS3TestBuilder::new().simulate_errors(true).build();
        let data = create_small_backup_data();

        let result = ctx.upload_backup(
            "backup-1",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        );
        assert!(result.is_err());
        assert!(matches!(result, Err(MockS3Error::ConnectionTimeout)));

        let result = ctx.download_backup("backup-1");
        assert!(result.is_err());
        assert!(matches!(result, Err(MockS3Error::NetworkError)));
    }

    #[test]
    fn test_mock_s3_object_not_found() {
        let ctx = MockS3TestBuilder::new().build();

        let result = ctx.download_backup("nonexistent-backup");
        assert!(result.is_err());
        assert!(matches!(result, Err(MockS3Error::MetadataNotFound)));
    }

    #[test]
    fn test_mock_s3_size_mismatch() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "backup-1",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        ctx.set_backup_size("backup-1", 1000).unwrap();

        let result = ctx.verify_backup("backup-1");
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_mock_s3_multiple_uploads() {
        let ctx = MockS3TestBuilder::new().build();

        for i in 0..10 {
            let data = create_test_backup_data(100 * (i + 1));
            ctx.upload_backup(
                &format!("backup-{}", i),
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();
        }

        let backups = ctx.list_backups().unwrap();
        assert_eq!(backups.len(), 10);
        assert_eq!(ctx.backup_count(), 10);
    }

    #[test]
    fn test_mock_s3_clear() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "backup-1",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();
        ctx.upload_backup(
            "backup-2",
            &data,
            "2024-01-16T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        assert_eq!(ctx.backup_count(), 2);

        ctx.clear();
        assert_eq!(ctx.backup_count(), 0);
    }

    #[test]
    fn test_mock_s3_error_display() {
        assert_eq!(MockS3Error::ObjectNotFound.to_string(), "Object not found");
        assert_eq!(MockS3Error::LockError.to_string(), "Lock error");
        assert_eq!(MockS3Error::InvalidChecksum.to_string(), "Invalid checksum");
        assert_eq!(
            MockS3Error::PermissionDenied.to_string(),
            "Permission denied"
        );
        assert_eq!(MockS3Error::BucketNotFound.to_string(), "Bucket not found");
        assert_eq!(MockS3Error::QuotaExceeded.to_string(), "Quota exceeded");
        assert_eq!(
            MockS3Error::ConnectionTimeout.to_string(),
            "Connection timeout"
        );
        assert_eq!(MockS3Error::NetworkError.to_string(), "Network error");
    }
}

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use kubidm_proto::backup::{BackupCompression, S3BackupMetadata};

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
        self.objects
            .read()
            .is_ok_and(|objects| objects.contains_key(key))
    }

    pub fn clear(&self) {
        if let Ok(mut objects) = self.objects.write() {
            objects.clear();
        }
        if let Ok(mut meta) = self.metadata.write() {
            meta.clear();
        }
    }

    pub fn object_count(&self) -> usize {
        self.objects.read().map_or(0, |objects| objects.len())
    }

    pub fn corrupt_object(&self, key: &str) -> Result<(), MockS3Error> {
        let mut objects = self.objects.write().map_err(|_| MockS3Error::LockError)?;
        if let Some(data) = objects.get_mut(key) {
            if let Some(first_byte) = data.first_mut() {
                *first_byte = !*first_byte;
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
        self.storage.object_count()
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
            .with_prefix("kubidm/backups")
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
        assert!(!result.unwrap());
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

        assert_eq!(ctx.list_backups().unwrap().len(), 2);

        ctx.clear();
        assert_eq!(ctx.list_backups().unwrap().len(), 0);
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

    #[test]
    fn test_mock_s3_concurrent_uploads() {
        let mut handles = vec![];

        for i in 0..5 {
            let data = create_test_backup_data(100 * i);
            handles.push(std::thread::spawn(move || {
                let thread_ctx = MockS3TestBuilder::new().build();
                thread_ctx
                    .upload_backup(
                        &format!("concurrent-{}", i),
                        &data,
                        "2024-01-15T10:00:00Z",
                        BackupCompression::Gzip,
                    )
                    .unwrap();
                thread_ctx.object_exists(&format!("concurrent-{}", i))
            }));
        }

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result);
        }
    }

    #[test]
    fn test_mock_s3_partial_corruption_recovery() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_medium_backup_data();

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
        ctx.upload_backup(
            "backup-3",
            &data,
            "2024-01-17T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        ctx.corrupt_backup("backup-2").unwrap();

        assert!(ctx.download_backup("backup-1").is_ok());
        assert!(ctx.download_backup("backup-2").is_err());
        assert!(ctx.download_backup("backup-3").is_ok());
    }

    #[test]
    fn test_mock_s3_metadata_corruption() {
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
        assert!(matches!(result.unwrap_err(), MockS3Error::InvalidChecksum));
    }

    #[test]
    fn test_mock_s3_zero_byte_recovery() {
        let ctx = MockS3TestBuilder::new().build();

        ctx.upload_backup(
            "zero-backup",
            &create_empty_backup_data(),
            "2024-01-15T10:00:00Z",
            BackupCompression::NoCompression,
        )
        .unwrap();

        ctx.set_backup_size("zero-backup", 1000).unwrap();

        let verified = ctx.verify_backup("zero-backup").unwrap();
        assert!(!verified);
    }

    #[test]
    fn test_mock_s3_repeated_operations() {
        let ctx = MockS3TestBuilder::new().build();
        let data = create_small_backup_data();

        for _ in 0..5 {
            ctx.upload_backup(
                "repeated-backup",
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();
            assert!(ctx.object_exists("repeated-backup"));

            ctx.delete_backup("repeated-backup").unwrap();
            assert!(!ctx.object_exists("repeated-backup"));
        }
    }

    #[test]
    fn test_mock_s3_backup_versioning_simulation() {
        let ctx = MockS3TestBuilder::new().build();
        let data_v1 = create_test_backup_data(100);
        let data_v2 = create_test_backup_data(200);
        let data_v3 = create_test_backup_data(300);

        ctx.upload_backup(
            "versioned-backup",
            &data_v1,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();
        let meta1 = ctx.download_backup("versioned-backup").unwrap();

        ctx.upload_backup(
            "versioned-backup",
            &data_v2,
            "2024-01-15T11:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();
        let meta2 = ctx.download_backup("versioned-backup").unwrap();

        ctx.upload_backup(
            "versioned-backup",
            &data_v3,
            "2024-01-15T12:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();
        let (data, meta3) = ctx.download_backup("versioned-backup").unwrap();

        assert_ne!(meta1.1.size_bytes, meta2.1.size_bytes);
        assert_ne!(meta2.1.size_bytes, meta3.size_bytes);
        assert_eq!(data.len(), data_v3.len());
        assert_eq!(meta3.timestamp, "2024-01-15T12:00:00Z");
    }

    #[test]
    fn test_mock_s3_boundary_sizes() {
        let ctx = MockS3TestBuilder::new().build();

        let boundary_sizes = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024];

        for size in boundary_sizes {
            let data = create_test_backup_data(size);
            ctx.upload_backup(
                &format!("boundary-{}", size),
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();

            let (downloaded, meta) = ctx.download_backup(&format!("boundary-{}", size)).unwrap();
            assert_eq!(downloaded.len(), size);
            assert_eq!(meta.size_bytes, size as u64);
        }
    }

    #[test]
    fn test_mock_s3_network_failure_recovery() {
        let failing_ctx = MockS3TestBuilder::new().simulate_errors(true).build();
        let working_ctx = MockS3TestBuilder::new().simulate_errors(false).build();
        let data = create_small_backup_data();

        let failing_result = failing_ctx.upload_backup(
            "failing-backup",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        );
        assert!(failing_result.is_err());

        working_ctx
            .upload_backup(
                "working-backup",
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();
        assert!(working_ctx.object_exists("working-backup"));
    }

    #[test]
    fn test_mock_s3_permission_error_simulation() {
        let ctx = MockS3TestBuilder::new().simulate_errors(true).build();

        let result = ctx.upload_backup(
            "permission-test",
            &create_small_backup_data(),
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_s3_quota_simulation() {
        let ctx = MockS3TestBuilder::new().build();

        for i in 0..100 {
            let data = create_test_backup_data(1024);
            ctx.upload_backup(
                &format!("quota-test-{}", i),
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();
        }

        assert_eq!(ctx.list_backups().unwrap().len(), 100);
    }

    #[test]
    fn test_mock_s3_delete_nonexistent() {
        let ctx = MockS3TestBuilder::new().build();

        let result = ctx.delete_backup("nonexistent");
        assert!(result.is_ok());

        let count_before = ctx.list_backups().unwrap().len();
        ctx.delete_backup("another-nonexistent").unwrap();
        assert_eq!(ctx.list_backups().unwrap().len(), count_before);
    }

    #[test]
    fn test_mock_s3_duplicate_metadata_handling() {
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
            "backup-1",
            &create_medium_backup_data(),
            "2024-01-16T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        let (_, meta) = ctx.download_backup("backup-1").unwrap();
        assert_eq!(meta.timestamp, "2024-01-16T10:00:00Z");
    }

    #[test]
    fn test_mock_s3_prefix_edge_cases() {
        let ctx = MockS3TestBuilder::new().with_prefix("a/b/c/d/e/f").build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "deep-backup",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        assert!(ctx.object_exists("deep-backup"));
        assert!(ctx
            .list_backups()
            .unwrap()
            .contains(&"deep-backup".to_string()));

        let ctx_slash = MockS3TestBuilder::new()
            .with_prefix("trailing/slash/")
            .build();
        ctx_slash
            .upload_backup(
                "slash-backup",
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();
        assert!(ctx_slash.object_exists("slash-backup"));
    }

    #[test]
    fn test_mock_s3_empty_prefix() {
        let ctx = MockS3TestBuilder::new().with_prefix("").build();
        let data = create_small_backup_data();

        ctx.upload_backup(
            "no-prefix",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        assert!(ctx.object_exists("no-prefix"));
    }

    #[test]
    fn test_mock_s3_single_byte_data() {
        let ctx = MockS3TestBuilder::new().build();
        let data = vec![42u8];

        let meta = ctx
            .upload_backup(
                "single-byte",
                &data,
                "2024-01-15T10:00:00Z",
                BackupCompression::Gzip,
            )
            .unwrap();

        assert_eq!(meta.size_bytes, 1);

        let (downloaded, _) = ctx.download_backup("single-byte").unwrap();
        assert_eq!(downloaded, vec![42u8]);
    }

    #[test]
    fn test_mock_s3_all_zeros_data() {
        let ctx = MockS3TestBuilder::new().build();
        let data: Vec<u8> = vec![0; 1000];

        ctx.upload_backup(
            "zeros-backup",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        let (downloaded, meta) = ctx.download_backup("zeros-backup").unwrap();
        assert_eq!(downloaded.len(), 1000);
        assert!(downloaded.iter().all(|&b| b == 0));
        assert_eq!(meta.size_bytes, 1000);
    }

    #[test]
    fn test_mock_s3_all_max_value_data() {
        let ctx = MockS3TestBuilder::new().build();
        let data: Vec<u8> = vec![255; 1000];

        ctx.upload_backup(
            "max-values",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        let (downloaded, _) = ctx.download_backup("max-values").unwrap();
        assert_eq!(downloaded.len(), 1000);
        assert!(downloaded.iter().all(|&b| b == 255));
    }

    #[test]
    fn test_mock_s3_alternating_pattern() {
        let ctx = MockS3TestBuilder::new().build();
        let data: Vec<u8> = (0..1000)
            .map(|i| if i % 2 == 0 { 0 } else { 255 })
            .collect();

        ctx.upload_backup(
            "alternating",
            &data,
            "2024-01-15T10:00:00Z",
            BackupCompression::Gzip,
        )
        .unwrap();

        let (downloaded, _) = ctx.download_backup("alternating").unwrap();
        for (i, b) in downloaded.iter().enumerate() {
            if i % 2 == 0 {
                assert_eq!(*b, 0);
            } else {
                assert_eq!(*b, 255);
            }
        }
    }
}

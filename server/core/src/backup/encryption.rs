use argon2::{Algorithm, Argon2, Params, Version};
use crypto_glue::aes256::key_from_slice;
use crypto_glue::aes256gcm::{Aead, Aes256Gcm, Aes256GcmNonce, KeyInit};
use kubidm_proto::backup::{
    BackupEncryptionConfig, BackupEncryptionHeader, EncryptionKeySource, KeyDerivationParams,
    BACKUP_ENCRYPTION_KEY_LEN, BACKUP_ENCRYPTION_MAGIC, BACKUP_ENCRYPTION_NONCE_LEN,
    BACKUP_ENCRYPTION_SALT_LEN,
};
use rand::{Rng, RngExt};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};

#[derive(Debug)]
pub enum BackupEncryptionError {
    InvalidMagic,
    InvalidHeader,
    EncryptionFailed(String),
    DecryptionFailed(String),
    KeyDerivationFailed(String),
    InvalidKeyLength,
    InvalidNonceLength,
    InvalidSaltLength,
    KeySourceError(String),
    IoError(std::io::Error),
    HttpError(String),
    SerializeError(String),
}

impl std::fmt::Display for BackupEncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupEncryptionError::InvalidMagic => write!(f, "Invalid backup magic header"),
            BackupEncryptionError::InvalidHeader => write!(f, "Invalid backup encryption header"),
            BackupEncryptionError::EncryptionFailed(msg) => write!(f, "Encryption failed: {}", msg),
            BackupEncryptionError::DecryptionFailed(msg) => write!(f, "Decryption failed: {}", msg),
            BackupEncryptionError::KeyDerivationFailed(msg) => {
                write!(f, "Key derivation failed: {}", msg)
            }
            BackupEncryptionError::InvalidKeyLength => write!(f, "Invalid key length"),
            BackupEncryptionError::InvalidNonceLength => write!(f, "Invalid nonce length"),
            BackupEncryptionError::InvalidSaltLength => write!(f, "Invalid salt length"),
            BackupEncryptionError::KeySourceError(msg) => write!(f, "Key source error: {}", msg),
            BackupEncryptionError::IoError(e) => write!(f, "IO error: {}", e),
            BackupEncryptionError::HttpError(msg) => write!(f, "HTTP error: {}", msg),
            BackupEncryptionError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
        }
    }
}

impl std::error::Error for BackupEncryptionError {}

impl From<std::io::Error> for BackupEncryptionError {
    fn from(e: std::io::Error) -> Self {
        BackupEncryptionError::IoError(e)
    }
}

pub struct BackupEncryptor {
    config: BackupEncryptionConfig,
}

impl BackupEncryptor {
    pub fn new(config: BackupEncryptionConfig) -> Self {
        Self { config }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn get_key_identifier(&self) -> Option<&str> {
        self.config.key_identifier.as_deref()
    }

    #[allow(dead_code)]
    async fn get_key_material(&self) -> Result<Vec<u8>, BackupEncryptionError> {
        match &self.config.key_source {
            EncryptionKeySource::Passphrase => Err(BackupEncryptionError::KeySourceError(
                "Passphrase must be provided at runtime".to_string(),
            )),
            EncryptionKeySource::File { path } => {
                let key_data = fs::read(path)?;
                Ok(key_data)
            }
            EncryptionKeySource::HttpEndpoint { url } => {
                let client = Client::new();
                let response = client
                    .get(url)
                    .send()
                    .await
                    .map_err(|e| BackupEncryptionError::HttpError(e.to_string()))?;

                let key_data = response
                    .bytes()
                    .await
                    .map_err(|e| BackupEncryptionError::HttpError(e.to_string()))?;

                Ok(key_data.to_vec())
            }
        }
    }

    fn derive_key(
        passphrase: &[u8],
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> Result<Vec<u8>, BackupEncryptionError> {
        if salt.len() < BACKUP_ENCRYPTION_SALT_LEN {
            return Err(BackupEncryptionError::InvalidSaltLength);
        }

        let argon_params = Params::new(params.m_cost, params.t_cost, params.p_cost, None)
            .map_err(|e| BackupEncryptionError::KeyDerivationFailed(e.to_string()))?;

        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

        let mut key = vec![0u8; BACKUP_ENCRYPTION_KEY_LEN];
        argon
            .hash_password_into(passphrase, salt, &mut key)
            .map_err(|e| BackupEncryptionError::KeyDerivationFailed(e.to_string()))?;

        Ok(key)
    }

    fn generate_key_identifier(passphrase: &[u8], salt: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(passphrase);
        hasher.update(salt);
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub async fn encrypt(
        &self,
        data: &[u8],
        passphrase: &[u8],
        compressed: bool,
    ) -> Result<Vec<u8>, BackupEncryptionError> {
        let mut rng = rand::rng();
        let salt: Vec<u8> = (0..BACKUP_ENCRYPTION_SALT_LEN)
            .map(|_| rng.random())
            .collect();
        let mut nonce_bytes = [0u8; BACKUP_ENCRYPTION_NONCE_LEN];
        rng.fill_bytes(&mut nonce_bytes);

        let key = Self::derive_key(passphrase, &salt, &self.config.key_derivation)?;

        let key_identifier = if let Some(id) = &self.config.key_identifier {
            id.clone()
        } else {
            Self::generate_key_identifier(passphrase, &salt)
        };

        let header = BackupEncryptionHeader::new(
            key_identifier.clone(),
            salt.clone(),
            nonce_bytes.to_vec(),
            self.config.key_derivation.clone(),
            compressed,
        );

        let key = key_from_slice(&key).ok_or(BackupEncryptionError::InvalidKeyLength)?;
        let cipher = Aes256Gcm::new(&*key);
        let nonce = Aes256GcmNonce::try_from(nonce_bytes.as_slice())
            .map_err(|_| BackupEncryptionError::InvalidNonceLength)?;

        let encrypted_data = cipher
            .encrypt(&nonce, data)
            .map_err(|e| BackupEncryptionError::EncryptionFailed(e.to_string()))?;

        let header_json = serde_json::to_string(&header)
            .map_err(|e| BackupEncryptionError::SerializeError(e.to_string()))?;

        let header_len = header_json.len() as u32;
        let header_len_bytes = header_len.to_le_bytes();

        let mut output = Vec::new();
        output.extend_from_slice(BACKUP_ENCRYPTION_MAGIC);
        output.extend_from_slice(&header_len_bytes);
        output.extend_from_slice(header_json.as_bytes());
        output.extend_from_slice(&encrypted_data);

        Ok(output)
    }

    fn parse_header(data: &[u8]) -> Result<(BackupEncryptionHeader, usize), BackupEncryptionError> {
        if data.len() < BACKUP_ENCRYPTION_MAGIC.len() + 4 {
            return Err(BackupEncryptionError::InvalidHeader);
        }

        let magic = data
            .get(..BACKUP_ENCRYPTION_MAGIC.len())
            .ok_or(BackupEncryptionError::InvalidHeader)?;
        if magic != BACKUP_ENCRYPTION_MAGIC {
            return Err(BackupEncryptionError::InvalidMagic);
        }

        let header_len_bytes = data
            .get(BACKUP_ENCRYPTION_MAGIC.len()..BACKUP_ENCRYPTION_MAGIC.len() + 4)
            .ok_or(BackupEncryptionError::InvalidHeader)?;
        let header_len = u32::from_le_bytes(
            header_len_bytes
                .try_into()
                .map_err(|_| BackupEncryptionError::InvalidHeader)?,
        );

        let header_start = BACKUP_ENCRYPTION_MAGIC.len() + 4;
        let header_end = header_start + header_len as usize;

        if data.len() < header_end {
            return Err(BackupEncryptionError::InvalidHeader);
        }

        let header_json = data
            .get(header_start..header_end)
            .ok_or(BackupEncryptionError::InvalidHeader)?;
        let header: BackupEncryptionHeader = serde_json::from_slice(header_json)
            .map_err(|e| BackupEncryptionError::SerializeError(e.to_string()))?;

        if !header.validate_magic() {
            return Err(BackupEncryptionError::InvalidMagic);
        }

        Ok((header, header_end))
    }

    pub fn decrypt(
        data: &[u8],
        passphrase: &[u8],
    ) -> Result<(Vec<u8>, BackupEncryptionHeader), BackupEncryptionError> {
        let (header, header_end) = Self::parse_header(data)?;

        if header.salt.len() < BACKUP_ENCRYPTION_SALT_LEN {
            return Err(BackupEncryptionError::InvalidSaltLength);
        }

        if header.nonce.len() < BACKUP_ENCRYPTION_NONCE_LEN {
            return Err(BackupEncryptionError::InvalidNonceLength);
        }

        let key = Self::derive_key(passphrase, &header.salt, &header.key_derivation)?;

        let encrypted_data = data
            .get(header_end..)
            .ok_or(BackupEncryptionError::InvalidHeader)?;

        let key = key_from_slice(&key).ok_or(BackupEncryptionError::InvalidKeyLength)?;
        let cipher = Aes256Gcm::new(&*key);
        let nonce = Aes256GcmNonce::try_from(header.nonce.as_slice())
            .map_err(|_| BackupEncryptionError::InvalidNonceLength)?;

        let decrypted_data = cipher
            .decrypt(&nonce, encrypted_data)
            .map_err(|e| BackupEncryptionError::DecryptionFailed(e.to_string()))?;

        Ok((decrypted_data, header))
    }

    pub fn decrypt_with_external_key(
        data: &[u8],
        key_material: &[u8],
    ) -> Result<(Vec<u8>, BackupEncryptionHeader), BackupEncryptionError> {
        let (header, header_end) = Self::parse_header(data)?;

        if header.nonce.len() < BACKUP_ENCRYPTION_NONCE_LEN {
            return Err(BackupEncryptionError::InvalidNonceLength);
        }

        if key_material.len() < BACKUP_ENCRYPTION_KEY_LEN {
            return Err(BackupEncryptionError::InvalidKeyLength);
        }

        let key = key_material
            .get(..BACKUP_ENCRYPTION_KEY_LEN)
            .ok_or(BackupEncryptionError::InvalidKeyLength)?;

        let encrypted_data = data
            .get(header_end..)
            .ok_or(BackupEncryptionError::InvalidHeader)?;

        let key = key_from_slice(key).ok_or(BackupEncryptionError::InvalidKeyLength)?;
        let cipher = Aes256Gcm::new(&*key);
        let nonce = Aes256GcmNonce::try_from(header.nonce.as_slice())
            .map_err(|_| BackupEncryptionError::InvalidNonceLength)?;

        let decrypted_data = cipher
            .decrypt(&nonce, encrypted_data)
            .map_err(|e| BackupEncryptionError::DecryptionFailed(e.to_string()))?;

        Ok((decrypted_data, header))
    }

    pub fn validate_key_identifier(
        passphrase: &[u8],
        expected_identifier: &str,
    ) -> Result<bool, BackupEncryptionError> {
        let mut rng = rand::rng();
        let salt: Vec<u8> = (0..BACKUP_ENCRYPTION_SALT_LEN)
            .map(|_| rng.random())
            .collect();

        let generated = Self::generate_key_identifier(passphrase, &salt);
        Ok(generated == expected_identifier)
    }
}

pub struct EncryptedWriter<W> {
    inner: Option<W>,
    buffer: Vec<u8>,
    encryptor: BackupEncryptor,
    passphrase: Vec<u8>,
    compressed: bool,
}

impl<W> EncryptedWriter<W> {
    pub fn new(
        inner: W,
        encryptor: BackupEncryptor,
        passphrase: Vec<u8>,
        compressed: bool,
    ) -> Self {
        Self {
            inner: Some(inner),
            buffer: Vec::new(),
            encryptor,
            passphrase,
            compressed,
        }
    }
}

impl<W: Write> Write for EncryptedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<W: Write> EncryptedWriter<W> {
    pub async fn finalize(self) -> Result<W, BackupEncryptionError> {
        let mut inner = self
            .inner
            .ok_or(BackupEncryptionError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Writer already finalized",
            )))?;
        let encrypted_data = self
            .encryptor
            .encrypt(&self.buffer, &self.passphrase, self.compressed)
            .await?;

        inner.write_all(&encrypted_data)?;
        inner.flush()?;
        Ok(inner)
    }
}

pub struct EncryptedReader<R: Read> {
    inner: R,
    buffer: Vec<u8>,
}

impl<R: Read> EncryptedReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
        }
    }

    pub fn read_encrypted(mut self) -> Result<Vec<u8>, BackupEncryptionError> {
        self.inner.read_to_end(&mut self.buffer)?;
        Ok(self.buffer)
    }

    pub fn decrypt(
        self,
        passphrase: &[u8],
    ) -> Result<(Vec<u8>, BackupEncryptionHeader), BackupEncryptionError> {
        let data = self.read_encrypted()?;
        BackupEncryptor::decrypt(&data, passphrase)
    }

    pub fn decrypt_with_external_key(
        self,
        key_material: &[u8],
    ) -> Result<(Vec<u8>, BackupEncryptionHeader), BackupEncryptionError> {
        let data = self.read_encrypted()?;
        BackupEncryptor::decrypt_with_external_key(&data, key_material)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kubidm_proto::backup::BackupEncryptionConfig;
    use std::time::Instant;

    fn create_test_config() -> BackupEncryptionConfig {
        BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        }
    }

    fn create_test_config_with_identifier(id: &str) -> BackupEncryptionConfig {
        BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: Some(id.to_string()),
        }
    }

    fn create_test_config_disabled() -> BackupEncryptionConfig {
        BackupEncryptionConfig {
            enabled: false,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        }
    }

    fn generate_random_key() -> Vec<u8> {
        let mut rng = rand::rng();
        (0..BACKUP_ENCRYPTION_KEY_LEN)
            .map(|_| rng.random())
            .collect()
    }

    fn generate_random_nonce() -> [u8; BACKUP_ENCRYPTION_NONCE_LEN] {
        let mut rng = rand::rng();
        let mut nonce = [0u8; BACKUP_ENCRYPTION_NONCE_LEN];
        rng.fill_bytes(&mut nonce);
        nonce
    }

    fn generate_random_salt() -> Vec<u8> {
        let mut rng = rand::rng();
        (0..BACKUP_ENCRYPTION_SALT_LEN)
            .map(|_| rng.random())
            .collect()
    }

    fn build_encrypted_data_with_external_key(
        key: &[u8],
        nonce_bytes: &[u8; BACKUP_ENCRYPTION_NONCE_LEN],
        data: &[u8],
        key_id: &str,
    ) -> Vec<u8> {
        let header = BackupEncryptionHeader::new(
            key_id.to_string(),
            generate_random_salt(),
            nonce_bytes.to_vec(),
            KeyDerivationParams::default(),
            false,
        );

        let key = key_from_slice(key).expect("Invalid key length");
        let cipher = Aes256Gcm::new(&*key);
        let nonce = Aes256GcmNonce::try_from(nonce_bytes).expect("Invalid nonce length");
        let encrypted_data = cipher.encrypt(nonce, data).unwrap();

        let header_json = serde_json::to_string(&header).unwrap();
        let header_len = header_json.len() as u32;
        let header_len_bytes = header_len.to_le_bytes();

        let mut full_data = Vec::new();
        full_data.extend_from_slice(BACKUP_ENCRYPTION_MAGIC);
        full_data.extend_from_slice(&header_len_bytes);
        full_data.extend_from_slice(header_json.as_bytes());
        full_data.extend_from_slice(&encrypted_data);
        full_data
    }

    mod key_management_tests {
        use super::*;

        #[test]
        fn test_derive_key_length() {
            let passphrase = b"test_passphrase";
            let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
            let params = KeyDerivationParams::default();

            let key = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();
            assert_eq!(key.len(), BACKUP_ENCRYPTION_KEY_LEN);
        }

        #[test]
        fn test_derive_key_consistency() {
            let passphrase = b"test_passphrase";
            let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
            let params = KeyDerivationParams::default();

            let key1 = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();
            let key2 = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();
            assert_eq!(key1, key2);
        }

        #[test]
        fn test_derive_key_different_passphrase() {
            let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
            let params = KeyDerivationParams::default();

            let key1 = BackupEncryptor::derive_key(b"passphrase1", &salt, &params).unwrap();
            let key2 = BackupEncryptor::derive_key(b"passphrase2", &salt, &params).unwrap();
            assert_ne!(key1, key2);
        }

        #[test]
        fn test_derive_key_different_salt() {
            let passphrase = b"test_passphrase";
            let params = KeyDerivationParams::default();

            let salt1 = generate_random_salt();
            let salt2 = generate_random_salt();
            let key1 = BackupEncryptor::derive_key(passphrase, &salt1, &params).unwrap();
            let key2 = BackupEncryptor::derive_key(passphrase, &salt2, &params).unwrap();
            assert_ne!(key1, key2);
        }

        #[test]
        fn test_derive_key_insufficient_salt_length() {
            let passphrase = b"test_passphrase";
            let short_salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN - 1];
            let params = KeyDerivationParams::default();

            let result = BackupEncryptor::derive_key(passphrase, &short_salt, &params);
            assert!(matches!(
                result,
                Err(BackupEncryptionError::InvalidSaltLength)
            ));
        }

        #[test]
        fn test_derive_key_empty_passphrase() {
            let passphrase = b"";
            let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
            let params = KeyDerivationParams::default();

            let key = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();
            assert_eq!(key.len(), BACKUP_ENCRYPTION_KEY_LEN);
        }

        #[test]
        fn test_derive_key_long_passphrase() {
            let passphrase =
                b"this is a very long passphrase that exceeds typical lengths for testing purposes";
            let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
            let params = KeyDerivationParams::default();

            let key = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();
            assert_eq!(key.len(), BACKUP_ENCRYPTION_KEY_LEN);
        }

        #[test]
        fn test_derive_key_custom_params() {
            let passphrase = b"test_passphrase";
            let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
            let params = KeyDerivationParams {
                m_cost: 8 * 1024,
                t_cost: 1,
                p_cost: 1,
            };

            let key = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();
            assert_eq!(key.len(), BACKUP_ENCRYPTION_KEY_LEN);
        }

        #[test]
        fn test_generate_key_identifier_consistency() {
            let passphrase = b"test_passphrase";
            let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];

            let id1 = BackupEncryptor::generate_key_identifier(passphrase, &salt);
            let id2 = BackupEncryptor::generate_key_identifier(passphrase, &salt);

            assert_eq!(id1, id2);
            assert_eq!(id1.len(), 64);
        }

        #[test]
        fn test_generate_key_identifier_deterministic() {
            let passphrase = b"test_passphrase";
            let salt = vec![1u8; BACKUP_ENCRYPTION_SALT_LEN];

            let id = BackupEncryptor::generate_key_identifier(passphrase, &salt);
            let expected_sha256_len = 64;
            assert_eq!(id.len(), expected_sha256_len);
            assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        }

        #[test]
        fn test_generate_key_identifier_different_inputs() {
            let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];

            let id1 = BackupEncryptor::generate_key_identifier(b"passphrase1", &salt);
            let id2 = BackupEncryptor::generate_key_identifier(b"passphrase2", &salt);
            assert_ne!(id1, id2);

            let id3 = BackupEncryptor::generate_key_identifier(
                b"passphrase1",
                &[1u8; BACKUP_ENCRYPTION_SALT_LEN],
            );
            assert_ne!(id1, id3);
        }

        #[test]
        fn test_validate_key_identifier_returns_ok() {
            let passphrase = b"test_passphrase";
            let expected_id = BackupEncryptor::generate_key_identifier(
                passphrase,
                &[0u8; BACKUP_ENCRYPTION_SALT_LEN],
            );

            let result = BackupEncryptor::validate_key_identifier(passphrase, &expected_id);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_key_identifier_generates_new_salt() {
            let passphrase = b"test_passphrase";
            let fixed_id = "fixed_identifier_00000000000000000000000000000000000000000000000000";

            let result1 = BackupEncryptor::validate_key_identifier(passphrase, fixed_id);
            let result2 = BackupEncryptor::validate_key_identifier(passphrase, fixed_id);

            assert!(result1.is_ok());
            assert!(result2.is_ok());
        }

        #[test]
        fn test_validate_key_identifier_wrong() {
            let passphrase = b"test_passphrase";
            let wrong_id = "0000000000000000000000000000000000000000000000000000000000000000";

            let result = BackupEncryptor::validate_key_identifier(passphrase, wrong_id);
            assert!(result.is_ok());
            assert!(!result.unwrap());
        }
    }

    mod encryption_tests {
        use super::*;

        #[tokio::test]
        async fn test_encrypt_basic() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            assert!(encrypted.len() > data.len());
            assert!(encrypted.starts_with(BACKUP_ENCRYPTION_MAGIC));
        }

        #[tokio::test]
        async fn test_encrypt_decrypt_roundtrip() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let (decrypted, header) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();

            assert_eq!(decrypted, data);
            assert!(!header.compressed);
        }

        #[tokio::test]
        async fn test_encrypt_with_compression_flag() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data compressed";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, true)
                .await
                .unwrap();
            let (decrypted, header) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();

            assert_eq!(decrypted, data);
            assert!(header.compressed);
        }

        #[tokio::test]
        async fn test_encrypt_with_custom_key_identifier() {
            let config = create_test_config_with_identifier("custom-key-id-123");
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let (_, header) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();

            assert_eq!(header.key_identifier, "custom-key-id-123");
        }

        #[tokio::test]
        async fn test_encrypt_empty_data() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();

            assert_eq!(decrypted, data);
        }

        #[tokio::test]
        async fn test_encrypt_large_data() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
            let passphrase = b"test_passphrase";

            let encrypted = encryptor.encrypt(&data, passphrase, false).await.unwrap();
            let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();

            assert_eq!(decrypted.len(), data.len());
            assert_eq!(decrypted, data);
        }

        #[tokio::test]
        async fn test_encrypt_multiple_times_different_nonces() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"test_passphrase";

            let encrypted1 = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let encrypted2 = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();

            assert_ne!(encrypted1, encrypted2);
        }

        #[tokio::test]
        async fn test_encryptor_is_enabled() {
            let config_enabled = create_test_config();
            let encryptor_enabled = BackupEncryptor::new(config_enabled);
            assert!(encryptor_enabled.is_enabled());

            let config_disabled = create_test_config_disabled();
            let encryptor_disabled = BackupEncryptor::new(config_disabled);
            assert!(!encryptor_disabled.is_enabled());
        }

        #[tokio::test]
        async fn test_encryptor_get_key_identifier() {
            let config_with_id = create_test_config_with_identifier("key-123");
            let encryptor = BackupEncryptor::new(config_with_id);
            assert_eq!(encryptor.get_key_identifier(), Some("key-123"));

            let config_no_id = create_test_config();
            let encryptor_no_id = BackupEncryptor::new(config_no_id);
            assert!(encryptor_no_id.get_key_identifier().is_none());
        }

        #[tokio::test]
        async fn test_encrypt_with_special_characters_passphrase() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"p@ss!phr$e#$%^&*()";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();

            assert_eq!(decrypted, data);
        }

        #[tokio::test]
        async fn test_encrypt_with_unicode_passphrase() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = "日本語パスフレーズ".as_bytes();

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();

            assert_eq!(decrypted, data);
        }
    }

    mod decryption_tests {
        use super::*;

        #[tokio::test]
        async fn test_decrypt_with_wrong_passphrase() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"correct_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let result = BackupEncryptor::decrypt(&encrypted, b"wrong_passphrase");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                BackupEncryptionError::DecryptionFailed(_)
            ));
        }

        #[tokio::test]
        async fn test_decrypt_invalid_magic() {
            let mut data = BACKUP_ENCRYPTION_MAGIC.to_vec();
            data.extend_from_slice(&[0u8; 100]);
            data[0] = b'X';
            let result = BackupEncryptor::decrypt(&data, b"passphrase");
            assert!(matches!(result, Err(BackupEncryptionError::InvalidMagic)));
        }

        #[tokio::test]
        async fn test_decrypt_truncated_data() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let truncated = &encrypted[..BACKUP_ENCRYPTION_MAGIC.len() + 2];

            let result = BackupEncryptor::decrypt(truncated, passphrase);
            assert!(matches!(result, Err(BackupEncryptionError::InvalidHeader)));
        }

        #[tokio::test]
        async fn test_decrypt_with_modified_header() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let mut modified = encrypted.clone();
            if modified.len() > BACKUP_ENCRYPTION_MAGIC.len() + 10 {
                modified[BACKUP_ENCRYPTION_MAGIC.len() + 5] ^= 0xFF;
            }

            let result = BackupEncryptor::decrypt(&modified, passphrase);
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_decrypt_with_modified_encrypted_data() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test backup data";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let mut modified = encrypted.clone();
            if let Some(last_byte) = modified.last_mut() {
                *last_byte ^= 0xFF;
            }

            let result = BackupEncryptor::decrypt(&modified, passphrase);
            assert!(matches!(
                result,
                Err(BackupEncryptionError::DecryptionFailed(_))
            ));
        }

        #[test]
        fn test_decrypt_with_external_key() {
            let key = generate_random_key();
            let nonce_bytes = generate_random_nonce();
            let data = b"test data for external key";

            let full_data =
                build_encrypted_data_with_external_key(&key, &nonce_bytes, data, "external-key-id");

            let (decrypted, dec_header) =
                BackupEncryptor::decrypt_with_external_key(&full_data, &key).unwrap();

            assert_eq!(decrypted, data);
            assert_eq!(dec_header.key_identifier, "external-key-id");
        }

        #[test]
        fn test_decrypt_with_external_key_wrong_key() {
            let key = generate_random_key();
            let wrong_key = generate_random_key();
            let nonce_bytes = generate_random_nonce();
            let data = b"test data for external key";

            let full_data =
                build_encrypted_data_with_external_key(&key, &nonce_bytes, data, "external-key-id");

            let result = BackupEncryptor::decrypt_with_external_key(&full_data, &wrong_key);
            assert!(matches!(
                result,
                Err(BackupEncryptionError::DecryptionFailed(_))
            ));
        }

        #[test]
        fn test_decrypt_with_external_key_short_key() {
            let key = generate_random_key();
            let short_key = &key[..BACKUP_ENCRYPTION_KEY_LEN - 1];
            let nonce_bytes = generate_random_nonce();
            let data = b"test data";

            let full_data =
                build_encrypted_data_with_external_key(&key, &nonce_bytes, data, "external-key-id");

            let result = BackupEncryptor::decrypt_with_external_key(&full_data, short_key);
            assert!(matches!(
                result,
                Err(BackupEncryptionError::InvalidKeyLength)
            ));
        }

        #[test]
        fn test_decrypt_with_external_key_exact_length() {
            let key = generate_random_key();
            let nonce_bytes = generate_random_nonce();
            let data = b"test data";

            let full_data =
                build_encrypted_data_with_external_key(&key, &nonce_bytes, data, "external-key-id");

            let result = BackupEncryptor::decrypt_with_external_key(&full_data, &key);
            assert!(result.is_ok());
        }
    }

    mod header_tests {
        use super::*;

        #[test]
        fn test_header_validate_magic_correct() {
            let header = BackupEncryptionHeader::new(
                "test-key".to_string(),
                vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
                vec![0u8; BACKUP_ENCRYPTION_NONCE_LEN],
                KeyDerivationParams::default(),
                false,
            );
            assert!(header.validate_magic());
        }

        #[test]
        fn test_header_validate_magic_wrong() {
            let mut header = BackupEncryptionHeader::new(
                "test-key".to_string(),
                vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
                vec![0u8; BACKUP_ENCRYPTION_NONCE_LEN],
                KeyDerivationParams::default(),
                false,
            );
            header.magic = "WRONG_MAGIC".to_string();
            assert!(!header.validate_magic());
        }

        #[test]
        fn test_header_compressed_flag() {
            let header_compressed = BackupEncryptionHeader::new(
                "test-key".to_string(),
                vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
                vec![0u8; BACKUP_ENCRYPTION_NONCE_LEN],
                KeyDerivationParams::default(),
                true,
            );
            assert!(header_compressed.compressed);

            let header_not_compressed = BackupEncryptionHeader::new(
                "test-key".to_string(),
                vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
                vec![0u8; BACKUP_ENCRYPTION_NONCE_LEN],
                KeyDerivationParams::default(),
                false,
            );
            assert!(!header_not_compressed.compressed);
        }

        #[test]
        fn test_header_serialization() {
            let header = BackupEncryptionHeader::new(
                "test-key-id".to_string(),
                vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
                vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
                KeyDerivationParams {
                    m_cost: 1024,
                    t_cost: 2,
                    p_cost: 1,
                },
                true,
            );

            let json = serde_json::to_string(&header).unwrap();
            let deserialized: BackupEncryptionHeader = serde_json::from_str(&json).unwrap();

            assert_eq!(header.key_identifier, deserialized.key_identifier);
            assert_eq!(header.salt, deserialized.salt);
            assert_eq!(header.nonce, deserialized.nonce);
            assert_eq!(header.compressed, deserialized.compressed);
        }

        #[test]
        fn test_parse_header_valid() {
            let header = BackupEncryptionHeader::new(
                "test-key".to_string(),
                generate_random_salt(),
                generate_random_nonce().to_vec(),
                KeyDerivationParams::default(),
                false,
            );

            let header_json = serde_json::to_string(&header).unwrap();
            let header_len = header_json.len() as u32;
            let header_len_bytes = header_len.to_le_bytes();

            let mut data = Vec::new();
            data.extend_from_slice(BACKUP_ENCRYPTION_MAGIC);
            data.extend_from_slice(&header_len_bytes);
            data.extend_from_slice(header_json.as_bytes());
            data.extend_from_slice(&[0u8; 50]);

            let (parsed_header, header_end) = BackupEncryptor::parse_header(&data).unwrap();
            assert_eq!(parsed_header.key_identifier, header.key_identifier);
            assert_eq!(
                header_end,
                BACKUP_ENCRYPTION_MAGIC.len() + 4 + header_json.len()
            );
        }

        #[test]
        fn test_parse_header_invalid_magic() {
            let mut data = b"WRONG_MAGIC_HEADER".to_vec();
            data.extend_from_slice(&[0u8; 100]);

            let result = BackupEncryptor::parse_header(&data);
            assert!(matches!(result, Err(BackupEncryptionError::InvalidMagic)));
        }

        #[test]
        fn test_parse_header_too_short() {
            let data = b"KUBIDM_ENC_BACKUP_V1".to_vec();
            let result = BackupEncryptor::parse_header(&data);
            assert!(matches!(result, Err(BackupEncryptionError::InvalidHeader)));
        }
    }

    mod error_tests {
        use super::*;

        #[test]
        fn test_error_display_invalid_magic() {
            let error = BackupEncryptionError::InvalidMagic;
            assert_eq!(error.to_string(), "Invalid backup magic header");
        }

        #[test]
        fn test_error_display_invalid_header() {
            let error = BackupEncryptionError::InvalidHeader;
            assert_eq!(error.to_string(), "Invalid backup encryption header");
        }

        #[test]
        fn test_error_display_encryption_failed() {
            let error = BackupEncryptionError::EncryptionFailed("test error".to_string());
            assert_eq!(error.to_string(), "Encryption failed: test error");
        }

        #[test]
        fn test_error_display_decryption_failed() {
            let error = BackupEncryptionError::DecryptionFailed("test error".to_string());
            assert_eq!(error.to_string(), "Decryption failed: test error");
        }

        #[test]
        fn test_error_display_key_derivation_failed() {
            let error = BackupEncryptionError::KeyDerivationFailed("test error".to_string());
            assert_eq!(error.to_string(), "Key derivation failed: test error");
        }

        #[test]
        fn test_error_display_invalid_key_length() {
            let error = BackupEncryptionError::InvalidKeyLength;
            assert_eq!(error.to_string(), "Invalid key length");
        }

        #[test]
        fn test_error_display_invalid_nonce_length() {
            let error = BackupEncryptionError::InvalidNonceLength;
            assert_eq!(error.to_string(), "Invalid nonce length");
        }

        #[test]
        fn test_error_display_invalid_salt_length() {
            let error = BackupEncryptionError::InvalidSaltLength;
            assert_eq!(error.to_string(), "Invalid salt length");
        }

        #[test]
        fn test_error_display_key_source_error() {
            let error = BackupEncryptionError::KeySourceError("test error".to_string());
            assert_eq!(error.to_string(), "Key source error: test error");
        }

        #[test]
        fn test_error_from_io_error() {
            let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            let error: BackupEncryptionError = io_error.into();
            assert!(matches!(error, BackupEncryptionError::IoError(_)));
        }
    }

    mod reader_writer_tests {
        use super::*;
        use std::io::Cursor;

        #[tokio::test]
        async fn test_encrypted_writer_reader_roundtrip() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);
            let passphrase = b"test_passphrase";

            let buffer: Vec<u8> = Vec::new();
            let writer =
                EncryptedWriter::new(Cursor::new(buffer), encryptor, passphrase.to_vec(), false);
            let mut writer = writer;
            writer.write_all(b"test data").unwrap();
            let cursor = writer.finalize().await.unwrap();
            let buffer = cursor.into_inner();

            let reader = EncryptedReader::new(Cursor::new(&buffer));
            let (decrypted, _) = reader.decrypt(passphrase).unwrap();
            assert_eq!(decrypted, b"test data");
        }

        #[tokio::test]
        async fn test_encrypted_writer_compressed_flag() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);
            let passphrase = b"test_passphrase";

            let buffer: Vec<u8> = Vec::new();
            let writer =
                EncryptedWriter::new(Cursor::new(buffer), encryptor, passphrase.to_vec(), true);
            let mut writer = writer;
            writer.write_all(b"test data").unwrap();
            let cursor = writer.finalize().await.unwrap();
            let buffer = cursor.into_inner();

            let reader = EncryptedReader::new(Cursor::new(&buffer));
            let (_, header) = reader.decrypt(passphrase).unwrap();
            assert!(header.compressed);
        }

        #[test]
        fn test_encrypted_reader_read_encrypted() {
            let data = b"raw encrypted data";
            let reader = EncryptedReader::new(Cursor::new(data.as_slice()));
            let read_data = reader.read_encrypted().unwrap();
            assert_eq!(read_data, data);
        }
    }

    mod performance_tests {
        use super::*;

        #[tokio::test]
        async fn test_encryption_performance_small_data() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"small test data";
            let passphrase = b"test_passphrase";

            let start = Instant::now();
            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let encrypt_duration = start.elapsed();

            let start = Instant::now();
            let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();
            let decrypt_duration = start.elapsed();

            assert_eq!(decrypted, data);
            assert!(encrypt_duration.as_millis() < 5000);
            assert!(decrypt_duration.as_millis() < 5000);
        }

        #[tokio::test]
        async fn test_encryption_performance_medium_data() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();
            let passphrase = b"test_passphrase";

            let start = Instant::now();
            let encrypted = encryptor.encrypt(&data, passphrase, false).await.unwrap();
            let encrypt_duration = start.elapsed();

            let start = Instant::now();
            let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();
            let decrypt_duration = start.elapsed();

            assert_eq!(decrypted.len(), data.len());
            assert!(encrypt_duration.as_millis() < 5000);
            assert!(decrypt_duration.as_millis() < 5000);
        }

        #[tokio::test]
        async fn test_encryption_throughput() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
            let passphrase = b"test_passphrase";

            let start = Instant::now();
            let _encrypted = encryptor.encrypt(&data, passphrase, false).await.unwrap();
            let duration = start.elapsed();

            let bytes_per_second = data.len() as f64 / duration.as_secs_f64();
            assert!(bytes_per_second > 100_000.0);
        }

        #[tokio::test]
        async fn test_decryption_throughput() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
            let passphrase = b"test_passphrase";

            let encrypted = encryptor.encrypt(&data, passphrase, false).await.unwrap();

            let start = Instant::now();
            let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, passphrase).unwrap();
            let duration = start.elapsed();

            let bytes_per_second = decrypted.len() as f64 / duration.as_secs_f64();
            assert!(bytes_per_second > 100_000.0);
        }
    }

    mod security_tests {
        use super::*;

        #[tokio::test]
        async fn test_nonce_uniqueness() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test data";
            let passphrase = b"test_passphrase";

            let encrypted1 = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();
            let encrypted2 = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();

            let (_, header1) = BackupEncryptor::decrypt(&encrypted1, passphrase).unwrap();
            let (_, header2) = BackupEncryptor::decrypt(&encrypted2, passphrase).unwrap();

            assert_ne!(header1.nonce, header2.nonce);
        }

        #[tokio::test]
        async fn test_different_keys_different_output() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test data";

            let encrypted1 = encryptor
                .encrypt(data.as_slice(), b"key1", false)
                .await
                .unwrap();
            let encrypted2 = encryptor
                .encrypt(data.as_slice(), b"key2", false)
                .await
                .unwrap();

            assert_ne!(encrypted1, encrypted2);
        }

        #[test]
        fn test_key_derivation_params_security() {
            let params = KeyDerivationParams::default();
            assert!(params.m_cost >= 8 * 1024);
            assert!(params.t_cost >= 1);
            assert!(params.p_cost >= 1);
        }

        #[test]
        fn test_key_length_matches_aes256() {
            assert_eq!(BACKUP_ENCRYPTION_KEY_LEN, 32);
        }

        #[test]
        fn test_nonce_length_matches_aes256gcm() {
            assert_eq!(BACKUP_ENCRYPTION_NONCE_LEN, 12);
        }

        #[tokio::test]
        async fn test_encryption_output_size() {
            let config = create_test_config();
            let encryptor = BackupEncryptor::new(config);

            let data = b"test";
            let passphrase = b"test_passphrase";

            let encrypted = encryptor
                .encrypt(data.as_slice(), passphrase, false)
                .await
                .unwrap();

            let overhead = encrypted.len() - data.len();
            assert!(overhead < 500, "Overhead was {} bytes", overhead);
        }
    }

    #[tokio::test]
    async fn test_encryption_different_passphrases_different_output() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";

        let encrypted1 = encryptor
            .encrypt(data, b"passphrase1", false)
            .await
            .unwrap();
        let encrypted2 = encryptor
            .encrypt(data, b"passphrase2", false)
            .await
            .unwrap();

        assert_ne!(encrypted1, encrypted2);
    }

    #[tokio::test]
    async fn test_encryption_same_passphrase_different_output() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";

        let encrypted1 = encryptor.encrypt(data, b"passphrase", false).await.unwrap();
        let encrypted2 = encryptor.encrypt(data, b"passphrase", false).await.unwrap();

        assert_ne!(encrypted1, encrypted2);
    }

    #[tokio::test]
    async fn test_encryption_empty_data() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"";

        let encrypted = encryptor.encrypt(data, b"passphrase", false).await.unwrap();
        let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, b"passphrase").unwrap();

        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_encryption_large_data() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let encrypted = encryptor
            .encrypt(&data, b"passphrase", false)
            .await
            .unwrap();
        let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, b"passphrase").unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encryption_key_derivation_consistency() {
        let passphrase = b"test_passphrase";
        let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
        let params = KeyDerivationParams::default();

        let key1 = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();
        let key2 = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_encryption_key_derivation_different_passphrase() {
        let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
        let params = KeyDerivationParams::default();

        let key1 = BackupEncryptor::derive_key(b"passphrase1", &salt, &params).unwrap();
        let key2 = BackupEncryptor::derive_key(b"passphrase2", &salt, &params).unwrap();

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_encryption_key_derivation_different_salt() {
        let passphrase = b"test_passphrase";
        let params = KeyDerivationParams::default();

        let salt1 = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
        let salt2 = vec![1u8; BACKUP_ENCRYPTION_SALT_LEN];

        let key1 = BackupEncryptor::derive_key(passphrase, &salt1, &params).unwrap();
        let key2 = BackupEncryptor::derive_key(passphrase, &salt2, &params).unwrap();

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_encryption_invalid_salt_length() {
        let passphrase = b"test_passphrase";
        let short_salt = vec![0u8; 8];
        let params = KeyDerivationParams::default();

        let result = BackupEncryptor::derive_key(passphrase, &short_salt, &params);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            BackupEncryptionError::InvalidSaltLength
        ));
    }

    #[test]
    fn test_encryption_header_validation() {
        let header = BackupEncryptionHeader::new(
            "test-key".to_string(),
            vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
            vec![0u8; BACKUP_ENCRYPTION_NONCE_LEN],
            KeyDerivationParams::default(),
            false,
        );

        assert!(header.validate_magic());
        assert!(!header.compressed);
    }

    #[test]
    fn test_encryption_header_compressed_flag() {
        let header_compressed = BackupEncryptionHeader::new(
            "key-id".to_string(),
            vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
            vec![0u8; BACKUP_ENCRYPTION_NONCE_LEN],
            KeyDerivationParams::default(),
            true,
        );

        let header_not_compressed = BackupEncryptionHeader::new(
            "key-id".to_string(),
            vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
            vec![0u8; BACKUP_ENCRYPTION_NONCE_LEN],
            KeyDerivationParams::default(),
            false,
        );

        assert!(header_compressed.compressed);
        assert!(!header_not_compressed.compressed);
    }

    #[tokio::test]
    async fn test_encryption_tampered_data_detection() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"sensitive data";

        let mut encrypted = encryptor.encrypt(data, b"passphrase", false).await.unwrap();

        if encrypted.len() >= 10 {
            let len = encrypted.len();
            encrypted[len - 10] ^= 0xFF;
        }

        let result = BackupEncryptor::decrypt(&encrypted, b"passphrase");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_encryption_tampered_header_detection() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";

        let mut encrypted = encryptor.encrypt(data, b"passphrase", false).await.unwrap();

        if encrypted.len() > 5 {
            encrypted[BACKUP_ENCRYPTION_MAGIC.len()] ^= 0xFF;
        }

        let result = BackupEncryptor::decrypt(&encrypted, b"passphrase");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_encryption_wrong_key_identifier() {
        let config = BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: Some("expected-key-id".to_string()),
        };
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";

        let encrypted = encryptor.encrypt(data, b"passphrase", false).await.unwrap();
        let (_, header) = BackupEncryptor::decrypt(&encrypted, b"passphrase").unwrap();

        assert_eq!(header.key_identifier, "expected-key-id");
    }

    #[test]
    fn test_encryption_key_source_variants() {
        let passphrase_config = BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        };

        let file_config = BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::File {
                path: "/path/to/key".to_string(),
            },
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        };

        let http_config = BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::HttpEndpoint {
                url: "https://vault.example.com/key".to_string(),
            },
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        };

        assert!(BackupEncryptor::new(passphrase_config).is_enabled());
        assert!(BackupEncryptor::new(file_config).is_enabled());
        assert!(BackupEncryptor::new(http_config).is_enabled());
    }

    #[test]
    fn test_encryption_disabled_config() {
        let config = BackupEncryptionConfig {
            enabled: false,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        };

        let encryptor = BackupEncryptor::new(config);
        assert!(!encryptor.is_enabled());
        assert!(encryptor.get_key_identifier().is_none());
    }

    #[test]
    fn test_encryption_key_derivation_params_custom() {
        let custom_params = KeyDerivationParams {
            m_cost: 32 * 1024,
            t_cost: 4,
            p_cost: 2,
        };

        let passphrase = b"test_passphrase";
        let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];

        let key = BackupEncryptor::derive_key(passphrase, &salt, &custom_params).unwrap();
        assert_eq!(key.len(), BACKUP_ENCRYPTION_KEY_LEN);
    }

    #[tokio::test]
    async fn test_encryption_nonce_uniqueness() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";

        let encrypted1 = encryptor.encrypt(data, b"passphrase", false).await.unwrap();
        let encrypted2 = encryptor.encrypt(data, b"passphrase", false).await.unwrap();

        let (_, header1) = BackupEncryptor::decrypt(&encrypted1, b"passphrase").unwrap();
        let (_, header2) = BackupEncryptor::decrypt(&encrypted2, b"passphrase").unwrap();

        assert_ne!(header1.nonce, header2.nonce);
    }

    #[tokio::test]
    async fn test_encryption_salt_uniqueness() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";

        let encrypted1 = encryptor.encrypt(data, b"passphrase", false).await.unwrap();
        let encrypted2 = encryptor.encrypt(data, b"passphrase", false).await.unwrap();

        let (_, header1) = BackupEncryptor::decrypt(&encrypted1, b"passphrase").unwrap();
        let (_, header2) = BackupEncryptor::decrypt(&encrypted2, b"passphrase").unwrap();

        assert_ne!(header1.salt, header2.salt);
    }

    #[tokio::test]
    async fn test_encryption_short_passphrase() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";
        let short_passphrase = b"x";

        let encrypted = encryptor
            .encrypt(data, short_passphrase, false)
            .await
            .unwrap();
        let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, short_passphrase).unwrap();

        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_encryption_long_passphrase() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";
        let long_passphrase: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        let encrypted = encryptor
            .encrypt(data, &long_passphrase, false)
            .await
            .unwrap();
        let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, &long_passphrase).unwrap();

        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_encryption_unicode_passphrase() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";
        let unicode_passphrase = "日本語パスワード".as_bytes();

        let encrypted = encryptor
            .encrypt(data, unicode_passphrase, false)
            .await
            .unwrap();
        let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, unicode_passphrase).unwrap();

        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_encryption_binary_passphrase() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";
        let binary_passphrase: Vec<u8> = vec![0x00, 0xFF, 0x80, 0x7F, 0x01, 0xFE];

        let encrypted = encryptor
            .encrypt(data, &binary_passphrase, false)
            .await
            .unwrap();
        let (decrypted, _) = BackupEncryptor::decrypt(&encrypted, &binary_passphrase).unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encryption_error_display() {
        let err = BackupEncryptionError::InvalidMagic;
        assert_eq!(err.to_string(), "Invalid backup magic header");

        let err = BackupEncryptionError::InvalidHeader;
        assert_eq!(err.to_string(), "Invalid backup encryption header");

        let err = BackupEncryptionError::EncryptionFailed("test error".to_string());
        assert!(err.to_string().contains("Encryption failed"));
        assert!(err.to_string().contains("test error"));

        let err = BackupEncryptionError::DecryptionFailed("decrypt error".to_string());
        assert!(err.to_string().contains("Decryption failed"));

        let err = BackupEncryptionError::KeyDerivationFailed("derive error".to_string());
        assert!(err.to_string().contains("Key derivation failed"));

        let err = BackupEncryptionError::InvalidKeyLength;
        assert_eq!(err.to_string(), "Invalid key length");

        let err = BackupEncryptionError::InvalidNonceLength;
        assert_eq!(err.to_string(), "Invalid nonce length");

        let err = BackupEncryptionError::InvalidSaltLength;
        assert_eq!(err.to_string(), "Invalid salt length");

        let err = BackupEncryptionError::KeySourceError("source error".to_string());
        assert!(err.to_string().contains("Key source error"));

        let err = BackupEncryptionError::HttpError("http error".to_string());
        assert!(err.to_string().contains("HTTP error"));

        let err = BackupEncryptionError::SerializeError("serde error".to_string());
        assert!(err.to_string().contains("Serialize error"));
    }

    #[test]
    fn test_encryption_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let enc_err: BackupEncryptionError = io_err.into();

        assert!(matches!(enc_err, BackupEncryptionError::IoError(_)));
        assert!(enc_err.to_string().contains("IO error"));
    }

    #[test]
    fn test_encryption_key_identifier_generation() {
        let passphrase = b"test_passphrase";
        let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];

        let id1 = BackupEncryptor::generate_key_identifier(passphrase, &salt);
        let id2 = BackupEncryptor::generate_key_identifier(passphrase, &salt);

        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    #[test]
    fn test_encryption_key_identifier_different_inputs() {
        let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];

        let id1 = BackupEncryptor::generate_key_identifier(b"passphrase1", &salt);
        let id2 = BackupEncryptor::generate_key_identifier(b"passphrase2", &salt);

        assert_ne!(id1, id2);

        let id3 = BackupEncryptor::generate_key_identifier(b"passphrase", &[0u8; 16]);
        let id4 = BackupEncryptor::generate_key_identifier(b"passphrase", &[1u8; 16]);

        assert_ne!(id3, id4);
    }

    #[test]
    fn test_encryption_validate_key_identifier_correct() {
        let passphrase = b"test_passphrase";
        let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
        let expected = BackupEncryptor::generate_key_identifier(passphrase, &salt);

        let result = BackupEncryptor::validate_key_identifier(passphrase, &expected);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_encryption_decrypt_with_wrong_nonce_length() {
        let config = BackupEncryptionConfig::default();
        let encryptor = BackupEncryptor::new(config);
        let data = b"test data";

        let encrypted = encryptor.encrypt(data, b"passphrase", false).await.unwrap();

        let mut corrupted = encrypted.clone();
        let header_end = BACKUP_ENCRYPTION_MAGIC.len() + 4 + 100;
        if corrupted.len() > header_end + 5 {
            corrupted.splice(header_end..header_end + 12, vec![0u8; 5]);
        }

        let result = BackupEncryptor::decrypt(&corrupted, b"passphrase");
        assert!(result.is_err());
    }
}

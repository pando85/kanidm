use argon2::{Algorithm, Argon2, Params, Version};
use crypto_glue::aes256gcm::{Aead, Aes256Gcm, KeyInit};
use generic_array::GenericArray;
use kanidm_proto::backup::{
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

        let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
        let nonce = GenericArray::from_slice(&nonce_bytes);

        let encrypted_data = cipher
            .encrypt(nonce, data)
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

        let magic = &data[..BACKUP_ENCRYPTION_MAGIC.len()];
        if magic != BACKUP_ENCRYPTION_MAGIC {
            return Err(BackupEncryptionError::InvalidMagic);
        }

        let header_len_bytes =
            &data[BACKUP_ENCRYPTION_MAGIC.len()..BACKUP_ENCRYPTION_MAGIC.len() + 4];
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

        let header_json = &data[header_start..header_end];
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

        let encrypted_data = &data[header_end..];

        let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
        let nonce = GenericArray::from_slice(&header.nonce);

        let decrypted_data = cipher
            .decrypt(nonce, encrypted_data)
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

        let key = &key_material[..BACKUP_ENCRYPTION_KEY_LEN];

        let encrypted_data = &data[header_end..];

        let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
        let nonce = GenericArray::from_slice(&header.nonce);

        let decrypted_data = cipher
            .decrypt(nonce, encrypted_data)
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
    use kanidm_proto::backup::BackupEncryptionConfig;

    #[test]
    fn test_derive_key() {
        let passphrase = b"test_passphrase";
        let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];
        let params = KeyDerivationParams::default();

        let key = BackupEncryptor::derive_key(passphrase, &salt, &params).unwrap();
        assert_eq!(key.len(), BACKUP_ENCRYPTION_KEY_LEN);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_roundtrip() {
        let config = BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        };
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
    async fn test_encrypt_decrypt_with_compression_flag() {
        let config = BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        };
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
    async fn test_decrypt_wrong_passphrase() {
        let config = BackupEncryptionConfig {
            enabled: true,
            key_source: EncryptionKeySource::Passphrase,
            key_derivation: KeyDerivationParams::default(),
            key_identifier: None,
        };
        let encryptor = BackupEncryptor::new(config);

        let data = b"test backup data";
        let passphrase = b"correct_passphrase";

        let encrypted = encryptor
            .encrypt(data.as_slice(), passphrase, false)
            .await
            .unwrap();

        let result = BackupEncryptor::decrypt(&encrypted, b"wrong_passphrase");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_key_identifier() {
        let passphrase = b"test_passphrase";
        let salt = vec![0u8; BACKUP_ENCRYPTION_SALT_LEN];

        let id1 = BackupEncryptor::generate_key_identifier(passphrase, &salt);
        let id2 = BackupEncryptor::generate_key_identifier(passphrase, &salt);

        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = BACKUP_ENCRYPTION_MAGIC.to_vec();
        data.extend_from_slice(&[0u8; 100]);
        data[0] = b'X';
        let result = BackupEncryptor::decrypt(&data, b"passphrase");
        assert!(matches!(result, Err(BackupEncryptionError::InvalidMagic)));
    }

    #[test]
    fn test_encrypt_decrypt_with_external_key() {
        let mut rng = rand::rng();
        let key: Vec<u8> = (0..BACKUP_ENCRYPTION_KEY_LEN)
            .map(|_| rng.random())
            .collect();
        let mut nonce_bytes = [0u8; BACKUP_ENCRYPTION_NONCE_LEN];
        rng.fill_bytes(&mut nonce_bytes);

        let header = BackupEncryptionHeader::new(
            "external-key-id".to_string(),
            vec![0u8; BACKUP_ENCRYPTION_SALT_LEN],
            nonce_bytes.to_vec(),
            KeyDerivationParams::default(),
            false,
        );

        let data = b"test data for external key";

        let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
        let nonce = GenericArray::from_slice(&nonce_bytes);
        let encrypted_data = cipher.encrypt(nonce, data.as_slice()).unwrap();

        let header_json = serde_json::to_string(&header).unwrap();
        let header_len = header_json.len() as u32;
        let header_len_bytes = header_len.to_le_bytes();

        let mut full_data = Vec::new();
        full_data.extend_from_slice(BACKUP_ENCRYPTION_MAGIC);
        full_data.extend_from_slice(&header_len_bytes);
        full_data.extend_from_slice(header_json.as_bytes());
        full_data.extend_from_slice(&encrypted_data);

        let (decrypted, dec_header) =
            BackupEncryptor::decrypt_with_external_key(&full_data, &key).unwrap();

        assert_eq!(decrypted, data);
        assert_eq!(dec_header.key_identifier, "external-key-id");
    }
}

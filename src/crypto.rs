//! Cifrado en reposo de credenciales de cámara (HU 4.1).
//!
//! AES-256-GCM (autenticado). La clave (32 bytes) viene de `DB_ENCRYPTION_KEY`
//! en base64 y vive fuera de la BD; así, un dump/backup de Postgres no revela
//! las URLs RTSP sin la clave. Formato del blob: `nonce(12) || ciphertext+tag`,
//! con nonce ALEATORIO por cifrado.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;

/// Tamaño de nonce recomendado para GCM (96 bits).
const NONCE_LEN: usize = 12;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("DB_ENCRYPTION_KEY inválida: {0}")]
    InvalidKey(String),
    #[error("fallo de cifrado")]
    Encrypt,
    #[error("fallo de descifrado (clave incorrecta o dato manipulado)")]
    Decrypt,
    #[error("blob cifrado demasiado corto")]
    BlobTooShort,
    #[error("el texto descifrado no es UTF-8 válido")]
    NotUtf8,
}

/// Cifrador AES-256-GCM. Se construye una vez desde la clave y se reutiliza.
#[derive(Clone)]
pub struct Cipher {
    cipher: Aes256Gcm,
}

impl Cipher {
    /// Construye el cifrador desde la clave en base64 (debe decodificar a 32 bytes).
    pub fn from_base64_key(b64: &str) -> Result<Self, CryptoError> {
        let raw = STANDARD
            .decode(b64.trim())
            .map_err(|e| CryptoError::InvalidKey(format!("no es base64 válido: {e}")))?;
        if raw.len() != 32 {
            return Err(CryptoError::InvalidKey(format!(
                "se esperaban 32 bytes (AES-256), hay {}",
                raw.len()
            )));
        }
        let key = Key::<Aes256Gcm>::from_slice(&raw);
        Ok(Self {
            cipher: Aes256Gcm::new(key),
        })
    }

    /// Cifra texto y devuelve `nonce(12) || ciphertext+tag`.
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        let mut rng = OsRng;
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::Encrypt)?;

        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Descifra un blob con formato `nonce(12) || ciphertext+tag`.
    pub fn decrypt(&self, blob: &[u8]) -> Result<String, CryptoError> {
        if blob.len() < NONCE_LEN {
            return Err(CryptoError::BlobTooShort);
        }
        let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::Decrypt)?;

        String::from_utf8(plaintext).map_err(|_| CryptoError::NotUtf8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clave de 32 bytes en base64, solo para tests.
    fn test_key() -> String {
        STANDARD.encode([7u8; 32])
    }

    #[test]
    fn roundtrip() {
        let c = Cipher::from_base64_key(&test_key()).unwrap();
        let pt = "rtsp://user:pass@10.0.0.5/axis-media/media.amp";
        let blob = c.encrypt(pt).unwrap();
        assert_ne!(blob.as_slice(), pt.as_bytes(), "el blob debe estar cifrado");
        assert_eq!(c.decrypt(&blob).unwrap(), pt);
    }

    #[test]
    fn distinct_nonces_produce_distinct_blobs() {
        let c = Cipher::from_base64_key(&test_key()).unwrap();
        let a = c.encrypt("mismo").unwrap();
        let b = c.encrypt("mismo").unwrap();
        assert_ne!(a, b, "cada cifrado debe usar un nonce aleatorio distinto");
    }

    #[test]
    fn tampered_blob_fails() {
        let c = Cipher::from_base64_key(&test_key()).unwrap();
        let mut blob = c.encrypt("dato").unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0xff; // corromper el tag/ciphertext
        assert!(c.decrypt(&blob).is_err());
    }

    #[test]
    fn wrong_key_size_rejected() {
        let short = STANDARD.encode([0u8; 16]);
        assert!(Cipher::from_base64_key(&short).is_err());
    }

    #[test]
    fn non_base64_key_rejected() {
        assert!(Cipher::from_base64_key("no-es-base64-!!!").is_err());
    }
}

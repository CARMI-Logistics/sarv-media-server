//! Primitivos de secretos por proyecto (Argon2id).
//!
//! Responsabilidad única: hashear y verificar secretos. Los secretos se guardan
//! HASHEADOS, nunca en claro. Fail-closed: cualquier fallo de verificación → false.

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;

/// Genera el hash Argon2id de un secreto (subcomando CLI para dar de alta proyectos).
pub fn hash_secret(secret: &str) -> Result<String, Box<dyn std::error::Error>> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map_err(|e| format!("error hasheando secreto: {e}"))?
        .to_string();
    Ok(hash)
}

/// Verifica un secreto contra su hash Argon2id. Cualquier fallo → false.
pub fn verify_secret(hash: &str, secret: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = hash_secret("s3cret").unwrap();
        assert!(verify_secret(&hash, "s3cret"));
        assert!(!verify_secret(&hash, "malo"));
    }

    #[test]
    fn invalid_hash_is_rejected() {
        assert!(!verify_secret("no-es-un-hash-valido", "loquesea"));
    }
}

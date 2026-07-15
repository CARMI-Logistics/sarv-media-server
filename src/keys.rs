//! Ciclo de vida de la clave de firma RS256 (HU 2.2).
//!
//! Responsabilidad única: obtener el material de firma. Carga la clave privada
//! RSA desde disco (volumen persistente) o, si no existe, la genera y la guarda
//! UNA sola vez. Así la clave sobrevive a los reinicios del backend y los JWT ya
//! emitidos siguen validando (antes se generaba efímera en cada arranque).

use std::fs;
use std::path::Path;

use jsonwebtoken::EncodingKey;
use rand::rngs::OsRng;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use rsa::{RsaPrivateKey, RsaPublicKey};
use tracing::info;

/// Material de firma listo para usar.
pub struct SigningMaterial {
    /// Clave para firmar JWT (RS256).
    pub encoding_key: EncodingKey,
    /// Clave pública, para construir el JWKS.
    pub public_key: RsaPublicKey,
}

/// Carga la clave privada desde `path`; si no existe, la genera y la persiste.
pub fn load_or_create(path: &str) -> Result<SigningMaterial, Box<dyn std::error::Error>> {
    let private_key = if Path::new(path).exists() {
        info!("Cargando clave de firma existente desde {}", path);
        let pem = fs::read_to_string(path)?;
        RsaPrivateKey::from_pkcs8_pem(&pem)?
    } else {
        info!("No hay clave en {}; generando una nueva (RSA 2048)...", path);
        let mut rng = OsRng;
        let key = RsaPrivateKey::new(&mut rng, 2048)?;
        persist(&key, path)?;
        info!("Clave de firma generada y guardada en {}", path);
        key
    };

    let public_key = RsaPublicKey::from(&private_key);
    let private_pem = private_key.to_pkcs8_pem(LineEnding::LF)?;
    let encoding_key = EncodingKey::from_rsa_pem(private_pem.as_bytes())?;

    Ok(SigningMaterial {
        encoding_key,
        public_key,
    })
}

/// Escribe la clave en disco (creando el directorio) con permisos restrictivos.
fn persist(key: &RsaPrivateKey, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let pem = key.to_pkcs8_pem(LineEnding::LF)?;
    fs::write(path, pem.as_bytes())?;
    set_restrictive_perms(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_restrictive_perms(path: &str) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_restrictive_perms(_path: &str) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::traits::PublicKeyParts;

    #[test]
    fn load_or_create_persists_and_reuses_key() {
        let dir = std::env::temp_dir().join(format!("mtx-keys-test-{}", std::process::id()));
        let path = dir.join("jwt_private_key.pem");
        let p = path.to_str().unwrap();

        // Primera vez: genera y persiste.
        let first = load_or_create(p).unwrap();
        assert!(path.exists(), "la clave debe quedar guardada en disco");

        // Segunda vez: carga la misma clave.
        let second = load_or_create(p).unwrap();

        // Misma clave => mismo módulo público (la firma sobrevive al reinicio).
        assert_eq!(first.public_key.n(), second.public_key.n());

        std::fs::remove_dir_all(&dir).ok();
    }
}

//! Credenciales por proyecto (HU 2.2).
//!
//! Responsabilidad única: cargar el almacén de proyectos y verificar sus
//! credenciales. Los secretos se guardan HASHEADOS (Argon2id), nunca en claro.
//! Fail-closed: si el archivo no existe o es inválido, el almacén queda vacío y
//! nadie se autentica (se elimina el antiguo backdoor admin/admin).

use std::collections::HashMap;
use std::fs;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;
use serde::Deserialize;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
struct Client {
    client_id: String,
    secret_hash: String,
}

#[derive(Debug, Deserialize)]
struct ClientsFile {
    clients: Vec<Client>,
}

/// Almacén en memoria: client_id -> hash del secreto.
pub struct ClientStore {
    by_id: HashMap<String, String>,
}

impl ClientStore {
    /// Carga el almacén desde el archivo JSON. Ante cualquier fallo, queda vacío
    /// (fail-closed).
    pub fn load(path: &str) -> Self {
        let by_id = match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<ClientsFile>(&content) {
                Ok(file) => file
                    .clients
                    .into_iter()
                    .map(|c| (c.client_id, c.secret_hash))
                    .collect(),
                Err(err) => {
                    warn!("clients.json inválido ({}): almacén vacío (fail-closed)", err);
                    HashMap::new()
                }
            },
            Err(_) => {
                warn!("No se encontró {}: almacén vacío (fail-closed)", path);
                HashMap::new()
            }
        };
        info!("Almacén de credenciales: {} proyecto(s)", by_id.len());
        Self { by_id }
    }

    /// Verifica credenciales: el proyecto existe y el secreto coincide con el hash.
    pub fn verify(&self, client_id: &str, secret: &str) -> bool {
        let Some(stored) = self.by_id.get(client_id) else {
            return false;
        };
        let Ok(parsed) = PasswordHash::new(stored) else {
            return false;
        };
        Argon2::default()
            .verify_password(secret.as_bytes(), &parsed)
            .is_ok()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Genera el hash Argon2id de un secreto (subcomando CLI para dar de alta proyectos).
pub fn hash_secret(secret: &str) -> Result<String, Box<dyn std::error::Error>> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map_err(|e| format!("error hasheando secreto: {e}"))?
        .to_string();
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = hash_secret("s3cret").unwrap();
        let store = ClientStore {
            by_id: HashMap::from([("sigac".to_string(), hash)]),
        };
        assert!(store.verify("sigac", "s3cret"));
        assert!(!store.verify("sigac", "malo"));
        assert!(!store.verify("desconocido", "s3cret"));
    }

    #[test]
    fn missing_file_is_empty_and_fail_closed() {
        let store = ClientStore::load("/ruta/inexistente/clients.json");
        assert!(store.is_empty());
        assert!(!store.verify("cualquiera", "loquesea"));
    }
}

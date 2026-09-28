//! Entidades del dominio (HU 4.1).
//!
//! Tipos puros de negocio, sin dependencias de infraestructura. En particular,
//! `Camera.rtsp_url` va EN CLARO aquí; el cifrado en reposo es responsabilidad
//! del adaptador de datos, no del dominio.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Severidad de un diagnóstico de cámara.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Warn,
    Error,
}

impl Severity {
    /// Representación en texto tal como se guarda en la columna `severity`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Ok => "ok",
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }

    /// Interpreta el texto de la BD; cualquier valor desconocido se trata como
    /// `Error` (fail hacia "requiere atención").
    pub fn from_db(s: &str) -> Self {
        match s {
            "ok" => Severity::Ok,
            "warn" => Severity::Warn,
            _ => Severity::Error,
        }
    }
}

/// Proyecto consumidor (reemplaza clients.json). `secret_hash` es Argon2id.
///
/// `jwt_exp_minutes`/`jwt_never_expires` son mutuamente excluyentes:
/// `jwt_never_expires` (si `true`) gana sobre `jwt_exp_minutes`; si no,
/// `jwt_exp_minutes` (si tiene valor) gana sobre el default global
/// `JWT_EXP_MINUTES`. Ver `AppState::generate_jwt`.
#[derive(Debug, Clone)]
pub struct Project {
    pub id: Uuid,
    pub client_id: String,
    pub secret_hash: String,
    pub all_cameras: bool,
    pub enabled: bool,
    pub jwt_exp_minutes: Option<i32>,
    pub jwt_never_expires: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Alta de un proyecto (el id y los timestamps los pone la capa de datos).
#[derive(Debug, Clone)]
pub struct NewProject {
    pub client_id: String,
    pub secret_hash: String,
    pub all_cameras: bool,
    pub enabled: bool,
    pub jwt_exp_minutes: Option<i32>,
    pub jwt_never_expires: bool,
}

/// Cámara. `rtsp_url` en claro en el dominio; el adaptador la cifra/descifra.
#[derive(Debug, Clone)]
pub struct Camera {
    pub id: Uuid,
    pub path: String,
    pub rtsp_url: String,
    pub record: bool,
    pub enabled: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Alta de una cámara.
#[derive(Debug, Clone)]
pub struct NewCamera {
    pub path: String,
    pub rtsp_url: String,
    pub record: bool,
    pub enabled: bool,
    pub description: Option<String>,
}

/// Registro de un diagnóstico/fallo (lo llena el agente).
/// `diagnosis`/`raw` van SIN credenciales (redactado).
#[derive(Debug, Clone)]
pub struct Failure {
    pub id: i64,
    pub camera_path: String,
    pub detected_at: DateTime<Utc>,
    pub severity: Severity,
    pub diagnosis: Option<String>,
    pub raw: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Alta de un registro de historial.
#[derive(Debug, Clone)]
pub struct NewFailure {
    pub camera_path: String,
    pub detected_at: DateTime<Utc>,
    pub severity: Severity,
    pub diagnosis: Option<String>,
    pub raw: Option<serde_json::Value>,
}

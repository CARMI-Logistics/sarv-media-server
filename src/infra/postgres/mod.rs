//! Adaptadores Postgres de los puertos de dominio (HU 4.1, sub-paso 6).
//!
//! Cada repo mapea filas de la BD (structs `FromRow` privados) a entidades del
//! dominio. Los errores de sqlx se traducen a `RepoError` (no se filtra sqlx
//! hacia el dominio). Path A: queries en runtime, validadas por pruebas de
//! integración (sub-paso 8).

use crate::domain::ports::RepoError;

pub mod camera_repo;
pub mod failure_repo;
pub mod project_repo;

pub use camera_repo::PgCameraRepo;
pub use failure_repo::PgFailureRepo;
pub use project_repo::PgProjectRepo;

/// Traduce errores de sqlx a errores de dominio.
pub(crate) fn map_sqlx_err(e: sqlx::Error) -> RepoError {
    match &e {
        sqlx::Error::RowNotFound => RepoError::NotFound,
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            RepoError::Conflict(db.message().to_string())
        }
        _ => RepoError::Backend(e.to_string()),
    }
}

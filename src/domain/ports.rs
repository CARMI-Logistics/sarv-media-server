//! Puertos (contratos) del dominio (HU 4.1).
//!
//! Traits async con `async-trait` para que sean dyn-compatibles (`Arc<dyn ...>`).
//! Los adaptadores en `infra/` los implementan; los servicios dependen de estos
//! traits, nunca de la implementación concreta (Inversión de Dependencias).

use async_trait::async_trait;
use uuid::Uuid;

use super::models::{Camera, Failure, NewCamera, NewFailure, NewProject, Project};

/// Error de almacenamiento del dominio. NO expone tipos de infraestructura
/// (sqlx, etc.); el adaptador traduce sus errores a estas variantes.
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("recurso no encontrado")]
    NotFound,
    #[error("conflicto de unicidad: {0}")]
    Conflict(String),
    #[error("error de almacenamiento: {0}")]
    Backend(String),
}

pub type RepoResult<T> = Result<T, RepoError>;

/// Proyectos consumidores y su acceso a cámaras (relación n-a-n).
#[async_trait]
pub trait ProjectRepo: Send + Sync {
    async fn find_by_client_id(&self, client_id: &str) -> RepoResult<Option<Project>>;
    async fn find_by_id(&self, id: Uuid) -> RepoResult<Option<Project>>;
    async fn list_all(&self) -> RepoResult<Vec<Project>>;
    async fn create(&self, new: NewProject) -> RepoResult<Project>;
    async fn update(&self, project: &Project) -> RepoResult<Project>;
    async fn delete(&self, id: Uuid) -> RepoResult<()>;

    /// Reemplaza el conjunto de cámaras permitidas del proyecto (n-a-n).
    async fn set_cameras(&self, project_id: Uuid, camera_ids: &[Uuid]) -> RepoResult<()>;

    /// Paths de las cámaras con acceso EXPLÍCITO del proyecto. Ignora la bandera
    /// `all_cameras`; eso lo resuelve el servicio de autorización (HU 4.4).
    async fn allowed_camera_paths(&self, project_id: Uuid) -> RepoResult<Vec<String>>;

    /// IDs de las cámaras asignadas explícitamente al proyecto (n-a-n).
    async fn assigned_camera_ids(&self, project_id: Uuid) -> RepoResult<Vec<Uuid>>;
}

/// Cámaras: fuente de verdad; el reconciler las lleva a MediaMTX (HU 4.2).
#[async_trait]
pub trait CameraRepo: Send + Sync {
    async fn list_all(&self) -> RepoResult<Vec<Camera>>;
    async fn list_enabled(&self) -> RepoResult<Vec<Camera>>;
    async fn find_by_id(&self, id: Uuid) -> RepoResult<Option<Camera>>;
    async fn find_by_path(&self, path: &str) -> RepoResult<Option<Camera>>;
    async fn create(&self, new: NewCamera) -> RepoResult<Camera>;
    async fn update(&self, camera: &Camera) -> RepoResult<Camera>;
    async fn delete(&self, id: Uuid) -> RepoResult<()>;
}

/// Historial de fallos/diagnósticos (lo llena el agente, HU 4.6).
#[async_trait]
pub trait FailureRepo: Send + Sync {
    async fn record(&self, new: NewFailure) -> RepoResult<Failure>;
    async fn list_by_camera(&self, camera_path: &str, limit: i64) -> RepoResult<Vec<Failure>>;
    /// Último registro de una cámara (para deduplicar alertas, HU 4.6).
    async fn latest_by_camera(&self, camera_path: &str) -> RepoResult<Option<Failure>>;
}

/// Error al aprovisionar rutas en el servidor de streaming (HU 4.2).
/// No expone tipos de infraestructura (reqwest, etc.); el adaptador los traduce.
#[derive(Debug, thiserror::Error)]
pub enum ProvisionError {
    #[error("la ruta no existe en el servidor de streaming")]
    NotFound,
    #[error("error del servidor de streaming: {0}")]
    Backend(String),
}

pub type ProvisionResult<T> = Result<T, ProvisionError>;

/// Puerto de aprovisionamiento: lleva la configuración de cámaras (fuente de
/// verdad en la BD) al servidor de streaming (MediaMTX) vía su Control API.
/// El reconciler depende de este puerto, no del cliente HTTP concreto (DIP).
#[async_trait]
pub trait CameraProvisioner: Send + Sync {
    /// Alta o actualización (upsert) de la ruta de la cámara.
    async fn apply(&self, camera: &Camera) -> ProvisionResult<()>;
    /// Baja de una ruta por su nombre.
    async fn remove(&self, path: &str) -> ProvisionResult<()>;
    /// Nombres de las rutas de CÁMARA gestionables (pull RTSP): excluye
    /// publishers y patrones regex, para que el reconciler solo administre lo suyo.
    async fn list_paths(&self) -> ProvisionResult<Vec<String>>;
}

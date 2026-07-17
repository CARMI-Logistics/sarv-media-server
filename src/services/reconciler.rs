//! Reconciler (HU 4.2): sincroniza la BD (fuente de verdad) con MediaMTX.
//!
//! Depende de los puertos `CameraRepo` y `CameraProvisioner` (DIP), no de sus
//! implementaciones. Es idempotente y resiliente: un fallo al aplicar/eliminar
//! una cámara se registra y cuenta, pero no aborta el reconcile completo; solo
//! los fallos de listado (BD/MediaMTX) son fatales para que la tarea reintente.

use std::collections::HashSet;
use std::sync::Arc;

use tracing::{info, warn};

use crate::domain::models::Camera;
use crate::domain::ports::{CameraProvisioner, CameraRepo, ProvisionError, RepoError};

#[derive(Debug, thiserror::Error)]
pub enum ReconcileError {
    #[error("error leyendo cámaras de la BD: {0}")]
    Repo(#[from] RepoError),
    #[error("error con el servidor de streaming: {0}")]
    Provision(#[from] ProvisionError),
}

pub struct ReconcilerService {
    cameras: Arc<dyn CameraRepo>,
    provisioner: Arc<dyn CameraProvisioner>,
}

impl ReconcilerService {
    pub fn new(cameras: Arc<dyn CameraRepo>, provisioner: Arc<dyn CameraProvisioner>) -> Self {
        Self {
            cameras,
            provisioner,
        }
    }

    /// Sincroniza el estado deseado (cámaras enabled en la BD) con MediaMTX:
    /// aplica cada cámara y elimina las rutas huérfanas concretas.
    pub async fn reconcile_all(&self) -> Result<(), ReconcileError> {
        // Estado deseado (fatal si la BD falla → la tarea reintenta).
        let cameras = self.cameras.list_enabled().await?;

        // Guarda de seguridad: con la BD vacía NO tocamos MediaMTX (evita borrar
        // rutas cargadas por otra vía cuando la BD aún no está poblada/migrada).
        if cameras.is_empty() {
            warn!("Reconcile: 0 cámaras habilitadas en la BD; no se aplica ni elimina nada (seguridad)");
            return Ok(());
        }

        let desired: HashSet<&str> = cameras.iter().map(|c| c.path.as_str()).collect();

        // Aplicar cada cámara (upsert). Fallos por cámara: se cuentan, no abortan.
        let mut applied = 0usize;
        let mut errors = 0usize;
        for camera in &cameras {
            match self.provisioner.apply(camera).await {
                Ok(()) => applied += 1,
                Err(e) => {
                    errors += 1;
                    warn!("no se pudo aplicar la cámara '{}': {}", camera.path, e);
                }
            }
        }

        // Huérfanos: rutas de cámara (pull RTSP) que ya no están en la BD.
        // list_paths ya excluye publishers y regex, así que no se tocan.
        let existing = self.provisioner.list_paths().await?;
        let mut removed = 0usize;
        for name in existing {
            if name.starts_with('~') || desired.contains(name.as_str()) {
                continue;
            }
            match self.provisioner.remove(&name).await {
                Ok(()) => removed += 1,
                Err(ProvisionError::NotFound) => {} // ya no estaba: idempotente
                Err(e) => {
                    errors += 1;
                    warn!("no se pudo eliminar la ruta huérfana '{}': {}", name, e);
                }
            }
        }

        info!(
            "Reconcile: {} aplicada(s), {} huérfana(s) eliminada(s), {} error(es)",
            applied, removed, errors
        );
        Ok(())
    }

    /// Sincronización puntual de una cámara (alta/edición vía endpoints, HU 4.5).
    #[allow(dead_code)] // lo invocan los endpoints de administración en HU 4.5
    pub async fn apply_camera(&self, camera: &Camera) -> Result<(), ProvisionError> {
        self.provisioner.apply(camera).await
    }

    /// Baja puntual de una ruta (borrado de cámara vía endpoints, HU 4.5).
    #[allow(dead_code)] // lo invocan los endpoints de administración en HU 4.5
    pub async fn remove_camera(&self, path: &str) -> Result<(), ProvisionError> {
        self.provisioner.remove(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::ReconcilerService;
    use crate::domain::models::{Camera, NewCamera};
    use crate::domain::ports::{CameraProvisioner, CameraRepo, ProvisionResult, RepoResult};
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    /// Repo falso: solo implementa `list_enabled` (lo único que usa el reconciler).
    struct FakeCameraRepo {
        cameras: Vec<Camera>,
    }

    #[async_trait]
    impl CameraRepo for FakeCameraRepo {
        async fn list_enabled(&self) -> RepoResult<Vec<Camera>> {
            Ok(self.cameras.clone())
        }
        async fn list_all(&self) -> RepoResult<Vec<Camera>> {
            Ok(self.cameras.clone())
        }
        async fn find_by_id(&self, _: Uuid) -> RepoResult<Option<Camera>> {
            unimplemented!()
        }
        async fn find_by_path(&self, _: &str) -> RepoResult<Option<Camera>> {
            unimplemented!()
        }
        async fn create(&self, _: NewCamera) -> RepoResult<Camera> {
            unimplemented!()
        }
        async fn update(&self, _: &Camera) -> RepoResult<Camera> {
            unimplemented!()
        }
        async fn delete(&self, _: Uuid) -> RepoResult<()> {
            unimplemented!()
        }
    }

    /// Provisioner falso: registra lo aplicado/eliminado y devuelve `existing`.
    #[derive(Default)]
    struct FakeProvisioner {
        existing: Vec<String>,
        applied: Mutex<Vec<String>>,
        removed: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl CameraProvisioner for FakeProvisioner {
        async fn apply(&self, camera: &Camera) -> ProvisionResult<()> {
            self.applied.lock().unwrap().push(camera.path.clone());
            Ok(())
        }
        async fn remove(&self, path: &str) -> ProvisionResult<()> {
            self.removed.lock().unwrap().push(path.to_string());
            Ok(())
        }
        async fn list_paths(&self) -> ProvisionResult<Vec<String>> {
            Ok(self.existing.clone())
        }
    }

    fn camera(path: &str) -> Camera {
        Camera {
            id: Uuid::new_v4(),
            path: path.into(),
            rtsp_url: "rtsp://user:pass@10.0.0.1/s".into(),
            record: true,
            enabled: true,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn applies_all_and_removes_orphans() {
        let repo = Arc::new(FakeCameraRepo {
            cameras: vec![camera("a"), camera("b")],
        });
        let prov = Arc::new(FakeProvisioner {
            existing: vec!["a".into(), "orphan".into()],
            ..Default::default()
        });
        ReconcilerService::new(repo, prov.clone())
            .reconcile_all()
            .await
            .unwrap();

        let mut applied = prov.applied.lock().unwrap().clone();
        applied.sort();
        assert_eq!(applied, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(prov.removed.lock().unwrap().clone(), vec!["orphan".to_string()]);
    }

    #[tokio::test]
    async fn empty_db_is_noop() {
        let repo = Arc::new(FakeCameraRepo { cameras: vec![] });
        let prov = Arc::new(FakeProvisioner {
            existing: vec!["a".into()],
            ..Default::default()
        });
        ReconcilerService::new(repo, prov.clone())
            .reconcile_all()
            .await
            .unwrap();

        assert!(prov.applied.lock().unwrap().is_empty());
        assert!(
            prov.removed.lock().unwrap().is_empty(),
            "con la BD vacía no debe eliminar nada"
        );
    }

    #[tokio::test]
    async fn no_orphans_when_all_match() {
        let repo = Arc::new(FakeCameraRepo {
            cameras: vec![camera("a"), camera("b")],
        });
        let prov = Arc::new(FakeProvisioner {
            existing: vec!["a".into(), "b".into()],
            ..Default::default()
        });
        ReconcilerService::new(repo, prov.clone())
            .reconcile_all()
            .await
            .unwrap();

        assert!(prov.removed.lock().unwrap().is_empty());
    }
}

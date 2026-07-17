//! Autenticación de proyectos contra la BD (HU 4.3).
//!
//! Reemplaza el almacén basado en `clients.json`. Depende del puerto
//! `ProjectRepo` (DIP). Fail-closed: proyecto inexistente, deshabilitado,
//! secreto incorrecto o error de BD → autenticación denegada.

use std::sync::Arc;

use tracing::warn;

use crate::domain::models::Project;
use crate::domain::ports::ProjectRepo;

pub struct AuthService {
    projects: Arc<dyn ProjectRepo>,
}

impl AuthService {
    pub fn new(projects: Arc<dyn ProjectRepo>) -> Self {
        Self { projects }
    }

    /// Devuelve el proyecto si las credenciales son válidas y está habilitado.
    pub async fn authenticate(&self, client_id: &str, secret: &str) -> Option<Project> {
        let project = match self.projects.find_by_client_id(client_id).await {
            Ok(Some(p)) => p,
            Ok(None) => return None,
            Err(e) => {
                // Fail-closed ante error de almacenamiento (sin filtrar detalles).
                warn!("error consultando el proyecto '{}': {}", client_id, e);
                return None;
            }
        };

        if !project.enabled {
            warn!("proyecto deshabilitado: {}", client_id);
            return None;
        }

        if crate::secret::verify_secret(&project.secret_hash, secret) {
            Some(project)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AuthService;
    use crate::domain::models::{NewProject, Project};
    use crate::domain::ports::{ProjectRepo, RepoError, RepoResult};
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    /// Repo falso: solo implementa `find_by_client_id`.
    struct FakeProjectRepo {
        project: Option<Project>,
        fail: bool,
    }

    #[async_trait]
    impl ProjectRepo for FakeProjectRepo {
        async fn find_by_client_id(&self, client_id: &str) -> RepoResult<Option<Project>> {
            if self.fail {
                return Err(RepoError::Backend("boom".into()));
            }
            Ok(self.project.clone().filter(|p| p.client_id == client_id))
        }
        async fn find_by_id(&self, _: Uuid) -> RepoResult<Option<Project>> {
            unimplemented!()
        }
        async fn list_all(&self) -> RepoResult<Vec<Project>> {
            unimplemented!()
        }
        async fn create(&self, _: NewProject) -> RepoResult<Project> {
            unimplemented!()
        }
        async fn update(&self, _: &Project) -> RepoResult<Project> {
            unimplemented!()
        }
        async fn delete(&self, _: Uuid) -> RepoResult<()> {
            unimplemented!()
        }
        async fn set_cameras(&self, _: Uuid, _: &[Uuid]) -> RepoResult<()> {
            unimplemented!()
        }
        async fn allowed_camera_paths(&self, _: Uuid) -> RepoResult<Vec<String>> {
            unimplemented!()
        }
    }

    fn project(client_id: &str, secret: &str, enabled: bool) -> Project {
        Project {
            id: Uuid::new_v4(),
            client_id: client_id.into(),
            secret_hash: crate::secret::hash_secret(secret).unwrap(),
            all_cameras: true,
            enabled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn service(project: Option<Project>) -> AuthService {
        AuthService::new(Arc::new(FakeProjectRepo {
            project,
            fail: false,
        }))
    }

    #[tokio::test]
    async fn authenticates_valid_enabled_project() {
        let p = service(Some(project("sigac", "s3cret", true)))
            .authenticate("sigac", "s3cret")
            .await;
        assert_eq!(p.map(|p| p.client_id), Some("sigac".to_string()));
    }

    #[tokio::test]
    async fn rejects_wrong_secret() {
        let svc = service(Some(project("sigac", "s3cret", true)));
        assert!(svc.authenticate("sigac", "malo").await.is_none());
    }

    #[tokio::test]
    async fn rejects_disabled_project() {
        let svc = service(Some(project("sigac", "s3cret", false)));
        assert!(svc.authenticate("sigac", "s3cret").await.is_none());
    }

    #[tokio::test]
    async fn rejects_unknown_project() {
        assert!(service(None).authenticate("sigac", "s3cret").await.is_none());
    }

    #[tokio::test]
    async fn fails_closed_on_repo_error() {
        let svc = AuthService::new(Arc::new(FakeProjectRepo {
            project: None,
            fail: true,
        }));
        assert!(svc.authenticate("sigac", "s3cret").await.is_none());
    }
}

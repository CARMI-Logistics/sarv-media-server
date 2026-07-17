//! Endpoints de administración (HU 4.5): CRUD de cámaras y proyectos.
//!
//! Protegidos por `require_admin` (bearer ADMIN_API_TOKEN). Las respuestas NO
//! exponen secretos (rtsp_url / secret_hash).

use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::http::{header::AUTHORIZATION, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::models::{Camera, NewCamera, NewProject, Project};
use crate::domain::ports::RepoError;
use crate::AppState;

/// Middleware: exige `Authorization: Bearer <ADMIN_API_TOKEN>`. Fail-closed:
/// si no hay token configurado, se rechaza todo el panel de administración.
pub async fn require_admin(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let configured = state.config.admin_api_token.as_str();
    if configured.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "administración no configurada (ADMIN_API_TOKEN)",
        )
            .into_response();
    }

    let auth_header = req.headers().get(AUTHORIZATION).and_then(|v| v.to_str().ok());
    if is_authorized(configured, auth_header) {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, "no autorizado").into_response()
    }
}

/// Lógica pura de autorización: token configurado no vacío y bearer coincidente.
fn is_authorized(configured: &str, auth_header: Option<&str>) -> bool {
    if configured.is_empty() {
        return false;
    }
    auth_header
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| token == configured)
}

/// Router de administración (se monta bajo /admin con el middleware require_admin).
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/cameras", get(list_cameras).post(create_camera))
        .route(
            "/cameras/:id",
            get(get_camera).patch(update_camera).delete(delete_camera),
        )
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/:id",
            get(get_project).patch(update_project).delete(delete_project),
        )
}

/// Traduce un error de repositorio a una respuesta HTTP (sin filtrar detalles).
fn repo_err(e: RepoError) -> (StatusCode, String) {
    match e {
        RepoError::NotFound => (StatusCode::NOT_FOUND, "recurso no encontrado".to_string()),
        RepoError::Conflict(msg) => (StatusCode::CONFLICT, msg),
        RepoError::Backend(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "error interno".to_string(),
        ),
    }
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Respuesta de cámara SIN la `rtsp_url` (no se expone la credencial).
#[derive(Serialize, ToSchema)]
pub struct CameraResponse {
    pub id: Uuid,
    pub path: String,
    pub record: bool,
    pub enabled: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Camera> for CameraResponse {
    fn from(c: Camera) -> Self {
        Self {
            id: c.id,
            path: c.path,
            record: c.record,
            enabled: c.enabled,
            description: c.description,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

/// Alta de cámara.
#[derive(Deserialize, ToSchema)]
pub struct CreateCameraRequest {
    pub path: String,
    pub rtsp_url: String,
    pub record: Option<bool>,
    pub enabled: Option<bool>,
    pub description: Option<String>,
}

/// Edición parcial de cámara (solo los campos presentes se actualizan).
#[derive(Deserialize, ToSchema)]
pub struct UpdateCameraRequest {
    pub rtsp_url: Option<String>,
    pub record: Option<bool>,
    pub enabled: Option<bool>,
    pub description: Option<String>,
}

/// Respuesta de proyecto SIN el `secret_hash` (incluye sus cámaras asignadas).
#[derive(Serialize, ToSchema)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub client_id: String,
    pub all_cameras: bool,
    pub enabled: bool,
    pub camera_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Alta de proyecto. El secreto llega en claro y se hashea (Argon2id).
#[derive(Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub client_id: String,
    pub secret: String,
    #[serde(default)]
    pub all_cameras: bool,
    #[serde(default)]
    pub camera_ids: Vec<Uuid>,
}

/// Edición parcial de proyecto (solo los campos presentes se actualizan).
#[derive(Deserialize, ToSchema)]
pub struct UpdateProjectRequest {
    pub secret: Option<String>,        // rotar el secreto
    pub all_cameras: Option<bool>,
    pub enabled: Option<bool>,
    pub camera_ids: Option<Vec<Uuid>>, // reasignar cámaras
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get, path = "/admin/cameras", tag = "Administration",
    security(("admin_token" = [])),
    responses(
        (status = 200, description = "Lista de cámaras", body = [CameraResponse]),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn list_cameras(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CameraResponse>>, StatusCode> {
    let cameras = state
        .camera_repo
        .list_all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(cameras.into_iter().map(CameraResponse::from).collect()))
}

#[utoipa::path(
    post, path = "/admin/cameras", tag = "Administration",
    security(("admin_token" = [])),
    request_body = CreateCameraRequest,
    responses(
        (status = 201, description = "Cámara creada", body = CameraResponse),
        (status = 401, description = "No autorizado"),
        (status = 409, description = "Path de cámara duplicado")
    )
)]
pub async fn create_camera(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCameraRequest>,
) -> Result<(StatusCode, Json<CameraResponse>), (StatusCode, String)> {
    let camera = state
        .camera_repo
        .create(NewCamera {
            path: req.path,
            rtsp_url: req.rtsp_url,
            record: req.record.unwrap_or(true),
            enabled: req.enabled.unwrap_or(true),
            description: req.description,
        })
        .await
        .map_err(repo_err)?;

    // Sync best-effort con MediaMTX (el reconcile periódico converge si falla).
    if let Err(e) = state.reconciler.apply_camera(&camera).await {
        warn!("no se pudo aplicar '{}' en MediaMTX: {}", camera.path, e);
    }
    Ok((StatusCode::CREATED, Json(camera.into())))
}

#[utoipa::path(
    get, path = "/admin/cameras/{id}", tag = "Administration",
    security(("admin_token" = [])),
    params(("id" = Uuid, Path, description = "ID de la cámara")),
    responses(
        (status = 200, description = "Cámara", body = CameraResponse),
        (status = 404, description = "No encontrada"),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn get_camera(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<CameraResponse>, (StatusCode, String)> {
    let camera = state
        .camera_repo
        .find_by_id(id)
        .await
        .map_err(repo_err)?
        .ok_or((StatusCode::NOT_FOUND, "cámara no encontrada".to_string()))?;
    Ok(Json(camera.into()))
}

#[utoipa::path(
    patch, path = "/admin/cameras/{id}", tag = "Administration",
    security(("admin_token" = [])),
    params(("id" = Uuid, Path, description = "ID de la cámara")),
    request_body = UpdateCameraRequest,
    responses(
        (status = 200, description = "Cámara actualizada", body = CameraResponse),
        (status = 404, description = "No encontrada"),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn update_camera(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCameraRequest>,
) -> Result<Json<CameraResponse>, (StatusCode, String)> {
    let mut camera = state
        .camera_repo
        .find_by_id(id)
        .await
        .map_err(repo_err)?
        .ok_or((StatusCode::NOT_FOUND, "cámara no encontrada".to_string()))?;

    if let Some(url) = req.rtsp_url {
        camera.rtsp_url = url;
    }
    if let Some(record) = req.record {
        camera.record = record;
    }
    if let Some(enabled) = req.enabled {
        camera.enabled = enabled;
    }
    if let Some(description) = req.description {
        camera.description = Some(description);
    }

    let updated = state.camera_repo.update(&camera).await.map_err(repo_err)?;

    // Best-effort: si quedó deshabilitada, quitarla de MediaMTX; si no, aplicarla.
    let sync = if updated.enabled {
        state.reconciler.apply_camera(&updated).await
    } else {
        state.reconciler.remove_camera(&updated.path).await
    };
    if let Err(e) = sync {
        warn!("sync de MediaMTX falló para '{}': {}", updated.path, e);
    }
    Ok(Json(updated.into()))
}

#[utoipa::path(
    delete, path = "/admin/cameras/{id}", tag = "Administration",
    security(("admin_token" = [])),
    params(("id" = Uuid, Path, description = "ID de la cámara")),
    responses(
        (status = 204, description = "Cámara eliminada"),
        (status = 404, description = "No encontrada"),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn delete_camera(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Cargamos primero para conocer el path y quitarlo de MediaMTX.
    let camera = state
        .camera_repo
        .find_by_id(id)
        .await
        .map_err(repo_err)?
        .ok_or((StatusCode::NOT_FOUND, "cámara no encontrada".to_string()))?;

    state.camera_repo.delete(id).await.map_err(repo_err)?;

    if let Err(e) = state.reconciler.remove_camera(&camera.path).await {
        warn!("no se pudo quitar '{}' de MediaMTX: {}", camera.path, e);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Construye la respuesta de proyecto (incluye sus cámaras asignadas).
async fn to_project_response(
    state: &AppState,
    project: Project,
) -> Result<ProjectResponse, (StatusCode, String)> {
    let camera_ids = state
        .project_repo
        .assigned_camera_ids(project.id)
        .await
        .map_err(repo_err)?;
    Ok(ProjectResponse {
        id: project.id,
        client_id: project.client_id,
        all_cameras: project.all_cameras,
        enabled: project.enabled,
        camera_ids,
        created_at: project.created_at,
        updated_at: project.updated_at,
    })
}

#[utoipa::path(
    get, path = "/admin/projects", tag = "Administration",
    security(("admin_token" = [])),
    responses(
        (status = 200, description = "Lista de proyectos", body = [ProjectResponse]),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ProjectResponse>>, (StatusCode, String)> {
    let projects = state.project_repo.list_all().await.map_err(repo_err)?;
    let mut out = Vec::with_capacity(projects.len());
    for project in projects {
        out.push(to_project_response(&state, project).await?);
    }
    Ok(Json(out))
}

#[utoipa::path(
    post, path = "/admin/projects", tag = "Administration",
    security(("admin_token" = [])),
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "Proyecto creado", body = ProjectResponse),
        (status = 401, description = "No autorizado"),
        (status = 409, description = "client_id duplicado")
    )
)]
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), (StatusCode, String)> {
    let secret_hash = crate::secret::hash_secret(&req.secret)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "error interno".to_string()))?;

    let project = state
        .project_repo
        .create(NewProject {
            client_id: req.client_id,
            secret_hash,
            all_cameras: req.all_cameras,
            enabled: true,
        })
        .await
        .map_err(repo_err)?;

    if !req.camera_ids.is_empty() {
        state
            .project_repo
            .set_cameras(project.id, &req.camera_ids)
            .await
            .map_err(repo_err)?;
    }

    let resp = to_project_response(&state, project).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

#[utoipa::path(
    get, path = "/admin/projects/{id}", tag = "Administration",
    security(("admin_token" = [])),
    params(("id" = Uuid, Path, description = "ID del proyecto")),
    responses(
        (status = 200, description = "Proyecto", body = ProjectResponse),
        (status = 404, description = "No encontrado"),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectResponse>, (StatusCode, String)> {
    let project = state
        .project_repo
        .find_by_id(id)
        .await
        .map_err(repo_err)?
        .ok_or((StatusCode::NOT_FOUND, "proyecto no encontrado".to_string()))?;
    Ok(Json(to_project_response(&state, project).await?))
}

#[utoipa::path(
    patch, path = "/admin/projects/{id}", tag = "Administration",
    security(("admin_token" = [])),
    params(("id" = Uuid, Path, description = "ID del proyecto")),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "Proyecto actualizado", body = ProjectResponse),
        (status = 404, description = "No encontrado"),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>, (StatusCode, String)> {
    let mut project = state
        .project_repo
        .find_by_id(id)
        .await
        .map_err(repo_err)?
        .ok_or((StatusCode::NOT_FOUND, "proyecto no encontrado".to_string()))?;

    if let Some(secret) = req.secret {
        project.secret_hash = crate::secret::hash_secret(&secret)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "error interno".to_string()))?;
    }
    if let Some(all_cameras) = req.all_cameras {
        project.all_cameras = all_cameras;
    }
    if let Some(enabled) = req.enabled {
        project.enabled = enabled;
    }

    let updated = state.project_repo.update(&project).await.map_err(repo_err)?;

    if let Some(camera_ids) = req.camera_ids {
        state
            .project_repo
            .set_cameras(updated.id, &camera_ids)
            .await
            .map_err(repo_err)?;
    }

    Ok(Json(to_project_response(&state, updated).await?))
}

#[utoipa::path(
    delete, path = "/admin/projects/{id}", tag = "Administration",
    security(("admin_token" = [])),
    params(("id" = Uuid, Path, description = "ID del proyecto")),
    responses(
        (status = 204, description = "Proyecto eliminado"),
        (status = 404, description = "No encontrado"),
        (status = 401, description = "No autorizado")
    )
)]
pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    state.project_repo.delete(id).await.map_err(repo_err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::is_authorized;

    #[test]
    fn empty_config_denies_all() {
        assert!(!is_authorized("", Some("Bearer x")));
    }

    #[test]
    fn correct_bearer_allows() {
        assert!(is_authorized("secret", Some("Bearer secret")));
    }

    #[test]
    fn wrong_token_denies() {
        assert!(!is_authorized("secret", Some("Bearer otro")));
    }

    #[test]
    fn missing_header_denies() {
        assert!(!is_authorized("secret", None));
    }

    #[test]
    fn missing_bearer_prefix_denies() {
        assert!(!is_authorized("secret", Some("secret")));
    }
}

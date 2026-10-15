//! Endpoint de consumo para proyectos (SIGAC/Odin, HU 4.7).
//!
//! Lista las cámaras a las que el proyecto tiene acceso, según SU JWT. El `id`
//! es el identificador ESTABLE que el consumidor referencia (no cambia aunque
//! cambien el path o la configuración). No se expone la rtsp_url.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{
    header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE},
    HeaderMap, StatusCode,
};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use jsonwebtoken::{Algorithm, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::models::Camera;
use crate::services::snapshot::{parse_width, SnapshotError};
use crate::{AppState, Claims, MtxPermission};

/// Referencia estable de cámara para consumidores.
#[derive(Serialize, ToSchema)]
pub struct CameraRef {
    /// Identificador estable de la cámara (no cambia aunque cambie el path).
    pub id: Uuid,
    /// Nombre de la ruta en el servidor de streaming (para construir la URL HLS).
    pub path: String,
    pub description: Option<String>,
}

impl From<Camera> for CameraRef {
    fn from(c: Camera) -> Self {
        Self {
            id: c.id,
            path: c.path,
            description: c.description,
        }
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/cameras", get(list_my_cameras))
        .route("/cameras/:path/snapshot", get(camera_snapshot))
}

/// Extrae el token del header `Authorization: Bearer <jwt>`.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers.get(AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")
}

/// Valida el Bearer JWT del proyecto con la clave pública (RS256 + exp).
fn validate_bearer(state: &AppState, headers: &HeaderMap) -> Option<Claims> {
    let token = bearer_token(headers)?;
    let validation = Validation::new(Algorithm::RS256);
    jsonwebtoken::decode::<Claims>(token, &state.decoding_key, &validation)
        .ok()
        .map(|data| data.claims)
}

/// Lista las cámaras a las que el proyecto (por su JWT) tiene acceso.
///
/// Contrato para consumidores (SIGAC/Odin): guarda el `id` (identificador
/// ESTABLE) como referencia y asócialo a tu contexto de negocio
/// (warehouse/location/alias). No dupliques la configuración ni las
/// credenciales de la cámara. La URL HLS se construye con `path`:
/// `https://<host>/<path>/index.m3u8?jwt=<token>`.
#[utoipa::path(
    get, path = "/cameras", tag = "Consumer",
    security(("project_jwt" = [])),
    responses(
        (status = 200, description = "Cámaras accesibles para el proyecto", body = [CameraRef]),
        (status = 401, description = "JWT ausente o inválido")
    )
)]
pub async fn list_my_cameras(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<CameraRef>>, StatusCode> {
    let claims = validate_bearer(&state, &headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let cameras = state
        .camera_repo
        .list_enabled()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out: Vec<CameraRef> = accessible(cameras, &claims.mediamtx_permissions)
        .into_iter()
        .map(CameraRef::from)
        .collect();
    Ok(Json(out))
}

/// Query de `GET /cameras/{path}/snapshot`. `width` llega como texto para que
/// un valor no numérico también responda 422 (y no el 400 genérico de axum).
#[derive(Deserialize)]
pub struct SnapshotQuery {
    width: Option<String>,
}

/// Imagen JPEG en vivo de una cámara (encuadre completo, sin recorte).
///
/// Requiere el permiso `read` sobre `path` en el JWT (un permiso con `path`
/// vacío equivale a todas las cámaras). `width` (160–1920, default 1280)
/// escala conservando la proporción. Nunca se cachea.
#[utoipa::path(
    get, path = "/cameras/{path}/snapshot", tag = "Consumer",
    security(("project_jwt" = [])),
    params(
        ("path" = String, Path, description = "Path de la cámara"),
        ("width" = Option<u32>, Query, description = "Ancho en px (160–1920, default 1280)")
    ),
    responses(
        (status = 200, description = "JPEG de la cámara", content_type = "image/jpeg", body = Vec<u8>),
        (status = 401, description = "JWT ausente, inválido o vencido"),
        (status = 403, description = "Sin permiso read sobre el path"),
        (status = 404, description = "Cámara inexistente"),
        (status = 422, description = "width fuera de rango"),
        (status = 502, description = "Fallo del decodificador"),
        (status = 504, description = "Sin frame en 10 s")
    )
)]
pub async fn camera_snapshot(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Query(query): Query<SnapshotQuery>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let claims = validate_bearer(&state, &headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let token = bearer_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let width = parse_width(query.width.as_deref()).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    if !can_read(&claims.mediamtx_permissions, &path) {
        return Err(StatusCode::FORBIDDEN);
    }
    ensure_camera_enabled(&state, &path).await?;

    let jpeg = state
        .snapshot
        .capture(&path, token, width)
        .await
        .map_err(|e| match e {
            SnapshotError::Timeout => StatusCode::GATEWAY_TIMEOUT,
            SnapshotError::Decoder => StatusCode::BAD_GATEWAY,
        })?;
    Ok(([(CONTENT_TYPE, "image/jpeg"), (CACHE_CONTROL, "no-store")], jpeg).into_response())
}

/// 404 si la cámara no existe o está deshabilitada (no hay stream que leer).
async fn ensure_camera_enabled(state: &AppState, path: &str) -> Result<(), StatusCode> {
    match state.camera_repo.find_by_path(path).await {
        Ok(Some(camera)) if camera.enabled => Ok(()),
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// ¿Hay permiso `read` sobre `path`? (`path` vacío en el permiso = todas).
fn can_read(permissions: &[MtxPermission], path: &str) -> bool {
    permissions
        .iter()
        .any(|p| p.action == "read" && (p.path.is_empty() || p.path == path))
}

/// Filtra las cámaras según los permisos `read` del token: coincide con lo
/// reproducible. `path` vacío en algún permiso `read` → acceso a todas.
fn accessible(cameras: Vec<Camera>, permissions: &[MtxPermission]) -> Vec<Camera> {
    let read_paths: Vec<&str> = permissions
        .iter()
        .filter(|p| p.action == "read")
        .map(|p| p.path.as_str())
        .collect();
    let all = read_paths.iter().any(|p| p.is_empty());
    let allowed: HashSet<&str> = read_paths.into_iter().filter(|p| !p.is_empty()).collect();
    cameras
        .into_iter()
        .filter(|c| all || allowed.contains(c.path.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::accessible;
    use crate::domain::models::Camera;
    use crate::MtxPermission;
    use chrono::Utc;
    use uuid::Uuid;

    fn cam(path: &str) -> Camera {
        Camera {
            id: Uuid::new_v4(),
            path: path.into(),
            rtsp_url: "rtsp://x".into(),
            record: true,
            enabled: true,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn perm(action: &str, path: &str) -> MtxPermission {
        MtxPermission {
            action: action.into(),
            path: path.into(),
        }
    }

    #[test]
    fn all_access_returns_everything() {
        let cams = vec![cam("a"), cam("b")];
        let perms = vec![perm("read", ""), perm("playback", "")];
        assert_eq!(accessible(cams, &perms).len(), 2);
    }

    #[test]
    fn specific_paths_filter() {
        let cams = vec![cam("a"), cam("b"), cam("c")];
        let perms = vec![
            perm("read", "a"),
            perm("playback", "a"),
            perm("read", "c"),
            perm("playback", "c"),
        ];
        let paths: Vec<String> = accessible(cams, &perms).into_iter().map(|c| c.path).collect();
        assert_eq!(paths, vec!["a".to_string(), "c".to_string()]);
    }

    #[test]
    fn no_read_permissions_returns_none() {
        let cams = vec![cam("a")];
        let perms = vec![perm("playback", "a")]; // sin 'read'
        assert!(accessible(cams, &perms).is_empty());
    }
}

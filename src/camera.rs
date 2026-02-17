use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};
use serde::Serialize;

use crate::models::{ApiResponse, Camera, CameraQuery};
use crate::AppState;

#[derive(Serialize)]
pub struct ThumbnailResponse {
    pub camera_id: i64,
    pub thumbnail_url: Option<String>,
}

pub async fn list_cameras(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CameraQuery>,
) -> Result<Json<ApiResponse<Vec<Camera>>>, (StatusCode, Json<ApiResponse<Vec<Camera>>>)> {
    match state.db.list_cameras(query.search.as_deref()) {
        Ok(cameras) => Ok(Json(ApiResponse::ok(cameras))),
        Err(e) => {
            warn!("Error listando cámaras: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string()))))
        }
    }
}

pub async fn get_camera(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<Camera>>, (StatusCode, Json<ApiResponse<Camera>>)> {
    match state.db.get_camera(id) {
        Ok(Some(cam)) => Ok(Json(ApiResponse::ok(cam))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Cámara no encontrada")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn create_camera(
    State(state): State<Arc<AppState>>,
    Json(cam): Json<Camera>,
) -> Result<(StatusCode, Json<ApiResponse<Camera>>), (StatusCode, Json<ApiResponse<Camera>>)> {
    if cam.name.is_empty() || cam.host.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("name y host son requeridos"))));
    }
    match state.db.create_camera(&cam) {
        Ok(id) => {
            info!("Cámara creada: {} (id={})", cam.name, id);
            
            // Sync to MediaMTX
            let _ = sync_camera_to_mediamtx(&state, &cam).await;
            
            // Start thumbnail capture background task
            let stream_name = format!("camera-{}", cam.name);
            crate::thumbnail::start_thumbnail_capture(id, &stream_name, "/app/data/thumbnails").await;
            
            match state.db.get_camera(id) {
                Ok(Some(created)) => Ok((StatusCode::CREATED, Json(ApiResponse::ok(created)))),
                _ => Ok((StatusCode::CREATED, Json(ApiResponse::ok(cam)))),
            }
        }
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") {
                "Ya existe una cámara con ese nombre".to_string()
            } else {
                e.to_string()
            };
            Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err(&msg))))
        }
    }
}

pub async fn update_camera(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(cam): Json<Camera>,
) -> Result<Json<ApiResponse<Camera>>, (StatusCode, Json<ApiResponse<Camera>>)> {
    if cam.name.is_empty() || cam.host.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("name y host son requeridos"))));
    }
    // Get old camera to remove from MediaMTX if name changed
    let old_cam = state.db.get_camera(id).ok().flatten();

    match state.db.update_camera(id, &cam) {
        Ok(true) => {
            info!("Cámara actualizada: {} (id={})", cam.name, id);
            // Remove old path from MediaMTX if name changed
            if let Some(old) = old_cam {
                if old.name != cam.name {
                    let _ = remove_camera_from_mediamtx(&state, &old.name).await;
                }
            }
            let _ = sync_camera_to_mediamtx(&state, &cam).await;
            match state.db.get_camera(id) {
                Ok(Some(updated)) => Ok(Json(ApiResponse::ok(updated))),
                _ => Ok(Json(ApiResponse::ok(cam))),
            }
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Cámara no encontrada")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn delete_camera(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let cam = state.db.get_camera(id).ok().flatten();
    match state.db.delete_camera(id) {
        Ok(true) => {
            info!("Cámara eliminada: id={}", id);
            if let Some(c) = cam {
                let _ = remove_camera_from_mediamtx(&state, &c.name).await;
            }
            Ok(Json(ApiResponse::ok("Cámara eliminada".to_string())))
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Cámara no encontrada")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn sync_all_cameras(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let cameras = state.db.list_cameras(None).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))
    })?;

    let mut synced = 0;
    let mut errors = 0;
    for cam in &cameras {
        if cam.enabled {
            match sync_camera_to_mediamtx(&state, cam).await {
                Ok(_) => synced += 1,
                Err(_) => errors += 1,
            }
        }
    }
    let msg = format!("Sync completado: {} cámaras sincronizadas, {} errores", synced, errors);
    info!("{}", msg);
    Ok(Json(ApiResponse::ok(msg)))
}

// =========================================================================
// Camera status (queries MediaMTX paths API)
// =========================================================================

#[derive(serde::Serialize)]
pub struct CameraStatus {
    pub name: String,
    pub ready: bool,
}

pub async fn camera_statuses(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<CameraStatus>>>, (StatusCode, Json<ApiResponse<Vec<CameraStatus>>>)> {
    let url = format!("{}/v3/paths/list", state.mediamtx_api_url);
    let client = &state.http_client;
    match client.get(&url)
        .basic_auth(&state.mediamtx_api_user, Some(&state.mediamtx_api_pass))
        .send().await
    {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let items = body.get("items").and_then(|v| v.as_array());
            let statuses: Vec<CameraStatus> = match items {
                Some(arr) => arr.iter().filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    let ready = item.get("ready").and_then(|v| v.as_bool()).unwrap_or(false);
                    Some(CameraStatus { name, ready })
                }).collect(),
                None => vec![],
            };
            Ok(Json(ApiResponse::ok(statuses)))
        }
        Ok(resp) => {
            let status = resp.status();
            warn!("MediaMTX paths API returned {}", status);
            Err((StatusCode::BAD_GATEWAY, Json(ApiResponse::err(&format!("MediaMTX API error: {}", status)))))
        }
        Err(e) => {
            warn!("Cannot reach MediaMTX paths API: {}", e);
            Err((StatusCode::BAD_GATEWAY, Json(ApiResponse::err("No se pudo conectar a MediaMTX"))))
        }
    }
}

// =========================================================================
// MediaMTX API integration
// =========================================================================

async fn sync_camera_to_mediamtx(state: &AppState, cam: &Camera) -> Result<(), String> {
    if !cam.enabled {
        return Ok(());
    }

    let url = format!("{}/v3/config/paths/add/{}", state.mediamtx_api_url, cam.name);
    let body = serde_json::json!({
        "source": cam.rtsp_url(),
        "sourceOnDemand": cam.source_on_demand,
        "sourceOnDemandStartTimeout": "10s",
        "sourceOnDemandCloseAfter": "30s",
        "record": cam.record,
        "recordPath": "/recordings/%path/%Y-%m-%d_%H-%M-%S",
        "recordFormat": "fmp4",
        "recordPartDuration": "5s",
        "recordSegmentDuration": "1h",
        "recordDeleteAfter": "24h",
        "overridePublisher": false
    });

    let client = &state.http_client;
    match client.post(&url).basic_auth(&state.mediamtx_api_user, Some(&state.mediamtx_api_pass)).json(&body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Cámara {} sincronizada con MediaMTX", cam.name);
                Ok(())
            } else {
                // Try PATCH (edit) if add fails (path already exists)
                let edit_url = format!("{}/v3/config/paths/patch/{}", state.mediamtx_api_url, cam.name);
                match client.patch(&edit_url).basic_auth(&state.mediamtx_api_user, Some(&state.mediamtx_api_pass)).json(&body).send().await {
                    Ok(resp2) if resp2.status().is_success() => {
                        info!("Cámara {} actualizada en MediaMTX", cam.name);
                        Ok(())
                    }
                    _ => {
                        warn!("Error sincronizando cámara {} con MediaMTX", cam.name);
                        Err("Error sincronizando con MediaMTX".to_string())
                    }
                }
            }
        }
        Err(e) => {
            warn!("No se pudo conectar a MediaMTX API: {}", e);
            Err(e.to_string())
        }
    }
}

/// Get latest thumbnail for a camera
pub async fn get_camera_thumbnail(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<ApiResponse<ThumbnailResponse>> {
    let thumbnail_url = crate::thumbnail::get_latest_thumbnail(id, "/app/data/thumbnails").await;
    
    Json(ApiResponse::ok(ThumbnailResponse {
        camera_id: id,
        thumbnail_url,
    }))
}

async fn remove_camera_from_mediamtx(state: &AppState, name: &str) -> Result<(), String> {
    let url = format!("{}/v3/config/paths/delete/{}", state.mediamtx_api_url, name);
    let client = &state.http_client;
    match client.delete(&url).basic_auth(&state.mediamtx_api_user, Some(&state.mediamtx_api_pass)).send().await {
        Ok(_) => {
            info!("Cámara {} removida de MediaMTX", name);
            Ok(())
        }
        Err(e) => {
            warn!("Error removiendo cámara de MediaMTX: {}", e);
            Err(e.to_string())
        }
    }
}

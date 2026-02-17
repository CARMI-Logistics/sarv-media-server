use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::{ApiResponse, Capture, CaptureQuery};
use crate::AppState;

/// Take a screenshot of a camera stream via ffmpeg
pub async fn take_screenshot(
    State(state): State<Arc<AppState>>,
    Path(camera_id): Path<i64>,
) -> Result<Json<ApiResponse<Capture>>, (StatusCode, Json<ApiResponse<Capture>>)> {
    let cam = match state.db.get_camera(camera_id) {
        Ok(Some(c)) => c,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Cámara no encontrada")))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    };

    let now = time::OffsetDateTime::now_utc();
    let date_str = now.format(&time::format_description::parse("[year]-[month]-[day]").unwrap()).unwrap_or_default();
    let time_str = now.format(&time::format_description::parse("[hour]-[minute]-[second]").unwrap()).unwrap_or_default();

    let dir = format!("/app/data/captures/{}/{}", cam.name, date_str);
    std::fs::create_dir_all(&dir).ok();

    let filename = format!("screenshot_{}.jpg", time_str);
    let file_path = format!("{}/{}", dir, filename);
    let relative_path = format!("{}/{}/{}", cam.name, date_str, filename);

    // Build stream URL - try HLS first for snapshot
    let stream_url = format!("http://localhost:8888/{}/", cam.name);

    let output = tokio::process::Command::new("ffmpeg")
        .args(["-y", "-i", &stream_url, "-frames:v", "1", "-q:v", "2", &file_path])
        .arg("-timeout").arg("5000000")
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let file_size = std::fs::metadata(&file_path).map(|m| m.len() as i64).unwrap_or(0);
            match state.db.create_capture(camera_id, &cam.name, "screenshot", &relative_path, file_size) {
                Ok(id) => {
                    info!("Screenshot taken for camera {} -> {}", cam.name, relative_path);
                    let capture = Capture {
                        id,
                        camera_id,
                        camera_name: cam.name,
                        capture_type: "screenshot".to_string(),
                        file_path: relative_path,
                        file_size,
                        created_at: Some(now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()),
                    };
                    Ok(Json(ApiResponse::ok(capture)))
                }
                Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            warn!("FFmpeg screenshot failed for {}: {}", cam.name, stderr);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&format!("Error capturando screenshot: {}", stderr.chars().take(200).collect::<String>())))))
        }
        Err(e) => {
            warn!("Failed to run ffmpeg: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("ffmpeg no disponible"))))
        }
    }
}

/// List captures filtered by camera_id, date, type
pub async fn list_captures(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CaptureQuery>,
) -> Result<Json<ApiResponse<Vec<Capture>>>, (StatusCode, Json<ApiResponse<Vec<Capture>>>)> {
    match state.db.list_captures(query.camera_id, query.date.as_deref(), query.capture_type.as_deref()) {
        Ok(captures) => Ok(Json(ApiResponse::ok(captures))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

/// Delete a capture (file + DB record)
pub async fn delete_capture(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match state.db.delete_capture(id) {
        Ok(Some(path)) => {
            let full_path = format!("/app/data/captures/{}", path);
            std::fs::remove_file(&full_path).ok();
            info!("Capture deleted: {}", path);
            Ok(Json(ApiResponse::ok("Captura eliminada".to_string())))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Captura no encontrada")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

/// Get thumbnail for a camera (latest auto-generated thumbnail)
pub async fn get_thumbnail(
    State(state): State<Arc<AppState>>,
    Path(camera_id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let cam = match state.db.get_camera(camera_id) {
        Ok(Some(c)) => c,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Cámara no encontrada")))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    };

    let thumb_path = format!("/app/data/thumbnails/{}.jpg", cam.name);
    if std::path::Path::new(&thumb_path).exists() {
        Ok(Json(ApiResponse::ok(format!("/data/thumbnails/{}.jpg", cam.name))))
    } else {
        Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Thumbnail no disponible"))))
    }
}

/// Toggle thumbnail generation on/off
pub async fn toggle_thumbnails(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ApiResponse<bool>>)> {
    let current = state.db.get_setting("thumbnails_enabled").unwrap_or(None).unwrap_or_else(|| "true".to_string());
    let new_val = if current == "true" { "false" } else { "true" };
    state.db.set_setting("thumbnails_enabled", new_val).ok();
    let enabled = new_val == "true";
    info!("Thumbnails toggled to: {}", enabled);
    Ok(Json(ApiResponse::ok(enabled)))
}

/// Get current thumbnail setting
pub async fn get_thumbnail_setting(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<bool>> {
    let enabled = state.db.get_setting("thumbnails_enabled").unwrap_or(None).unwrap_or_else(|| "true".to_string()) == "true";
    Json(ApiResponse::ok(enabled))
}

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, warn, error};

use crate::models::{ApiResponse, CreateMosaicRequest, UpdateMosaicRequest, MosaicWithCameras};
use crate::AppState;

pub async fn list_mosaics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<MosaicWithCameras>>>, (StatusCode, Json<ApiResponse<Vec<MosaicWithCameras>>>)> {
    match state.db.list_mosaics() {
        Ok(mosaics) => Ok(Json(ApiResponse::ok(mosaics))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn get_mosaic(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<MosaicWithCameras>>, (StatusCode, Json<ApiResponse<MosaicWithCameras>>)> {
    match state.db.get_mosaic(id) {
        Ok(Some(m)) => Ok(Json(ApiResponse::ok(m))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Mosaico no encontrado")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn create_mosaic(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMosaicRequest>,
) -> Result<(StatusCode, Json<ApiResponse<MosaicWithCameras>>), (StatusCode, Json<ApiResponse<MosaicWithCameras>>)> {
    if req.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("name es requerido"))));
    }
    if req.camera_ids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("Se requiere al menos una cámara"))));
    }

    let max_cameras = layout_max_cameras(&req.layout);
    if req.camera_ids.len() > max_cameras {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err(
            &format!("Layout {} soporta máximo {} cámaras, se enviaron {}", req.layout, max_cameras, req.camera_ids.len())
        ))));
    }

    match state.db.create_mosaic(&req.name, &req.layout, &req.camera_ids) {
        Ok(id) => {
            info!("Mosaico creado: {} (id={})", req.name, id);
            match state.db.get_mosaic(id) {
                Ok(Some(m)) => Ok((StatusCode::CREATED, Json(ApiResponse::ok(m)))),
                _ => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error obteniendo mosaico creado")))),
            }
        }
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") {
                "Ya existe un mosaico con ese nombre".to_string()
            } else {
                e.to_string()
            };
            Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err(&msg))))
        }
    }
}

pub async fn update_mosaic(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateMosaicRequest>,
) -> Result<Json<ApiResponse<MosaicWithCameras>>, (StatusCode, Json<ApiResponse<MosaicWithCameras>>)> {
    match state.db.update_mosaic(id, &req.name, &req.layout, &req.camera_ids) {
        Ok(true) => {
            info!("Mosaico actualizado: {} (id={})", req.name, id);
            match state.db.get_mosaic(id) {
                Ok(Some(m)) => Ok(Json(ApiResponse::ok(m))),
                _ => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error obteniendo mosaico")))),
            }
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Mosaico no encontrado")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn delete_mosaic(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    // Stop if active
    if let Ok(Some(m)) = state.db.get_mosaic(id) {
        if m.mosaic.active {
            let _ = stop_ffmpeg(m.mosaic.pid).await;
            let _ = state.db.set_mosaic_active(id, false, None);
        }
    }
    match state.db.delete_mosaic(id) {
        Ok(true) => {
            info!("Mosaico eliminado: id={}", id);
            Ok(Json(ApiResponse::ok("Mosaico eliminado".to_string())))
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Mosaico no encontrado")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

// =========================================================================
// Start / Stop mosaic FFmpeg process
// =========================================================================

pub async fn start_mosaic(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let mosaic = state.db.get_mosaic(id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))
    })?.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiResponse::err("Mosaico no encontrado")))
    })?;

    if mosaic.mosaic.active {
        return Err((StatusCode::CONFLICT, Json(ApiResponse::err("Mosaico ya está activo"))));
    }

    if mosaic.cameras.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("Mosaico no tiene cámaras asignadas"))));
    }

    let (cols, rows) = parse_layout(&mosaic.mosaic.layout);
    let num_cameras = mosaic.cameras.len();
    let cell_w: u32 = 640;
    let cell_h: u32 = 360;

    // Build FFmpeg command
    let mut args: Vec<String> = vec!["-y".to_string()];

    // Input streams
    for cam in &mosaic.cameras {
        let url = if cam.username.is_empty() {
            format!("{}://{}:{}{}", cam.protocol, cam.host, cam.port, cam.path)
        } else {
            format!("{}://{}:{}@{}:{}{}", cam.protocol, cam.username, cam.password, cam.host, cam.port, cam.path)
        };
        args.extend_from_slice(&[
            "-rtsp_transport".to_string(), "tcp".to_string(),
            "-timeout".to_string(), "5000000".to_string(),
            "-i".to_string(), url,
        ]);
    }

    // Build filter_complex
    let total_cells = (cols * rows) as usize;

    // For 1x1 layout, skip xstack (FFmpeg xstack needs >= 2 inputs)
    let filter_and_map = if total_cells <= 1 {
        let filter = format!("[0:v]scale={}:{},setpts=PTS-STARTPTS[out]", cell_w, cell_h);
        (filter, "[out]".to_string())
    } else {
        let mut filter = String::new();

        // Scale each real camera input
        for i in 0..num_cameras {
            filter.push_str(&format!("[{}:v]scale={}:{},setpts=PTS-STARTPTS[v{}];", i, cell_w, cell_h, i));
        }

        // Add black padding for empty cells
        if num_cameras < total_cells {
            for i in num_cameras..total_cells {
                filter.push_str(&format!(
                    "color=black:s={}x{}:r=1[v{}];",
                    cell_w, cell_h, i
                ));
            }
        }

        // Build xstack inputs and layout using explicit pixel positions
        let inputs_list: Vec<String> = (0..total_cells).map(|i| format!("[v{}]", i)).collect();
        filter.push_str(&inputs_list.join(""));

        let mut layout_parts = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let x = c * cell_w;
                let y = r * cell_h;
                layout_parts.push(format!("{}_{}", x, y));
            }
        }

        filter.push_str(&format!(
            "xstack=inputs={}:layout={}:fill=black[out]",
            total_cells,
            layout_parts.join("|")
        ));
        (filter, "[out]".to_string())
    };

    // Derive RTSP host from MediaMTX API URL (works in Docker and locally)
    let rtsp_host = state.mediamtx_api_url
        .replace("http://", "")
        .replace("https://", "")
        .split(':')
        .next()
        .unwrap_or("mediamtx")
        .to_string();
    let output_path = format!("mosaic-{}", mosaic.mosaic.name);
    let output_url = format!("rtsp://{}:8554/{}", rtsp_host, output_path);

    args.extend_from_slice(&[
        "-filter_complex".to_string(), filter_and_map.0,
        "-map".to_string(), filter_and_map.1,
        "-c:v".to_string(), "libx264".to_string(),
        "-preset".to_string(), "ultrafast".to_string(),
        "-tune".to_string(), "zerolatency".to_string(),
        "-g".to_string(), "30".to_string(),
        "-an".to_string(),
        "-f".to_string(), "rtsp".to_string(),
        "-rtsp_transport".to_string(), "tcp".to_string(),
        output_url.clone(),
    ]);

    info!("Iniciando mosaico FFmpeg: {} con {} cámaras ({})", mosaic.mosaic.name, num_cameras, mosaic.mosaic.layout);

    match Command::new("ffmpeg")
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => {
            let pid: u32 = child.id().unwrap_or(0);
            let _ = state.db.set_mosaic_active(id, true, Some(pid));
            info!("Mosaico {} iniciado con PID {} -> {}", mosaic.mosaic.name, pid, output_url);
            let msg = format!("Mosaico iniciado. Stream: http://localhost:8888/{}/index.m3u8", output_path);
            Ok(Json(ApiResponse::ok(msg)))
        }
        Err(e) => {
            error!("Error iniciando FFmpeg: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&format!("Error iniciando FFmpeg: {}", e)))))
        }
    }
}

pub async fn stop_mosaic(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let mosaic = state.db.get_mosaic(id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))
    })?.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiResponse::err("Mosaico no encontrado")))
    })?;

    if !mosaic.mosaic.active {
        return Err((StatusCode::CONFLICT, Json(ApiResponse::err("Mosaico no está activo"))));
    }

    match stop_ffmpeg(mosaic.mosaic.pid).await {
        Ok(_) => {
            let _ = state.db.set_mosaic_active(id, false, None);
            info!("Mosaico {} detenido", mosaic.mosaic.name);
            Ok(Json(ApiResponse::ok("Mosaico detenido".to_string())))
        }
        Err(e) => {
            // Force deactivate anyway
            let _ = state.db.set_mosaic_active(id, false, None);
            warn!("Error deteniendo FFmpeg (desactivado de todas formas): {}", e);
            Ok(Json(ApiResponse::ok("Mosaico marcado como detenido".to_string())))
        }
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn parse_layout(layout: &str) -> (u32, u32) {
    let parts: Vec<&str> = layout.split('x').collect();
    if parts.len() == 2 {
        let cols = parts[0].parse::<u32>().unwrap_or(2);
        let rows = parts[1].parse::<u32>().unwrap_or(2);
        (cols, rows)
    } else {
        (2, 2)
    }
}

fn layout_max_cameras(layout: &str) -> usize {
    let (cols, rows) = parse_layout(layout);
    (cols * rows) as usize
}

async fn stop_ffmpeg(pid: Option<i64>) -> Result<(), String> {
    if let Some(pid) = pid {
        info!("Deteniendo FFmpeg PID: {}", pid);
        let result = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output()
            .await;
        match result {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(format!("kill failed: {:?}", output.status)),
            Err(e) => Err(format!("Error ejecutando kill: {}", e)),
        }
    } else {
        Err("No PID disponible".to_string())
    }
}

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::info;

use crate::models::{ApiResponse, MosaicShare, CreateMosaicShareRequest};
use crate::AppState;

/// Create a share link for a mosaic
pub async fn create_share(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMosaicShareRequest>,
) -> Result<(StatusCode, Json<ApiResponse<MosaicShare>>), (StatusCode, Json<ApiResponse<MosaicShare>>)> {
    // Validate mosaic exists
    let mosaic = match state.db.get_mosaic(req.mosaic_id) {
        Ok(Some(m)) => m,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Mosaico no encontrado")))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    };

    if req.emails.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("Se requiere al menos un email"))));
    }
    if req.duration_hours < 1 {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("La duración mínima es 1 hora"))));
    }

    // Generate token
    let token: String = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..32).map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 { (b'0' + idx) as char } else { (b'a' + idx - 10) as char }
        }).collect()
    };

    // Calculate expires_at
    let now = time::OffsetDateTime::now_utc();
    let expires = now + time::Duration::hours(req.duration_hours);
    let expires_str = expires.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();

    let emails_str = req.emails.join(",");

    match state.db.create_mosaic_share(
        req.mosaic_id,
        &mosaic.mosaic.name,
        &token,
        &emails_str,
        &expires_str,
        req.schedule_start.as_deref(),
        req.schedule_end.as_deref(),
    ) {
        Ok(id) => {
            info!("Mosaic share created: mosaic={} token={} emails={}", mosaic.mosaic.name, token, emails_str);
            
            // Send email notification to all recipients
            let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
            let share_link = format!("{}/shared/{}", frontend_url, token);
            
            for email in req.emails.iter() {
                let _ = state.email_service.send_mosaic_share(
                    email,
                    &mosaic.mosaic.name,
                    &share_link,
                    &expires_str
                ).await;
            }
            
            let share = MosaicShare {
                id,
                mosaic_id: req.mosaic_id,
                mosaic_name: mosaic.mosaic.name.clone(),
                token: token.clone(),
                emails: emails_str,
                expires_at: expires_str,
                schedule_start: req.schedule_start,
                schedule_end: req.schedule_end,
                active: true,
                created_at: Some(now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()),
            };
            Ok((StatusCode::CREATED, Json(ApiResponse::ok(share))))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

#[derive(serde::Deserialize)]
pub struct ShareListQuery {
    pub mosaic_id: Option<i64>,
}

/// List all shares (optionally filtered by mosaic_id)
pub async fn list_shares(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ShareListQuery>,
) -> Result<Json<ApiResponse<Vec<MosaicShare>>>, (StatusCode, Json<ApiResponse<Vec<MosaicShare>>>)> {
    match state.db.list_mosaic_shares(query.mosaic_id) {
        Ok(shares) => Ok(Json(ApiResponse::ok(shares))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

/// Delete a share
pub async fn delete_share(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match state.db.delete_mosaic_share(id) {
        Ok(true) => {
            info!("Mosaic share deleted: id={}", id);
            Ok(Json(ApiResponse::ok("Enlace eliminado".to_string())))
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Enlace no encontrado")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

/// Toggle share active/inactive
pub async fn toggle_share(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match state.db.toggle_mosaic_share(id) {
        Ok(true) => Ok(Json(ApiResponse::ok("Estado actualizado".to_string()))),
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Enlace no encontrado")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

/// Public endpoint: validate a share token and return mosaic info if valid
/// This does NOT require JWT auth - it's the public share access point
pub async fn validate_share(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<Json<ApiResponse<ShareAccess>>, (StatusCode, Json<ApiResponse<ShareAccess>>)> {
    let share = match state.db.get_mosaic_share_by_token(&token) {
        Ok(Some(s)) => s,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Enlace no válido")))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    };

    if !share.active {
        return Err((StatusCode::FORBIDDEN, Json(ApiResponse::err("Este enlace ha sido desactivado"))));
    }

    // Check expiration
    let now = time::OffsetDateTime::now_utc();
    if let Ok(expires) = time::OffsetDateTime::parse(&share.expires_at, &time::format_description::well_known::Rfc3339) {
        if now > expires {
            return Err((StatusCode::FORBIDDEN, Json(ApiResponse::err("Este enlace ha expirado"))));
        }
    }

    // Check schedule window
    if let (Some(start), Some(end)) = (&share.schedule_start, &share.schedule_end) {
        let current_time = now.format(&time::format_description::parse("[hour]:[minute]").unwrap()).unwrap_or_default();
        if current_time < *start || current_time > *end {
            return Err((StatusCode::FORBIDDEN, Json(ApiResponse::err(&format!(
                "Este mosaico solo está disponible entre {} y {}", start, end
            )))));
        }
    }

    // Get the mosaic stream info
    let mosaic = state.db.get_mosaic(share.mosaic_id).ok().flatten();
    let stream_name = format!("mosaic-{}", share.mosaic_name);

    Ok(Json(ApiResponse::ok(ShareAccess {
        mosaic_name: share.mosaic_name,
        stream_name,
        expires_at: share.expires_at,
        schedule_start: share.schedule_start,
        schedule_end: share.schedule_end,
        is_active: mosaic.map(|m| m.mosaic.active).unwrap_or(false),
    })))
}

#[derive(serde::Serialize)]
pub struct ShareAccess {
    pub mosaic_name: String,
    pub stream_name: String,
    pub expires_at: String,
    pub schedule_start: Option<String>,
    pub schedule_end: Option<String>,
    pub is_active: bool,
}

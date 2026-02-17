use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::info;

use crate::models::{ApiResponse, Notification};
use crate::AppState;

pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Notification>>>, (StatusCode, Json<ApiResponse<Vec<Notification>>>)> {
    match state.db.list_notifications(100) {
        Ok(notifs) => Ok(Json(ApiResponse::ok(notifs))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

#[derive(serde::Serialize)]
pub struct NotificationSummary {
    pub unread_count: i64,
    pub notifications: Vec<Notification>,
}

pub async fn get_notification_summary(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<NotificationSummary>>, (StatusCode, Json<ApiResponse<NotificationSummary>>)> {
    let count = state.db.unread_notification_count().unwrap_or(0);
    let notifs = state.db.list_notifications(20).unwrap_or_default();
    Ok(Json(ApiResponse::ok(NotificationSummary {
        unread_count: count,
        notifications: notifs,
    })))
}

pub async fn mark_read(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match state.db.mark_notification_read(id) {
        Ok(true) => Ok(Json(ApiResponse::ok("Marcada como leída".to_string()))),
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Notificación no encontrada")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn mark_all_read(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match state.db.mark_all_notifications_read() {
        Ok(()) => Ok(Json(ApiResponse::ok("Todas marcadas como leídas".to_string()))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn delete_notification(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match state.db.delete_notification(id) {
        Ok(true) => {
            info!("Notification deleted: id={}", id);
            Ok(Json(ApiResponse::ok("Notificación eliminada".to_string())))
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Notificación no encontrada")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

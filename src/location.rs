use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use tracing::{info, warn};
use std::sync::Arc;

use crate::{
    models::{Location, Area, AreaWithLocation, ApiResponse},
    AppState,
};

// ============================================================================
// Location endpoints
// ============================================================================

pub async fn list_locations(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Location>>>, (StatusCode, Json<ApiResponse<Vec<Location>>>)> {
    match state.db.list_locations() {
        Ok(locations) => Ok(Json(ApiResponse {
            success: true,
            data: Some(locations),
            error: None,
        })),
        Err(e) => {
            warn!("Error listando locations: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(vec![]),
                    error: Some("Error listando locations".to_string()),
                }),
            ))
        }
    }
}

pub async fn create_location(
    State(state): State<Arc<AppState>>,
    Json(location): Json<Location>,
) -> Result<Json<ApiResponse<i64>>, (StatusCode, Json<ApiResponse<i64>>)> {
    if location.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: Some(0),
                error: Some("El nombre de la ubicación es requerido".to_string()),
            }),
        ));
    }

    match state.db.create_location(&location) {
        Ok(id) => {
            info!("Location creada: {} (ID: {})", location.name, id);
            Ok(Json(ApiResponse {
                success: true,
                data: Some(id),
                error: None,
            }))
        }
        Err(e) => {
            warn!("Error creando location: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(0),
                    error: Some("Error creando location".to_string()),
                }),
            ))
        }
    }
}

pub async fn update_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(location): Json<Location>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ApiResponse<bool>>)> {
    if location.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: Some(false),
                error: Some("El nombre de la ubicación es requerido".to_string()),
            }),
        ));
    }

    match state.db.update_location(id, &location) {
        Ok(true) => {
            info!("Location actualizada: {} (ID: {})", location.name, id);
            Ok(Json(ApiResponse {
                success: true,
                data: Some(true),
                error: None,
            }))
        }
        Ok(false) => {
            warn!("Location no encontrada: {}", id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Location no encontrada".to_string()),
                }),
            ))
        }
        Err(e) => {
            warn!("Error actualizando location: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Error actualizando location".to_string()),
                }),
            ))
        }
    }
}

pub async fn delete_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ApiResponse<bool>>)> {
    // Check if location is a system location
    match state.db.get_location(id) {
        Ok(Some(loc)) if loc.is_system => {
            warn!("Intento de eliminar location del sistema: {}", id);
            return Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("No se pueden eliminar ubicaciones del sistema".to_string()),
                }),
            ));
        }
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Location no encontrada".to_string()),
                }),
            ));
        }
        Err(e) => {
            warn!("Error verificando location: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Error verificando location".to_string()),
                }),
            ));
        }
        _ => {}
    }

    match state.db.delete_location(id) {
        Ok(true) => {
            info!("Location eliminada: {}", id);
            Ok(Json(ApiResponse {
                success: true,
                data: Some(true),
                error: None,
            }))
        }
        Ok(false) => {
            warn!("Location no encontrada: {}", id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Location no encontrada".to_string()),
                }),
            ))
        }
        Err(e) => {
            warn!("Error eliminando location: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Error eliminando location".to_string()),
                }),
            ))
        }
    }
}

// ============================================================================
// Area endpoints
// ============================================================================

pub async fn list_areas(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<AreaWithLocation>>>, (StatusCode, Json<ApiResponse<Vec<AreaWithLocation>>>)> {
    match state.db.list_areas() {
        Ok(areas) => Ok(Json(ApiResponse {
            success: true,
            data: Some(areas),
            error: None,
        })),
        Err(e) => {
            warn!("Error listando areas: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(vec![]),
                    error: Some("Error listando areas".to_string()),
                }),
            ))
        }
    }
}

pub async fn list_areas_by_location(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<Area>>>, (StatusCode, Json<ApiResponse<Vec<Area>>>)> {
    match state.db.list_areas_by_location(location_id) {
        Ok(areas) => Ok(Json(ApiResponse {
            success: true,
            data: Some(areas),
            error: None,
        })),
        Err(e) => {
            warn!("Error listando areas por location: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(vec![]),
                    error: Some("Error listando areas".to_string()),
                }),
            ))
        }
    }
}

pub async fn create_area(
    State(state): State<Arc<AppState>>,
    Json(area): Json<Area>,
) -> Result<Json<ApiResponse<i64>>, (StatusCode, Json<ApiResponse<i64>>)> {
    if area.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: Some(0),
                error: Some("El nombre del área es requerido".to_string()),
            }),
        ));
    }

    if area.location_id <= 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: Some(0),
                error: Some("La ubicación es requerida".to_string()),
            }),
        ));
    }

    match state.db.create_area(&area) {
        Ok(id) => {
            info!("Area creada: {} (ID: {})", area.name, id);
            Ok(Json(ApiResponse {
                success: true,
                data: Some(id),
                error: None,
            }))
        }
        Err(e) => {
            warn!("Error creando area: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(0),
                    error: Some("Error creando area".to_string()),
                }),
            ))
        }
    }
}

pub async fn update_area(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(area): Json<Area>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ApiResponse<bool>>)> {
    if area.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: Some(false),
                error: Some("El nombre del área es requerido".to_string()),
            }),
        ));
    }

    if area.location_id <= 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: Some(false),
                error: Some("La ubicación es requerida".to_string()),
            }),
        ));
    }

    match state.db.update_area(id, &area) {
        Ok(true) => {
            info!("Area actualizada: {} (ID: {})", area.name, id);
            Ok(Json(ApiResponse {
                success: true,
                data: Some(true),
                error: None,
            }))
        }
        Ok(false) => {
            warn!("Area no encontrada: {}", id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Area no encontrada".to_string()),
                }),
            ))
        }
        Err(e) => {
            warn!("Error actualizando area: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Error actualizando area".to_string()),
                }),
            ))
        }
    }
}

pub async fn delete_area(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ApiResponse<bool>>)> {
    match state.db.delete_area(id) {
        Ok(true) => {
            info!("Area eliminada: {}", id);
            Ok(Json(ApiResponse {
                success: true,
                data: Some(true),
                error: None,
            }))
        }
        Ok(false) => {
            warn!("Area no encontrada: {}", id);
            Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Area no encontrada".to_string()),
                }),
            ))
        }
        Err(e) => {
            warn!("Error eliminando area: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    data: Some(false),
                    error: Some("Error eliminando area".to_string()),
                }),
            ))
        }
    }
}

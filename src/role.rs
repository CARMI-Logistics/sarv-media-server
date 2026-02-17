use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::info;

use crate::models::{ApiResponse, RoleWithPermissions, CreateRoleRequest, UpdateRoleRequest};
use crate::AppState;

pub async fn list_roles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<RoleWithPermissions>>>, (StatusCode, Json<ApiResponse<Vec<RoleWithPermissions>>>)> {
    match state.db.list_roles() {
        Ok(roles) => Ok(Json(ApiResponse::ok(roles))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

pub async fn create_role(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<(StatusCode, Json<ApiResponse<String>>), (StatusCode, Json<ApiResponse<String>>)> {
    if req.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("El nombre es requerido"))));
    }
    match state.db.create_role(&req.name, &req.description, &req.permissions) {
        Ok(id) => {
            info!("Role created: {} (id={})", req.name, id);
            Ok((StatusCode::CREATED, Json(ApiResponse::ok(format!("Rol creado con id {}", id)))))
        }
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") { "Ya existe un rol con ese nombre" } else { "Error creando rol" };
            Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err(msg))))
        }
    }
}

pub async fn update_role(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    if req.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("El nombre es requerido"))));
    }
    match state.db.update_role(id, &req.name, &req.description, &req.permissions) {
        Ok(true) => {
            info!("Role updated: id={}", id);
            Ok(Json(ApiResponse::ok("Rol actualizado".to_string())))
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Rol no encontrado")))),
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") { "Ya existe un rol con ese nombre" } else { "Error actualizando rol" };
            Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err(msg))))
        }
    }
}

pub async fn delete_role(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match state.db.delete_role(id) {
        Ok(true) => {
            info!("Role deleted: id={}", id);
            Ok(Json(ApiResponse::ok("Rol eliminado".to_string())))
        }
        Ok(false) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("No se puede eliminar un rol del sistema")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

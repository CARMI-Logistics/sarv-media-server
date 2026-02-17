use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::{ApiResponse, UserPublic, CreateUserRequest, UpdateUserRequest, ForgotPasswordRequest, ResetPasswordRequest};
use crate::AppState;

// =========================================================================
// User CRUD (admin only)
// =========================================================================

pub async fn list_users(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<UserPublic>>>, (StatusCode, Json<ApiResponse<Vec<UserPublic>>>)> {
    match state.db.list_users() {
        Ok(users) => Ok(Json(ApiResponse::ok(users))),
        Err(e) => {
            warn!("Error listing users: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string()))))
        }
    }
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserPublic>>), (StatusCode, Json<ApiResponse<UserPublic>>)> {
    if req.username.is_empty() || req.email.is_empty() || req.password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("username, email y password son requeridos"))));
    }

    // SOC2 Security: Validate email format
    if !crate::security::is_valid_email(&req.email) {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("Formato de email inválido"))));
    }

    // SOC2 Security: Validate password strength
    if let Err(msg) = crate::security::validate_password_strength(&req.password) {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err(&msg))));
    }

    let hash = match bcrypt::hash(&req.password, 10) {
        Ok(h) => h,
        Err(e) => {
            warn!("Bcrypt hash error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error procesando contraseña"))));
        }
    };

    match state.db.create_user(&req.username, &req.email, &hash, &req.role) {
        Ok(id) => {
            info!("User created: {} (id={})", req.username, id);
            
            // Send welcome email
            let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
            let _ = state.email_service.send_welcome_email(&req.email, &req.username, &frontend_url).await;
            
            // Audit log for SOC2 compliance
            crate::security::AuditLog::new(
                &req.username,
                "CREATE_USER",
                &format!("user/{}", id),
                "system",
                "success"
            ).log();
            
            match state.db.get_user_by_id(id) {
                Ok(Some(u)) => Ok((StatusCode::CREATED, Json(ApiResponse::ok(UserPublic::from(u))))),
                _ => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error obteniendo usuario creado")))),
            }
        }
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") {
                "El nombre de usuario o email ya existe"
            } else {
                "Error creando usuario"
            };
            Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err(msg))))
        }
    }
}

pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserPublic>>, (StatusCode, Json<ApiResponse<UserPublic>>)> {
    let password_hash = match &req.password {
        Some(pw) if !pw.is_empty() => {
            match bcrypt::hash(pw, 10) {
                Ok(h) => Some(h),
                Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error procesando contraseña")))),
            }
        }
        _ => None,
    };

    match state.db.update_user(id, &req.username, &req.email, password_hash.as_deref(), &req.role, req.active) {
        Ok(true) => {
            info!("User updated: id={}", id);
            match state.db.get_user_by_id(id) {
                Ok(Some(u)) => Ok(Json(ApiResponse::ok(UserPublic::from(u)))),
                _ => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error obteniendo usuario")))),
            }
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Usuario no encontrado")))),
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") { "Username o email duplicado" } else { "Error actualizando" };
            Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err(msg))))
        }
    }
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match state.db.delete_user(id) {
        Ok(true) => {
            info!("User deleted: id={}", id);
            Ok(Json(ApiResponse::ok("Usuario eliminado".to_string())))
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::err("Usuario no encontrado")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string())))),
    }
}

// =========================================================================
// Forgot / Reset Password
// =========================================================================

pub async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Json<ApiResponse<String>> {
    // Always return success to prevent email enumeration (SOC2 compliance)
    let success_msg = "Si el email existe, recibirás un enlace para restablecer tu contraseña".to_string();

    let user = match state.db.get_user_by_email(&req.email) {
        Ok(Some(u)) => u,
        _ => return Json(ApiResponse::ok(success_msg)),
    };

    // Generate a random token
    let token_bytes: [u8; 32] = rand::random();
    let token = hex::encode(token_bytes);

    // Token expires in 1 hour
    let expires_at = (time::OffsetDateTime::now_utc() + time::Duration::hours(1))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    if let Err(e) = state.db.create_reset_token(user.id, &token, &expires_at) {
        warn!("Error creating reset token: {}", e);
        return Json(ApiResponse::ok(success_msg));
    }

    // Send password reset email using EmailService with Handlebars template
    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let reset_link = format!("{}/reset-password?token={}", frontend_url, token);
    
    if let Err(e) = state.email_service.send_password_reset(&req.email, &user.username, &reset_link).await {
        warn!("Failed to send password reset email: {}", e);
        // Still return success to prevent email enumeration
    } else {
        info!("Password reset email sent to {}", req.email);
    }

    // Audit log for SOC2 compliance
    crate::security::AuditLog::new(
        &user.username,
        "PASSWORD_RESET_REQUEST",
        &format!("user/{}", user.id),
        "system",
        "success"
    ).log();

    Json(ApiResponse::ok(success_msg))
}

pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    if req.token.is_empty() || req.password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("Token y contraseña son requeridos"))));
    }

    if req.password.len() < 6 {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("La contraseña debe tener al menos 6 caracteres"))));
    }

    let user_id = match state.db.validate_reset_token(&req.token) {
        Ok(Some(id)) => id,
        Ok(None) => return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::err("Token inválido o expirado")))),
        Err(e) => {
            warn!("Error validating reset token: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error interno"))));
        }
    };

    let hash = match bcrypt::hash(&req.password, 10) {
        Ok(h) => h,
        Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error procesando contraseña")))),
    };

    if let Err(e) = state.db.update_user_password(user_id, &hash) {
        warn!("Error updating password: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("Error actualizando contraseña"))));
    }

    if let Err(e) = state.db.consume_reset_token(&req.token) {
        warn!("Error consuming token: {}", e);
    }

    info!("Password reset successful for user_id={}", user_id);
    Ok(Json(ApiResponse::ok("Contraseña actualizada exitosamente".to_string())))
}

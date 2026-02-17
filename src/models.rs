use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// User models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct User {
    #[serde(default)]
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(default, skip_serializing)]
    pub password_hash: String,
    #[serde(default = "default_user_role")]
    pub role: String,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn default_user_role() -> String { "viewer".to_string() }

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserPublic {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl From<User> for UserPublic {
    fn from(u: User) -> Self {
        Self {
            id: u.id, username: u.username, email: u.email,
            role: u.role, active: u.active, created_at: u.created_at, updated_at: u.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(default = "default_user_role")]
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub username: String,
    pub email: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "default_user_role")]
    pub role: String,
    #[serde(default = "default_true")]
    pub active: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

// ============================================================================
// Location and Area models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Location {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Area {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub location_id: i64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AreaWithLocation {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub location_id: i64,
    pub location_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

// ============================================================================
// Camera models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Camera {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: i64,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub record: bool,
    #[serde(default)]
    pub source_on_demand: bool,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub area: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn default_port() -> i64 { 554 }
fn default_path() -> String { "/defaultPrimary?streamType=m".to_string() }
fn default_protocol() -> String { "rtsp".to_string() }
fn default_true() -> bool { true }

impl Camera {
    pub fn rtsp_url(&self) -> String {
        if self.username.is_empty() {
            format!("{}://{}:{}{}", self.protocol, self.host, self.port, self.path)
        } else {
            format!("{}://{}:{}@{}:{}{}", self.protocol, self.username, self.password, self.host, self.port, self.path)
        }
    }
}

// ============================================================================
// Mosaic models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Mosaic {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub pid: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
}

fn default_layout() -> String { "2x2".to_string() }

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct MosaicCamera {
    pub position: i64,
    pub camera_id: i64,
    pub camera_name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub password: String,
    pub path: String,
    pub protocol: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct MosaicWithCameras {
    #[serde(flatten)]
    pub mosaic: Mosaic,
    pub cameras: Vec<MosaicCamera>,
}

// ============================================================================
// Capture models (screenshots/videos by camera)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Capture {
    #[serde(default)]
    pub id: i64,
    pub camera_id: i64,
    pub camera_name: String,
    /// "screenshot" or "video"
    pub capture_type: String,
    /// Relative path inside /app/data/captures/
    pub file_path: String,
    /// File size in bytes
    #[serde(default)]
    pub file_size: i64,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CaptureQuery {
    pub camera_id: Option<i64>,
    pub date: Option<String>,
    pub capture_type: Option<String>,
}

// ============================================================================
// Notification models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Notification {
    #[serde(default)]
    pub id: i64,
    /// "disconnect", "storage", "system", "camera", "user"
    pub category: String,
    pub title: String,
    pub message: String,
    /// "info", "warning", "error", "success"
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub created_at: Option<String>,
}

fn default_severity() -> String { "info".to_string() }

// ============================================================================
// Role & Permission models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Role {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Permission {
    #[serde(default)]
    pub id: i64,
    pub role_id: i64,
    /// Module: "cameras", "mosaics", "locations", "users", "captures", "notifications", "roles", "settings"
    pub module: String,
    pub can_view: bool,
    pub can_create: bool,
    pub can_edit: bool,
    pub can_delete: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RoleWithPermissions {
    #[serde(flatten)]
    pub role: Role,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<PermissionInput>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<PermissionInput>,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct PermissionInput {
    pub module: String,
    #[serde(default)]
    pub can_view: bool,
    #[serde(default)]
    pub can_create: bool,
    #[serde(default)]
    pub can_edit: bool,
    #[serde(default)]
    pub can_delete: bool,
}

// ============================================================================
// Mosaic Share models
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct MosaicShare {
    #[serde(default)]
    pub id: i64,
    pub mosaic_id: i64,
    pub mosaic_name: String,
    pub token: String,
    /// Comma-separated emails
    pub emails: String,
    /// ISO datetime when the share expires
    pub expires_at: String,
    /// Optional: start time of daily availability window (HH:MM)
    #[serde(default)]
    pub schedule_start: Option<String>,
    /// Optional: end time of daily availability window (HH:MM)
    #[serde(default)]
    pub schedule_end: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMosaicShareRequest {
    pub mosaic_id: i64,
    pub emails: Vec<String>,
    /// Duration in hours
    pub duration_hours: i64,
    /// Optional daily schedule
    pub schedule_start: Option<String>,
    pub schedule_end: Option<String>,
}

// ============================================================================
// Settings model
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

// ============================================================================
// Request/Response models
// ============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMosaicRequest {
    pub name: String,
    #[serde(default = "default_layout")]
    pub layout: String,
    pub camera_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMosaicRequest {
    pub name: String,
    #[serde(default = "default_layout")]
    pub layout: String,
    pub camera_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CameraQuery {
    pub search: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    pub fn err(msg: &str) -> Self {
        Self { success: false, data: None, error: Some(msg.to_string()) }
    }
}

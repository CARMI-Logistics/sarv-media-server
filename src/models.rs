use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

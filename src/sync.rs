use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use tracing::warn;

use crate::models::ApiResponse;
use crate::AppState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncStatus {
    pub is_running: bool,
    pub last_sync: Option<String>,
    pub next_sync_in: Option<i64>,
    pub files_synced: i64,
    pub total_size_gb: f64,
    pub errors: i64,
    pub status_message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncLog {
    pub timestamp: String,
    pub message: String,
    pub level: String,
}

/// Get current sync status
pub async fn get_sync_status(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<SyncStatus>>, (StatusCode, Json<ApiResponse<SyncStatus>>)> {
    match read_sync_status() {
        Ok(status) => Ok(Json(ApiResponse::ok(status))),
        Err(e) => {
            warn!("Error reading sync status: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e))))
        }
    }
}

/// Get recent sync logs
pub async fn get_sync_logs(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<SyncLog>>>, (StatusCode, Json<ApiResponse<Vec<SyncLog>>>)> {
    match read_sync_logs(100) {
        Ok(logs) => Ok(Json(ApiResponse::ok(logs))),
        Err(e) => {
            warn!("Error reading sync logs: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e))))
        }
    }
}

fn read_sync_status() -> Result<SyncStatus, String> {
    // Check if container is running via Docker socket or logs
    let log_file = "/app/data/sync_logs/backup.log";
    let timestamp_file = "/app/data/sync_logs/last_sync.timestamp";
    
    // Default status
    let mut status = SyncStatus {
        is_running: false,
        last_sync: None,
        next_sync_in: None,
        files_synced: 0,
        total_size_gb: 0.0,
        errors: 0,
        status_message: "Sistema de sincronización detenido".to_string(),
    };
    
    // Try to read last sync timestamp
    if let Ok(ts_content) = fs::read_to_string(timestamp_file) {
        if let Ok(timestamp) = ts_content.trim().parse::<i64>() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            
            let elapsed = now - timestamp;
            status.last_sync = Some(format!("Hace {} minutos", elapsed / 60));
            
            // Next sync in 1 hour (3600 seconds) - configurable
            let sync_interval = 3600;
            let next_in = sync_interval - elapsed;
            
            if next_in > 0 {
                status.next_sync_in = Some(next_in);
                status.is_running = true;
                status.status_message = format!("Próxima sincronización en {} minutos", next_in / 60);
            }
        }
    }
    
    // Try to read backup log for stats
    if let Ok(log_content) = fs::read_to_string(log_file) {
        // Parse log for stats
        let lines: Vec<&str> = log_content.lines().collect();
        
        // Count files synced
        let synced_count = lines.iter()
            .filter(|line| line.contains("Copied (new)") || line.contains("Copied (replaced"))
            .count();
        
        status.files_synced = synced_count as i64;
        
        // Count errors
        let error_count = lines.iter()
            .filter(|line| line.contains("ERROR"))
            .count();
        
        status.errors = error_count as i64;
        
        // Extract total size if available
        for line in lines.iter().rev().take(20) {
            if line.contains("GiB") && line.contains("/") {
                // Try to extract size like "33.434 GiB / 104.589 GiB"
                if let Some(size_part) = line.split("GiB").next() {
                    if let Some(last_num) = size_part.split_whitespace().last() {
                        if let Ok(size) = last_num.parse::<f64>() {
                            status.total_size_gb = size;
                            break;
                        }
                    }
                }
            }
        }
        
        // Update status message if syncing
        if status.is_running && status.errors < 10 {
            status.status_message = format!(
                "Sistema activo - {} archivos sincronizados, {:.1} GB procesados",
                status.files_synced,
                status.total_size_gb
            );
        } else if status.errors >= 10 {
            status.status_message = format!("Advertencia: {} errores detectados", status.errors);
        }
    }
    
    Ok(status)
}

fn read_sync_logs(limit: usize) -> Result<Vec<SyncLog>, String> {
    let log_file = "/app/data/sync_logs/backup.log";
    
    let mut logs = Vec::new();
    
    if let Ok(content) = fs::read_to_string(log_file) {
        let lines: Vec<&str> = content.lines().rev().take(limit).collect();
        
        for line in lines {
            // Parse log format: [2026-02-17 06:18:24] [INFO] message
            let mut timestamp = String::new();
            let mut level = "INFO".to_string();
            let mut message = line.to_string();
            
            if line.starts_with('[') {
                if let Some(ts_end) = line.find("] [") {
                    timestamp = line[1..ts_end].to_string();
                    
                    let rest = &line[ts_end + 3..];
                    if let Some(level_end) = rest.find(']') {
                        level = rest[..level_end].to_string();
                        message = rest[level_end + 2..].to_string();
                    }
                }
            } else if line.contains("INFO :") {
                level = "INFO".to_string();
                message = line.split("INFO :").nth(1).unwrap_or(line).to_string();
            } else if line.contains("ERROR :") {
                level = "ERROR".to_string();
                message = line.split("ERROR :").nth(1).unwrap_or(line).to_string();
            } else if line.contains("NOTICE:") {
                level = "NOTICE".to_string();
                message = line.split("NOTICE:").nth(1).unwrap_or(line).to_string();
            }
            
            logs.push(SyncLog {
                timestamp,
                message: message.trim().to_string(),
                level,
            });
        }
    }
    
    Ok(logs)
}

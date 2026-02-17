//! Thumbnail capture system for cameras
//! 
//! Automatically captures thumbnails from camera streams every minute

use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

/// Start thumbnail capture background task for a camera
pub async fn start_thumbnail_capture(camera_id: i64, stream_name: &str, thumbnail_dir: &str) {
    let stream_name = stream_name.to_string();
    let thumbnail_dir = thumbnail_dir.to_string();
    
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60)); // Every minute
        
        loop {
            ticker.tick().await;
            
            if let Err(e) = capture_thumbnail(camera_id, &stream_name, &thumbnail_dir).await {
                warn!("Failed to capture thumbnail for camera {}: {}", camera_id, e);
            }
        }
    });
}

/// Capture a single thumbnail from stream
async fn capture_thumbnail(camera_id: i64, stream_name: &str, thumbnail_dir: &str) -> Result<(), String> {
    // Ensure directory exists
    tokio::fs::create_dir_all(thumbnail_dir)
        .await
        .map_err(|e| format!("Failed to create thumbnail dir: {}", e))?;

    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let filename = format!("camera_{}_thumb_{}.jpg", camera_id, timestamp);
    let output_path = Path::new(thumbnail_dir).join(&filename);
    
    // Try WebRTC first, fallback to HLS
    let stream_url = format!("http://localhost:8889/{}", stream_name);
    
    // Use ffmpeg to capture a frame
    let output = Command::new("ffmpeg")
        .args(&[
            "-rtsp_transport", "tcp",
            "-i", &stream_url,
            "-vframes", "1",
            "-q:v", "2",
            "-y",
            output_path.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await
        .map_err(|e| format!("FFmpeg execution failed: {}", e))?;

    if output.status.success() {
        info!("Thumbnail captured for camera {}: {}", camera_id, filename);
        
        // Keep only last 10 thumbnails per camera
        cleanup_old_thumbnails(camera_id, thumbnail_dir, 10).await;
        
        Ok(())
    } else {
        Err(format!("FFmpeg failed with status: {}", output.status))
    }
}

/// Clean up old thumbnails, keeping only the N most recent
async fn cleanup_old_thumbnails(camera_id: i64, thumbnail_dir: &str, keep: usize) {
    let pattern = format!("camera_{}_thumb_", camera_id);
    
    match tokio::fs::read_dir(thumbnail_dir).await {
        Ok(mut entries) => {
            let mut files: Vec<(String, u64)> = Vec::new();
            
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with(&pattern) && name.ends_with(".jpg") {
                        // Extract timestamp from filename
                        if let Some(ts_str) = name.strip_prefix(&pattern).and_then(|s| s.strip_suffix(".jpg")) {
                            if let Ok(ts) = ts_str.parse::<u64>() {
                                files.push((name, ts));
                            }
                        }
                    }
                }
            }
            
            // Sort by timestamp descending
            files.sort_by(|a, b| b.1.cmp(&a.1));
            
            // Delete old files
            for (name, _) in files.iter().skip(keep) {
                let path = Path::new(thumbnail_dir).join(name);
                let _ = tokio::fs::remove_file(path).await;
            }
        }
        Err(e) => warn!("Failed to read thumbnail directory: {}", e),
    }
}

/// Get latest thumbnail path for a camera
pub async fn get_latest_thumbnail(camera_id: i64, thumbnail_dir: &str) -> Option<String> {
    let pattern = format!("camera_{}_thumb_", camera_id);
    
    match tokio::fs::read_dir(thumbnail_dir).await {
        Ok(mut entries) => {
            let mut files: Vec<(String, u64)> = Vec::new();
            
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with(&pattern) && name.ends_with(".jpg") {
                        if let Some(ts_str) = name.strip_prefix(&pattern).and_then(|s| s.strip_suffix(".jpg")) {
                            if let Ok(ts) = ts_str.parse::<u64>() {
                                files.push((name.clone(), ts));
                            }
                        }
                    }
                }
            }
            
            files.sort_by(|a, b| b.1.cmp(&a.1));
            files.first().map(|(name, _)| format!("/data/thumbnails/{}", name))
        }
        Err(_) => None,
    }
}

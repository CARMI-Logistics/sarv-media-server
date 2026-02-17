//! Security middleware and utilities for SOC2 compliance
//!
//! - Rate limiting
//! - Security headers
//! - Audit logging
//! - Input sanitization

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Rate limiter state
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        
        // Clean up old requests
        let entry = requests.entry(key.to_string()).or_insert_with(Vec::new);
        entry.retain(|&time| now.duration_since(time) < self.window);
        
        // Check limit
        if entry.len() >= self.max_requests {
            false
        } else {
            entry.push(now);
            true
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract IP or use a default key
    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // Global rate limiter: 100 requests per minute
    static RATE_LIMITER: once_cell::sync::Lazy<RateLimiter> = 
        once_cell::sync::Lazy::new(|| RateLimiter::new(100, 60));

    if !RATE_LIMITER.check_rate_limit(ip) {
        warn!("Rate limit exceeded for IP: {}", ip);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Security headers middleware (SOC2 compliance)
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // SOC2 Security Headers
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "X-Frame-Options",
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        "Strict-Transport-Security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ws: wss:;"
        ),
    );
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "Permissions-Policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );

    response
}

/// Audit log entry for SOC2 compliance
#[derive(Debug, serde::Serialize)]
pub struct AuditLog {
    pub timestamp: String,
    pub user: String,
    pub action: String,
    pub resource: String,
    pub ip_address: String,
    pub status: String,
}

impl AuditLog {
    pub fn new(user: &str, action: &str, resource: &str, ip: &str, status: &str) -> Self {
        Self {
            timestamp: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            user: user.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            ip_address: ip.to_string(),
            status: status.to_string(),
        }
    }

    pub fn log(&self) {
        info!(
            target: "audit",
            timestamp = %self.timestamp,
            user = %self.user,
            action = %self.action,
            resource = %self.resource,
            ip = %self.ip_address,
            status = %self.status,
            "AUDIT_LOG"
        );
    }
}

/// Sanitize input strings to prevent injection attacks
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "-_@.".contains(*c))
        .collect()
}

/// Validate email format
pub fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() > 5
}

/// Validate password strength (SOC2 compliance)
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("La contraseña debe tener al menos 8 caracteres".to_string());
    }
    
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    
    if !has_lowercase || !has_uppercase || !has_digit {
        return Err("La contraseña debe contener mayúsculas, minúsculas y números".to_string());
    }
    
    Ok(())
}

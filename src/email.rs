//! Email service with Handlebars templates
//!
//! Integrates with Resend API for transactional emails

use handlebars::Handlebars;
use serde::Serialize;
use std::collections::HashMap;
use tracing::{info, warn};

/// Email service with templating
pub struct EmailService {
    handlebars: Handlebars<'static>,
    resend_api_key: String,
    from_email: String,
    http_client: reqwest::Client,
}

impl EmailService {
    pub fn new(resend_api_key: String, from_email: String, http_client: reqwest::Client) -> Self {
        let mut handlebars = Handlebars::new();
        
        // Register email templates
        handlebars
            .register_template_string("reset_password", include_str!("../templates/reset_password.hbs"))
            .unwrap_or_else(|e| {
                warn!("Failed to register reset_password template: {}", e);
            });
        
        handlebars
            .register_template_string("welcome", include_str!("../templates/welcome.hbs"))
            .unwrap_or_else(|e| {
                warn!("Failed to register welcome template: {}", e);
            });
        
        handlebars
            .register_template_string("share_mosaic", include_str!("../templates/share_mosaic.hbs"))
            .unwrap_or_else(|e| {
                warn!("Failed to register share_mosaic template: {}", e);
            });

        Self {
            handlebars,
            resend_api_key,
            from_email,
            http_client,
        }
    }

    /// Send password reset email
    pub async fn send_password_reset(
        &self,
        to_email: &str,
        username: &str,
        reset_link: &str,
    ) -> Result<(), String> {
        let mut data = HashMap::new();
        data.insert("username", username);
        data.insert("reset_link", reset_link);

        let html = self
            .handlebars
            .render("reset_password", &data)
            .map_err(|e| format!("Template render error: {}", e))?;

        self.send_email(to_email, "Restablecer contraseña", &html)
            .await
    }

    /// Send welcome email to new user
    pub async fn send_welcome_email(
        &self,
        to_email: &str,
        username: &str,
        login_url: &str,
    ) -> Result<(), String> {
        let mut data = HashMap::new();
        data.insert("username", username);
        data.insert("login_url", login_url);

        let html = self
            .handlebars
            .render("welcome", &data)
            .map_err(|e| format!("Template render error: {}", e))?;

        self.send_email(to_email, "Bienvenido a CamManager", &html)
            .await
    }

    /// Send mosaic share notification
    pub async fn send_mosaic_share(
        &self,
        to_email: &str,
        mosaic_name: &str,
        share_link: &str,
        expires_at: &str,
    ) -> Result<(), String> {
        #[derive(Serialize)]
        struct ShareData {
            mosaic_name: String,
            share_link: String,
            expires_at: String,
        }

        let data = ShareData {
            mosaic_name: mosaic_name.to_string(),
            share_link: share_link.to_string(),
            expires_at: expires_at.to_string(),
        };

        let html = self
            .handlebars
            .render("share_mosaic", &data)
            .map_err(|e| format!("Template render error: {}", e))?;

        self.send_email(to_email, &format!("Mosaico compartido: {}", mosaic_name), &html)
            .await
    }

    /// Internal method to send email via Resend API
    async fn send_email(&self, to: &str, subject: &str, html: &str) -> Result<(), String> {
        if self.resend_api_key.is_empty() {
            return Err("RESEND_API_KEY not configured".to_string());
        }

        let body = serde_json::json!({
            "from": self.from_email,
            "to": [to],
            "subject": subject,
            "html": html
        });

        match self
            .http_client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", self.resend_api_key))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                info!("Email sent successfully to {}", to);
                Ok(())
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                warn!("Resend API error {}: {}", status, body);
                Err(format!("Email send failed: {} - {}", status, body))
            }
            Err(e) => {
                warn!("Failed to send email: {}", e);
                Err(format!("Network error: {}", e))
            }
        }
    }
}

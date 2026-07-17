//! Adaptador de `CameraProvisioner` contra la Control API de MediaMTX (HU 4.2).
//!
//! Habla http interno con `MEDIAMTX_API_URL` (p.ej. http://mediamtx:9997).
//! SEGURIDAD: la `rtsp_url` (con credenciales) viaja en el BODY de add/patch y
//! NUNCA se registra en logs; los errores solo incluyen el estado HTTP y el
//! mensaje del servidor (la URL de la Control API no contiene credenciales).

use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;

use crate::domain::models::Camera;
use crate::domain::ports::{CameraProvisioner, ProvisionError, ProvisionResult};

pub struct MediaMtxProvisioner {
    client: Client,
    /// Base de la Control API, sin barra final (p.ej. `http://mediamtx:9997`).
    base_url: String,
}

impl MediaMtxProvisioner {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("no se pudo construir el cliente HTTP");
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// Config de ruta para MediaMTX. El resto lo cubre `pathDefaults` del YAML.
    fn path_config(camera: &Camera) -> serde_json::Value {
        json!({
            "source": camera.rtsp_url,
            "record": camera.record,
        })
    }

    /// Lee las rutas configuradas con su `source`/`record` (migración one-time
    /// YAML→BD). A diferencia de `list_paths`, trae el detalle necesario.
    pub async fn list_source_paths(&self) -> ProvisionResult<Vec<ImportedPath>> {
        let url = format!("{}/v3/config/paths/list", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(to_backend)?;
        if !resp.status().is_success() {
            return Err(status_err(resp).await);
        }
        let list: FullPathsList = resp.json().await.map_err(to_backend)?;
        Ok(list
            .items
            .into_iter()
            .map(|i| ImportedPath {
                name: i.name,
                source: i.source,
                record: i.record.unwrap_or(true),
            })
            .collect())
    }
}

/// Traduce un error de transporte a error de dominio (sin credenciales: la URL
/// de la Control API no las contiene).
fn to_backend(e: reqwest::Error) -> ProvisionError {
    ProvisionError::Backend(e.to_string())
}

/// Construye un error de dominio a partir de una respuesta no exitosa.
async fn status_err(resp: reqwest::Response) -> ProvisionError {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    ProvisionError::Backend(format!("HTTP {status}: {body}"))
}

#[async_trait]
impl CameraProvisioner for MediaMtxProvisioner {
    async fn apply(&self, camera: &Camera) -> ProvisionResult<()> {
        let body = Self::path_config(camera);

        // Intento de alta.
        let add_url = format!("{}/v3/config/paths/add/{}", self.base_url, camera.path);
        let resp = self
            .client
            .post(&add_url)
            .json(&body)
            .send()
            .await
            .map_err(to_backend)?;
        if resp.status().is_success() {
            return Ok(());
        }

        // Ya existe (o el alta la rechazó): actualizamos con PATCH → idempotencia.
        let patch_url = format!("{}/v3/config/paths/patch/{}", self.base_url, camera.path);
        let resp = self
            .client
            .patch(&patch_url)
            .json(&body)
            .send()
            .await
            .map_err(to_backend)?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(status_err(resp).await)
        }
    }

    async fn remove(&self, path: &str) -> ProvisionResult<()> {
        let url = format!("{}/v3/config/paths/delete/{}", self.base_url, path);
        let resp = self.client.delete(&url).send().await.map_err(to_backend)?;
        match resp.status() {
            s if s.is_success() => Ok(()),
            StatusCode::NOT_FOUND => Err(ProvisionError::NotFound),
            _ => Err(status_err(resp).await),
        }
    }

    async fn list_paths(&self) -> ProvisionResult<Vec<String>> {
        // Solo rutas de cámara gestionables (pull RTSP, no regex): así el
        // reconciler nunca elimina publishers (p.ej. "mosaic") ni patrones "~...".
        let paths = self.list_source_paths().await?;
        Ok(paths
            .into_iter()
            .filter(|p| {
                !p.name.starts_with('~')
                    && p.source.as_deref().is_some_and(|s| s.starts_with("rtsp://"))
            })
            .map(|p| p.name)
            .collect())
    }
}

/// Ruta importada del MediaMTX vivo (para la migración one-time YAML→BD).
pub struct ImportedPath {
    pub name: String,
    pub source: Option<String>,
    pub record: bool,
}

/// Respuesta detallada de `GET /v3/config/paths/list` para la migración
/// (ignora los demás campos del PathConf de MediaMTX).
#[derive(Deserialize)]
struct FullPathsList {
    items: Vec<FullPathItem>,
}

#[derive(Deserialize)]
struct FullPathItem {
    name: String,
    source: Option<String>,
    record: Option<bool>,
}

//! Adaptador Postgres de `CameraRepo` (HU 4.1).
//!
//! Cifra/descifra `rtsp_url` con el `Cipher` al escribir/leer: en la BD vive
//! `rtsp_url_enc` (bytea), en el dominio vive `rtsp_url` en claro.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::map_sqlx_err;
use crate::crypto::Cipher;
use crate::domain::models::{Camera, NewCamera};
use crate::domain::ports::{CameraRepo, RepoError, RepoResult};

#[derive(sqlx::FromRow)]
struct CameraRow {
    id: Uuid,
    path: String,
    rtsp_url_enc: Vec<u8>,
    record: bool,
    enabled: bool,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub struct PgCameraRepo {
    pool: PgPool,
    cipher: Cipher,
}

impl PgCameraRepo {
    pub fn new(pool: PgPool, cipher: Cipher) -> Self {
        Self { pool, cipher }
    }

    /// Mapea una fila a la entidad de dominio descifrando la URL.
    fn to_camera(&self, r: CameraRow) -> RepoResult<Camera> {
        let rtsp_url = self
            .cipher
            .decrypt(&r.rtsp_url_enc)
            .map_err(|e| RepoError::Backend(format!("descifrado de rtsp_url: {e}")))?;
        Ok(Camera {
            id: r.id,
            path: r.path,
            rtsp_url,
            record: r.record,
            enabled: r.enabled,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }

    fn to_cameras(&self, rows: Vec<CameraRow>) -> RepoResult<Vec<Camera>> {
        rows.into_iter().map(|r| self.to_camera(r)).collect()
    }
}

#[async_trait]
impl CameraRepo for PgCameraRepo {
    async fn list_all(&self) -> RepoResult<Vec<Camera>> {
        let rows = sqlx::query_as::<_, CameraRow>(
            "SELECT id, path, rtsp_url_enc, record, enabled, description, created_at, updated_at
             FROM cameras ORDER BY path",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        self.to_cameras(rows)
    }

    async fn list_enabled(&self) -> RepoResult<Vec<Camera>> {
        let rows = sqlx::query_as::<_, CameraRow>(
            "SELECT id, path, rtsp_url_enc, record, enabled, description, created_at, updated_at
             FROM cameras WHERE enabled = true ORDER BY path",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        self.to_cameras(rows)
    }

    async fn find_by_id(&self, id: Uuid) -> RepoResult<Option<Camera>> {
        let row = sqlx::query_as::<_, CameraRow>(
            "SELECT id, path, rtsp_url_enc, record, enabled, description, created_at, updated_at
             FROM cameras WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(|r| self.to_camera(r)).transpose()
    }

    async fn find_by_path(&self, path: &str) -> RepoResult<Option<Camera>> {
        let row = sqlx::query_as::<_, CameraRow>(
            "SELECT id, path, rtsp_url_enc, record, enabled, description, created_at, updated_at
             FROM cameras WHERE path = $1",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(|r| self.to_camera(r)).transpose()
    }

    async fn create(&self, new: NewCamera) -> RepoResult<Camera> {
        let enc = self
            .cipher
            .encrypt(&new.rtsp_url)
            .map_err(|e| RepoError::Backend(format!("cifrado de rtsp_url: {e}")))?;
        let row = sqlx::query_as::<_, CameraRow>(
            "INSERT INTO cameras (id, path, rtsp_url_enc, record, enabled, description)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, path, rtsp_url_enc, record, enabled, description, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(new.path)
        .bind(enc)
        .bind(new.record)
        .bind(new.enabled)
        .bind(new.description)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        self.to_camera(row)
    }

    async fn update(&self, camera: &Camera) -> RepoResult<Camera> {
        let enc = self
            .cipher
            .encrypt(&camera.rtsp_url)
            .map_err(|e| RepoError::Backend(format!("cifrado de rtsp_url: {e}")))?;
        let row = sqlx::query_as::<_, CameraRow>(
            "UPDATE cameras
             SET path = $2, rtsp_url_enc = $3, record = $4, enabled = $5, description = $6
             WHERE id = $1
             RETURNING id, path, rtsp_url_enc, record, enabled, description, created_at, updated_at",
        )
        .bind(camera.id)
        .bind(camera.path.as_str())
        .bind(enc)
        .bind(camera.record)
        .bind(camera.enabled)
        .bind(camera.description.as_deref())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        match row {
            Some(r) => self.to_camera(r),
            None => Err(RepoError::NotFound),
        }
    }

    async fn delete(&self, id: Uuid) -> RepoResult<()> {
        let res = sqlx::query("DELETE FROM cameras WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        if res.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::PgCameraRepo;
    use crate::crypto::Cipher;
    use crate::domain::models::NewCamera;
    use crate::domain::ports::{CameraRepo, RepoError};
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use sqlx::PgPool;
    use uuid::Uuid;

    fn cipher() -> Cipher {
        Cipher::from_base64_key(&STANDARD.encode([9u8; 32])).unwrap()
    }

    fn sample(path: &str) -> NewCamera {
        NewCamera {
            path: path.into(),
            rtsp_url: "rtsp://u:p@10.0.0.9/stream".into(),
            record: true,
            enabled: true,
            description: Some("cam de prueba".into()),
        }
    }

    #[sqlx::test]
    async fn create_read_roundtrip_decrypts(pool: PgPool) {
        let repo = PgCameraRepo::new(pool, cipher());
        let created = repo.create(sample("cam-x")).await.unwrap();
        assert_eq!(created.rtsp_url, "rtsp://u:p@10.0.0.9/stream");

        let found = repo.find_by_path("cam-x").await.unwrap().unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.rtsp_url, "rtsp://u:p@10.0.0.9/stream");
    }

    #[sqlx::test]
    async fn rtsp_url_stored_encrypted(pool: PgPool) {
        let repo = PgCameraRepo::new(pool.clone(), cipher());
        repo.create(sample("cam-enc")).await.unwrap();

        // Leer el bytea crudo: NO debe contener el texto plano de la URL.
        let (enc,): (Vec<u8>,) =
            sqlx::query_as("SELECT rtsp_url_enc FROM cameras WHERE path = $1")
                .bind("cam-enc")
                .fetch_one(&pool)
                .await
                .unwrap();
        let as_text = String::from_utf8_lossy(&enc);
        assert!(!as_text.contains("10.0.0.9"), "la URL no debe estar en claro");
        assert!(!as_text.contains("rtsp://"), "la URL no debe estar en claro");
    }

    #[sqlx::test]
    async fn list_enabled_excludes_disabled(pool: PgPool) {
        let repo = PgCameraRepo::new(pool, cipher());
        repo.create(sample("on")).await.unwrap();
        let mut off = sample("off");
        off.enabled = false;
        repo.create(off).await.unwrap();

        let enabled = repo.list_enabled().await.unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].path, "on");
        assert_eq!(repo.list_all().await.unwrap().len(), 2);
    }

    #[sqlx::test]
    async fn update_reencrypts(pool: PgPool) {
        let repo = PgCameraRepo::new(pool, cipher());
        let mut cam = repo.create(sample("cam-u")).await.unwrap();
        cam.rtsp_url = "rtsp://new:pw@10.0.0.10/z".into();
        cam.enabled = false;
        let updated = repo.update(&cam).await.unwrap();
        assert_eq!(updated.rtsp_url, "rtsp://new:pw@10.0.0.10/z");
        assert!(!updated.enabled);

        let found = repo.find_by_id(cam.id).await.unwrap().unwrap();
        assert_eq!(found.rtsp_url, "rtsp://new:pw@10.0.0.10/z");
    }

    #[sqlx::test]
    async fn delete_missing_is_not_found(pool: PgPool) {
        let repo = PgCameraRepo::new(pool, cipher());
        let err = repo.delete(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, RepoError::NotFound));
    }
}

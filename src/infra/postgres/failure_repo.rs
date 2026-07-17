//! Adaptador Postgres de `FailureRepo` (HU 4.1). Lo llenará el agente (HU 4.6).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::map_sqlx_err;
use crate::domain::models::{Failure, NewFailure, Severity};
use crate::domain::ports::{FailureRepo, RepoResult};

#[derive(sqlx::FromRow)]
struct FailureRow {
    id: i64,
    camera_path: String,
    detected_at: DateTime<Utc>,
    severity: String,
    diagnosis: Option<String>,
    raw: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

impl From<FailureRow> for Failure {
    fn from(r: FailureRow) -> Self {
        Failure {
            id: r.id,
            camera_path: r.camera_path,
            detected_at: r.detected_at,
            severity: Severity::from_db(&r.severity),
            diagnosis: r.diagnosis,
            raw: r.raw,
            created_at: r.created_at,
        }
    }
}

pub struct PgFailureRepo {
    pool: PgPool,
}

impl PgFailureRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FailureRepo for PgFailureRepo {
    async fn record(&self, new: NewFailure) -> RepoResult<Failure> {
        let row = sqlx::query_as::<_, FailureRow>(
            "INSERT INTO failure_history (camera_path, detected_at, severity, diagnosis, raw)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, camera_path, detected_at, severity, diagnosis, raw, created_at",
        )
        .bind(new.camera_path)
        .bind(new.detected_at)
        .bind(new.severity.as_str())
        .bind(new.diagnosis)
        .bind(new.raw)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    async fn list_by_camera(&self, camera_path: &str, limit: i64) -> RepoResult<Vec<Failure>> {
        let rows = sqlx::query_as::<_, FailureRow>(
            "SELECT id, camera_path, detected_at, severity, diagnosis, raw, created_at
             FROM failure_history
             WHERE camera_path = $1
             ORDER BY detected_at DESC
             LIMIT $2",
        )
        .bind(camera_path)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn latest_by_camera(&self, camera_path: &str) -> RepoResult<Option<Failure>> {
        let row = sqlx::query_as::<_, FailureRow>(
            "SELECT id, camera_path, detected_at, severity, diagnosis, raw, created_at
             FROM failure_history
             WHERE camera_path = $1
             ORDER BY detected_at DESC
             LIMIT 1",
        )
        .bind(camera_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.map(Into::into))
    }
}

#[cfg(test)]
mod tests {
    use super::PgFailureRepo;
    use crate::domain::models::{NewFailure, Severity};
    use crate::domain::ports::FailureRepo;
    use chrono::Utc;
    use sqlx::PgPool;

    fn sample(camera_path: &str, severity: Severity) -> NewFailure {
        NewFailure {
            camera_path: camera_path.into(),
            detected_at: Utc::now(),
            severity,
            diagnosis: Some("cámara caída".into()),
            raw: Some(serde_json::json!({"code": 500})),
        }
    }

    #[sqlx::test]
    async fn record_and_list_by_camera(pool: PgPool) {
        let repo = PgFailureRepo::new(pool);
        repo.record(sample("cam-1", Severity::Error)).await.unwrap();
        repo.record(sample("cam-1", Severity::Warn)).await.unwrap();
        repo.record(sample("cam-2", Severity::Ok)).await.unwrap();

        assert_eq!(repo.list_by_camera("cam-1", 10).await.unwrap().len(), 2);
        assert_eq!(repo.list_by_camera("cam-2", 10).await.unwrap().len(), 1);
        assert_eq!(repo.list_by_camera("cam-x", 10).await.unwrap().len(), 0);
    }

    #[sqlx::test]
    async fn severity_and_raw_roundtrip(pool: PgPool) {
        let repo = PgFailureRepo::new(pool);
        let f = repo.record(sample("c", Severity::Warn)).await.unwrap();
        assert_eq!(f.severity, Severity::Warn);
        assert_eq!(f.raw, Some(serde_json::json!({"code": 500})));
    }

    #[sqlx::test]
    async fn latest_by_camera_returns_most_recent(pool: PgPool) {
        let repo = PgFailureRepo::new(pool);
        let older = NewFailure {
            camera_path: "cam".into(),
            detected_at: Utc::now() - chrono::Duration::hours(1),
            severity: Severity::Warn,
            diagnosis: None,
            raw: None,
        };
        let newer = NewFailure {
            camera_path: "cam".into(),
            detected_at: Utc::now(),
            severity: Severity::Error,
            diagnosis: None,
            raw: None,
        };
        repo.record(older).await.unwrap();
        repo.record(newer).await.unwrap();

        let latest = repo.latest_by_camera("cam").await.unwrap().unwrap();
        assert_eq!(latest.severity, Severity::Error);
        assert!(repo.latest_by_camera("noexiste").await.unwrap().is_none());
    }
}

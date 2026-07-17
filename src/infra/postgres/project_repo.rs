//! Adaptador Postgres de `ProjectRepo` (HU 4.1).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::map_sqlx_err;
use crate::domain::models::{NewProject, Project};
use crate::domain::ports::{ProjectRepo, RepoError, RepoResult};

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: Uuid,
    client_id: String,
    secret_hash: String,
    all_cameras: bool,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ProjectRow> for Project {
    fn from(r: ProjectRow) -> Self {
        Project {
            id: r.id,
            client_id: r.client_id,
            secret_hash: r.secret_hash,
            all_cameras: r.all_cameras,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct PgProjectRepo {
    pool: PgPool,
}

impl PgProjectRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepo for PgProjectRepo {
    async fn find_by_client_id(&self, client_id: &str) -> RepoResult<Option<Project>> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "SELECT id, client_id, secret_hash, all_cameras, enabled, created_at, updated_at
             FROM projects WHERE client_id = $1",
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.map(Into::into))
    }

    async fn find_by_id(&self, id: Uuid) -> RepoResult<Option<Project>> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "SELECT id, client_id, secret_hash, all_cameras, enabled, created_at, updated_at
             FROM projects WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.map(Into::into))
    }

    async fn list_all(&self) -> RepoResult<Vec<Project>> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            "SELECT id, client_id, secret_hash, all_cameras, enabled, created_at, updated_at
             FROM projects ORDER BY client_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create(&self, new: NewProject) -> RepoResult<Project> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "INSERT INTO projects (id, client_id, secret_hash, all_cameras, enabled)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, client_id, secret_hash, all_cameras, enabled, created_at, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(new.client_id)
        .bind(new.secret_hash)
        .bind(new.all_cameras)
        .bind(new.enabled)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    async fn update(&self, project: &Project) -> RepoResult<Project> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "UPDATE projects
             SET client_id = $2, secret_hash = $3, all_cameras = $4, enabled = $5
             WHERE id = $1
             RETURNING id, client_id, secret_hash, all_cameras, enabled, created_at, updated_at",
        )
        .bind(project.id)
        .bind(project.client_id.as_str())
        .bind(project.secret_hash.as_str())
        .bind(project.all_cameras)
        .bind(project.enabled)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(Into::into).ok_or(RepoError::NotFound)
    }

    async fn delete(&self, id: Uuid) -> RepoResult<()> {
        let res = sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        if res.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    async fn set_cameras(&self, project_id: Uuid, camera_ids: &[Uuid]) -> RepoResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;
        sqlx::query("DELETE FROM project_cameras WHERE project_id = $1")
            .bind(project_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        for camera_id in camera_ids {
            sqlx::query("INSERT INTO project_cameras (project_id, camera_id) VALUES ($1, $2)")
                .bind(project_id)
                .bind(camera_id)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;
        }
        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn allowed_camera_paths(&self, project_id: Uuid) -> RepoResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT c.path
             FROM project_cameras pc
             JOIN cameras c ON c.id = pc.camera_id
             WHERE pc.project_id = $1
             ORDER BY c.path",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(|(path,)| path).collect())
    }

    async fn assigned_camera_ids(&self, project_id: Uuid) -> RepoResult<Vec<Uuid>> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT camera_id FROM project_cameras WHERE project_id = $1 ORDER BY camera_id",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::PgProjectRepo;
    use crate::domain::models::NewProject;
    use crate::domain::ports::{ProjectRepo, RepoError};
    use sqlx::PgPool;
    use uuid::Uuid;

    fn sample(client_id: &str) -> NewProject {
        NewProject {
            client_id: client_id.into(),
            secret_hash: "$argon2id$dummy-hash".into(),
            all_cameras: false,
            enabled: true,
        }
    }

    #[sqlx::test]
    async fn create_and_find(pool: PgPool) {
        let repo = PgProjectRepo::new(pool);
        let created = repo.create(sample("sigac")).await.unwrap();
        assert_eq!(created.client_id, "sigac");

        let found = repo.find_by_client_id("sigac").await.unwrap().unwrap();
        assert_eq!(found.id, created.id);
        assert!(repo.find_by_client_id("noexiste").await.unwrap().is_none());
    }

    #[sqlx::test]
    async fn duplicate_client_id_is_conflict(pool: PgPool) {
        let repo = PgProjectRepo::new(pool);
        repo.create(sample("dup")).await.unwrap();
        let err = repo.create(sample("dup")).await.unwrap_err();
        assert!(matches!(err, RepoError::Conflict(_)));
    }

    #[sqlx::test]
    async fn update_and_delete(pool: PgPool) {
        let repo = PgProjectRepo::new(pool);
        let mut p = repo.create(sample("odin")).await.unwrap();
        p.all_cameras = true;
        p.enabled = false;
        let updated = repo.update(&p).await.unwrap();
        assert!(updated.all_cameras);
        assert!(!updated.enabled);

        repo.delete(p.id).await.unwrap();
        assert!(repo.find_by_id(p.id).await.unwrap().is_none());
        assert!(matches!(repo.delete(p.id).await.unwrap_err(), RepoError::NotFound));
    }

    #[sqlx::test]
    async fn set_cameras_and_allowed_paths(pool: PgPool) {
        let repo = PgProjectRepo::new(pool.clone());
        let project = repo.create(sample("proj")).await.unwrap();

        // Cámaras insertadas directamente (aquí probamos la relación n-a-n, no el cifrado).
        let cam1 = Uuid::new_v4();
        let cam2 = Uuid::new_v4();
        for (id, path) in [(cam1, "cam-a"), (cam2, "cam-b")] {
            sqlx::query(
                "INSERT INTO cameras (id, path, rtsp_url_enc, record, enabled)
                 VALUES ($1, $2, $3, true, true)",
            )
            .bind(id)
            .bind(path)
            .bind(vec![0u8, 1, 2, 3])
            .execute(&pool)
            .await
            .unwrap();
        }

        repo.set_cameras(project.id, &[cam1, cam2]).await.unwrap();
        let mut paths = repo.allowed_camera_paths(project.id).await.unwrap();
        paths.sort();
        assert_eq!(paths, vec!["cam-a".to_string(), "cam-b".to_string()]);

        // Reemplaza el conjunto por una sola cámara.
        repo.set_cameras(project.id, &[cam1]).await.unwrap();
        assert_eq!(
            repo.allowed_camera_paths(project.id).await.unwrap(),
            vec!["cam-a".to_string()]
        );
    }
}

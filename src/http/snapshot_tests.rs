//! Tests de `GET /cameras/{path}/snapshot` (sub-plan A, paso 4): el router real
//! de `consumer` con repositorios falsos y un `ffmpeg` simulado, sin Postgres.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, DecodingKey, EncodingKey, Header};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

use super::consumer;
use crate::domain::models::{
    Camera, Failure, NewCamera, NewFailure, NewProject, Project,
};
use crate::domain::ports::{
    CameraProvisioner, CameraRepo, FailureRepo, ProjectRepo, ProvisionResult, RepoResult,
};
use crate::services::auth::AuthService;
use crate::services::reconciler::ReconcilerService;
use crate::services::snapshot::testing::{FakeFfmpeg, FAKE_JPEG};
use crate::services::snapshot::SnapshotService;
use crate::{AppState, Claims, Config, Jwks, MtxPermission};

const AMPLE_TIMEOUT: Duration = Duration::from_secs(5);

// --- Dobles de prueba: solo `find_by_path` tiene comportamiento ---------------

struct FakeCameraRepo {
    cameras: Vec<Camera>,
}

#[async_trait]
impl CameraRepo for FakeCameraRepo {
    async fn find_by_path(&self, path: &str) -> RepoResult<Option<Camera>> {
        Ok(self.cameras.iter().find(|c| c.path == path).cloned())
    }
    async fn list_all(&self) -> RepoResult<Vec<Camera>> {
        Ok(self.cameras.clone())
    }
    async fn list_enabled(&self) -> RepoResult<Vec<Camera>> {
        Ok(self.cameras.iter().filter(|c| c.enabled).cloned().collect())
    }
    async fn find_by_id(&self, _: Uuid) -> RepoResult<Option<Camera>> {
        unimplemented!()
    }
    async fn create(&self, _: NewCamera) -> RepoResult<Camera> {
        unimplemented!()
    }
    async fn update(&self, _: &Camera) -> RepoResult<Camera> {
        unimplemented!()
    }
    async fn delete(&self, _: Uuid) -> RepoResult<()> {
        unimplemented!()
    }
}

struct UnusedProjectRepo;

#[async_trait]
impl ProjectRepo for UnusedProjectRepo {
    async fn find_by_client_id(&self, _: &str) -> RepoResult<Option<Project>> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: Uuid) -> RepoResult<Option<Project>> {
        unimplemented!()
    }
    async fn list_all(&self) -> RepoResult<Vec<Project>> {
        unimplemented!()
    }
    async fn create(&self, _: NewProject) -> RepoResult<Project> {
        unimplemented!()
    }
    async fn update(&self, _: &Project) -> RepoResult<Project> {
        unimplemented!()
    }
    async fn delete(&self, _: Uuid) -> RepoResult<()> {
        unimplemented!()
    }
    async fn set_cameras(&self, _: Uuid, _: &[Uuid]) -> RepoResult<()> {
        unimplemented!()
    }
    async fn allowed_camera_paths(&self, _: Uuid) -> RepoResult<Vec<String>> {
        unimplemented!()
    }
    async fn assigned_camera_ids(&self, _: Uuid) -> RepoResult<Vec<Uuid>> {
        unimplemented!()
    }
}

struct UnusedFailureRepo;

#[async_trait]
impl FailureRepo for UnusedFailureRepo {
    async fn record(&self, _: NewFailure) -> RepoResult<Failure> {
        unimplemented!()
    }
    async fn list_by_camera(&self, _: &str, _: i64) -> RepoResult<Vec<Failure>> {
        unimplemented!()
    }
    async fn list_recent(&self, _: i64) -> RepoResult<Vec<Failure>> {
        unimplemented!()
    }
    async fn latest_by_camera(&self, _: &str) -> RepoResult<Option<Failure>> {
        unimplemented!()
    }
}

struct UnusedProvisioner;

#[async_trait]
impl CameraProvisioner for UnusedProvisioner {
    async fn apply(&self, _: &Camera) -> ProvisionResult<()> {
        unimplemented!()
    }
    async fn remove(&self, _: &str) -> ProvisionResult<()> {
        unimplemented!()
    }
    async fn list_paths(&self) -> ProvisionResult<Vec<String>> {
        unimplemented!()
    }
}

// --- Fixtures -----------------------------------------------------------------

/// Par de claves RS256; la generación RSA es lenta, así que se hace una sola vez.
fn signing_keys() -> &'static (EncodingKey, DecodingKey) {
    static KEYS: OnceLock<(EncodingKey, DecodingKey)> = OnceLock::new();
    KEYS.get_or_init(|| {
        let path = std::env::temp_dir().join(format!("snapshot-tests-{}/jwt.pem", Uuid::new_v4()));
        let material = crate::keys::load_or_create(path.to_str().unwrap()).expect("claves de prueba");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        (material.encoding_key, material.decoding_key)
    })
}

fn camera(path: &str, enabled: bool) -> Camera {
    Camera {
        id: Uuid::new_v4(),
        path: path.into(),
        rtsp_url: "rtsp://x".into(),
        record: true,
        enabled,
        description: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn perm(action: &str, path: &str) -> MtxPermission {
    MtxPermission {
        action: action.into(),
        path: path.into(),
    }
}

fn token(permissions: Vec<MtxPermission>, expires_in_secs: i64) -> String {
    let claims = Claims {
        sub: "proyecto".into(),
        exp: OffsetDateTime::now_utc().unix_timestamp() + expires_in_secs,
        mediamtx_permissions: permissions,
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("key1".into());
    encode(&header, &claims, &signing_keys().0).expect("firmar JWT de prueba")
}

fn valid_token(permissions: Vec<MtxPermission>) -> String {
    token(permissions, 3600)
}

fn config() -> Config {
    Config {
        server_port: 0,
        jwt_exp_minutes: 60,
        jwt_private_key_path: String::new(),
        clients_path: String::new(),
        database_url: String::new(),
        db_encryption_key: String::new(),
        mediamtx_api_url: String::new(),
        reconcile_interval_secs: 300,
        admin_api_token: String::new(),
        mediamtx_rtsp_url: "rtsp://mediamtx:8554".into(),
        snapshot_max_concurrent: 4,
    }
}

/// Cámaras del repo falso: `cam-a` y `cam-b` habilitadas, `cam-off` deshabilitada.
fn app(ffmpeg: &FakeFfmpeg, snapshot_timeout: Duration) -> axum::Router {
    let camera_repo: Arc<dyn CameraRepo> = Arc::new(FakeCameraRepo {
        cameras: vec![camera("cam-a", true), camera("cam-b", true), camera("cam-off", false)],
    });
    let project_repo: Arc<dyn ProjectRepo> = Arc::new(UnusedProjectRepo);
    let state = Arc::new(AppState {
        encoding_key: signing_keys().0.clone(),
        decoding_key: signing_keys().1.clone(),
        jwks: Jwks { keys: vec![] },
        auth: Arc::new(AuthService::new(project_repo.clone())),
        config: config(),
        project_repo,
        camera_repo: camera_repo.clone(),
        failure_repo: Arc::new(UnusedFailureRepo),
        reconciler: Arc::new(ReconcilerService::new(camera_repo, Arc::new(UnusedProvisioner))),
        snapshot: Arc::new(SnapshotService::new(
            ffmpeg.bin(),
            "rtsp://mediamtx:8554",
            2,
            snapshot_timeout,
        )),
    });
    consumer::router().with_state(state)
}

async fn get(app: axum::Router, uri: &str, bearer: Option<&str>) -> Response {
    let mut request = Request::builder().uri(uri);
    if let Some(token) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    app.oneshot(request.body(Body::empty()).unwrap()).await.unwrap()
}

async fn status_of(ffmpeg: &FakeFfmpeg, uri: &str, bearer: Option<&str>) -> StatusCode {
    get(app(ffmpeg, AMPLE_TIMEOUT), uri, bearer).await.status()
}

// --- 401 ----------------------------------------------------------------------

#[tokio::test]
async fn without_token_is_401() {
    let ffmpeg = FakeFfmpeg::succeeding();
    assert_eq!(status_of(&ffmpeg, "/cameras/cam-a/snapshot", None).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn malformed_token_is_401() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let status = status_of(&ffmpeg, "/cameras/cam-a/snapshot", Some("no-es-un-jwt")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_token_is_401() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let expired = token(vec![perm("read", "")], -3600);
    let status = status_of(&ffmpeg, "/cameras/cam-a/snapshot", Some(&expired)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// --- 403 ----------------------------------------------------------------------

#[tokio::test]
async fn token_without_permission_on_the_path_is_403() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let other_camera = valid_token(vec![perm("read", "cam-b")]);
    let status = status_of(&ffmpeg, "/cameras/cam-a/snapshot", Some(&other_camera)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn token_with_only_playback_on_the_path_is_403() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let playback_only = valid_token(vec![perm("playback", "cam-a")]);
    let status = status_of(&ffmpeg, "/cameras/cam-a/snapshot", Some(&playback_only)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// --- 404 ----------------------------------------------------------------------

#[tokio::test]
async fn unknown_camera_is_404() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let all = valid_token(vec![perm("read", "")]);
    let status = status_of(&ffmpeg, "/cameras/no-existe/snapshot", Some(&all)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn disabled_camera_is_404() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let all = valid_token(vec![perm("read", "")]);
    let status = status_of(&ffmpeg, "/cameras/cam-off/snapshot", Some(&all)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// --- 422 ----------------------------------------------------------------------

#[tokio::test]
async fn invalid_width_is_422_and_never_runs_ffmpeg() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let all = valid_token(vec![perm("read", "")]);
    for width in ["50", "159", "1921", "0", "abc", "-1"] {
        let uri = format!("/cameras/cam-a/snapshot?width={width}");
        let status = status_of(&ffmpeg, &uri, Some(&all)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "width={width}");
    }
    assert!(!ffmpeg.path("args").exists(), "ffmpeg no debió ejecutarse");
}

// --- 200: forma de la respuesta ----------------------------------------------

#[tokio::test]
async fn all_cameras_token_with_empty_permission_path_gets_a_jpeg() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let all = valid_token(vec![perm("read", ""), perm("playback", "")]);
    let response = get(app(&ffmpeg, AMPLE_TIMEOUT), "/cameras/cam-a/snapshot", Some(&all)).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body.as_ref(), FAKE_JPEG);
}

#[tokio::test]
async fn token_scoped_to_the_path_gets_a_jpeg() {
    let ffmpeg = FakeFfmpeg::succeeding();
    let scoped = valid_token(vec![perm("read", "cam-a")]);
    let status = status_of(&ffmpeg, "/cameras/cam-a/snapshot", Some(&scoped)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn width_defaults_to_1280_and_is_forwarded_when_given() {
    let all = valid_token(vec![perm("read", "")]);

    let default_run = FakeFfmpeg::succeeding();
    get(app(&default_run, AMPLE_TIMEOUT), "/cameras/cam-a/snapshot", Some(&all)).await;
    assert!(default_run.recorded_args().contains(&"scale=1280:-2".to_string()));

    let custom_run = FakeFfmpeg::succeeding();
    get(app(&custom_run, AMPLE_TIMEOUT), "/cameras/cam-a/snapshot?width=640", Some(&all)).await;
    let args = custom_run.recorded_args();
    assert!(args.contains(&"scale=640:-2".to_string()));
    assert!(args.contains(&format!("rtsp://mediamtx:8554/cam-a?jwt={all}")));
}

// --- 502 / 504 ----------------------------------------------------------------

#[tokio::test]
async fn decoder_failure_is_502() {
    let ffmpeg = FakeFfmpeg::new("exit 1");
    let all = valid_token(vec![perm("read", "")]);
    let status = status_of(&ffmpeg, "/cameras/cam-a/snapshot", Some(&all)).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn camera_that_never_yields_a_frame_is_504() {
    let ffmpeg = FakeFfmpeg::new("exec sleep 30");
    let all = valid_token(vec![perm("read", "")]);
    let response = get(
        app(&ffmpeg, Duration::from_millis(300)),
        "/cameras/cam-a/snapshot",
        Some(&all),
    )
    .await;
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
}

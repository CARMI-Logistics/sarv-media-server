//! Backend de autenticación JWT para MediaMTX
//!
//! Este servicio genera tokens JWT firmados con RS256 y expone un JWKS
//! para que MediaMTX pueda validar los tokens.

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::RsaPublicKey;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{env, sync::Arc};
use time::{Duration, OffsetDateTime};
use tracing::{info, warn};
use utoipa::{OpenApi, ToSchema};
use utoipa_scalar::{Scalar, Servable};

mod crypto;
// Dominio: algunos puertos/modelos (failures) aún no los invoca un endpoint
// (se consumen desde HU 4.6); se permite dead_code hasta entonces.
#[allow(dead_code)]
mod domain;
mod infra;
mod keys;
mod secret;
mod services;

use domain::ports::{CameraProvisioner, CameraRepo, FailureRepo, ProjectRepo};
use infra::mediamtx::MediaMtxProvisioner;
use infra::postgres::{PgCameraRepo, PgFailureRepo, PgProjectRepo};
use services::auth::AuthService;
use services::reconciler::ReconcilerService;

// ============================================================================
// Configuración
// ============================================================================

/// Configuración del servidor leída de variables de entorno
#[derive(Clone)]
struct Config {
    /// Puerto del servidor HTTP
    server_port: u16,
    /// Minutos de expiración del JWT
    jwt_exp_minutes: i64,
    /// Ruta del archivo de la clave privada RSA (volumen persistente)
    jwt_private_key_path: String,
    /// Ruta del archivo JSON con credenciales (solo para el subcomando
    /// `migrate-clients`; la autenticación en runtime ya usa la BD).
    #[allow(dead_code)]
    clients_path: String,
    /// Cadena de conexión a Postgres (contiene credenciales → se redacta en logs)
    database_url: String,
    /// Clave AES-256-GCM en base64 para cifrado en reposo (secreto → se redacta)
    db_encryption_key: String,
    /// URL de la Control API de MediaMTX (interno, sin credenciales)
    mediamtx_api_url: String,
    /// Intervalo del reconcile periódico, en segundos
    reconcile_interval_secs: u64,
}

/// `Debug` manual: NUNCA imprime la cadena de conexión ni la clave de cifrado.
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("server_port", &self.server_port)
            .field("jwt_exp_minutes", &self.jwt_exp_minutes)
            .field("jwt_private_key_path", &self.jwt_private_key_path)
            .field("clients_path", &self.clients_path)
            .field("database_url", &"<redactado>")
            .field("db_encryption_key", &"<redactado>")
            .field("mediamtx_api_url", &self.mediamtx_api_url)
            .field("reconcile_interval_secs", &self.reconcile_interval_secs)
            .finish()
    }
}

impl Config {
    /// Carga la configuración desde variables de entorno con valores por defecto
    fn from_env() -> Self {
        let server_port = env::var("SERVER_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8080);

        let jwt_exp_minutes = env::var("JWT_EXP_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let jwt_private_key_path = env::var("JWT_PRIVATE_KEY_PATH")
            .unwrap_or_else(|_| "/keys/jwt_private_key.pem".to_string());

        let clients_path = env::var("CLIENTS_PATH")
            .unwrap_or_else(|_| "/config/clients.json".to_string());

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://mediamtx:mediamtx@localhost:5432/mediamtx".to_string());

        let db_encryption_key = env::var("DB_ENCRYPTION_KEY").unwrap_or_default();

        let mediamtx_api_url =
            env::var("MEDIAMTX_API_URL").unwrap_or_else(|_| "http://mediamtx:9997".to_string());

        let reconcile_interval_secs = env::var("RECONCILE_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);

        Self {
            server_port,
            jwt_exp_minutes,
            jwt_private_key_path,
            clients_path,
            database_url,
            db_encryption_key,
            mediamtx_api_url,
            reconcile_interval_secs,
        }
    }
}

// ============================================================================
// Modelos de Claims para JWT
// ============================================================================

/// Individual permission entry for MediaMTX stream access control.
/// 
/// Each permission defines an action that can be performed on a specific path.
/// An empty path grants access to all streams.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[schema(example = json!({"action": "read", "path": ""}))] 
struct MtxPermission {
    /// The action type. Valid values:
    /// - `read`: View/consume streams
    /// - `publish`: Push/broadcast streams  
    /// - `playback`: Access recorded content
    #[schema(example = "read")]
    action: String,
    
    /// Target stream path. Use empty string for wildcard access to all paths.
    #[schema(example = "")]
    path: String,
}

/// JWT Claims payload for MediaMTX authentication.
/// 
/// This structure represents the decoded JWT payload that MediaMTX
/// uses to authorize stream access. The `mediamtx_permissions` claim
/// is specifically required by MediaMTX for permission validation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct Claims {
    /// Subject identifier (the project / client id)
    #[schema(example = "sigac")]
    sub: String,
    
    /// Token expiration time as Unix timestamp (seconds since epoch)
    #[schema(example = 1733817600)]
    exp: i64,
    
    /// Array of MediaMTX permissions granted to this token.
    /// MediaMTX reads this claim to determine stream access rights.
    mediamtx_permissions: Vec<MtxPermission>,
}

// ============================================================================
// JWKS (JSON Web Key Set)
// ============================================================================

/// JSON Web Key (JWK) representing a public RSA key.
/// 
/// This structure follows the JWK specification (RFC 7517) and contains
/// the public key components needed to verify RS256 JWT signatures.
#[derive(Debug, Serialize, Clone, ToSchema)]
struct Jwk {
    /// Key type. Always "RSA" for this implementation.
    #[schema(example = "RSA")]
    kty: String,
    
    /// Public key use. "sig" indicates this key is used for signatures.
    #[serde(rename = "use")]
    #[schema(example = "sig")]
    use_: String,
    
    /// Algorithm intended for use with this key.
    #[schema(example = "RS256")]
    alg: String,
    
    /// Key ID. Used to match specific keys when multiple are present.
    #[schema(example = "key1")]
    kid: String,
    
    /// RSA public key modulus (Base64 URL-encoded)
    #[schema(example = "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw")]
    n: String,
    
    /// RSA public key exponent (Base64 URL-encoded)
    #[schema(example = "AQAB")]
    e: String,
}

/// JSON Web Key Set (JWKS) containing public keys for JWT verification.
/// 
/// This endpoint is consumed by MediaMTX to validate JWT signatures.
/// The JWKS format follows RFC 7517 specification.
#[derive(Debug, Serialize, Clone, ToSchema)]
struct Jwks {
    /// Array of JSON Web Keys. MediaMTX will use the `kid` header
    /// from incoming JWTs to select the appropriate key.
    keys: Vec<Jwk>,
}

// ============================================================================
// Estado de la aplicación
// ============================================================================

/// Estado global compartido entre handlers
struct AppState {
    /// Clave para firmar JWT (RS256)
    encoding_key: EncodingKey,
    /// JWKS preconstruido en memoria
    jwks: Jwks,
    /// Autenticación de proyectos contra la BD (HU 4.3)
    auth: Arc<AuthService>,
    /// Configuración
    config: Config,
    /// Repositorios (puertos) respaldados por Postgres (HU 4.1).
    /// Aún sin consumir por los handlers; se usan desde HU 4.2+.
    #[allow(dead_code)]
    project_repo: Arc<dyn ProjectRepo>,
    #[allow(dead_code)]
    camera_repo: Arc<dyn CameraRepo>,
    #[allow(dead_code)]
    failure_repo: Arc<dyn FailureRepo>,
    /// Reconciler BD → MediaMTX (HU 4.2). Lo usa la tarea de arranque y, en
    /// HU 4.5, los endpoints de administración para sync puntual.
    reconciler: Arc<ReconcilerService>,
}

impl AppState {
    /// Crea un nuevo AppState generando un par de claves RSA
    fn new(config: Config, db: PgPool) -> Result<Self, Box<dyn std::error::Error>> {
        // Carga la clave de firma persistente (o la genera y guarda la 1a vez).
        let material = keys::load_or_create(&config.jwt_private_key_path)?;

        // Construir JWKS desde la clave pública
        let jwks = Self::build_jwks(&material.public_key)?;
        info!("JWKS construido con kid='key1'");

        // Cifrador de credenciales de cámara en reposo (fail-closed: sin clave
        // válida no arrancamos; no podríamos almacenar cámaras de forma segura).
        let cipher = crypto::Cipher::from_base64_key(&config.db_encryption_key)?;
        info!("Cifrado en reposo inicializado (AES-256-GCM)");

        // Repositorios (adaptadores Postgres) detrás de los puertos del dominio.
        let project_repo: Arc<dyn ProjectRepo> = Arc::new(PgProjectRepo::new(db.clone()));
        let camera_repo: Arc<dyn CameraRepo> = Arc::new(PgCameraRepo::new(db.clone(), cipher));
        let failure_repo: Arc<dyn FailureRepo> = Arc::new(PgFailureRepo::new(db));

        // Autenticación de proyectos contra la BD (HU 4.3).
        let auth = Arc::new(AuthService::new(project_repo.clone()));

        // Reconciler BD → MediaMTX (HU 4.2).
        let provisioner: Arc<dyn CameraProvisioner> =
            Arc::new(MediaMtxProvisioner::new(&config.mediamtx_api_url));
        let reconciler = Arc::new(ReconcilerService::new(camera_repo.clone(), provisioner));

        Ok(Self {
            encoding_key: material.encoding_key,
            jwks,
            auth,
            config,
            project_repo,
            camera_repo,
            failure_repo,
            reconciler,
        })
    }

    /// Construye el JWKS a partir de la clave pública RSA
    fn build_jwks(public_key: &RsaPublicKey) -> Result<Jwks, Box<dyn std::error::Error>> {
        // Obtener los componentes n y e de la clave pública
        use rsa::traits::PublicKeyParts;
        
        let n_bytes = public_key.n().to_bytes_be();
        let e_bytes = public_key.e().to_bytes_be();

        // Codificar en Base64 URL-safe sin padding
        let n_b64 = URL_SAFE_NO_PAD.encode(&n_bytes);
        let e_b64 = URL_SAFE_NO_PAD.encode(&e_bytes);

        let jwk = Jwk {
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
            alg: "RS256".to_string(),
            kid: "key1".to_string(),
            n: n_b64,
            e: e_b64,
        };

        Ok(Jwks { keys: vec![jwk] })
    }

    /// Genera un JWT firmado con RS256
    fn generate_jwt(&self, client_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = OffsetDateTime::now_utc();
        let exp = now + Duration::minutes(self.config.jwt_exp_minutes);

        let claims = Claims {
            sub: client_id.to_string(),
            exp: exp.unix_timestamp(),
            mediamtx_permissions: vec![
                MtxPermission {
                    action: "read".to_string(),
                    path: "".to_string(),
                },
                MtxPermission {
                    action: "publish".to_string(),
                    path: "".to_string(),
                },
                MtxPermission {
                    action: "playback".to_string(),
                    path: "".to_string(),
                },
            ],
        };

        // Header con kid y algoritmo RS256
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("key1".to_string());

        encode(&header, &claims, &self.encoding_key)
    }
}

// ============================================================================
// Request/Response models
// ============================================================================

/// Authentication request payload.
/// 
/// Submit user credentials to obtain a JWT token for MediaMTX access.
#[derive(Debug, Deserialize, ToSchema)]
struct LoginRequest {
    /// Project/client identifier
    #[schema(example = "sigac", min_length = 1, max_length = 100)]
    client_id: String,

    /// Project secret (transmitted securely over HTTPS in production)
    #[schema(example = "s3cret", min_length = 1)]
    client_secret: String,
}

/// Successful authentication response containing the JWT.
/// 
/// The returned token should be included in MediaMTX requests either:
/// - As a query parameter: `?jwt=<token>`
/// - As an Authorization header: `Authorization: Bearer <token>`
#[derive(Debug, Serialize, ToSchema)]
struct LoginResponse {
    /// Signed JWT token (RS256 algorithm).
    /// Contains user identity and MediaMTX permissions.
    /// Default expiration: 60 minutes (configurable via JWT_EXP_MINUTES).
    #[schema(example = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiIsImtpZCI6ImtleTEifQ.eyJzdWIiOiJhZG1pbiIsImV4cCI6MTczMzgxNzYwMCwibWVkaWFtdHhfcGVybWlzc2lvbnMiOlt7ImFjdGlvbiI6InJlYWQiLCJwYXRoIjoiIn1dfQ.signature")]
    token: String,
}

/// Error response returned when a request fails.
/// 
/// All API errors follow this consistent format for easy client handling.
#[derive(Debug, Serialize, ToSchema)]
struct ErrorResponse {
    /// Human-readable error message describing what went wrong
    #[schema(example = "Invalid credentials")]
    error: String,
}

/// Health check response indicating service status.
/// 
/// Use this endpoint for container orchestration health probes
/// and monitoring systems.
#[derive(Debug, Serialize, ToSchema)]
struct HealthResponse {
    /// Service health status. "ok" indicates the service is operational.
    #[schema(example = "ok")]
    status: String,
    
    /// Service identifier
    #[schema(example = "mediamtx-auth-backend")]
    service: String,
    
    /// Semantic version of the deployed service
    #[schema(example = "0.1.0")]
    version: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Retrieve the JSON Web Key Set (JWKS) for token validation.
///
/// This endpoint exposes the public keys used to verify JWT signatures.
/// MediaMTX fetches this endpoint (configured via `authJWTJWKS`) to validate
/// incoming tokens. The keys are generated at server startup and remain
/// constant for the lifetime of the service.
///
/// ## Usage
/// Configure MediaMTX with:
/// ```yaml
/// authJWTJWKS: http://mediamtx-backend:8080/jwks
/// ```
#[utoipa::path(
    get,
    path = "/jwks",
    tag = "JWT & Token Management",
    operation_id = "getJwks",
    responses(
        (status = 200, description = "JWKS containing public keys for signature verification", body = Jwks,
            example = json!({
                "keys": [{
                    "kty": "RSA",
                    "use": "sig",
                    "alg": "RS256",
                    "kid": "key1",
                    "n": "0vx7agoebGcQ...",
                    "e": "AQAB"
                }]
            })
        )
    )
)]
async fn get_jwks(State(state): State<Arc<AppState>>) -> Json<Jwks> {
    info!("Solicitud de JWKS recibida");
    Json(state.jwks.clone())
}

/// Authenticate user and generate a JWT token.
///
/// Validates the provided credentials and returns a signed JWT token
/// that can be used to access MediaMTX streams. The token is signed
/// using RS256 (RSA Signature with SHA-256) algorithm.
///
/// ## Token Contents
/// The generated JWT includes:
/// - `sub`: Subject identifier (project / client id)
/// - `exp`: Expiration timestamp
/// - `mediamtx_permissions`: Array of granted permissions
///
/// ## Default Permissions
/// Tokens grant full access (read, publish, playback) to all paths.
///
/// ## Token Usage
/// Include the token in MediaMTX requests:
/// ```
/// GET http://localhost:8888/stream/index.m3u8?jwt=<token>
/// ```
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "Authentication",
    operation_id = "login",
    request_body(content = LoginRequest, description = "Project credentials",
        example = json!({"client_id": "sigac", "client_secret": "s3cret"})
    ),
    responses(
        (status = 200, description = "Authentication successful. Returns signed JWT.", body = LoginResponse,
            example = json!({"token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiIsImtpZCI6ImtleTEifQ..."})
        ),
        (status = 401, description = "Authentication failed. Invalid client_id or client_secret.", body = ErrorResponse,
            example = json!({"error": "Invalid credentials"})
        ),
        (status = 500, description = "Internal server error during token generation.", body = ErrorResponse,
            example = json!({"error": "Token generation failed"})
        )
    )
)]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Intento de login para proyecto: {}", payload.client_id);

    // Validar credenciales contra la BD (fail-closed).
    let project = match state
        .auth
        .authenticate(&payload.client_id, &payload.client_secret)
        .await
    {
        Some(project) => project,
        None => {
            warn!("Credenciales inválidas para proyecto: {}", payload.client_id);
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Credenciales inválidas".to_string(),
                }),
            ));
        }
    };

    // Generar JWT
    match state.generate_jwt(&project.client_id) {
        Ok(token) => {
            info!("JWT generado exitosamente para proyecto: {}", project.client_id);
            Ok(Json(LoginResponse { token }))
        }
        Err(e) => {
            warn!("Error generando JWT: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Error generando token".to_string(),
                }),
            ))
        }
    }
}

/// Service health check endpoint.
///
/// Returns the current operational status of the authentication backend.
/// Use this endpoint for:
/// - Kubernetes/Docker health probes
/// - Load balancer health checks
/// - Monitoring and alerting systems
///
/// A 200 response indicates the service is ready to handle requests.
#[utoipa::path(
    get,
    path = "/health",
    tag = "System & Monitoring",
    operation_id = "healthCheck",
    responses(
        (status = 200, description = "Service is healthy and operational", body = HealthResponse,
            example = json!({
                "status": "ok",
                "service": "mediamtx-auth-backend",
                "version": "0.1.0"
            })
        )
    )
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "mediamtx-auth-backend".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// ============================================================================
// OpenAPI Documentation
// ============================================================================

#[derive(OpenApi)]
#[openapi(
    info(
        title = "MediaMTX Authentication API",
        version = "1.0.0",
        description = r#"
## Overview

This API provides JWT-based authentication for [MediaMTX](https://github.com/bluenviron/mediamtx) streaming server. It generates RS256-signed tokens that MediaMTX uses to authorize stream access.

## Authentication Flow

```
┌──────────┐                    ┌──────────────┐                    ┌──────────┐
│  Client  │                    │ Auth Backend │                    │ MediaMTX │
└────┬─────┘                    └──────┬───────┘                    └────┬─────┘
     │                                 │                                  │
     │  1. POST /auth/login            │                                  │
     │  {client_id, client_secret}     │                                  │
     │────────────────────────────────>│                                  │
     │                                 │                                  │
     │  2. JWT Token (RS256)           │                                  │
     │<────────────────────────────────│                                  │
     │                                 │                                  │
     │  3. Stream Request + JWT        │                                  │
     │───────────────────────────────────────────────────────────────────>│
     │                                 │                                  │
     │                                 │  4. GET /jwks (cached)           │
     │                                 │<─────────────────────────────────│
     │                                 │                                  │
     │                                 │  5. JWKS Response                │
     │                                 │─────────────────────────────────>│
     │                                 │                                  │
     │  6. Stream Data                 │                                  │
     │<──────────────────────────────────────────────────────────────────│
```

## Quick Start

### 1. Obtain a Token
```bash
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"client_id": "sigac", "client_secret": "s3cret"}'
```

### 2. Access Streams
```bash
# HLS
curl "http://localhost:8888/stream/index.m3u8?jwt=YOUR_TOKEN"

# Or use Authorization header
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8888/stream/index.m3u8
```

## Token Structure

The JWT contains the following claims:

| Claim | Description |
|-------|-------------|
| `sub` | Subject (client id) |
| `exp` | Expiration timestamp |
| `mediamtx_permissions` | Array of permission objects |

### Permission Object
```json
{
  "action": "read|publish|playback",
  "path": "" // empty = all paths
}
```

## Security Considerations

- Tokens are signed with RS256 (2048-bit RSA keys)
- Keys are persisted across restarts (mounted volume)
- Per-project credentials, secrets stored hashed (Argon2id); no shared user
- Default token expiration: 60 minutes (configurable via JWT_EXP_MINUTES)
- Token renewal: clients re-authenticate via `POST /auth/login` before expiry
  to obtain a fresh token (no refresh tokens; stateless machine-to-machine)
- Use HTTPS in production environments
"#,
        contact(
            name = "API Support",
            email = "support@example.com",
            url = "https://github.com/your-org/mediamtx-auth-backend"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "http://mediamtx-backend:8080", description = "Docker internal network")
    ),
    tags(
        (name = "Authentication", description = "User authentication and token generation"),
        (name = "JWT & Token Management", description = "JSON Web Key Set and token validation endpoints"),
        (name = "System & Monitoring", description = "Health checks and service status")
    ),
    paths(
        get_jwks,
        login,
        health
    ),
    components(
        schemas(
            LoginRequest,
            LoginResponse,
            ErrorResponse,
            HealthResponse,
            Jwks,
            Jwk,
            Claims,
            MtxPermission
        )
    )
)]
struct ApiDoc;

// ============================================================================
// Main
// ============================================================================

/// Intentos del reconcile inicial (MediaMTX puede tardar en estar listo).
const RECONCILE_BOOT_ATTEMPTS: u32 = 10;

/// Lanza el reconciler en segundo plano: reconcile inicial con reintentos y
/// luego periódico. No bloquea el arranque del servidor HTTP.
fn spawn_reconciler(reconciler: Arc<ReconcilerService>, interval_secs: u64) {
    tokio::spawn(async move {
        for attempt in 1..=RECONCILE_BOOT_ATTEMPTS {
            match reconciler.reconcile_all().await {
                Ok(()) => break,
                Err(e) => {
                    warn!(
                        "Reconcile inicial falló (intento {}/{}): {}",
                        attempt, RECONCILE_BOOT_ATTEMPTS, e
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
        let interval = std::time::Duration::from_secs(interval_secs);
        loop {
            tokio::time::sleep(interval).await;
            if let Err(e) = reconciler.reconcile_all().await {
                warn!("Reconcile periódico falló: {}", e);
            }
        }
    });
}

/// Subcomando one-time: importa a la BD las cámaras configuradas en el MediaMTX
/// vivo (source RTSP), cifrando la URL. Idempotente: omite las que ya existan.
/// SEGURIDAD: solo registra el nombre de la ruta, nunca la URL con credenciales.
async fn migrate_cameras(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let pool = infra::db::connect_with_retry(&config.database_url).await?;
    infra::db::run_migrations(&pool).await?;
    let cipher = crypto::Cipher::from_base64_key(&config.db_encryption_key)?;
    let repo = PgCameraRepo::new(pool, cipher);
    let provisioner = MediaMtxProvisioner::new(&config.mediamtx_api_url);

    let paths = provisioner.list_source_paths().await?;
    let (mut imported, mut skipped) = (0u32, 0u32);

    for p in paths {
        // Solo cámaras de pull (source RTSP); saltamos regex y publishers.
        let source = match p.source {
            Some(s) if !p.name.starts_with('~') && s.starts_with("rtsp://") => s,
            _ => {
                skipped += 1;
                continue;
            }
        };
        if repo.find_by_path(&p.name).await?.is_some() {
            info!("ya existe en BD, omito: {}", p.name);
            skipped += 1;
            continue;
        }
        repo.create(domain::models::NewCamera {
            path: p.name.clone(),
            rtsp_url: source,
            record: p.record,
            enabled: true,
            description: None,
        })
        .await?;
        imported += 1;
        info!("importada: {}", p.name);
    }

    info!(
        "Migración de cámaras: {} importada(s), {} omitida(s)",
        imported, skipped
    );
    Ok(())
}

/// Subcomando one-time: importa a la BD los proyectos de `clients.json`
/// (client_id + secret_hash ya hasheado) con `all_cameras=true` para preservar
/// el comportamiento actual. Idempotente: omite los que ya existan.
async fn migrate_clients(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(serde::Deserialize)]
    struct ClientEntry {
        client_id: String,
        secret_hash: String,
    }
    #[derive(serde::Deserialize)]
    struct ClientsFile {
        clients: Vec<ClientEntry>,
    }

    let content = std::fs::read_to_string(&config.clients_path)
        .map_err(|e| format!("no se pudo leer {}: {}", config.clients_path, e))?;
    let file: ClientsFile =
        serde_json::from_str(&content).map_err(|e| format!("clients.json inválido: {e}"))?;

    let pool = infra::db::connect_with_retry(&config.database_url).await?;
    infra::db::run_migrations(&pool).await?;
    let repo = PgProjectRepo::new(pool);

    let (mut imported, mut skipped) = (0u32, 0u32);
    for c in file.clients {
        if repo.find_by_client_id(&c.client_id).await?.is_some() {
            info!("ya existe en BD, omito: {}", c.client_id);
            skipped += 1;
            continue;
        }
        repo.create(domain::models::NewProject {
            client_id: c.client_id.clone(),
            secret_hash: c.secret_hash,
            all_cameras: true,
            enabled: true,
        })
        .await?;
        imported += 1;
        info!("importado proyecto: {}", c.client_id);
    }

    info!(
        "Migración de proyectos: {} importado(s), {} omitido(s)",
        imported, skipped
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Subcomando: generar el hash Argon2 de un secreto (alta de proyectos).
    //   mediamtx-auth-backend hash <secreto>
    let args: Vec<String> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("hash") {
        let secret = args
            .get(2)
            .ok_or("uso: mediamtx-auth-backend hash <secreto>")?;
        println!("{}", secret::hash_secret(secret)?);
        return Ok(());
    }

    // Cargar variables de entorno desde .env
    dotenvy::dotenv().ok();

    // Inicializar tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mediamtx_auth_backend=info".parse()?)
                .add_directive("tower_http=info".parse()?),
        )
        .init();

    // Cargar configuración
    let config = Config::from_env();
    info!("Configuración cargada: {:?}", config);

    // Subcomando one-time: importar a la BD las cámaras del MediaMTX vivo.
    if args.get(1).map(String::as_str) == Some("migrate-cameras") {
        return migrate_cameras(&config).await;
    }

    // Subcomando one-time: importar a la BD los proyectos de clients.json.
    if args.get(1).map(String::as_str) == Some("migrate-clients") {
        return migrate_clients(&config).await;
    }

    // Conexión a Postgres (con reintento) y migraciones de esquema al arranque
    // (HU 4.1). Fail-closed: si la BD o las migraciones fallan, no arrancamos.
    let db_pool = infra::db::connect_with_retry(&config.database_url).await?;
    infra::db::run_migrations(&db_pool).await?;

    // Crear estado de la aplicación
    let state = Arc::new(AppState::new(config.clone(), db_pool)?);

    // Reconciler en segundo plano (HU 4.2): sincroniza BD → MediaMTX al arranque
    // (con reintentos) y luego periódicamente para sanar deriva.
    spawn_reconciler(state.reconciler.clone(), config.reconcile_interval_secs);

    // Construir router con documentación OpenAPI
    let app = Router::new()
        // Endpoints de la API
        .route("/health", get(health))
        .route("/jwks", get(get_jwks))
        .route("/auth/login", post(login))
        // Documentación OpenAPI (Scalar UI)
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        // Endpoint para obtener el JSON de OpenAPI
        .route("/openapi.json", get(openapi_json))
        .with_state(state);

    // Iniciar servidor
    let addr = format!("0.0.0.0:{}", config.server_port);
    info!("Servidor iniciando en http://{}", addr);
    info!("Endpoints disponibles:");
    info!("  GET  /health       - Health check");
    info!("  GET  /jwks         - JSON Web Key Set");
    info!("  POST /auth/login   - Login y obtención de JWT");
    info!("  GET  /docs         - Documentación API (Scalar)");
    info!("  GET  /openapi.json - Especificación OpenAPI");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Devuelve la especificación OpenAPI en formato JSON
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

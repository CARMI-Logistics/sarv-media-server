//! Backend de autenticación JWT para MediaMTX
//!
//! Este servicio genera tokens JWT firmados con RS256 y expone un JWKS
//! para que MediaMTX pueda validar los tokens.
//!
//! También gestiona cámaras (CRUD + búsqueda) y generación de mosaicos FFmpeg.

mod db;
mod models;
mod camera;
mod mosaic;
mod location;
mod user;
mod capture;
mod notification;
mod role;
mod mosaic_share;
mod security;
mod email;
mod thumbnail;
mod sync;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post, put, delete},
    Json, Router,
    middleware::{self, Next},
    response::Response,
};
use axum::http::Request;
use tower_http::services::ServeDir;
use tower_http::cors::{CorsLayer, Any};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{encode, decode, Algorithm, EncodingKey, DecodingKey, Header, Validation};
use rand::rngs::OsRng;
use rsa::{pkcs8::EncodePrivateKey, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use time::{Duration, OffsetDateTime};
use tracing::{info, warn};
use utoipa::{OpenApi, ToSchema};
use utoipa_scalar::{Scalar, Servable};

// ============================================================================
// Configuración
// ============================================================================

/// Configuración del servidor leída de variables de entorno
#[derive(Debug, Clone)]
struct Config {
    /// Puerto del servidor HTTP
    server_port: u16,
    /// Minutos de expiración del JWT
    jwt_exp_minutes: i64,
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

        Self {
            server_port,
            jwt_exp_minutes,
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
    /// Subject identifier (typically the username or user ID)
    #[schema(example = "admin")]
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
pub struct AppState {
    /// Clave para firmar JWT (RS256)
    encoding_key: EncodingKey,
    /// Clave para verificar JWT (RS256)
    decoding_key: DecodingKey,
    /// JWKS preconstruido en memoria
    jwks: Jwks,
    /// Configuración
    config: Config,
    /// Base de datos SQLite
    pub db: db::Database,
    /// URL de la API de MediaMTX
    pub mediamtx_api_url: String,
    /// Usuario para la API de MediaMTX (Basic Auth)
    pub mediamtx_api_user: String,
    /// Contraseña para la API de MediaMTX (Basic Auth)
    pub mediamtx_api_pass: String,
    /// HTTP client reutilizable (connection pooling)
    pub http_client: reqwest::Client,
    /// Email service with templates
    pub email_service: email::EmailService,
}

impl AppState {
    /// Crea un nuevo AppState generando un par de claves RSA
    fn new(config: Config, db: db::Database, mediamtx_api_url: String, mediamtx_api_user: String, mediamtx_api_pass: String) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Generando par de claves RSA 2048 bits...");

        // Generar par de claves RSA 2048 bits
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048)?;
        let public_key = RsaPublicKey::from(&private_key);

        // Obtener la clave privada en formato PEM para jsonwebtoken
        let private_pem = private_key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)?;
        let encoding_key = EncodingKey::from_rsa_pem(private_pem.as_bytes())?;

        // Obtener la clave pública en formato PEM para verificación
        use rsa::pkcs8::EncodePublicKey;
        let public_pem = public_key.to_public_key_pem(rsa::pkcs8::LineEnding::LF)?;
        let decoding_key = DecodingKey::from_rsa_pem(public_pem.as_bytes())?;

        // Construir JWKS desde la clave pública
        let jwks = Self::build_jwks(&public_key)?;

        info!("Par de claves RSA generado exitosamente");
        info!("JWKS construido con kid='key1'");

        let http_client = reqwest::Client::new();

        // Initialize email service
        let resend_api_key = env::var("RESEND_API_KEY").unwrap_or_default();
        let from_email = env::var("EMAIL_FROM").unwrap_or_else(|_| "noreply@example.com".to_string());
        let email_service = email::EmailService::new(resend_api_key, from_email, http_client.clone());

        Ok(Self {
            encoding_key,
            decoding_key,
            jwks,
            config,
            db,
            mediamtx_api_url,
            mediamtx_api_user,
            mediamtx_api_pass,
            http_client,
            email_service,
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
    fn generate_jwt(&self, username: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = OffsetDateTime::now_utc();
        let exp = now + Duration::minutes(self.config.jwt_exp_minutes);

        let claims = Claims {
            sub: username.to_string(),
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
// JWT Auth Middleware
// ============================================================================

async fn jwt_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse { error: "Token requerido. Usa Authorization: Bearer <token>".to_string() }),
            ));
        }
    };

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_required_spec_claims(&["sub", "exp"]);

    match decode::<Claims>(token, &state.decoding_key, &validation) {
        Ok(token_data) => {
            // Guardar username en las extensiones de la request para uso en middleware de autorización
            req.extensions_mut().insert(token_data.claims.sub.clone());
            Ok(next.run(req).await)
        }
        Err(e) => {
            warn!("JWT inválido: {}", e);
            let msg = match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => "Token expirado",
                jsonwebtoken::errors::ErrorKind::InvalidSignature => "Firma inválida",
                _ => "Token inválido",
            };
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse { error: msg.to_string() }),
            ))
        }
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
    /// User account identifier
    #[schema(example = "admin", min_length = 1, max_length = 100)]
    username: String,
    
    /// User password (transmitted securely over HTTPS in production)
    #[schema(example = "admin", min_length = 1)]
    password: String,
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
/// - `sub`: Username/subject identifier
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
    request_body(content = LoginRequest, description = "User credentials", 
        example = json!({"username": "admin", "password": "admin"})
    ),
    responses(
        (status = 200, description = "Authentication successful. Returns signed JWT.", body = LoginResponse,
            example = json!({"token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiIsImtpZCI6ImtleTEifQ..."})
        ),
        (status = 401, description = "Authentication failed. Invalid username or password.", body = ErrorResponse,
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
    info!("Intento de login para usuario: {}", payload.username);

    // Look up user in the database
    let user = match state.db.get_user_by_username(&payload.username) {
        Ok(Some(u)) => u,
        Ok(None) => {
            warn!("Usuario no encontrado: {}", payload.username);
            return Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Credenciales inválidas".to_string() })));
        }
        Err(e) => {
            warn!("DB error looking up user: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Error interno".to_string() })));
        }
    };

    if !user.active {
        warn!("Usuario desactivado: {}", payload.username);
        return Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Cuenta desactivada".to_string() })));
    }

    // Verify password with bcrypt
    match bcrypt::verify(&payload.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            warn!("Contraseña incorrecta para usuario: {}", payload.username);
            return Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Credenciales inválidas".to_string() })));
        }
        Err(e) => {
            warn!("Bcrypt verify error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Error interno".to_string() })));
        }
    }

    // Generate JWT
    match state.generate_jwt(&payload.username) {
        Ok(token) => {
            info!("JWT generado exitosamente para usuario: {}", payload.username);
            Ok(Json(LoginResponse { token }))
        }
        Err(e) => {
            warn!("Error generando JWT: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Error generando token".to_string() })))
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
     │  {username, password}           │                                  │
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
  -d '{"username": "admin", "password": "admin"}'
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
| `sub` | Subject (username) |
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
- Keys are generated at startup (ephemeral)
- Default token expiration: 60 minutes
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Cargar variables de entorno desde .env
    dotenvy::dotenv().ok();

    // Inicializar tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    // Cargar configuración
    let config = Config::from_env();
    info!("Configuración cargada: {:?}", config);

    // Inicializar base de datos SQLite
    let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "/app/data/cameras.db".to_string());
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let database = db::Database::new(&db_path)?;
    database.seed_if_empty()?;

    // Seed default admin user (password: admin)
    let admin_hash = bcrypt::hash("admin", 10).expect("Failed to hash admin password");
    database.seed_admin_user(&admin_hash)?;

    // Seed default roles (admin, operator, viewer)
    database.seed_default_roles()?;

    // Ensure captures and thumbnails directories exist
    std::fs::create_dir_all("/app/data/captures").ok();
    std::fs::create_dir_all("/app/data/thumbnails").ok();

    // MediaMTX API URL and credentials
    let mediamtx_api_url = env::var("MEDIAMTX_API_URL")
        .unwrap_or_else(|_| "http://mediamtx:9997".to_string());
    let mediamtx_api_user = env::var("MEDIAMTX_API_USER")
        .unwrap_or_else(|_| "admin".to_string());
    let mediamtx_api_pass = env::var("MEDIAMTX_API_PASS")
        .unwrap_or_else(|_| "mediamtx_secret".to_string());

    // Crear estado de la aplicación
    let state = Arc::new(AppState::new(config.clone(), database, mediamtx_api_url, mediamtx_api_user, mediamtx_api_pass)?);

    // CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Subrouter de cámaras (protegido por JWT)
    let camera_routes = Router::new()
        .route("/", get(camera::list_cameras).post(camera::create_camera))
        .route("/:id", get(camera::get_camera).put(camera::update_camera).delete(camera::delete_camera))
        .route("/:id/thumbnail", get(camera::get_camera_thumbnail))
        .route("/sync", post(camera::sync_all_cameras))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de mosaicos (protegido por JWT)
    let mosaic_routes = Router::new()
        .route("/", get(mosaic::list_mosaics).post(mosaic::create_mosaic))
        .route("/:id", get(mosaic::get_mosaic).put(mosaic::update_mosaic).delete(mosaic::delete_mosaic))
        .route("/:id/start", post(mosaic::start_mosaic))
        .route("/:id/stop", post(mosaic::stop_mosaic))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de locations (protegido por JWT)
    let location_routes = Router::new()
        .route("/", get(location::list_locations).post(location::create_location))
        .route("/:id", put(location::update_location).delete(location::delete_location))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de areas (protegido por JWT)
    let area_routes = Router::new()
        .route("/", get(location::list_areas).post(location::create_area))
        .route("/:id", put(location::update_area).delete(location::delete_area))
        .route("/location/:location_id", get(location::list_areas_by_location))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de usuarios (protegido por JWT)
    let user_routes = Router::new()
        .route("/", get(user::list_users).post(user::create_user))
        .route("/:id", put(user::update_user).delete(user::delete_user))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de capturas (protegido por JWT)
    let capture_routes = Router::new()
        .route("/", get(capture::list_captures))
        .route("/screenshot/:camera_id", post(capture::take_screenshot))
        .route("/thumbnail/:camera_id", get(capture::get_thumbnail))
        .route("/thumbnails/toggle", post(capture::toggle_thumbnails))
        .route("/thumbnails/setting", get(capture::get_thumbnail_setting))
        .route("/:id", delete(capture::delete_capture))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de notificaciones (protegido por JWT)
    let notification_routes = Router::new()
        .route("/", get(notification::list_notifications))
        .route("/summary", get(notification::get_notification_summary))
        .route("/read-all", post(notification::mark_all_read))
        .route("/:id/read", post(notification::mark_read))
        .route("/:id", delete(notification::delete_notification))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de roles (protegido por JWT)
    let role_routes = Router::new()
        .route("/", get(role::list_roles).post(role::create_role))
        .route("/:id", put(role::update_role).delete(role::delete_role))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de mosaic shares (protegido por JWT)
    let share_routes = Router::new()
        .route("/", get(mosaic_share::list_shares).post(mosaic_share::create_share))
        .route("/:id", delete(mosaic_share::delete_share))
        .route("/:id/toggle", post(mosaic_share::toggle_share))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Subrouter de sync status (protegido por JWT)
    let sync_routes = Router::new()
        .route("/status", get(sync::get_sync_status))
        .route("/logs", get(sync::get_sync_logs))
        .route_layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Static files directory
    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "/app/static".to_string());
    info!("Sirviendo archivos estáticos desde: {}", static_dir);

    // Construir router con documentación OpenAPI y SOC2 security middleware
    let app = Router::new()
        .route("/health", get(health))
        .route("/jwks", get(get_jwks))
        .route("/auth/login", post(login))
        .route("/auth/forgot-password", post(user::forgot_password))
        .route("/auth/reset-password", post(user::reset_password))
        // Public share access (no JWT required)
        .route("/share/:token", get(mosaic_share::validate_share))
        .nest("/api/cameras", camera_routes)
        .nest("/api/mosaics", mosaic_routes)
        .nest("/api/locations", location_routes)
        .nest("/api/areas", area_routes)
        .nest("/api/users", user_routes)
        .nest("/api/captures", capture_routes)
        .nest("/api/notifications", notification_routes)
        .nest("/api/roles", role_routes)
        .nest("/api/shares", share_routes)
        .nest("/api/sync", sync_routes)
        // Serve captures and thumbnails as static files
        .nest_service("/data", ServeDir::new("/app/data"))
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        .route("/openapi.json", get(openapi_json))
        .with_state(state.clone())
        // SOC2 Security: Apply security headers middleware
        .layer(middleware::from_fn(security::security_headers_middleware))
        // SOC2 Security: Apply rate limiting middleware
        .layer(middleware::from_fn(security::rate_limit_middleware))
        .layer(cors)
        .fallback_service(
            ServeDir::new(&static_dir)
                .fallback(tower_http::services::ServeFile::new(format!("{}/index.html", static_dir)))
        );

    // Limpiar mosaicos huérfanos de ejecuciones anteriores
    if let Ok(count) = state.db.cleanup_orphaned_mosaics() {
        if count > 0 {
            info!("Limpiados {} mosaicos huérfanos de ejecuciones anteriores", count);
        }
    }

    // Iniciar servidor
    let addr = format!("0.0.0.0:{}", config.server_port);
    info!("Servidor iniciando en http://{}", addr);
    info!("Endpoints disponibles:");
    info!("  GET  /health             - Health check");
    info!("  GET  /jwks               - JSON Web Key Set");
    info!("  POST /auth/login         - Login y obtención de JWT");
    info!("  GET  /api/cameras        - Listar cámaras");
    info!("  POST /api/cameras        - Crear cámara");
    info!("  GET  /api/cameras/:id    - Obtener cámara");
    info!("  PUT  /api/cameras/:id    - Actualizar cámara");
    info!("  DEL  /api/cameras/:id    - Eliminar cámara");
    info!("  POST /api/cameras/sync   - Sincronizar con MediaMTX");
    info!("  GET  /api/mosaics        - Listar mosaicos");
    info!("  POST /api/mosaics        - Crear mosaico");
    info!("  POST /api/mosaics/:id/start - Iniciar mosaico");
    info!("  POST /api/mosaics/:id/stop  - Detener mosaico");
    info!("  GET  /docs               - Documentación API (Scalar)");
    info!("  GET  /                   - UI de gestión");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Devuelve la especificación OpenAPI en formato JSON
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

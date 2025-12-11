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
struct AppState {
    /// Clave para firmar JWT (RS256)
    encoding_key: EncodingKey,
    /// JWKS preconstruido en memoria
    jwks: Jwks,
    /// Configuración
    config: Config,
}

impl AppState {
    /// Crea un nuevo AppState generando un par de claves RSA
    fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Generando par de claves RSA 2048 bits...");

        // Generar par de claves RSA 2048 bits
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048)?;
        let public_key = RsaPublicKey::from(&private_key);

        // Obtener la clave privada en formato PEM para jsonwebtoken
        let private_pem = private_key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)?;
        let encoding_key = EncodingKey::from_rsa_pem(private_pem.as_bytes())?;

        // Construir JWKS desde la clave pública
        let jwks = Self::build_jwks(&public_key)?;

        info!("Par de claves RSA generado exitosamente");
        info!("JWKS construido con kid='key1'");

        Ok(Self {
            encoding_key,
            jwks,
            config,
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

    // Validar credenciales (hardcoded para desarrollo)
    if payload.username != "admin" || payload.password != "admin" {
        warn!("Credenciales inválidas para usuario: {}", payload.username);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Credenciales inválidas".to_string(),
            }),
        ));
    }

    // Generar JWT
    match state.generate_jwt(&payload.username) {
        Ok(token) => {
            info!("JWT generado exitosamente para usuario: {}", payload.username);
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
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mediamtx_auth_backend=info".parse()?)
                .add_directive("tower_http=info".parse()?),
        )
        .init();

    // Cargar configuración
    let config = Config::from_env();
    info!("Configuración cargada: {:?}", config);

    // Crear estado de la aplicación
    let state = Arc::new(AppState::new(config.clone())?);

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

# MediaMTX Auth Backend

JWT authentication backend for MediaMTX streaming server, written in Rust.

## Features

- **RS256 JWT Authentication** - Secure token signing with 2048-bit RSA keys
- **JWKS Endpoint** - Standard JSON Web Key Set for token validation
- **OpenAPI Documentation** - Interactive API docs with Scalar UI
- **Docker Ready** - Multi-stage build with Docker Compose orchestration
- **MediaMTX Integration** - Seamless integration with MediaMTX streaming server

## Architecture

```
┌─────────────────┐     ┌─────────────────┐
│   Cliente/App   │     │    MediaMTX     │
└────────┬────────┘     └────────┬────────┘
         │                       │
         │ 1. POST /auth/login   │
         ▼                       │
┌─────────────────┐              │
│  Auth Backend   │◄─────────────┘
│   (Rust/Axum)   │  2. GET /jwks
└────────┬────────┘
         │
         │ 3. JWT Token
         ▼
┌─────────────────┐
│   Cliente usa   │
│   JWT en HLS    │
│   ?jwt=TOKEN    │
└─────────────────┘
```

## Requisitos

- Docker
- Docker Compose

## Inicio Rápido

### 1. Levantar los servicios

```bash
docker compose up --build
```

Esto iniciará:
- **mediamtx-backend**: Backend de autenticación en puerto `8080`
- **mediamtx**: Servidor de streaming en múltiples puertos

### 2. Obtener un token JWT

```bash
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin"}'
```

Respuesta:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiIsImtpZCI6ImtleTEifQ..."
}
```

### 3. Ver el JWKS

```bash
curl http://localhost:8080/jwks
```

### 4. Acceder a un stream HLS con JWT

```
http://localhost:8888/mosaic/index.m3u8?jwt=TU_TOKEN_AQUI
```

## Documentación API (Scalar)

La API está documentada con OpenAPI 3.0 y se puede explorar interactivamente:

- **Scalar UI**: http://localhost:8080/docs
- **OpenAPI JSON**: http://localhost:8080/openapi.json

![Scalar UI](https://scalar.com/og-image.png)

## Endpoints del Backend

| Método | Ruta            | Descripción                          |
|--------|-----------------|--------------------------------------|
| GET    | `/health`       | Health check                         |
| GET    | `/jwks`         | JSON Web Key Set para validación     |
| POST   | `/auth/login`   | Login y obtención de JWT             |
| GET    | `/docs`         | Documentación API (Scalar UI)        |
| GET    | `/openapi.json` | Especificación OpenAPI               |

## Puertos Expuestos

| Puerto | Protocolo | Servicio                |
|--------|-----------|-------------------------|
| 8080   | HTTP      | Auth Backend            |
| 8554   | RTSP      | MediaMTX RTSP           |
| 1935   | RTMP      | MediaMTX RTMP           |
| 8888   | HTTP      | MediaMTX HLS            |
| 8889   | HTTP      | MediaMTX WebRTC         |
| 9997   | HTTP      | MediaMTX API            |

## Configuración

### Variables de Entorno del Backend

| Variable         | Default | Descripción                    |
|------------------|---------|--------------------------------|
| `SERVER_PORT`    | 8080    | Puerto del servidor HTTP       |
| `JWT_EXP_MINUTES`| 60      | Minutos de expiración del JWT  |
| `RUST_LOG`       | info    | Nivel de logging               |

### Credenciales de Desarrollo

Por simplicidad, el backend acepta solo:
- **Usuario**: `admin`
- **Contraseña**: `admin`

## JWT Claims

El JWT generado incluye:

```json
{
  "sub": "admin",
  "exp": 1234567890,
  "mediamtx_permissions": [
    { "action": "read", "path": "" },
    { "action": "publish", "path": "" },
    { "action": "playback", "path": "" }
  ]
}
```

- `path: ""` significa acceso a todos los paths
- Acciones: `read`, `publish`, `playback`

## Flujo de Autenticación

1. El cliente hace POST a `/auth/login` con credenciales
2. El backend genera un JWT firmado con RS256
3. El cliente usa el JWT en las peticiones a MediaMTX:
   - Query string: `?jwt=TOKEN`
   - Header: `Authorization: Bearer TOKEN`
4. MediaMTX valida el JWT contra el JWKS del backend (`/jwks`)
5. MediaMTX verifica los permisos en `mediamtx_permissions`

## Project Structure

```
.
├── Cargo.toml           # Rust dependencies
├── src/
│   └── main.rs          # Backend source code
├── .env                 # Environment variables (git-ignored)
├── .env.example         # Environment template
├── Dockerfile           # Multi-stage Docker build
├── docker-compose.yml   # Service orchestration
├── Makefile             # Development commands
├── mediamtx.yml         # MediaMTX configuration
├── recordings/          # Stream recordings (git-ignored)
└── README.md            # This file
```

## Variables de Entorno (.env)

El archivo `.env` contiene la configuración del backend:

```env
# Puerto del servidor HTTP
SERVER_PORT=8080

# Minutos de expiración del JWT
JWT_EXP_MINUTES=60

# Nivel de logging
RUST_LOG=info
```

## Stream Paths

### Dynamic Publisher Path (Wildcard)

The `mediamtx.yml` includes a wildcard path that accepts **any stream** under `live/`:

```yaml
~^live/.*$:
  source: publisher
  record: yes
  overridePublisher: yes
```

**Usage Examples:**

```bash
# Publish via RTSP
ffmpeg -i input.mp4 -c copy -f rtsp rtsp://localhost:8554/live/my-stream

# Publish via RTMP  
ffmpeg -i input.mp4 -c copy -f flv rtmp://localhost:1935/live/my-stream

# Access via HLS (with JWT)
curl "http://localhost:8888/live/my-stream/index.m3u8?jwt=YOUR_TOKEN"
```

### Pre-configured Camera Paths

| Path | Source | Records |
|------|--------|---------|
| `mosaic` | Publisher | Yes |
| `entrance-guardhouse-*` | RTSP cameras | Yes |
| `W1-Door-*` | RTSP cameras | Yes |
| `W2-Door-*` | RTSP cameras | Yes |
| `truck-detection` | Publisher | No |
| `live/*` | **Any publisher** | Yes |

> **Note**: Edit `mediamtx.yml` to configure your actual camera RTSP URLs.

## Grabaciones

Las grabaciones se almacenan en `./recordings/` con el formato:
```
/recordings/{path}/{YYYY-MM-DD_HH-MM-SS}.mp4
```

## Desarrollo Local (sin Docker)

```bash
# Compilar
cargo build --release

# Ejecutar
SERVER_PORT=8080 JWT_EXP_MINUTES=60 cargo run --release
```

## Seguridad

Este es un backend de **desarrollo**. Para producción:

1. Implementar almacenamiento seguro de credenciales
2. Usar HTTPS
3. Almacenar claves RSA de forma persistente
4. Implementar rotación de claves
5. Añadir rate limiting
6. Validar y sanitizar inputs

## Licencia

MIT

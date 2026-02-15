# =============================================================================
# Dockerfile para MediaMTX Auth Backend (Rust) + SvelteKit Frontend
# Build multi-stage para imagen final mínima
# =============================================================================

# -----------------------------------------------------------------------------
# Etapa 1: Build Frontend (SvelteKit)
# -----------------------------------------------------------------------------
FROM node:22-slim AS frontend-builder

WORKDIR /app/frontend

# Copiar archivos de dependencias primero para cachear
COPY frontend/package.json frontend/package-lock.json* ./

RUN npm ci

# Copiar código fuente del frontend
COPY frontend/ ./

RUN npm run build

# -----------------------------------------------------------------------------
# Etapa 2: Build Backend (Rust)
# -----------------------------------------------------------------------------
FROM rust:bookworm AS builder

WORKDIR /app

# Copiar archivos de dependencias primero para cachear
COPY Cargo.toml Cargo.lock* ./

# Crear src dummy para compilar dependencias
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Compilar solo dependencias (cacheado)
RUN cargo build --release && rm -rf src

# Copiar código fuente real
COPY src ./src

# Recompilar con código real (touch para forzar rebuild)
RUN touch src/main.rs && cargo build --release

# -----------------------------------------------------------------------------
# Etapa 3: Runtime (imagen mínima)
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

# Instalar dependencias mínimas de runtime (curl para healthcheck, ffmpeg para mosaicos)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar binario compilado
COPY --from=builder /app/target/release/mediamtx-auth-backend /app/mediamtx-auth-backend

# Copiar archivos estáticos de la UI (SvelteKit build output)
COPY --from=frontend-builder /app/frontend/build /app/static

# Crear directorio para datos (SQLite DB)
RUN mkdir -p /app/data

# Puerto por defecto
EXPOSE 8080

# Variables de entorno por defecto
ENV SERVER_PORT=8080
ENV JWT_EXP_MINUTES=60
ENV RUST_LOG=info
ENV DATABASE_PATH=/app/data/cameras.db
ENV MEDIAMTX_API_URL=http://mediamtx:9997

# Ejecutar el backend
CMD ["/app/mediamtx-auth-backend"]

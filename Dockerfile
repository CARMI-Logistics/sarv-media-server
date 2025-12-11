# =============================================================================
# Dockerfile para MediaMTX Auth Backend (Rust)
# Build multi-stage para imagen final mínima
# =============================================================================

# -----------------------------------------------------------------------------
# Etapa 1: Build
# -----------------------------------------------------------------------------
FROM rust:latest AS builder

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
# Etapa 2: Runtime (imagen mínima)
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

# Instalar dependencias mínimas de runtime (incluye curl para healthcheck)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar binario compilado
COPY --from=builder /app/target/release/mediamtx-auth-backend /app/mediamtx-auth-backend

# Puerto por defecto
EXPOSE 8080

# Variables de entorno por defecto
ENV SERVER_PORT=8080
ENV JWT_EXP_MINUTES=60
ENV RUST_LOG=info

# Ejecutar el backend
CMD ["/app/mediamtx-auth-backend"]

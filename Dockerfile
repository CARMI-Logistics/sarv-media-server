# =============================================================================
# Dockerfile para MediaMTX Auth Backend (Rust)
# Build multi-stage para imagen final mínima
# =============================================================================

# -----------------------------------------------------------------------------
# Etapa 1: Build
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

# Migraciones de esquema: sqlx::migrate! las embebe en tiempo de compilación,
# así que el directorio debe existir durante el build (HU 4.1).
COPY migrations ./migrations

# Recompilar con código real (touch para forzar rebuild)
RUN touch src/main.rs && cargo build --release

# -----------------------------------------------------------------------------
# Etapa 2: Runtime (imagen mínima)
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

# Instalar dependencias mínimas de runtime (incluye curl para healthcheck y
# ffmpeg para extraer el frame del endpoint GET /cameras/{path}/snapshot)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Usuario sin privilegios: ffmpeg parsea video de cámaras externas, así que el
# proceso no debe ser root ni tener la clave de firma al alcance de un exploit.
# /keys se crea con su dueño para que un volumen NUEVO lo herede; un volumen
# ya existente hay que corregirlo a mano (ver RUNBOOK.md §8).
RUN groupadd --system --gid 10001 app \
    && useradd --system --uid 10001 --gid app --no-create-home --shell /usr/sbin/nologin app \
    && mkdir /keys \
    && chown app:app /keys

WORKDIR /app

# Copiar binario compilado
COPY --from=builder /app/target/release/mediamtx-auth-backend /app/mediamtx-auth-backend

# Puerto por defecto
EXPOSE 8080

# Variables de entorno por defecto
ENV SERVER_PORT=8080
ENV JWT_EXP_MINUTES=60
ENV RUST_LOG=info

USER app

# Ejecutar el backend. ENTRYPOINT para que los args se pasen al binario:
#   (sin args)          -> arranca el servidor
#   hash <secreto>      -> imprime el hash Argon2 y sale
ENTRYPOINT ["/app/mediamtx-auth-backend"]

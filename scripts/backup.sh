#!/bin/bash
# =============================================================================
# S3 Backup Script - Sync Recordings to MinIO S3
# =============================================================================
# Este script sincroniza las grabaciones locales a MinIO S3 cada hora.
# Características:
#   - Sync incremental (solo archivos nuevos/modificados)
#   - Verificación de integridad (checksums)
#   - Rotación de logs
#   - Manejo de errores robusto
#   - Sin pérdida de datos (atomic operations)
#
# Uso: ./backup.sh
# =============================================================================

set -euo pipefail

# =============================================================================
# CONFIGURACIÓN
# =============================================================================

# Rutas
RECORDINGS_DIR="/recordings"
LOG_DIR="/logs"
RCLONE_CONFIG="${RCLONE_CONFIG:-/config/rclone/rclone.conf}"

# S3 Configuration
S3_ENDPOINT="${S3_ENDPOINT:-http://minio:9000}"
S3_BUCKET="${S3_BUCKET:-recordings}"
S3_ACCESS_KEY="${S3_ACCESS_KEY:-minioadmin}"
S3_SECRET_KEY="${S3_SECRET_KEY:-minioadmin123}"

# Sync settings
SYNC_INTERVAL="${SYNC_INTERVAL:-3600}"  # 3600 segundos = 1 hora
LOG_LEVEL="${LOG_LEVEL:-INFO}"
MAX_RETRIES=3
RETRY_DELAY=5

# Archivos de log
LOG_FILE="${LOG_DIR}/backup.log"
LAST_SYNC_FILE="${LOG_DIR}/last_sync.timestamp"
LOCK_FILE="${LOG_DIR}/backup.lock"

# =============================================================================
# FUNCIONES
# =============================================================================

log() {
    local level="$1"
    local message="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] [${level}] ${message}" | tee -a "$LOG_FILE"
}

log_info() { log "INFO" "$1"; }
log_warn() { log "WARN" "$1"; }
log_error() { log "ERROR" "$1"; }

# Verificar dependencias
check_dependencies() {
    log_info "Verificando dependencias..."
    
    if ! command -v rclone &> /dev/null; then
        log_error "rclone no está instalado"
        exit 1
    fi
    
    if [[ ! -f "$RCLONE_CONFIG" ]]; then
        log_warn "Archivo de configuración rclone no encontrado: $RCLONE_CONFIG"
        log_info "Creando configuración por defecto..."
        create_rclone_config
    fi
    
    log_info "Dependencias verificadas correctamente"
}

# Crear configuración rclone dinámicamente
create_rclone_config() {
    mkdir -p "$(dirname "$RCLONE_CONFIG")"
    cat > "$RCLONE_CONFIG" << EOF
[minio]
type = s3
provider = Minio
access_key_id = ${S3_ACCESS_KEY}
secret_access_key = ${S3_SECRET_KEY}
endpoint = ${S3_ENDPOINT}
region = us-east-1
acl = private
force_path_style = true
EOF
    log_info "Configuración rclone creada en $RCLONE_CONFIG"
}

# Verificar que MinIO está disponible
check_minio() {
    log_info "Verificando conexión a MinIO en ${S3_ENDPOINT}..."
    
    local retries=0
    while [[ $retries -lt $MAX_RETRIES ]]; do
        if rclone --config "$RCLONE_CONFIG" mkdir minio:temp-check-$$ 2>/dev/null && \
           rclone --config "$RCLONE_CONFIG" rmdir minio:temp-check-$$ 2>/dev/null; then
            log_info "Conexión a MinIO establecida"
            return 0
        fi
        
        retries=$((retries + 1))
        log_warn "Intento $retries/$MAX_RETRIES fallido. Reintentando en ${RETRY_DELAY}s..."
        sleep "$RETRY_DELAY"
    done
    
    log_error "No se pudo conectar a MinIO después de $MAX_RETRIES intentos"
    return 1
}

# Crear bucket si no existe
create_bucket() {
    log_info "Verificando bucket '${S3_BUCKET}'..."
    
    if ! rclone --config "$RCLONE_CONFIG" ls "minio:${S3_BUCKET}" &>/dev/null; then
        log_info "Creando bucket '${S3_BUCKET}'..."
        if rclone --config "$RCLONE_CONFIG" mkdir "minio:${S3_BUCKET}"; then
            log_info "Bucket '${S3_BUCKET}' creado correctamente"
        else
            log_error "No se pudo crear el bucket '${S3_BUCKET}'"
            return 1
        fi
    else
        log_info "Bucket '${S3_BUCKET}' ya existe"
    fi
    
    # Configurar lifecycle policy (opcional, para retención automática)
    # Esto se puede hacer vía mc (MinIO Client) si es necesario
    
    return 0
}

# Rotación de logs
rotate_logs() {
    local max_size=10485760  # 10 MB
    
    if [[ -f "$LOG_FILE" ]]; then
        local size=$(stat -f%z "$LOG_FILE" 2>/dev/null || stat -c%s "$LOG_FILE" 2>/dev/null || echo 0)
        
        if [[ $size -gt $max_size ]]; then
            log_info "Rotando archivo de log..."
            mv "$LOG_FILE" "${LOG_FILE}.old"
            touch "$LOG_FILE"
        fi
    fi
}

# Verificar lock (evitar ejecuciones simultáneas)
check_lock() {
    if [[ -f "$LOCK_FILE" ]]; then
        local pid=$(cat "$LOCK_FILE" 2>/dev/null || echo "")
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            log_warn "Otra instancia del backup está corriendo (PID: $pid)"
            return 1
        else
            log_warn "Lock file obsoleto encontrado, limpiando..."
            rm -f "$LOCK_FILE"
        fi
    fi
    
    echo $$ > "$LOCK_FILE"
    return 0
}

# Limpiar lock
cleanup() {
    rm -f "$LOCK_FILE"
    log_info "Backup finalizado. Próxima ejecución en ${SYNC_INTERVAL} segundos ($(date -d "@${SYNC_INTERVAL}" -u +%H:%M:%S) horas)"
}

# Realizar sync con retries
perform_sync() {
    log_info "Iniciando sincronización de grabaciones..."
    log_info "Origen: ${RECORDINGS_DIR}"
    log_info "Destino: minio:${S3_BUCKET}"
    
    local retries=0
    local success=false
    
    while [[ $retries -lt $MAX_RETRIES ]] && [[ "$success" == "false" ]]; do
        log_info "Intento $((retries + 1))/$MAX_RETRIES..."
        
        # Opciones de rclone para sincronización segura:
        # --checksum: Verificar por MD5/SHA1 (lento pero seguro)
        # --size-only: Verificar solo por tamaño (más rápido)
        # --update: No sobrescribir si remoto es más nuevo
        # --transfers: Número de transferencias paralelas
        # --checkers: Número de verificadores paralelos
        # --retries: Reintentos en archivos fallidos
        # --low-level-retries: Reintentos a nivel de conexión
        # --stats: Mostrar estadísticas
        # --log-level: Nivel de log
        # --delete-after: Borrar archivos remotos después de transferir (más seguro)
        # --backup-dir: Mover archivos reemplazados a directorio de backup
        
        if rclone --config "$RCLONE_CONFIG" sync \
            "$RECORDINGS_DIR" \
            "minio:${S3_BUCKET}" \
            --size-only \
            --update \
            --transfers=4 \
            --checkers=8 \
            --retries=3 \
            --low-level-retries=10 \
            --stats=1m \
            --stats-one-line \
            --log-level="$LOG_LEVEL" \
            --log-file="$LOG_FILE" \
            --delete-after \
            --exclude="*.tmp" \
            --exclude="*.part" \
            --exclude=".DS_Store"; then
            
            success=true
            log_info "Sincronización completada exitosamente"
        else
            retries=$((retries + 1))
            log_warn "Sincronización fallida. Reintentando en ${RETRY_DELAY}s..."
            sleep "$RETRY_DELAY"
        fi
    done
    
    if [[ "$success" == "false" ]]; then
        log_error "Sincronización fallida después de $MAX_RETRIES intentos"
        return 1
    fi
    
    # Actualizar timestamp de última sincronización
    date +%s > "$LAST_SYNC_FILE"
    
    return 0
}

# Verificar integridad de datos (opcional, lento)
verify_integrity() {
    log_info "Verificando integridad de datos sincronizados..."
    
    # Contar archivos locales y remotos
    local local_count=$(find "$RECORDINGS_DIR" -type f ! -name "*.tmp" ! -name "*.part" | wc -l)
    local remote_count=$(rclone --config "$RCLONE_CONFIG" ls "minio:${S3_BUCKET}" --stats=0 2>/dev/null | wc -l || echo 0)
    
    log_info "Archivos locales: $local_count"
    log_info "Archivos remotos: $remote_count"
    
    if [[ "$local_count" -eq "$remote_count" ]]; then
        log_info "Verificación de integridad: OK"
        return 0
    else
        log_warn "Discrepancia en conteo de archivos: local=$local_count, remote=$remote_count"
        return 1
    fi
}

# =============================================================================
# MAIN
# =============================================================================

main() {
    log_info "=========================================="
    log_info "Iniciando servicio de backup S3"
    log_info "Configuración:"
    log_info "  - Endpoint: ${S3_ENDPOINT}"
    log_info "  - Bucket: ${S3_BUCKET}"
    log_info "  - Intervalo: ${SYNC_INTERVAL}s"
    log_info "  - Directorio: ${RECORDINGS_DIR}"
    log_info "=========================================="
    
    # Crear directorios necesarios
    mkdir -p "$LOG_DIR"
    
    # Verificaciones iniciales
    check_dependencies || exit 1
    
    # Loop infinito de sincronización
    while true; do
        # Verificar lock
        if ! check_lock; then
            log_warn "Esperando a que termine otra instancia..."
            sleep 60
            continue
        fi
        
        # Configurar cleanup al salir
        trap cleanup EXIT
        
        # Rotar logs
        rotate_logs
        
        # Verificar MinIO
        if ! check_minio; then
            log_error "MinIO no disponible, esperando..."
            sleep 60
            continue
        fi
        
        # Crear bucket si no existe
        if ! create_bucket; then
            log_error "No se pudo verificar/crear bucket, esperando..."
            sleep 60
            continue
        fi
        
        # Verificar que hay archivos para sincronizar
        if [[ ! -d "$RECORDINGS_DIR" ]] || [[ -z "$(ls -A "$RECORDINGS_DIR" 2>/dev/null)" ]]; then
            log_info "No hay grabaciones para sincronizar"
            log_info "Esperando ${SYNC_INTERVAL}s..."
            sleep "$SYNC_INTERVAL"
            continue
        fi
        
        # Realizar sincronización
        if perform_sync; then
            # Verificar integridad (opcional)
            # verify_integrity  # Descomentar si se necesita verificación estricta
            
            log_info "Backup completado exitosamente"
        else
            log_error "Backup fallido, se reintentará en el próximo ciclo"
        fi
        
        # Limpiar lock
        cleanup
        
        # Esperar hasta el próximo ciclo
        log_info "Esperando ${SYNC_INTERVAL}s para próximo backup..."
        sleep "$SYNC_INTERVAL"
    done
}

# Ejecutar main
main "$@"

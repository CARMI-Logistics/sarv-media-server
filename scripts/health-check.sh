#!/bin/bash
# =============================================================================
# Health Check Script - Verificar estado del sistema de backup
# =============================================================================
# Este script verifica que todo el sistema de backup está funcionando correctamente.
# Uso: ./health-check.sh
# =============================================================================

set -euo pipefail

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuración
RCLONE_CONFIG="${RCLONE_CONFIG:-/config/rclone/rclone.conf}"
S3_ENDPOINT="${S3_ENDPOINT:-http://minio:9000}"
S3_BUCKET="${S3_BUCKET:-recordings}"
RECORDINGS_DIR="${RECORDINGS_DIR:-/recordings}"
LOG_DIR="${LOG_DIR:-/logs}"

LAST_SYNC_FILE="${LOG_DIR}/last_sync.timestamp"
MAX_SYNC_AGE=7200  # 2 horas en segundos

# Contador de errores
ERRORS=0

# Funciones de check
check_pass() {
    echo -e "${GREEN}✓${NC} $1"
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    ((ERRORS++)) || true
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Verificar MinIO
check_minio() {
    echo "Verificando MinIO..."
    
    if ! command -v rclone &> /dev/null; then
        check_fail "rclone no está instalado"
        return
    fi
    
    if rclone --config "$RCLONE_CONFIG" ls "minio:${S3_BUCKET}" &>/dev/null; then
        check_pass "MinIO accesible y bucket '${S3_BUCKET}' existe"
    else
        check_fail "No se puede acceder a MinIO o bucket '${S3_BUCKET}'"
    fi
}

# Verificar directorios
check_directories() {
    echo ""
    echo "Verificando directorios..."
    
    if [[ -d "$RECORDINGS_DIR" ]]; then
        check_pass "Directorio de grabaciones existe: $RECORDINGS_DIR"
        
        # Contar archivos
        local count=$(find "$RECORDINGS_DIR" -type f 2>/dev/null | wc -l)
        check_pass "Archivos locales: $count"
    else
        check_fail "Directorio de grabaciones no existe: $RECORDINGS_DIR"
    fi
    
    if [[ -d "$LOG_DIR" ]]; then
        check_pass "Directorio de logs existe: $LOG_DIR"
    else
        check_warn "Directorio de logs no existe: $LOG_DIR"
    fi
}

# Verificar última sincronización
check_last_sync() {
    echo ""
    echo "Verificando última sincronización..."
    
    if [[ -f "$LAST_SYNC_FILE" ]]; then
        local last_sync=$(cat "$LAST_SYNC_FILE")
        local now=$(date +%s)
        local diff=$((now - last_sync))
        local diff_min=$((diff / 60))
        local diff_hours=$((diff / 3600))
        
        if [[ $diff -lt $MAX_SYNC_AGE ]]; then
            if [[ $diff_hours -gt 0 ]]; then
                check_pass "Última sincronización: hace ${diff_hours}h ${diff_min}m"
            else
                check_pass "Última sincronización: hace ${diff_min}m"
            fi
        else
            check_fail "Última sincronización: hace ${diff_hours}h (máximo permitido: 2h)"
        fi
    else
        check_warn "No se encontró registro de última sincronización"
    fi
}

# Verificar espacio en disco
check_disk_space() {
    echo ""
    echo "Verificando espacio en disco..."
    
    if [[ -d "$RECORDINGS_DIR" ]]; then
        local usage=$(df -h "$RECORDINGS_DIR" | awk 'NR==2 {print $5}' | tr -d '%')
        local available=$(df -h "$RECORDINGS_DIR" | awk 'NR==2 {print $4}')
        
        if [[ $usage -lt 80 ]]; then
            check_pass "Espacio disponible: $available (${usage}% usado)"
        elif [[ $usage -lt 90 ]]; then
            check_warn "Espacio disponible: $available (${usage}% usado) - Atención"
        else
            check_fail "Espacio disponible: $available (${usage}% usado) - Crítico"
        fi
    fi
}

# Verificar logs
check_logs() {
    echo ""
    echo "Verificando logs..."
    
    local log_file="${LOG_DIR}/backup.log"
    if [[ -f "$log_file" ]]; then
        check_pass "Archivo de log existe"
        
        # Verificar errores recientes
        local recent_errors=$(tail -n 100 "$log_file" 2>/dev/null | grep -c "ERROR" || echo 0)
        if [[ $recent_errors -eq 0 ]]; then
            check_pass "No hay errores recientes en los logs"
        else
            check_warn "Hay $recent_errors errores recientes en los logs"
        fi
    else
        check_warn "No se encontró archivo de log"
    fi
}

# Verificar archivos remotos vs locales
check_sync_status() {
    echo ""
    echo "Verificando estado de sincronización..."
    
    local local_count=$(find "$RECORDINGS_DIR" -type f 2>/dev/null | wc -l)
    local remote_count=$(rclone --config "$RCLONE_CONFIG" ls "minio:${S3_BUCKET}" --stats=0 2>/dev/null | wc -l || echo 0)
    
    echo "  Archivos locales: $local_count"
    echo "  Archivos remotos: $remote_count"
    
    if [[ $local_count -eq $remote_count ]]; then
        check_pass "Todos los archivos están sincronizados"
    else
        local diff=$((local_count - remote_count))
        if [[ $diff -gt 0 ]]; then
            check_warn "Faltan $diff archivos por sincronizar"
        else
            check_warn "Hay $((-diff)) archivos adicionales en S3"
        fi
    fi
}

# =============================================================================
# MAIN
# =============================================================================

echo "=========================================="
echo "Health Check - Sistema de Backup"
echo "=========================================="
echo ""

check_minio
check_directories
check_last_sync
check_disk_space
check_logs
check_sync_status

echo ""
echo "=========================================="
if [[ $ERRORS -eq 0 ]]; then
    echo -e "${GREEN}Estado: OK${NC} - Todo funciona correctamente"
    exit 0
else
    echo -e "${RED}Estado: ERROR${NC} - Se encontraron $ERRORS problemas"
    exit 1
fi

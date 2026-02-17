#!/bin/bash
# =============================================================================
# Manual Sync Script - Sincronización manual bajo demanda
# =============================================================================
# Este script permite sincronizar manualmente las grabaciones a S3.
# Uso: ./manual-sync.sh [opciones]
# =============================================================================

set -euo pipefail

# Configuración
RCLONE_CONFIG="${RCLONE_CONFIG:-/config/rclone/rclone.conf}"
S3_BUCKET="${S3_BUCKET:-recordings}"
RECORDINGS_DIR="${RECORDINGS_DIR:-/recordings}"

# Parsear argumentos
DRY_RUN=false
VERBOSE=false
VERIFY=false

usage() {
    cat << EOF
Uso: $0 [opciones]

Opciones:
    -d, --dry-run      Simular sincronización sin hacer cambios
    -v, --verbose      Modo verbose con más detalle
    --verify           Verificar integridad después del sync
    -h, --help         Mostrar esta ayuda

Ejemplos:
    $0                    # Sync normal
    $0 --dry-run          # Simulación (no realiza cambios)
    $0 --verbose          # Sync con detalle
    $0 --verify           # Sync + verificación de integridad
EOF
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --verify)
            VERIFY=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Opción desconocida: $1"
            usage
            exit 1
            ;;
    esac
done

echo "=========================================="
echo "Sincronización Manual a S3"
echo "=========================================="
echo "Origen: $RECORDINGS_DIR"
echo "Destino: minio:$S3_BUCKET"
echo ""

# Construir comando RCLONE
RCLONE_OPTS=""

if [[ "$DRY_RUN" == "true" ]]; then
    RCLONE_OPTS="$RCLONE_OPTS --dry-run"
    echo "MODO SIMULACIÓN - No se realizarán cambios"
    echo ""
fi

if [[ "$VERBOSE" == "true" ]]; then
    RCLONE_OPTS="$RCLONE_OPTS --verbose"
else
    RCLONE_OPTS="$RCLONE_OPTS --stats=5s --stats-one-line"
fi

echo "Iniciando sync..."
echo ""

if rclone --config "$RCLONE_CONFIG" sync \
    "$RECORDINGS_DIR" \
    "minio:${S3_BUCKET}" \
    --size-only \
    --update \
    --transfers=8 \
    --checkers=16 \
    --retries=3 \
    --low-level-retries=10 \
    --log-level=INFO \
    --delete-after \
    --exclude="*.tmp" \
    --exclude="*.part" \
    --exclude=".DS_Store" \
    $RCLONE_OPTS; then
    
    echo ""
    echo "=========================================="
    echo "✓ Sincronización completada exitosamente"
    echo "=========================================="
    
    if [[ "$VERIFY" == "true" ]]; then
        echo ""
        echo "Verificando integridad..."
        local_count=$(find "$RECORDINGS_DIR" -type f 2>/dev/null | wc -l)
        remote_count=$(rclone --config "$RCLONE_CONFIG" ls "minio:${S3_BUCKET}" --stats=0 2>/dev/null | wc -l || echo 0)
        
        echo "Archivos locales: $local_count"
        echo "Archivos remotos: $remote_count"
        
        if [[ $local_count -eq $remote_count ]]; then
            echo "✓ Verificación exitosa"
        else
            echo "⚠ Diferencia detectada: $((local_count - remote_count)) archivos"
        fi
    fi
    
    exit 0
else
    echo ""
    echo "=========================================="
    echo "✗ Sincronización fallida"
    echo "=========================================="
    exit 1
fi

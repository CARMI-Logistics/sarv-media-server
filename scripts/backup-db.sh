#!/bin/bash
# =============================================================================
# SQLite Database Backup Script
# =============================================================================
# Automatic backup of cameras.db with retention policy
# Usage: ./backup-db.sh [--now]

set -e

# Configuration
DB_PATH="${DATABASE_PATH:-/app/data/cameras.db}"
BACKUP_DIR="/app/data/backups"
RETENTION_DAYS="${DB_BACKUP_RETENTION_DAYS:-30}"
S3_BACKUP="${DB_BACKUP_TO_S3:-true}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/cameras_${TIMESTAMP}.db"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Create backup directory if it doesn't exist
mkdir -p "${BACKUP_DIR}"

# Check if database exists
if [ ! -f "${DB_PATH}" ]; then
    log_error "Database not found at ${DB_PATH}"
    exit 1
fi

# Get database size
DB_SIZE=$(du -h "${DB_PATH}" | cut -f1)
log_info "Starting backup of database (${DB_SIZE})"

# Perform backup using SQLite VACUUM INTO (online backup)
log_info "Creating backup: ${BACKUP_FILE}"
if sqlite3 "${DB_PATH}" "VACUUM INTO '${BACKUP_FILE}'"; then
    log_info "Backup created successfully"
    
    # Verify backup integrity
    if sqlite3 "${BACKUP_FILE}" "PRAGMA integrity_check;" | grep -q "ok"; then
        log_info "Backup integrity verified"
    else
        log_error "Backup integrity check FAILED"
        rm -f "${BACKUP_FILE}"
        exit 1
    fi
else
    log_error "Backup creation FAILED"
    exit 1
fi

# Get backup size
BACKUP_SIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
log_info "Backup size: ${BACKUP_SIZE}"

# Clean up old backups (keep last N days)
log_info "Cleaning up backups older than ${RETENTION_DAYS} days..."
DELETED_COUNT=$(find "${BACKUP_DIR}" -name "cameras_*.db" -type f -mtime +${RETENTION_DAYS} -delete -print | wc -l)
if [ "${DELETED_COUNT}" -gt 0 ]; then
    log_info "Deleted ${DELETED_COUNT} old backup(s)"
fi

# Sync to S3/MinIO if enabled
if [ "${S3_BACKUP}" = "true" ]; then
    log_info "Syncing backups to S3..."
    
    # Check if rclone is available
    if command -v rclone &> /dev/null; then
        if rclone copy "${BACKUP_DIR}" minio:backups/database/ --config /config/rclone/rclone.conf; then
            log_info "Database backups synced to S3 successfully"
        else
            log_warn "Failed to sync backups to S3 (continuing anyway)"
        fi
    else
        log_warn "rclone not found, skipping S3 sync"
    fi
fi

# Summary
BACKUP_COUNT=$(find "${BACKUP_DIR}" -name "cameras_*.db" -type f | wc -l)
log_info "Backup complete! Total backups: ${BACKUP_COUNT}"
log_info "Latest backup: ${BACKUP_FILE}"

exit 0

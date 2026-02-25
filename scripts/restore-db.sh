#!/bin/bash
# =============================================================================
# SQLite Database Restore Script
# =============================================================================
# Restore database from backup
# Usage: ./restore-db.sh [backup_file]

set -e

DB_PATH="${DATABASE_PATH:-/app/data/cameras.db}"
BACKUP_DIR="/app/data/backups"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if backup file is provided
if [ -z "$1" ]; then
    log_info "Available backups:"
    ls -lht "${BACKUP_DIR}"/cameras_*.db 2>/dev/null | head -10 || log_error "No backups found"
    echo ""
    log_error "Usage: $0 <backup_file>"
    log_info "Example: $0 ${BACKUP_DIR}/cameras_20260217_120000.db"
    exit 1
fi

BACKUP_FILE="$1"

# Validate backup file exists
if [ ! -f "${BACKUP_FILE}" ]; then
    log_error "Backup file not found: ${BACKUP_FILE}"
    exit 1
fi

# Verify backup integrity
log_info "Verifying backup integrity..."
if ! sqlite3 "${BACKUP_FILE}" "PRAGMA integrity_check;" | grep -q "ok"; then
    log_error "Backup file is corrupted!"
    exit 1
fi
log_info "Backup integrity OK"

# Create backup of current database before restore
if [ -f "${DB_PATH}" ]; then
    CURRENT_BACKUP="${DB_PATH}.before_restore_$(date +%Y%m%d_%H%M%S)"
    log_warn "Creating safety backup of current database..."
    cp "${DB_PATH}" "${CURRENT_BACKUP}"
    log_info "Current database backed up to: ${CURRENT_BACKUP}"
fi

# Restore database
log_warn "Restoring database from: ${BACKUP_FILE}"
log_warn "This will replace: ${DB_PATH}"
read -p "Are you sure? (yes/no): " -r
if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    log_info "Restore cancelled"
    exit 0
fi

cp "${BACKUP_FILE}" "${DB_PATH}"
log_info "Database restored successfully"

# Verify restored database
if sqlite3 "${DB_PATH}" "PRAGMA integrity_check;" | grep -q "ok"; then
    log_info "Restored database integrity verified"
else
    log_error "Restored database integrity check FAILED!"
    exit 1
fi

log_info "Restore complete!"
log_warn "Remember to restart the backend service for changes to take effect"

exit 0

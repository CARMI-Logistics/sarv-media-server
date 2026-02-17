# =============================================================================
# Makefile - CamManager - Sistema Completo de Gestión de Cámaras
# =============================================================================

.PHONY: help build run up start stop down restart rebuild \
        logs logs-backend logs-frontend logs-mediamtx logs-minio logs-sync \
        dev dev-up dev-down dev-restart dev-logs \
        check test fmt lint \
        clean clean-recordings clean-logs clean-all \
        status health login jwks \
        sync streams recordings reset-db \
        s3-status s3-logs s3-sync s3-health s3-manual s3-ui \
        backup backup-db backup-config \
        maintenance clean-old-recordings monitor \
        shell-backend shell-mediamtx shell-frontend shell-minio shell-sync

# Colores para output
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
RESET := \033[0m

help: ## Muestra esta ayuda
	@echo "$(CYAN)╔════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(CYAN)║  CamManager - Sistema de Gestión de Cámaras       ║$(RESET)"
	@echo "$(CYAN)╚════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(YELLOW)Comandos disponibles:$(RESET)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(CYAN)Uso rápido:$(RESET)"
	@echo "  make dev-up        # Iniciar entorno de desarrollo"
	@echo "  make dev-logs      # Ver logs en tiempo real"
	@echo "  make s3-status     # Ver estado de sincronización S3"
	@echo "  make maintenance   # Mantenimiento completo del sistema"

# =============================================================================
# Docker
# =============================================================================

build: ## Construye las imágenes Docker
	@echo "$(CYAN)Construyendo imágenes...$(RESET)"
	docker compose build

run: ## Levanta todos los servicios
	@echo "$(CYAN)Iniciando servicios...$(RESET)"
	docker compose up -d
	@echo ""
	@echo "$(GREEN)Servicios iniciados:$(RESET)"
	@echo "  - Auth Backend: http://localhost:8080"
	@echo "  - API Docs:     http://localhost:8080/docs"
	@echo "  - MediaMTX API: http://localhost:9997"
	@echo "  - HLS:          http://localhost:8888"
	@echo "  - WebRTC:       http://localhost:8889"
	@echo "  - RTSP:         rtsp://localhost:8554"

up: build run ## Construye y levanta servicios

start: ## Inicia servicios existentes (sin rebuild)
	docker compose start

stop: ## Detiene los servicios
	@echo "$(YELLOW)Deteniendo servicios...$(RESET)"
	docker compose stop

down: ## Detiene y elimina contenedores
	@echo "$(YELLOW)Eliminando contenedores...$(RESET)"
	docker compose down

restart: down up ## Reconstruye y reinicia todo

rebuild: ## Rebuild rápido (down + build + up)
	@echo "$(CYAN)Rebuild completo...$(RESET)"
	docker compose down
	docker compose up --build -d
	@echo ""
	@echo "$(GREEN)Servicios reiniciados:$(RESET)"
	@echo "  - Auth Backend: http://localhost:8080"
	@echo "  - HLS:          http://localhost:8888"
	@echo "  - WebRTC:       http://localhost:8889"

logs: ## Muestra logs de todos los servicios
	docker compose logs -f

logs-backend: ## Muestra logs del backend
	docker compose logs -f mediamtx-backend

logs-mediamtx: ## Muestra logs de MediaMTX
	docker compose logs -f mediamtx

logs-frontend: ## Muestra logs del frontend
	docker compose logs -f frontend

logs-minio: ## Muestra logs de MinIO
	docker compose logs -f minio

logs-sync: ## Muestra logs de RClone Sync
	docker compose logs -f rclone-sync

# =============================================================================
# Desarrollo (Hot Reload)
# =============================================================================

dev-up: ## Inicia entorno de desarrollo con hot reload
	@echo "$(CYAN)Iniciando entorno de desarrollo...$(RESET)"
	docker compose -f docker-compose.dev.yml up --build -d
	@echo ""
	@echo "$(GREEN)✓ Servicios de desarrollo iniciados:$(RESET)"
	@echo "  $(CYAN)Frontend:$(RESET)  http://localhost:5173 (Vite HMR)"
	@echo "  $(CYAN)Backend:$(RESET)   http://localhost:8080 (cargo watch)"
	@echo "  $(CYAN)API Docs:$(RESET)  http://localhost:8080/docs"
	@echo "  $(CYAN)MinIO:$(RESET)     http://localhost:9001 (admin/admin123)"
	@echo "  $(CYAN)MediaMTX:$(RESET)  rtsp://localhost:8554"
	@echo ""
	@echo "$(YELLOW)Logs:$(RESET) make dev-logs"

dev-down: ## Detiene entorno de desarrollo
	@echo "$(YELLOW)Deteniendo desarrollo...$(RESET)"
	docker compose -f docker-compose.dev.yml down

dev-restart: ## Reinicia entorno de desarrollo
	@echo "$(CYAN)Reiniciando desarrollo...$(RESET)"
	docker compose -f docker-compose.dev.yml restart

dev-logs: ## Muestra logs del entorno de desarrollo
	docker compose -f docker-compose.dev.yml logs -f

dev-rebuild: ## Rebuild completo del entorno de desarrollo
	@echo "$(CYAN)Rebuild desarrollo...$(RESET)"
	docker compose -f docker-compose.dev.yml down
	docker compose -f docker-compose.dev.yml up --build -d
	@make dev-up

dev-status: ## Estado de servicios de desarrollo
	@docker compose -f docker-compose.dev.yml ps

# =============================================================================
# Desarrollo local
# =============================================================================

dev: ## Ejecuta el backend en modo desarrollo (local)
	@echo "$(CYAN)Iniciando en modo desarrollo...$(RESET)"
	RUST_LOG=debug cargo run

check: ## Verifica que el código compile
	cargo check

test: ## Ejecuta tests
	cargo test

fmt: ## Formatea el código
	cargo fmt

lint: ## Ejecuta clippy
	cargo clippy -- -D warnings

# =============================================================================
# MediaMTX & Cámaras
# =============================================================================

sync: ## Sincroniza todas las cámaras con MediaMTX
	@echo "$(CYAN)Sincronizando cámaras...$(RESET)"
	@TOKEN=$$(curl -s -X POST http://localhost:8080/auth/login \
		-H "Content-Type: application/json" \
		-d '{"username": "admin", "password": "admin"}' | jq -r '.token') && \
	curl -s -X POST http://localhost:8080/api/cameras/sync \
		-H "Authorization: Bearer $$TOKEN" | jq .

streams: ## Lista streams activos en MediaMTX
	@echo "$(CYAN)Streams activos:$(RESET)"
	@curl -s -u admin:mediamtx_secret http://localhost:9997/v3/paths/list | jq '.items[] | {name, source: .source.type, ready: .ready, readers: .readers}'

recordings: ## Muestra grabaciones recientes
	@echo "$(CYAN)Grabaciones:$(RESET)"
	@if [ -d "./recordings" ]; then \
		find ./recordings -type f -name "*.mp4" -o -name "*.m4s" | head -30; \
		echo ""; \
		echo "Total: $$(find ./recordings -type f | wc -l | tr -d ' ') archivos"; \
		echo "Tamaño: $$(du -sh ./recordings 2>/dev/null | cut -f1)"; \
	else \
		echo "$(YELLOW)No existe directorio ./recordings$(RESET)"; \
	fi

# =============================================================================
# Utilidades
# =============================================================================

clean: ## Limpia contenedores, imágenes y target
	@echo "$(YELLOW)Limpiando...$(RESET)"
	docker compose down -v --rmi local 2>/dev/null || true
	rm -rf target/
	@echo "$(GREEN)Limpieza completada$(RESET)"

reset-db: ## Resetea la base de datos (re-seed cámaras)
	@echo "$(RED)Eliminando base de datos...$(RESET)"
	docker compose exec mediamtx-backend rm -f /app/data/cameras.db
	@echo "$(CYAN)Reiniciando backend para re-seed...$(RESET)"
	docker compose restart mediamtx-backend
	@sleep 5
	@echo "$(GREEN)DB reseteada. Ejecuta 'make sync' para sincronizar con MediaMTX.$(RESET)"

status: ## Muestra estado de los servicios
	@docker compose ps
	@echo ""
	@echo "$(CYAN)Health:$(RESET)"
	@curl -s http://localhost:8080/health | jq . 2>/dev/null || echo "  Backend no disponible"

health: ## Verifica health del backend
	@curl -s http://localhost:8080/health | jq . || echo "Backend no disponible"

login: ## Obtiene un token JWT de prueba
	@echo "$(CYAN)Obteniendo token JWT...$(RESET)"
	@curl -s -X POST http://localhost:8080/auth/login \
		-H "Content-Type: application/json" \
		-d '{"username": "admin", "password": "admin"}' | jq .

jwks: ## Muestra el JWKS
	@curl -s http://localhost:8080/jwks | jq .

# =============================================================================
# S3 / MinIO - Sincronización y Backup
# =============================================================================

s3-status: ## Muestra estado de sincronización S3
	@echo "$(CYAN)Estado de Sincronización S3:$(RESET)"
	@docker exec rclone-sync /health-check.sh 2>/dev/null || echo "$(RED)✗ RClone Sync no está corriendo$(RESET)"

s3-logs: ## Muestra logs de sincronización
	@docker exec rclone-sync tail -50 /logs/backup.log 2>/dev/null || echo "$(RED)No hay logs disponibles$(RESET)"

s3-logs-live: ## Logs de sync en tiempo real
	@docker exec rclone-sync tail -f /logs/backup.log

s3-sync: ## Fuerza sincronización manual inmediata
	@echo "$(CYAN)Ejecutando sincronización manual...$(RESET)"
	@docker exec rclone-sync /manual-sync.sh

s3-sync-dry: ## Simula sincronización (dry-run)
	@echo "$(CYAN)Simulación de sincronización (dry-run):$(RESET)"
	@docker exec rclone-sync /manual-sync.sh --dry-run

s3-health: ## Verifica salud del sistema S3
	@echo "$(CYAN)Health Check S3:$(RESET)"
	@docker exec rclone-sync /health-check.sh

s3-ui: ## Abre consola web de MinIO
	@echo "$(CYAN)Abriendo MinIO Console...$(RESET)"
	@echo "URL: http://localhost:9001"
	@echo "User: minioadmin"
	@echo "Pass: minioadmin123"
	@open http://localhost:9001 2>/dev/null || xdg-open http://localhost:9001 2>/dev/null || echo "Abre manualmente: http://localhost:9001"

s3-size: ## Muestra tamaño de bucket S3
	@echo "$(CYAN)Tamaño del bucket S3:$(RESET)"
	@docker exec rclone-sync rclone size minio:recordings 2>/dev/null || echo "$(RED)Error consultando tamaño$(RESET)"

s3-list: ## Lista archivos en S3
	@echo "$(CYAN)Archivos en S3 (últimos 20):$(RESET)"
	@docker exec rclone-sync rclone ls minio:recordings 2>/dev/null | tail -20 || echo "$(RED)Error listando archivos$(RESET)"

s3-restart: ## Reinicia servicio de sincronización
	@echo "$(YELLOW)Reiniciando RClone Sync...$(RESET)"
	@docker compose -f docker-compose.dev.yml restart rclone-sync

# =============================================================================
# Backup y Mantenimiento
# =============================================================================

backup-db: ## Backup de la base de datos
	@echo "$(CYAN)Creando backup de base de datos...$(RESET)"
	@mkdir -p ./backups
	@docker exec backend-dev cat /app/data/cameras.db > ./backups/cameras_$(shell date +%Y%m%d_%H%M%S).db
	@echo "$(GREEN)✓ Backup creado en ./backups/$(RESET)"

backup-config: ## Backup de configuración
	@echo "$(CYAN)Creando backup de configuración...$(RESET)"
	@mkdir -p ./backups
	@tar czf ./backups/config_$(shell date +%Y%m%d_%H%M%S).tar.gz \
		docker-compose*.yml Makefile mediamtx.yml .env* scripts/ templates/ 2>/dev/null || true
	@echo "$(GREEN)✓ Backup de configuración creado$(RESET)"

backup: backup-db backup-config ## Backup completo (DB + config)

clean-recordings: ## Limpia grabaciones antiguas (>7 días)
	@echo "$(YELLOW)⚠ Limpiando grabaciones antiguas (>7 días)...$(RESET)"
	@find ./recordings -type f -mtime +7 -delete 2>/dev/null || true
	@echo "$(GREEN)✓ Limpieza completada$(RESET)"

clean-logs: ## Limpia logs antiguos
	@echo "$(YELLOW)Limpiando logs antiguos...$(RESET)"
	@docker exec rclone-sync sh -c 'find /logs -type f -name "*.log.old" -delete' 2>/dev/null || true
	@echo "$(GREEN)✓ Logs limpiados$(RESET)"

clean-old-recordings: ## Limpia grabaciones >30 días
	@echo "$(YELLOW)⚠ Limpiando grabaciones MUY antiguas (>30 días)...$(RESET)"
	@find ./recordings -type f -mtime +30 -delete 2>/dev/null || true
	@echo "$(GREEN)✓ Grabaciones antiguas eliminadas$(RESET)"

maintenance: ## Mantenimiento completo del sistema
	@echo "$(CYAN)╔════════════════════════════════════════╗$(RESET)"
	@echo "$(CYAN)║  Mantenimiento del Sistema             ║$(RESET)"
	@echo "$(CYAN)╚════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(YELLOW)1. Backup de base de datos...$(RESET)"
	@make backup-db
	@echo ""
	@echo "$(YELLOW)2. Backup de configuración...$(RESET)"
	@make backup-config
	@echo ""
	@echo "$(YELLOW)3. Limpieza de logs antiguos...$(RESET)"
	@make clean-logs
	@echo ""
	@echo "$(YELLOW)4. Verificando salud S3...$(RESET)"
	@make s3-health
	@echo ""
	@echo "$(YELLOW)5. Estado de servicios...$(RESET)"
	@make status
	@echo ""
	@echo "$(GREEN)✓ Mantenimiento completado$(RESET)"

monitor: ## Monitor en tiempo real de todos los servicios
	@echo "$(CYAN)Monitoreando servicios (Ctrl+C para salir)...$(RESET)"
	@watch -n 5 'docker compose -f docker-compose.dev.yml ps'

# =============================================================================
# Shells
# =============================================================================

shell-backend: ## Shell en contenedor backend
	@docker compose -f docker-compose.dev.yml exec backend-dev /bin/bash

shell-mediamtx: ## Shell en contenedor MediaMTX
	@docker compose -f docker-compose.dev.yml exec mediamtx /bin/sh

shell-frontend: ## Shell en contenedor frontend
	@docker compose -f docker-compose.dev.yml exec frontend-dev /bin/sh

shell-minio: ## Shell en contenedor MinIO
	@docker compose -f docker-compose.dev.yml exec minio /bin/sh

shell-sync: ## Shell en contenedor RClone Sync
	@docker compose -f docker-compose.dev.yml exec rclone-sync /bin/bash

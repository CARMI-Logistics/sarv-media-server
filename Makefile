# =============================================================================
# Makefile - MediaMTX Auth Backend
# =============================================================================

.PHONY: help build run up start stop down restart rebuild \
        logs logs-backend logs-mediamtx \
        dev check test fmt lint \
        clean status health login jwks \
        sync streams recordings reset-db \
        shell-backend shell-mediamtx

# Colores para output
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
RESET := \033[0m

help: ## Muestra esta ayuda
	@echo "$(CYAN)MediaMTX Auth Backend$(RESET)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(RESET) %s\n", $$1, $$2}'

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

shell-backend: ## Abre shell en el contenedor del backend
	docker compose exec mediamtx-backend /bin/bash

shell-mediamtx: ## Abre shell en el contenedor de MediaMTX
	docker compose exec mediamtx /bin/sh

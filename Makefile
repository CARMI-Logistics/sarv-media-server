# =============================================================================
# Makefile - MediaMTX Auth Backend
# =============================================================================

.PHONY: help build run stop logs clean dev test check

# Colores para output
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
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
	@echo "$(CYAN)Verificando ffmpeg...$(RESET)"
	@which ffmpeg > /dev/null || brew install ffmpeg
	@echo "$(CYAN)Iniciando servicios...$(RESET)"
	docker compose up -d
	@echo ""
	@echo "$(GREEN)Servicios iniciados:$(RESET)"
	@echo "  - Auth Backend: http://localhost:8080"
	@echo "  - API Docs:     http://localhost:8080/docs"
	@echo "  - MediaMTX API: http://localhost:9997"
	@echo "  - HLS:          http://localhost:8888"

up: build run ## Construye y levanta servicios

start: ## Inicia servicios existentes
	docker compose start

stop: ## Detiene los servicios
	@echo "$(YELLOW)Deteniendo servicios...$(RESET)"
	docker compose stop

down: ## Detiene y elimina contenedores
	@echo "$(YELLOW)Eliminando contenedores...$(RESET)"
	docker compose down

restart: stop run ## Reinicia los servicios

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
	cargo run

check: ## Verifica que el código compile
	cargo check

test: ## Ejecuta tests
	cargo test

fmt: ## Formatea el código
	cargo fmt

lint: ## Ejecuta clippy
	cargo clippy -- -D warnings

# =============================================================================
# Utilidades
# =============================================================================

clean: ## Limpia contenedores, imágenes y target
	@echo "$(YELLOW)Limpiando...$(RESET)"
	docker compose down -v --rmi local 2>/dev/null || true
	rm -rf target/
	@echo "$(GREEN)Limpieza completada$(RESET)"

status: ## Muestra estado de los servicios
	docker compose ps

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

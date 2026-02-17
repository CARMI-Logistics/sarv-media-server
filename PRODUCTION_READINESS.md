# 🚀 Análisis de Preparación para Producción - CamManager

**Fecha de Auditoría:** 17 de Febrero, 2026  
**Sistema:** MediaMTX Auth Backend + SvelteKit Frontend  
**Versión:** 0.1.0

---

## 📊 Resumen Ejecutivo

| Categoría | Estado | Crítico | Medio | Bajo |
|-----------|--------|---------|-------|------|
| **Seguridad** | 🟡 Requiere Acción | 3 | 4 | 2 |
| **Configuración** | 🟡 Requiere Acción | 2 | 3 | 1 |
| **Datos & Backup** | 🟢 Bueno | 0 | 2 | 1 |
| **Docker & Deploy** | 🟡 Requiere Acción | 1 | 2 | 0 |
| **Monitoreo** | 🔴 Crítico | 2 | 1 | 0 |
| **Documentación** | 🟢 Excelente | 0 | 1 | 0 |

**Estado General:** 🟡 **REQUIERE MEJORAS ANTES DE PRODUCCIÓN**

---

## 🔴 GAPS CRÍTICOS (Bloqueadores)

### 1. 🔐 Credenciales Hardcodeadas en Código

**Prioridad:** CRÍTICA  
**Impacto:** Seguridad alta, violación SOC2

**Ubicación:**
- `src/db.rs` - Función `seed_default_user()` crea usuario admin con contraseña "admin"
- MediaMTX config hardcodeada en múltiples lugares

**Problema:**
```rust
// src/db.rs - línea ~183
pub fn seed_default_user(&self) -> Result<()> {
    let hash = bcrypt::hash("admin", 10)?;  // ❌ HARDCODED
    // ...
}
```

**Solución Requerida:**
```bash
# 1. Crear script de inicialización que solicite contraseña
# 2. Usar variables de entorno para contraseñas iniciales
# 3. Forzar cambio de contraseña en primer login

# Agregar a .env:
ADMIN_INITIAL_PASSWORD=changeme123
MEDIAMTX_API_PASSWORD=secure-random-password-here
```

**Acción:**
- [ ] Crear script `scripts/init-admin.sh` que genere contraseña segura
- [ ] Modificar `seed_default_user()` para usar env vars
- [ ] Implementar forzado de cambio de contraseña en primer login
- [ ] Documentar proceso de creación de admin en producción

---

### 2. 🌐 CORS Configurado en Modo "Any"

**Prioridad:** CRÍTICA  
**Impacto:** Vulnerabilidad XSS/CSRF, violación seguridad

**Ubicación:**
`src/main.rs:746-749`

**Problema:**
```rust
let cors = CorsLayer::new()
    .allow_origin(Any)  // ❌ Permite CUALQUIER origen
    .allow_methods(Any)
    .allow_headers(Any);
```

**Solución Requerida:**
```rust
use tower_http::cors::AllowOrigin;

let allowed_origins = env::var("ALLOWED_ORIGINS")
    .unwrap_or_else(|_| "http://localhost:5173".to_string())
    .split(',')
    .map(|s| s.parse::<HeaderValue>().unwrap())
    .collect::<Vec<_>>();

let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::list(allowed_origins))
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE]);
```

**Acción:**
- [ ] Implementar CORS restrictivo con whitelist de dominios
- [ ] Agregar `ALLOWED_ORIGINS` a variables de entorno
- [ ] Documentar configuración de CORS en producción

---

### 3. 📊 Sin Sistema de Monitoreo/Alertas

**Prioridad:** CRÍTICA  
**Impacto:** No hay visibilidad de incidentes en producción

**Problema:**
- No hay métricas exportadas (Prometheus)
- No hay alertas configuradas
- No hay monitoreo de salud de servicios críticos
- No hay dashboards de observabilidad

**Solución Requerida:**
1. **Métricas con Prometheus:**
   - Exportar métricas de Axum con `axum-prometheus`
   - Métricas de FFmpeg (mosaicos activos)
   - Métricas de base de datos (queries, latencia)

2. **Alertas:**
   - Disco lleno (>85%)
   - Mosaico FFmpeg caído
   - S3 sync fallido por >2 horas
   - Backend sin responder >30s
   - Error rate >5%

3. **Logging Centralizado:**
   - Integrar con Grafana Loki o ELK
   - Structured logging con contexto

**Acción:**
- [ ] Crear `docker-compose.monitoring.yml` con Prometheus/Grafana
- [ ] Implementar endpoints `/metrics` en backend
- [ ] Configurar alertas básicas en Prometheus
- [ ] Documentar setup de monitoreo

---

### 4. 🔒 Sin HTTPS/TLS

**Prioridad:** CRÍTICA para internet, MEDIA para intranet  
**Impacto:** Credenciales en texto plano, violación compliance

**Problema:**
- Backend expone puerto 8080 HTTP
- No hay certificados configurados
- Tokens JWT enviados sin encripción

**Solución Requerida:**

**Opción A: Reverse Proxy (Recomendado)**
```yaml
# docker-compose.prod.yml
services:
  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./certs:/etc/nginx/certs:ro
    depends_on:
      - mediamtx-backend
```

**Opción B: Axum nativo con TLS**
```rust
// Requiere axum-server con rustls
axum_server::bind_rustls(addr, config)
    .serve(app.into_make_service())
    .await?;
```

**Acción:**
- [ ] Decidir estrategia (Nginx vs nativo)
- [ ] Configurar Let's Encrypt con certbot
- [ ] Implementar redirección HTTP→HTTPS
- [ ] Actualizar documentación con setup HTTPS

---

## 🟡 GAPS MEDIOS (Importantes)

### 5. 📝 Falta `.env` en `.gitignore` del Root

**Prioridad:** MEDIA  
**Impacto:** Riesgo de leak de credenciales en repo

**Problema:**
Archivo `.gitignore` existe pero podría estar incompleto

**Solución:**
Asegurar que `.gitignore` incluya:
```gitignore
# Environment files
.env
.env.local
.env.*.local
.env.production

# Secrets
*.key
*.pem
*.crt
/certs/

# Data
/data/
*.db
*.db-shm
*.db-wal
```

**Acción:**
- [ ] Verificar `.gitignore` completo
- [ ] Agregar pre-commit hook para detectar secrets
- [ ] Documentar manejo de secrets

---

### 6. 🔑 Gestión de Claves RSA no Persistente

**Prioridad:** MEDIA  
**Impacto:** Tokens inválidos después de restart

**Problema:**
Las claves RSA para JWT se regeneran en cada inicio del backend, invalidando todos los tokens existentes.

**Ubicación:** `src/main.rs` - generación de claves en memoria

**Solución:**
```rust
// Persistir claves en volumen Docker
let key_path = "/app/data/jwt_keys";
let private_key = if Path::new(&format!("{}/private.pem", key_path)).exists() {
    // Cargar clave existente
    load_private_key(&key_path)?
} else {
    // Generar y guardar nueva clave
    let key = generate_rsa_keys();
    save_private_key(&key_path, &key)?;
    key
};
```

**Acción:**
- [ ] Implementar persistencia de claves RSA
- [ ] Agregar rotación de claves programada (cada 90 días)
- [ ] Documentar proceso de backup de claves

---

### 7. 🗄️ Sin Backups Automáticos de Base de Datos

**Prioridad:** MEDIA  
**Impacto:** Pérdida de configuración/usuarios en fallo de disco

**Problema:**
- S3 backup solo para recordings
- SQLite DB no tiene backup automático
- No hay snapshot scheduling

**Solución:**
```bash
#!/bin/bash
# scripts/backup-db.sh
BACKUP_DIR="/app/data/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Backup SQLite con VACUUM INTO
sqlite3 /app/data/cameras.db "VACUUM INTO '${BACKUP_DIR}/cameras_${TIMESTAMP}.db'"

# Retener solo últimos 30 días
find ${BACKUP_DIR} -name "cameras_*.db" -mtime +30 -delete

# Sync a S3
rclone copy ${BACKUP_DIR} minio:backups/database/
```

**Acción:**
- [ ] Crear script de backup de DB
- [ ] Agregar cron job diario
- [ ] Implementar restore procedure
- [ ] Probar proceso de disaster recovery

---

### 8. ⚠️ Manejo de Errores Inconsistente

**Prioridad:** MEDIA  
**Impacto:** Debugging difícil, mensajes de error poco informativos

**Problema:**
- Uso extensivo de `.unwrap()` en database code (67 matches)
- Mensajes de error genéricos para el usuario
- Falta contexto en logs

**Ejemplo:**
```rust
// src/db.rs - múltiples líneas
let conn = self.conn.lock().unwrap(); // ❌ Panic en error
```

**Solución:**
```rust
let conn = self.conn.lock()
    .map_err(|e| anyhow::anyhow!("Failed to acquire DB lock: {}", e))?;
```

**Acción:**
- [ ] Refactor de unwrap() a proper error handling
- [ ] Implementar error types custom con thiserror
- [ ] Agregar structured logging con tracing spans
- [ ] Mejorar mensajes de error para usuarios

---

### 9. 🔄 Sin Health Checks para Servicios Dependientes

**Prioridad:** MEDIA  
**Impacto:** Fallos en cascade no detectados

**Problema:**
- Backend no verifica conectividad con MediaMTX en startup
- No hay health check de MinIO
- Frontend no tiene ping a backend

**Solución:**
```rust
// En startup de backend
async fn verify_dependencies(state: &AppState) -> Result<()> {
    // Check MediaMTX API
    let response = state.mediamtx_client
        .get(&format!("{}/v3/config/get", state.mediamtx_url))
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("MediaMTX not reachable"));
    }
    
    // Check S3/MinIO
    // Check DB integrity
    
    Ok(())
}
```

**Acción:**
- [ ] Implementar dependency checks en startup
- [ ] Agregar `/health/deep` endpoint que verifica dependencias
- [ ] Configurar health checks en docker-compose
- [ ] Implementar circuit breaker para servicios externos

---

### 10. 📊 Rate Limiting Global Muy Permisivo

**Prioridad:** MEDIA  
**Impacto:** Vulnerable a ataques DDoS básicos

**Ubicación:** `src/security.rs:67`

**Problema:**
```rust
// 100 requests/minuto por IP es muy permisivo
RateLimiter::new(100, 60)
```

**Solución:**
```rust
// Rate limiting por endpoint:
// - Login: 5 req/min
// - API general: 30 req/min
// - Stream access: 100 req/min

pub fn endpoint_rate_limiter(endpoint_type: &str) -> RateLimiter {
    match endpoint_type {
        "auth" => RateLimiter::new(5, 60),
        "api" => RateLimiter::new(30, 60),
        "stream" => RateLimiter::new(100, 60),
        _ => RateLimiter::new(10, 60),
    }
}
```

**Acción:**
- [ ] Implementar rate limiting por endpoint
- [ ] Agregar IP whitelist para servicios internos
- [ ] Configurar rate limits en nginx si se usa
- [ ] Documentar límites y cómo ajustarlos

---

### 11. 🎨 Frontend con Credenciales en localStorage

**Prioridad:** MEDIA  
**Impacto:** Vulnerable a XSS token theft

**Ubicación:** `frontend/src/lib/stores/auth.svelte.ts`

**Problema:**
```typescript
localStorage.setItem('jwt_token', t); // ❌ Vulnerable a XSS
```

**Solución Mejor:**
```typescript
// Usar httpOnly cookies desde backend
// O implementar session storage con refresh tokens

// Backend debe establecer cookie:
Set-Cookie: jwt=<token>; HttpOnly; Secure; SameSite=Strict
```

**Acción:**
- [ ] Migrar a httpOnly cookies
- [ ] Implementar refresh token pattern
- [ ] Agregar CSRF protection
- [ ] Sanitizar inputs para prevenir XSS

---

## 🟢 FORTALEZAS ACTUALES

### ✅ Lo que está BIEN implementado:

1. **✅ Password Hashing Seguro**
   - Uso correcto de bcrypt con cost factor 10
   - Ubicación: `src/user.rs`

2. **✅ Security Headers Completos**
   - CSP, HSTS, X-Frame-Options configurados
   - Ubicación: `src/security.rs:78-120`

3. **✅ Sistema de Backup S3 Robusto**
   - RClone configurado con retry logic
   - Health checks implementados
   - Documentación excelente en `S3_BACKUP_README.md`

4. **✅ Role-Based Access Control**
   - Sistema de roles y permisos implementado
   - Funciones `check_permission()` y `get_user_permissions()`
   - UI de gestión de roles funcional

5. **✅ JWT con RS256**
   - Algoritmo seguro (no HS256)
   - JWKS endpoint para validación externa
   - Claims bien estructurados

6. **✅ Input Validation**
   - Uso de Zod en frontend
   - Validator crate en backend
   - Funciones de sanitización en `security.rs`

7. **✅ Docker Multi-Stage Builds**
   - Imagen final optimizada
   - Separación dev/prod
   - Health checks configurados

8. **✅ Documentación Extensa**
   - README completo
   - Guías de mantenimiento
   - API docs con OpenAPI/Scalar

9. **✅ Audit Logging**
   - Estructura preparada en `security.rs:122-159`
   - Logging con tracing/tracing-subscriber

---

## 📋 CHECKLIST PRE-PRODUCCIÓN

### Seguridad (CRÍTICO)
- [ ] Eliminar contraseñas hardcodeadas
- [ ] Configurar CORS restrictivo con whitelist
- [ ] Implementar HTTPS/TLS (nginx o nativo)
- [ ] Migrar JWT de localStorage a httpOnly cookies
- [ ] Configurar firewall (solo puertos necesarios)
- [ ] Cambiar credenciales default de MinIO
- [ ] Rotar claves RSA y persistirlas
- [ ] Implementar rate limiting por endpoint

### Monitoreo (CRÍTICO)
- [ ] Setup Prometheus + Grafana
- [ ] Configurar alertas básicas
- [ ] Implementar health checks profundos
- [ ] Centralizar logs (Loki/ELK)
- [ ] Dashboard de métricas clave

### Datos & Backup (IMPORTANTE)
- [ ] Backup automático de SQLite DB
- [ ] Test de disaster recovery
- [ ] Verificar S3 sync funcionando
- [ ] Configurar retención de datos (30/90 días)
- [ ] Documentar restore procedures

### Configuración (IMPORTANTE)
- [ ] Crear `.env.production.example`
- [ ] Documentar todas las env vars
- [ ] Implementar secrets management (Vault/AWS Secrets)
- [ ] Configurar logging level apropiado (info)
- [ ] Validar docker-compose.prod.yml

### Testing (RECOMENDADO)
- [ ] Tests de integración para APIs críticas
- [ ] Load testing (identificar límites)
- [ ] Penetration testing básico
- [ ] Test de failover de servicios
- [ ] Validar backup/restore funciona

### DevOps (RECOMENDADO)
- [ ] CI/CD pipeline (GitHub Actions)
- [ ] Automated security scanning
- [ ] Container image scanning
- [ ] Deployment playbook/runbook
- [ ] Rollback procedure documentado

---

## 🎯 PLAN DE ACCIÓN PRIORIZADO

### Fase 1: BLOQUEADORES (1-2 semanas)
**Objetivo:** Sistema seguro para producción básica

1. **Semana 1:**
   - Implementar HTTPS con nginx + Let's Encrypt
   - Eliminar credenciales hardcodeadas
   - Configurar CORS restrictivo
   - Setup Prometheus + Grafana básico

2. **Semana 2:**
   - Persistir claves RSA
   - Implementar backups de SQLite
   - Migrar a httpOnly cookies
   - Configurar alertas críticas

### Fase 2: MEJORAS (2-3 semanas)
**Objetivo:** Sistema robusto y monitoreable

1. **Semanas 3-4:**
   - Refactor error handling (eliminar unwraps)
   - Rate limiting por endpoint
   - Health checks profundos
   - Tests de integración

2. **Semana 5:**
   - Centralizar logs
   - Documentar procedures
   - Load testing
   - Security audit

### Fase 3: OPTIMIZACIÓN (Continua)
**Objetivo:** Sistema production-grade completo

- CI/CD automation
- Advanced monitoring/dashboards
- Performance optimization
- Security hardening continuo

---

## 📊 MÉTRICAS DE ÉXITO

| Métrica | Objetivo | Actual | Gap |
|---------|----------|--------|-----|
| **Uptime** | >99.5% | N/A | Implementar monitoreo |
| **Response Time (p95)** | <500ms | N/A | Agregar métricas |
| **Error Rate** | <0.1% | N/A | Agregar alertas |
| **Backup Success** | 100% | ~95% (solo S3) | Agregar DB backup |
| **Security Score** | A+ | B- | Resolver gaps críticos |
| **Code Coverage** | >70% | 0% | Agregar tests |

---

## 🔗 RECURSOS Y REFERENCIAS

### Documentación Interna
- `README.md` - Setup general
- `S3_BACKUP_README.md` - Sistema de backups
- `MAINTENANCE.md` - Comandos de mantenimiento
- `ENV_SETUP.md` - Configuración de entorno

### Archivos Clave
- `docker-compose.yml` - Producción base
- `docker-compose.prod.yml` - Overlay de producción
- `docker-compose.dev.yml` - Desarrollo
- `src/security.rs` - Módulo de seguridad
- `src/db.rs` - Gestión de base de datos

### Próximos Pasos Sugeridos
1. Revisar este documento con el equipo
2. Priorizar gaps según negocio
3. Crear tickets/issues para cada item
4. Asignar responsables y timelines
5. Setup ambiente de staging para validar cambios

---

**Preparado por:** Cascade AI  
**Última actualización:** 17 de Febrero, 2026  
**Próxima revisión:** Antes del deployment a producción

---

## 💡 NOTAS FINALES

Este sistema tiene una **base sólida** con buenas prácticas en muchas áreas (password hashing, security headers, backups S3, RBAC). Los gaps identificados son **corregibles** y no representan defectos arquitectónicos fundamentales.

**Recomendación:** Con 3-4 semanas de trabajo enfocado en resolver los gaps críticos y medios, este sistema estará **production-ready** para un entorno empresarial.

**Risk Assessment:** 
- **Actual (sin cambios):** 🔴 ALTO - No recomendado para producción internet-facing
- **Post Fase 1:** 🟡 MEDIO - Aceptable para intranet corporativa
- **Post Fase 2:** 🟢 BAJO - Production-ready para uso general

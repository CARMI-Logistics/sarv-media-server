# Variables de Entorno - CamManager

## Variables Requeridas para SOC2 y Funcionalidades Completas

### Backend (Rust)

Crear archivo `.env` en la raíz del proyecto:

```bash
# ===========================
# Server Configuration
# ===========================
SERVER_PORT=8080
JWT_EXP_MINUTES=60
RUST_LOG=info

# ===========================
# Database
# ===========================
DATABASE_PATH=/app/data/cameras.db

# ===========================
# MediaMTX API
# ===========================
MEDIAMTX_API_URL=http://mediamtx:9997
MEDIAMTX_API_USER=admin
MEDIAMTX_API_PASS=mediamtx_secret

# ===========================
# Frontend URL (para emails)
# ===========================
FRONTEND_URL=http://localhost:5173

# ===========================
# Static Files
# ===========================
STATIC_DIR=/app/static

# ===========================
# Email Service (Resend)
# ===========================
# IMPORTANTE: Obtener API key en https://resend.com/api-keys
RESEND_API_KEY=re_xxxxxxxxxxxxxxxxxxxxxxxxxx
EMAIL_FROM=noreply@tudominio.com

# ===========================
# Seguridad SOC2
# ===========================
# Rate limiting: 100 requests/minuto (configurado en código)
# Security headers: aplicados automáticamente
# Audit logging: habilitado automáticamente en target=audit
```

### Frontend (SvelteKit)

Crear archivo `frontend/.env.local`:

```bash
# Backend API URL
# En desarrollo local:
VITE_API_URL=http://localhost:8080

# Con túneles (ngrok/localtunnel):
# VITE_API_URL=https://tu-backend-tunnel.loca.lt
```

## Configuración de Resend

### 1. Crear cuenta en Resend
- Ir a https://resend.com/signup
- Verificar dominio o usar dominio de prueba

### 2. Generar API Key
- Dashboard → API Keys → Create API Key
- Copiar la key y agregarla a `RESEND_API_KEY`

### 3. Configurar dominio verificado
```bash
EMAIL_FROM=noreply@tudominio.com
```

## Templates de Email Disponibles

El sistema incluye 3 templates de email con Handlebars:

1. **Password Reset** (`templates/reset_password.hbs`)
   - Enviado cuando usuario solicita restablecer contraseña
   - Variables: `username`, `reset_link`

2. **Welcome Email** (`templates/welcome.hbs`)
   - Enviado al crear nuevo usuario
   - Variables: `username`, `login_url`

3. **Mosaic Share** (`templates/share_mosaic.hbs`)
   - Enviado al compartir mosaico
   - Variables: `mosaic_name`, `share_link`, `expires_at`

## Características SOC2 Implementadas

### 1. Rate Limiting
- Límite global: 100 requests/minuto por IP
- Implementado con middleware en Rust
- Headers monitoreados: `X-Forwarded-For`

### 2. Security Headers
Aplicados automáticamente a todas las respuestas:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`
- `Strict-Transport-Security: max-age=31536000`
- `Content-Security-Policy` restrictivo
- `Referrer-Policy: strict-origin-when-cross-origin`

### 3. Password Strength Validation
- Mínimo 8 caracteres
- Requiere mayúsculas, minúsculas y números
- Hashing con bcrypt (cost 10)

### 4. Audit Logging
Logs estructurados para:
- Creación de usuarios
- Password reset requests
- Operaciones críticas

Ver logs con:
```bash
docker logs backend-dev | grep AUDIT_LOG
```

### 5. Email Enumeration Prevention
- Forgot password siempre retorna éxito
- Previene descubrimiento de emails existentes

## Sistema de Thumbnails Automáticos

### Configuración
Los thumbnails se capturan automáticamente cada 60 segundos para cada cámara activa.

```bash
# Directorio de almacenamiento
/app/data/thumbnails/

# Formato de archivos
camera_{id}_thumb_{timestamp}.jpg

# Retención: últimos 10 thumbnails por cámara
```

### API Endpoint
```bash
GET /api/cameras/:id/thumbnail
```

Respuesta:
```json
{
  "success": true,
  "data": {
    "camera_id": 1,
    "thumbnail_url": "/data/thumbnails/camera_1_thumb_1234567890.jpg"
  }
}
```

## Arquitectura de Seguridad

```
┌─────────────────────────────────────────────┐
│         Client (Browser/Mobile)             │
└────────────────┬────────────────────────────┘
                 │
                 │ HTTPS (Producción)
                 ▼
┌─────────────────────────────────────────────┐
│     Rate Limiting Middleware (100/min)      │
├─────────────────────────────────────────────┤
│     Security Headers Middleware             │
├─────────────────────────────────────────────┤
│     JWT Authentication Middleware           │
├─────────────────────────────────────────────┤
│     Axum Router (API Endpoints)             │
├─────────────────────────────────────────────┤
│     Business Logic                          │
│     - Password validation                   │
│     - Email service (Resend)                │
│     - Audit logging                         │
└─────────────────────────────────────────────┘
```

## Deployment Checklist

### Antes de desplegar a producción:

- [ ] Configurar `RESEND_API_KEY` con key válida
- [ ] Cambiar `EMAIL_FROM` a dominio verificado
- [ ] Actualizar `FRONTEND_URL` a URL de producción
- [ ] Habilitar HTTPS (usar certificado SSL/TLS)
- [ ] Configurar firewall para solo exponer puertos necesarios
- [ ] Revisar logs de audit: `RUST_LOG=info`
- [ ] Backup automático de base de datos SQLite
- [ ] Configurar monitoreo de rate limiting
- [ ] Validar templates de email funcionan correctamente
- [ ] Verificar thumbnails se generan y limpian correctamente

## Testing

### Test de email local (development)
```bash
# Sin RESEND_API_KEY configurado, los emails se logean pero no se envían
docker logs backend-dev | grep "email"
```

### Test de rate limiting
```bash
# Hacer más de 100 requests en 60 segundos
for i in {1..110}; do curl http://localhost:8080/health; done
# Debería retornar 429 Too Many Requests después de request 100
```

### Test de security headers
```bash
curl -I http://localhost:8080/health
# Verificar headers de seguridad en respuesta
```

### Test de thumbnails
```bash
# Verificar generación
curl http://localhost:8080/api/cameras/1/thumbnail

# Ver archivos generados
docker exec backend-dev ls -la /app/data/thumbnails/
```

## Troubleshooting

### Emails no se envían
1. Verificar `RESEND_API_KEY` está configurado
2. Verificar dominio está verificado en Resend
3. Ver logs: `docker logs backend-dev | grep "email"`

### Thumbnails no aparecen
1. Verificar FFmpeg está instalado: `docker exec backend-dev which ffmpeg`
2. Verificar cámara está online
3. Ver logs de captura: `docker logs backend-dev | grep "thumbnail"`

### Rate limiting demasiado restrictivo
Modificar en `src/security.rs`:
```rust
RateLimiter::new(200, 60) // 200 requests/minuto
```

## Contacto de Soporte

Para problemas de seguridad o compliance, contactar al equipo de seguridad.

# Guía de Mantenimiento - CamManager

Esta guía explica cómo usar los comandos `make` para el mantenimiento diario del sistema.

---

## 📋 Comandos Rápidos

```bash
make help              # Ver todos los comandos disponibles
make dev-up            # Iniciar sistema de desarrollo
make maintenance       # Ejecutar mantenimiento completo
make s3-status         # Ver estado de sincronización S3
```

---

## 🚀 Desarrollo

### Iniciar Sistema

```bash
# Iniciar todo (primera vez o después de cambios)
make dev-up

# Servicios disponibles:
# - Frontend:  http://localhost:5173
# - Backend:   http://localhost:8080
# - API Docs:  http://localhost:8080/docs
# - MinIO:     http://localhost:9001
```

### Detener Sistema

```bash
# Detener servicios
make dev-down

# Reiniciar servicios
make dev-restart

# Rebuild completo
make dev-rebuild
```

### Ver Logs

```bash
# Todos los servicios
make dev-logs

# Servicio específico
make logs-backend
make logs-frontend
make logs-sync
make logs-minio
```

---

## 📊 Monitoreo S3

### Estado de Sincronización

```bash
# Estado completo
make s3-status

# Logs recientes (últimas 50 líneas)
make s3-logs

# Logs en tiempo real
make s3-logs-live

# Health check
make s3-health
```

### Información del Bucket

```bash
# Tamaño total
make s3-size

# Listar archivos
make s3-list

# Abrir consola web MinIO
make s3-ui
```

### Sincronización Manual

```bash
# Forzar sync inmediato
make s3-sync

# Simular sync (sin hacer cambios)
make s3-sync-dry

# Reiniciar servicio de sync
make s3-restart
```

---

## 💾 Backup

### Backup Manual

```bash
# Backup de base de datos
make backup-db

# Backup de configuración
make backup-config

# Backup completo (DB + config)
make backup
```

**Los backups se guardan en:** `./backups/`

### Programar Backups Automáticos

Agregar a crontab:

```bash
# Backup diario a las 2 AM
0 2 * * * cd /ruta/al/proyecto && make backup-db

# Backup semanal de configuración (domingos 3 AM)
0 3 * * 0 cd /ruta/al/proyecto && make backup-config
```

---

## 🧹 Limpieza

### Limpiar Grabaciones Antiguas

```bash
# Grabaciones > 7 días
make clean-recordings

# Grabaciones > 30 días
make clean-old-recordings

# Logs antiguos
make clean-logs
```

### Limpieza Programada

```bash
# Agregar a crontab para limpieza semanal
0 1 * * 0 cd /ruta/al/proyecto && make clean-recordings
```

---

## 🔧 Mantenimiento Completo

### Comando Todo-en-Uno

```bash
make maintenance
```

Este comando ejecuta:
1. ✅ Backup de base de datos
2. ✅ Backup de configuración
3. ✅ Limpieza de logs antiguos
4. ✅ Verificación de salud S3
5. ✅ Estado de servicios

**Recomendación:** Ejecutar semanalmente

---

## 🛠️ Utilidades

### Estado del Sistema

```bash
# Estado de servicios
make status

# Estado desarrollo
make dev-status

# Monitor en tiempo real
make monitor
```

### Acceso a Shells

```bash
# Backend
make shell-backend

# Frontend
make shell-frontend

# MinIO
make shell-minio

# RClone Sync
make shell-sync

# MediaMTX
make shell-mediamtx
```

### Base de Datos

```bash
# Reset completo (CUIDADO: borra datos)
make reset-db
```

---

## 📡 MediaMTX

### Gestión de Cámaras

```bash
# Sincronizar cámaras con MediaMTX
make sync

# Ver streams activos
make streams

# Ver grabaciones
make recordings
```

---

## 🔐 Autenticación

```bash
# Obtener token JWT
make login

# Ver JWKS
make jwks

# Health check backend
make health
```

---

## 📅 Rutina de Mantenimiento Recomendada

### Diario
```bash
# Verificar estado
make status
make s3-status
```

### Semanal
```bash
# Mantenimiento completo
make maintenance

# Limpiar grabaciones antiguas
make clean-recordings
```

### Mensual
```bash
# Backup completo
make backup

# Limpiar grabaciones muy antiguas
make clean-old-recordings

# Verificar espacio en disco
df -h
```

---

## 🚨 Troubleshooting

### Backend no compila

```bash
# Ver logs de compilación
make logs-backend

# Rebuild forzado
make dev-rebuild
```

### Sync S3 no funciona

```bash
# Ver estado
make s3-status

# Ver logs
make s3-logs

# Reiniciar servicio
make s3-restart

# Verificar conectividad
make shell-sync
# Dentro del shell:
rclone lsd minio:
```

### Espacio en disco lleno

```bash
# Verificar uso
du -sh recordings/
du -sh /var/lib/docker/

# Limpiar grabaciones antiguas
make clean-old-recordings

# Limpiar contenedores no usados
docker system prune -a
```

### MinIO no accesible

```bash
# Reiniciar MinIO
docker compose -f docker-compose.dev.yml restart minio

# Verificar estado
docker logs minio-dev

# Verificar puerto
lsof -i :9001
```

---

## 🔄 Actualizaciones del Sistema

### Actualizar dependencias

```bash
# Backend (Rust)
cargo update

# Frontend (Node)
cd frontend && npm update

# Rebuild
make dev-rebuild
```

### Actualizar imágenes Docker

```bash
# Descargar últimas versiones
docker compose -f docker-compose.dev.yml pull

# Rebuild con nuevas imágenes
make dev-rebuild
```

---

## 📊 Monitoreo de Recursos

### CPU y Memoria

```bash
# Ver uso de recursos
docker stats

# Top de contenedores
docker compose -f docker-compose.dev.yml top
```

### Espacio en Disco

```bash
# Uso de grabaciones
du -sh recordings/

# Uso de Docker
docker system df

# Detalles
docker system df -v
```

### Logs del Sistema

```bash
# Tamaño de logs
docker compose -f docker-compose.dev.yml logs --tail 0 2>&1 | wc -l

# Ver logs más grandes
find /var/lib/docker/containers/ -name "*.log" -exec ls -lh {} \; | sort -k5 -hr | head -5
```

---

## 🎯 Checklist de Mantenimiento

### Pre-Deployment
- [ ] `make check` - Verificar compilación
- [ ] `make test` - Ejecutar tests
- [ ] `make backup` - Backup completo
- [ ] `make s3-status` - Verificar sincronización
- [ ] Verificar espacio en disco

### Post-Deployment
- [ ] `make status` - Verificar servicios corriendo
- [ ] `make health` - Health check backend
- [ ] `make s3-health` - Health check S3
- [ ] Verificar logs sin errores críticos
- [ ] Probar login en UI

### Mantenimiento Semanal
- [ ] `make maintenance` - Mantenimiento automático
- [ ] `make clean-recordings` - Limpiar grabaciones viejas
- [ ] Revisar logs de errores
- [ ] Verificar espacio disponible
- [ ] Probar restauración de backup

---

## 📝 Notas Importantes

### Backups
- Los backups se crean en `./backups/`
- **NO** están en .gitignore - debes moverlos a ubicación segura
- Probar restauración periódicamente

### Grabaciones
- Por defecto se retienen 24h en MediaMTX
- S3 sync cada hora (configurable)
- Limpieza automática en S3 según lifecycle policies

### Seguridad
- Cambiar credenciales default en producción
- MinIO: `minioadmin / minioadmin123` → cambiar
- Backend: `admin / admin` → cambiar
- Revisar `ENV_SETUP.md` para variables de entorno

---

## 🆘 Contacto de Soporte

Para problemas de mantenimiento:
1. Revisar logs: `make dev-logs`
2. Verificar estado: `make status`
3. Ejecutar health checks: `make health` y `make s3-health`
4. Documentar error y logs relevantes

---

**Última actualización:** Febrero 2026

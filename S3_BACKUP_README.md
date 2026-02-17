# S3 Backup System - Documentación
## Sistema de Respaldos Automáticos a MinIO S3

Este sistema proporciona un mecanismo robusto y sin pérdida de datos para sincronizar grabaciones de video a un almacenamiento S3-compatible (MinIO) cada hora.

---

## 📋 Componentes del Sistema

| Componente | Descripción | Puerto |
|------------|-------------|--------|
| **MinIO** | Servidor S3 compatible | 9000 (API), 9001 (Web) |
| **RClone Sync** | Servicio de sincronización automática | - |
| **Health Check** | Script de verificación de estado | - |
| **Manual Sync** | Script para sincronización bajo demanda | - |

---

## 🚀 Inicio Rápido

### 1. Levantar los servicios

```bash
docker compose -f docker-compose.dev.yml up -d
```

### 2. Verificar que todo funciona

```bash
# Health check completo
docker exec rclone-sync /health-check.sh

# O ver logs en tiempo real
docker logs -f rclone-sync
```

### 3. Acceder a la consola MinIO

- URL: http://localhost:9001
- Access Key: `minioadmin`
- Secret Key: `minioadmin123`

---

## ⚙️ Configuración

### Variables de Entorno

| Variable | Descripción | Default |
|----------|-------------|---------|
| `S3_ENDPOINT` | URL del servidor MinIO | `http://minio:9000` |
| `S3_ACCESS_KEY` | Access Key para MinIO | `minioadmin` |
| `S3_SECRET_KEY` | Secret Key para MinIO | `minioadmin123` |
| `S3_BUCKET` | Nombre del bucket | `recordings` |
| `SYNC_INTERVAL` | Intervalo entre syncs (segundos) | `3600` (1 hora) |
| `LOG_LEVEL` | Nivel de logging | `INFO` |

### Personalizar Configuración

1. **Cambiar credenciales** (producción):
   ```bash
   # Editar docker-compose.dev.yml
   environment:
     MINIO_ROOT_USER: your-secure-username
     MINIO_ROOT_PASSWORD: your-secure-password-32-chars
     S3_ACCESS_KEY: your-secure-username
     S3_SECRET_KEY: your-secure-password-32-chars
   ```

2. **Cambiar intervalo de sync**:
   ```bash
   # Cada 30 minutos (1800 segundos)
   environment:
     SYNC_INTERVAL: 1800
   ```

3. **Configurar encriptación**:
   ```bash
   # Editar scripts/rclone/rclone.conf
   [minio-crypt]
   type = crypt
   remote = minio:recordings
   password = your-strong-password
   password2 = your-salt
   ```

---

## 📁 Estructura de Archivos

```
scripts/
├── rclone/
│   └── rclone.conf          # Configuración de rclone
├── backup.sh                # Script principal de backup (automático)
├── health-check.sh          # Verificación de estado
└── manual-sync.sh          # Sincronización manual

recordings/                  # Directorio local de grabaciones
└── [cámara]/
    └── YYYY-MM-DD_HH-MM-SS.mp4

logs/                        # Logs de sincronización (volumen Docker)
└── backup.log
```

---

## 🔧 Comandos Útiles

### Sincronización Manual

```bash
# Sync normal
docker exec rclone-sync /manual-sync.sh

# Simulación (dry-run) - ver qué se sincronizaría sin hacerlo
docker exec rclone-sync /manual-sync.sh --dry-run

# Modo verbose con verificación
docker exec rclone-sync /manual-sync.sh --verbose --verify
```

### Verificar Estado del Sistema

```bash
# Health check completo
docker exec rclone-sync /health-check.sh

# Ver logs recientes
docker exec rclone-sync tail -f /logs/backup.log

# Estadísticas de MinIO
docker exec minio mc admin info local
```

### Gestión de MinIO

```bash
# Listar buckets
docker exec minio mc ls local

# Listar archivos en bucket
docker exec minio mc ls local/recordings

# Ver espacio usado
docker exec minio mc du local/recordings
```

---

## 🛡️ Características de Seguridad

### Sin Pérdida de Datos

1. **Sync Incremental**: Solo sube archivos nuevos o modificados
2. **Verificación por Tamaño**: Compara tamaños antes de sobrescribir
3. **Backup-After**: No borra archivos remotos hasta confirmar transferencia
4. **Reintentos Automáticos**: 3 intentos por archivo fallido
5. **Lock File**: Evita ejecuciones simultáneas

### Manejo de Errores

- **Retry Logic**: Reintentos con backoff exponencial
- **Log Rotation**: Logs automáticos con rotación por tamaño
- **Health Checks**: Verificación periódica del estado
- **Lock Mechanism**: Prevención de race conditions

### Encriptación (Opcional)

```bash
# Activar encriptación en rclone.conf
# Los archivos se encriptan antes de subir a S3
[minio-crypt]
type = crypt
remote = minio:recordings
filename_encryption = off
directory_name_encryption = false
password = your-strong-password-here
password2 = your-salt-here
```

---

## 📊 Monitoreo

### Logs

```bash
# Ver logs en tiempo real
docker logs -f rclone-sync

# Ver últimos 100 logs
docker exec rclone-sync tail -n 100 /logs/backup.log

# Buscar errores
docker exec rclone-sync grep "ERROR" /logs/backup.log | tail -20
```

### Métricas

```bash
# Tamaño del bucket
docker exec minio mc du local/recordings

# Número de archivos
rclone --config /config/rclone/rclone.conf ls minio:recordings | wc -l

# Comparar local vs remoto
/scripts/health-check.sh
```

---

## 🔍 Troubleshooting

### Problema: MinIO no inicia

```bash
# Verificar logs
docker logs minio-dev

# Verificar puertos
docker ps | grep minio

# Reiniciar servicio
docker compose -f docker-compose.dev.yml restart minio
```

### Problema: Sync no funciona

```bash
# Verificar conexión a MinIO
docker exec rclone-sync rclone --config /config/rclone/rclone.conf lsd minio:

# Verificar bucket existe
docker exec rclone-sync rclone --config /config/rclone/rclone.conf ls minio:recordings

# Ejecutar sync manual con verbose
docker exec rclone-sync /manual-sync.sh --verbose --dry-run
```

### Problema: Archivos no se sincronizan

```bash
# Verificar permisos
docker exec rclone-sync ls -la /recordings

# Verificar exclusiones
# .tmp, .part, .DS_Store son excluidos por defecto

# Forzar sync completo
docker exec rclone-sync /manual-sync.sh --verbose
```

### Problema: Espacio en disco lleno

```bash
# Verificar uso de disco
df -h

# Configurar retención en MinIO (lifecycle policy)
# Esto se hace vía consola web de MinIO (puerto 9001)

# O manualmente:
docker exec minio mc ilm add local/recordings \
  --expiry-days "30"  # Borrar archivos después de 30 días
```

---

## 📈 Optimización para 150 Cámaras

Para un sistema con 150 cámaras y 3TB/día (~100TB/mes):

### 1. Aumentar Recursos

```yaml
# docker-compose.dev.yml
services:
  minio:
    deploy:
      resources:
        limits:
          cpus: '4.0'
          memory: 8G
    environment:
      MINIO_PROMETHEUS_AUTH_TYPE: public  # Para monitoreo
  
  rclone-sync:
    environment:
      SYNC_INTERVAL: 1800  # Cada 30 minutos (más frecuente)
```

### 2. Configurar Retención Automática

```bash
# Lifecycle policy: mantener solo últimos 30 días
docker exec minio mc ilm add local/recordings \
  --expiry-days "30"
```

### 3. Volumen Externo para MinIO

```yaml
# docker-compose.dev.yml
services:
  minio:
    volumes:
      - /mnt/nas-minio:/data  # Disco dedicado grande
```

---

## 🔐 Seguridad en Producción

### 1. Cambiar Credenciales Default

```yaml
environment:
  MINIO_ROOT_USER: cammanager-admin
  MINIO_ROOT_PASSWORD: "TuPasswordSeguroDe32Caracteres!"
```

### 2. Habilitar HTTPS

```yaml
services:
  minio:
    command: server /data --console-address ":9001" --certs-dir /certs
    volumes:
      - ./certs:/certs:ro
```

### 3. Firewall

```bash
# Solo permitir acceso local
ufw allow from 10.0.0.0/8 to any port 9000
ufw deny 9001  # Consola solo vía VPN
```

### 4. Backup del Bucket

```bash
# Mirror a otro bucket (DR)
rclone sync minio:recordings minio-dr:recordings-backup
```

---

## 📝 Cambios para Producción

Antes de usar en producción, modificar:

1. ✅ Cambiar credenciales default de MinIO
2. ✅ Configurar HTTPS/TLS
3. ✅ Ajustar `SYNC_INTERVAL` según necesidad
4. ✅ Configurar lifecycle policies para retención
5. ✅ Montar volumen externo grande para MinIO
6. ✅ Configurar backups adicionales del bucket
7. ✅ Habilitar monitoreo (Prometheus/Grafana)
8. ✅ Configurar alertas por email/Slack

---

## 🆚 Comparación: MinIO Local vs AWS S3

| Característica | MinIO Local | AWS S3 |
|----------------|-------------|--------|
| **Costo** | Solo hardware | ~$23/TB/mes |
| **Latencia** | < 1ms | 20-100ms |
| **Egress** | Gratis | $0.09/GB |
| **Setup** | Simple | Complejo |
| **Escalabilidad** | Limitada por hardware | Infinita |
| **Durabilidad** | Depende de RAID | 99.999999999% |

**Recomendación**: MinIO local para hot storage (30 días), AWS Glacier para cold storage (archival).

---

## 📚 Referencias

- [RClone Documentation](https://rclone.org/docs/)
- [MinIO Documentation](https://docs.min.io/)
- [Docker Compose Reference](https://docs.docker.com/compose/)

---

## ✅ Checklist de Verificación

Después de la instalación, verificar:

- [ ] MinIO accesible en http://localhost:9001
- [ ] Bucket `recordings` creado automáticamente
- [ ] Primer sync completado sin errores
- [ ] Health check pasa todas las validaciones
- [ ] Logs mostrando "Sincronización completada"
- [ ] Archivos visibles en consola MinIO
- [ ] Sync automático cada hora funcionando

---

## 🆘 Soporte

Si encuentras problemas:

1. Revisar logs: `docker logs rclone-sync`
2. Ejecutar health check: `docker exec rclone-sync /health-check.sh`
3. Verificar configuración: `docker exec rclone-sync cat /config/rclone/rclone.conf`
4. Sync manual de prueba: `docker exec rclone-sync /manual-sync.sh --dry-run`

---

*Sistema creado para CamManager - Febrero 2026*

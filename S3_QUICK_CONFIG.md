# Configuración Rápida - Sistema S3 Backup

## ⚙️ Configuración del Intervalo de Sincronización

El sistema sincroniza automáticamente cada **1 hora por defecto**. Esto es completamente configurable.

### Cambiar el Intervalo

Editar `docker-compose.dev.yml`:

```yaml
services:
  rclone-sync:
    environment:
      - SYNC_INTERVAL=3600  # ← Cambiar este valor
```

### Intervalos Comunes

| Intervalo | Segundos | Configuración |
|-----------|----------|---------------|
| **15 minutos** | 900 | `SYNC_INTERVAL=900` |
| **30 minutos** | 1800 | `SYNC_INTERVAL=1800` |
| **1 hora** (default) | 3600 | `SYNC_INTERVAL=3600` |
| **2 horas** | 7200 | `SYNC_INTERVAL=7200` |
| **6 horas** | 21600 | `SYNC_INTERVAL=21600` |
| **12 horas** | 43200 | `SYNC_INTERVAL=43200` |
| **24 horas** | 86400 | `SYNC_INTERVAL=86400` |

### Aplicar Cambios

```bash
# 1. Editar docker-compose.dev.yml
# 2. Reiniciar el contenedor
docker compose -f docker-compose.dev.yml restart rclone-sync

# 3. Verificar nueva configuración
docker logs rclone-sync | grep "Intervalo:"
```

---

## 🚀 Comandos Esenciales

### Verificar Estado del Sistema

```bash
# Health check completo
docker exec rclone-sync /health-check.sh

# Ver logs en tiempo real
docker logs -f rclone-sync

# Ver progreso del sync actual
docker exec rclone-sync tail -f /logs/backup.log
```

### Sincronización Manual

```bash
# Sync completo inmediato
docker exec rclone-sync /manual-sync.sh

# Simulación (ver qué se sincronizaría sin hacerlo)
docker exec rclone-sync /manual-sync.sh --dry-run

# Sync con verificación de integridad
docker exec rclone-sync /manual-sync.sh --verify
```

### Acceso a MinIO

```bash
# Consola Web
open http://localhost:9001

# Credenciales
Usuario: minioadmin
Password: minioadmin123
```

### Ver Archivos Sincronizados

```bash
# Listar archivos en S3
docker exec rclone-sync rclone ls minio:recordings | head -20

# Contar archivos totales
docker exec rclone-sync rclone ls minio:recordings | wc -l

# Ver tamaño total
docker exec rclone-sync rclone size minio:recordings
```

---

## 🔧 Configuración Avanzada

### Cambiar Número de Transferencias Paralelas

Editar `scripts/backup.sh`, línea ~195:

```bash
--transfers=4    # ← Cambiar a 8 para más velocidad
--checkers=8     # ← Cambiar a 16 para más verificaciones
```

### Habilitar Encriptación de Archivos

1. Editar `scripts/rclone/rclone.conf`:

```ini
[minio-crypt]
type = crypt
remote = minio:recordings
filename_encryption = standard
directory_name_encryption = true
password = your-strong-password-32-chars-min
password2 = your-salt-32-chars-min
```

2. Cambiar en `scripts/backup.sh` la línea del sync:

```bash
# Cambiar de:
"minio:${S3_BUCKET}"

# A:
"minio-crypt:${S3_BUCKET}"
```

### Excluir Ciertos Archivos

Editar `scripts/backup.sh`, añadir más exclusiones:

```bash
--exclude="*.tmp" \
--exclude="*.part" \
--exclude=".DS_Store" \
--exclude="*.log" \          # ← Nuevo
--exclude="temp/*" \         # ← Nuevo
```

---

## 📊 Monitoreo

### Logs Importantes

```bash
# Ver últimos errores
docker exec rclone-sync grep "ERROR" /logs/backup.log | tail -20

# Ver última sincronización
docker exec rclone-sync grep "Sincronización completada" /logs/backup.log | tail -5

# Ver estadísticas de transferencia
docker exec rclone-sync grep "GiB" /logs/backup.log | tail -10
```

### Métricas del Sistema

```bash
# Espacio usado en MinIO
docker exec minio-dev du -sh /data

# Archivos locales vs remotos
echo "Local:" && find /recordings -type f | wc -l
echo "S3:" && docker exec rclone-sync rclone ls minio:recordings | wc -l
```

---

## ⚠️ Troubleshooting

### Problema: Sync muy lento

```bash
# Aumentar transferencias paralelas
# En docker-compose.dev.yml o en scripts/backup.sh
--transfers=8      # De 4 a 8
--checkers=16      # De 8 a 16
```

### Problema: Contenedor reiniciando

```bash
# Ver logs de error
docker logs rclone-sync --tail 100

# Verificar MinIO está corriendo
docker ps | grep minio

# Reiniciar servicios
docker compose -f docker-compose.dev.yml restart minio rclone-sync
```

### Problema: Archivos no se sincronizan

```bash
# Verificar permisos
docker exec rclone-sync ls -la /recordings

# Ver archivos excluidos
docker exec rclone-sync find /recordings -name "*.tmp" -o -name "*.part"

# Forzar sync completo
docker exec rclone-sync /manual-sync.sh --verbose
```

---

## 🔐 Seguridad en Producción

### 1. Cambiar Credenciales MinIO

```yaml
# docker-compose.dev.yml
services:
  minio:
    environment:
      MINIO_ROOT_USER: secure-admin-user
      MINIO_ROOT_PASSWORD: "SuperSecurePassword123!@#"
  
  rclone-sync:
    environment:
      - S3_ACCESS_KEY=secure-admin-user
      - S3_SECRET_KEY=SuperSecurePassword123!@#
```

### 2. Restringir Acceso a Puerto MinIO

```bash
# Solo permitir localhost
# En docker-compose.dev.yml, cambiar:
ports:
  - "127.0.0.1:9000:9000"  # Solo localhost
  - "127.0.0.1:9001:9001"  # Solo localhost
```

### 3. Habilitar HTTPS/TLS

Ver documentación completa en `S3_BACKUP_README.md`

---

## ✅ Checklist Post-Instalación

- [ ] Sistema sincronizando correctamente (verificar logs)
- [ ] MinIO accesible en http://localhost:9001
- [ ] Bucket `recordings` creado
- [ ] Health check pasa sin errores
- [ ] Intervalo configurado según necesidad
- [ ] Credenciales cambiadas (producción)
- [ ] Backup schedule documentado
- [ ] Monitoreo configurado (opcional)

---

## 📈 Optimización para 150 Cámaras

Si tienes 150 cámaras (vs 17 actuales):

```yaml
# docker-compose.dev.yml
services:
  minio:
    deploy:
      resources:
        limits:
          cpus: '4.0'
          memory: 8G
    volumes:
      - /mnt/external-disk:/data  # Disco externo grande
  
  rclone-sync:
    environment:
      - SYNC_INTERVAL=1800  # 30 minutos (más frecuente)
    
    # En scripts/backup.sh cambiar:
    --transfers=16  # Más paralelo
    --checkers=32   # Más verificadores
```

---

## 🆘 Soporte Rápido

```bash
# Status completo
docker compose -f docker-compose.dev.yml ps

# Reiniciar todo
docker compose -f docker-compose.dev.yml restart

# Ver configuración activa
docker exec rclone-sync env | grep -E "(S3_|SYNC_)"

# Test de conectividad S3
docker exec rclone-sync rclone lsd minio:
```

---

*Última actualización: Febrero 2026*

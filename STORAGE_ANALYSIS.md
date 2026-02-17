# Análisis de Almacenamiento de Video Grabado
## CamManager - Sistema de Gestión de Cámaras

---

## 📊 Configuración Actual

### Setup Existente (MediaMTX)
```yaml
recordPath: /recordings/%path/%Y-%m-%d_%H-%M-%S
recordFormat: fmp4              # Fragmented MP4
recordPartDuration: 5s           # Fragmentos de 5 segundos
recordSegmentDuration: 1h        # Archivos de 1 hora
recordDeleteAfter: 24h         # Retención: 24 horas
```

### Métricas del Sistema
| Parámetro | Valor |
|-----------|-------|
| **Cámaras activas** | 17 |
| **Calidad estimada** | 1080p @ 15fps |
| **Bitrate por cámara** | ~2 Mbps |
| **Consumo por cámara** | ~900 MB/hora |
| **Consumo total** | ~15.3 GB/hora |
| **Consumo diario (24h)** | ~367 GB |
| **Almacenamiento actual** | Docker volume local |

---

## 💾 Opción 1: SSD Local (PC Host)

### Factibilidad: ✅ ALTA

#### Implementación
```yaml
# docker-compose.yml
volumes:
  recordings:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /mnt/ssd-recordings  # Ruta del SSD montado
```

#### Ventajas
| Aspecto | Beneficio |
|---------|-----------|
| **Costo inicial** | $0 (usa hardware existente) |
| **Latencia** | < 1ms - acceso inmediato |
| **Ancho de banda** | Saturación del bus SATA/NVMe |
| **Control total** | Sin dependencias de terceros |
| **Sin costos recurrentes** | Una vez comprado, es tuyo |
| **Offline capability** | Funciona sin internet |

#### Desventajas
| Aspecto | Limitación |
|---------|------------|
| **Capacidad limitada** | Depende del tamaño del SSD |
| **Sin redundancia** | Fallo de disco = pérdida total |
| **Sin backup automático** | Necesita sistema de backups manual |
| **Escalabilidad** | Máximo físico del disco |
| **Acceso remoto** | Requiere VPN o expuesto a internet |

#### Costos (SSD Recomendado)

| Capacidad | Modelo (ejemplo) | Precio USD | Duración estimada* |
|-----------|------------------|------------|-------------------|
| **1 TB** | Samsung 870 EVO | $85 | 2.7 días de grabación |
| **2 TB** | Samsung 870 EVO | $150 | 5.4 días de grabación |
| **4 TB** | Samsung 870 EVO | $280 | 10.9 días de grabación |
| **4 TB** | WD Red NAS HDD | $120 | 10.9 días (más lento) |
| **8 TB** | Seagate IronWolf | $180 | 21.8 días de grabación |
| **16 TB** | Seagate Exos | $280 | 43.6 días de grabación |

*Con retención de 24h y todas las cámaras grabando 24/7

#### Recomendación de Configuración
```yaml
# mediamtx.yml - Ajustado para SSD
paths:
  camera-name:
    record: yes
    recordPath: /recordings/%path/%Y-%m-%d_%H-%M-%S
    recordFormat: fmp4
    recordPartDuration: 5s
    recordSegmentDuration: 6h      # Archivos más grandes
    recordDeleteAfter: 168h        # 7 días de retención (ajustar según SSD)
```

---

## ☁️ Opción 2: AWS S3

### Factibilidad: ✅ MEDIA-ALTA

#### Arquitectura Propuesta
```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   MediaMTX      │────▶│  AWS S3 Upload   │────▶│   S3 Bucket     │
│   (recordings)  │     │  (rclone/s3fs)   │     │  + Glacier     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                              │
                              ▼
                        ┌──────────────────┐
                        │  S3 Lifecycle    │
                        │  - 30d Standard  │
                        │  - 90d Glacier   │
                        │  - 1yr Glacier   │
                        └──────────────────┘
```

#### Precios AWS S3 (N. Virginia - us-east-1)

| Servicio | Costo | Unidad |
|----------|-------|--------|
| **S3 Standard** | $0.023 | por GB/mes |
| **S3 Standard-IA** | $0.0125 | por GB/mes |
| **S3 Glacier Instant** | $0.004 | por GB/mes |
| **S3 Glacier Deep** | $0.00099 | por GB/mes |
| **PUT requests** | $0.005 | por 1,000 requests |
| **GET requests** | $0.0004 | por 1,000 requests |
| **Data Transfer OUT** | $0.09 | por GB (primeros 10TB/mes) |
| **Data Transfer IN** | $0.00 | Gratis |

#### Estimación de Costos Mensuales

**Escenario A: Retención 7 días (hot) + 90 días (glacier)**
```
Datos diarios: 367 GB
Retención hot (7 días): 367 × 7 = 2,569 GB = ~2.5 TB
Retención glacier (90 días): 367 × 90 = 33,030 GB = ~32 TB

S3 Standard (2.5 TB): $0.023 × 2569 = $59.09/mes
S3 Glacier (32 TB): $0.004 × 33030 = $132.12/mes
Requests (estimado): ~$5/mes
Data Transfer OUT (revisiones): ~$10/mes

TOTAL MENSUAL: ~$206 USD
```

**Escenario B: Retención 1 día (hot) + 30 días (glacier)**
```
S3 Standard (1 día): 367 GB × $0.023 = $8.44/mes
S3 Glacier (30 días): 11,010 GB × $0.004 = $44.04/mes

TOTAL MENSUAL: ~$58 USD
```

**Escenario C: Solo 7 días hot (sin glacier)**
```
S3 Standard: 2,569 GB × $0.023 = $59.09/mes

TOTAL MENSUAL: ~$75 USD (solo storage)
```

#### Ventajas
| Aspecto | Beneficio |
|---------|-----------|
| **Escalabilidad infinita** | Sin límite de capacidad |
| **Durabilidad 99.999999999%** | (11 nines) |
| **Backup automático** | Multi-AZ por defecto |
| **Acceso global** | Desde cualquier lugar |
| **Lifecycle policies** | Migración automática a capas frías |
| **Integración AWS** | CloudFront, Lambda, etc. |

#### Desventajas
| Aspecto | Limitación |
|---------|------------|
| **Costo recurrente** | Pago mensual permanente |
| **Data Transfer costs** | $0.09/GB para ver videos |
| **Latencia** | 50-200ms dependiendo de ubicación |
| **Dependencia de internet** | Sin conexión = sin acceso |
| **Complejidad** | Configuración IAM, buckets, policies |
| **Cold storage delay** | Glacier: 1-5 minutos para recuperar |

#### Implementación
```bash
# Opción A: rclone para sync continuo
rclone sync /recordings s3:my-bucket/recordings \
  --transfers 4 \
  --s3-storage-class STANDARD_IA

# Opción B: s3fs (mount directo)
s3fs my-bucket /recordings \
  -o iam_role=auto \
  -o use_cache=/tmp/s3cache
```

---

## ☁️ Opción 3: Google Cloud Storage

### Factibilidad: ✅ MEDIA-ALTA

#### Precios GCS (us-central1)

| Clase | Costo Storage | Costo Retrieval | Mínimo Retención |
|-------|---------------|-----------------|------------------|
| **Standard** | $0.020/GB | $0.00/GB | 0 días |
| **Nearline** | $0.010/GB | $0.01/GB | 30 días |
| **Coldline** | $0.004/GB | $0.02/GB | 90 días |
| **Archive** | $0.0012/GB | $0.05/GB | 365 días |
| **Operations** | $0.005/10k | - | - |
| **Network egress** | $0.12/GB | - | - |

#### Estimación de Costos Mensuales

**Escenario A: 7 días Standard + 90 días Coldline**
```
Standard (7 días): 2,569 GB × $0.020 = $51.38/mes
Coldline (90 días): 33,030 GB × $0.004 = $132.12/mes
Operations: ~$5/mes
Egress (10 GB/mes revisión): $1.20/mes

TOTAL MENSUAL: ~$190 USD
```

**Escenario B: 1 día Standard + 30 días Nearline**
```
Standard (1 día): 367 GB × $0.020 = $7.34/mes
Nearline (30 días): 11,010 GB × $0.010 = $110.10/mes

TOTAL MENSUAL: ~$123 USD
```

#### Comparación AWS vs GCS

| Característica | AWS S3 | GCS |
|---------------|--------|-----|
| Precio Standard | $0.023/GB | $0.020/GB ✅ |
| Precio Archive | $0.00099/GB ✅ | $0.0012/GB |
| Retrieval cost | Variable | Variable |
| Egress | $0.09/GB ✅ | $0.12/GB |
| Free tier | 5 GB/mes | 5 GB/mes |
| Transfer acceleration | ✅ Sí | ✅ Sí |
| Object lock | ✅ Sí | ✅ Sí |

#### Ventajas sobre AWS
- Precio Standard ligeramente más barato
- Mejor integración con Firebase/Google Workspace
- Transfer Appliance para migraciones masivas

---

## 🔧 Opción Híbrida Recomendada: SSD + Cloud Cold Storage

### Arquitectura de 3 Capas (Hot → Warm → Cold)

```
┌─────────────────────────────────────────────────────────────────┐
│                    CAPA 1: HOT (SSD Local)                        │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │  Últimos    │    │   Acceso    │    │  Baja       │          │
│  │   7 días    │◀──▶│  inmediato  │    │  latencia   │          │
│  │  ~2.5 TB    │    │   < 1ms     │    │             │          │
│  └─────────────┘    └─────────────┘    └─────────────┘          │
│        │                                                         │
│        │ Sync automático (rclone cron cada hora)                 │
│        ▼                                                         │
├─────────────────────────────────────────────────────────────────┤
│                  CAPA 2: WARM (S3 Standard-IA)                    │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │   8-30      │    │  Recuperación│    │  Costo      │          │
│  │   días      │◀──▶│  rápida     │    │  medio      │          │
│  │  ~11 TB    │    │             │    │             │          │
│  └─────────────┘    └─────────────┘    └─────────────┘          │
│        │                                                         │
│        │ Lifecycle policy (automático después de 30 días)        │
│        ▼                                                         │
├─────────────────────────────────────────────────────────────────┤
│                  CAPA 3: COLD (S3 Glacier Deep)                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │   30-365    │    │  Recuperación│    │  Costo      │          │
│  │   días      │◀──▶│  12-48 hrs  │    │  mínimo     │          │
│  │  ~120 TB   │    │             │    │  $0.00099/GB│          │
│  └─────────────┘    └─────────────┘    └─────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

### Costo Total del Sistema Híbrido (Mensual)

| Componente | Capacidad | Costo |
|------------|-----------|-------|
| **SSD 4 TB** (compra única) | 7 días hot | $280 (CAPEX) |
| **S3 Standard-IA** | 23 días warm | $0.0125 × 8,441 GB = **$106** |
| **S3 Glacier Deep** | 335 días cold | $0.00099 × 122,945 GB = **$122** |
| **Requests/Transfer** | - | **$20** |
| **TOTAL MENSUAL** | 365 días | **~$248 USD** |

---

## 📊 Tabla Comparativa de Opciones

| Criterio | SSD Local | AWS S3 | GCS | Híbrido (Recomendado) |
|----------|-----------|--------|-----|----------------------|
| **CAPEX inicial** | $85-280 | $0 | $0 | $280 (SSD) |
| **OPEX mensual** | $0 | $60-206 | $55-190 | ~$248 |
| **Retención máx** | Limitada por SSD | Ilimitada | Ilimitada | 1 año+ |
| **Acceso hot** | ⚡ Instantáneo | ⚡ Rápido | ⚡ Rápido | ⚡ Instantáneo |
| **Acceso cold** | ❌ N/A | 🐌 1-5 min | 🐌 1-5 min | 🐌 1-5 min |
| **Durabilidad** | ⚠️ Media | ✅ 99.999999999% | ✅ 99.999999999% | ✅ Alta |
| **Backup automático** | ❌ Manual | ✅ Sí | ✅ Sí | ✅ Parcial |
| **Escalabilidad** | ❌ Limitada | ✅ Infinita | ✅ Infinita | ✅ Infinita |
| **Offline** | ✅ Sí | ❌ No | ❌ No | ⚠️ Parcial |
| **Complejidad** | 🟢 Baja | 🟡 Media | 🟡 Media | 🟡 Media |
| **Compliance** | 🔴 Baja | 🟢 SOC2/HIPAA | 🟢 SOC2/HIPAA | 🟢 Alta |

---

## 🎯 Recomendación Final

### Para tu caso (17 cámaras, 367 GB/día):

**OPCIÓN RECOMENDADA: Sistema Híbrido SSD + S3 Glacier**

```
Inversión inicial: $280 (SSD 4TB)
Costo mensual: ~$248 USD
Retención: 1 año completo
Total primer año: $280 + ($248 × 12) = $3,256
```

### Por qué esta opción:
1. ✅ Acceso inmediato a últimos 7 días (investigaciones urgentes)
2. ✅ Costo predecible y controlable
3. ✅ Cumplimiento normativo (retención 1 año+)
4. ✅ Backup en cloud contra desastres locales
5. ✅ Escalable si agregas más cámaras

### Implementación Sugerida

**Fase 1: SSD Inmediato (Semana 1)**
```bash
# 1. Comprar SSD 4TB SATA
# 2. Instalar en PC host
# 3. Montar en /mnt/ssd-recordings
# 4. Actualizar docker-compose.yml

volumes:
  recordings:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /mnt/ssd-recordings
```

**Fase 2: AWS S3 Setup (Semana 2)**
```bash
# 1. Crear bucket en AWS S3
# 2. Configurar lifecycle policies
# 3. Instalar rclone en el servidor
# 4. Configurar sync automático cada hora

# Lifecycle Policy JSON
{
  "Rules": [
    {
      "ID": "MoveToStandardIA",
      "Status": "Enabled",
      "Filter": {"Prefix": "recordings/"},
      "Transitions": [
        {
          "Days": 30,
          "StorageClass": "STANDARD_IA"
        },
        {
          "Days": 90,
          "StorageClass": "GLACIER"
        },
        {
          "Days": 365,
          "StorageClass": "DEEP_ARCHIVE"
        }
      ]
    }
  ]
}
```

**Fase 3: Automatización (Semana 3)**
```bash
# Script de sync cada hora (cron)
0 * * * * rclone sync /mnt/ssd-recordings s3:my-bucket/recordings \
  --transfers 4 \
  --delete-after \
  --log-file=/var/log/rclone-sync.log
```

---

## 💰 Proyección de Costos a 3 Años

| Escenario | Año 1 | Año 2 | Año 3 | Total 3 Años |
|-----------|-------|-------|-------|--------------|
| **SSD Solo (4TB, 10 días)** | $280 | $0 | $0 | **$280** ⚠️ Pérdida de datos antiguos |
| **AWS S3 (7 días hot)** | $2,472 | $2,472 | $2,472 | **$7,416** |
| **Híbrido (1 año retención)** | $3,256 | $2,976 | $2,976 | **$9,208** ✅ Mejor valor |

---

## 📝 Próximos Pasos

### Acciones Inmediatas
1. [ ] Determinar requerimiento legal de retención (¿cuántos días/meses?)
2. [ ] Comprar SSD 4TB SATA ($280)
3. [ ] Configurar AWS account (free tier incluye 5GB/mes)
4. [ ] Probar sync con rclone a S3
5. [ ] Configurar lifecycle policies

### Presupuesto Sugerido 2024
```
Inversión inicial (SSD):     $280
AWS S3 (12 meses):         $2,976
Contingencia (20%):        $651
────────────────────────────────
TOTAL:                     $3,907 USD
```

---

## 📚 Recursos Adicionales

- [AWS S3 Pricing Calculator](https://calculator.aws/#/addService/S3)
- [Google Cloud Storage Pricing](https://cloud.google.com/storage/pricing)
- [rclone Documentation](https://rclone.org/s3/)
- [MediaMTX Recording Docs](https://github.com/bluenviron/mediamtx#recording)

---

*Documento generado el: Febrero 2026*
*Próxima revisión recomendada: Junio 2026 (revisar consumo real vs estimado)*

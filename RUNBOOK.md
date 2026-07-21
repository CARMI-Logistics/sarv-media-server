# Runbook — Despliegue del media server (E3)

Guía operativa para desplegar y operar `sarv-media-server` en una **VM de GCP**,
con **Cloud SQL** (BD), **Tailscale** (acceso a las cámaras) y **Caddy** (HTTPS
público). Solo el puerto **443** (y **80** para el reto ACME) queda expuesto.

```
Internet ──443──> Caddy ──┬─> mediamtx-backend:8080  (/auth, /jwks, /docs, /admin, /cameras)
                          └─> mediamtx:8888          (HLS)
mediamtx ──(Tailscale, pull RTSP)──> cámaras 10.0.0.x
mediamtx-backend ──> cloudsql-proxy:5432 ──> Cloud SQL
```

---

## 1. Provisión (una sola vez, en GCP)

1. **VM (Compute Engine)** con estas specs:
   - **Máquina:** `e2-standard-2` (2 vCPU / 8 GB) para arrancar. Sube a `e2-standard-4` si la CPU/latencia sufren con muchos consumidores. MediaMTX hace **remux** (copy-codec), que es barato en CPU; lo pesado es la **red** (streams entrando + HLS saliendo). *Nota:* si alguna cámara obligara a **transcodificar** (p. ej. H.265, que rompe HLS), la CPU se dispara y hará falta más vCPU.
   - **OS:** Debian 12 o Ubuntu 22.04 LTS.
   - **Disco boot:** 30–50 GB `pd-balanced` (SSD) — OS + Docker + imágenes. **No** se necesita disco de grabaciones: la grabación va **deshabilitada** (`record: no` en `mediamtx.yml`) para no llenar el disco.
   - **IP:** externa **estática** (para el registro DNS A).
   - **Región:** cercana a cámaras/consumidores (menos latencia y egress).
   - Instalar Docker + plugin compose y configurar **rotación de logs** (evita llenar el disco):
   ```bash
   echo '{ "log-driver":"json-file", "log-opts": {"max-size":"50m","max-file":"10"} }' | sudo tee /etc/docker/daemon.json
   sudo systemctl enable docker && sudo systemctl restart docker
   ```
2. **Service account de la VM** con rol **`Cloud SQL Client`** (para que el Auth Proxy use ADC sin archivo de clave).
3. **Cloud SQL (PostgreSQL 16)**: crear instancia, base de datos y usuario. Anotar el **`INSTANCE_CONNECTION_NAME`** (`project:region:instance`).
4. **DNS**: registro **A** de `media.carmi.com` → IP pública de la VM.
5. **Firewall**: solo **80 y 443** públicos; **SSH (22) NO público**.
   | Puerto | Origen | Para qué |
   |---|---|---|
   | 443 | `0.0.0.0/0` | HTTPS (video + API) |
   | 80  | `0.0.0.0/0` | reto ACME de Let's Encrypt (emisión/renovación del cert) + redirect http→https |
   | 22  | **NO público** → Tailscale / IAP | administración |

   > **80 no sirve para tráfico en claro:** lo usa Caddy para el reto HTTP-01 de Let's Encrypt (sin él no puede emitir/renovar el certificado) y para redirigir cualquier `http://` a `https://`.
6. **Endurecimiento de SSH:** **no abrir el 22 a internet** (es la mayor protección; cambiar el número de puerto es solo "seguridad por oscuridad" y no aporta control real). Accede por:
   - **Tailscale** (ya instalado en la VM): SSH contra la IP de la tailnet, o
   - **IAP de GCP**: `gcloud compute ssh <vm> --tunnel-through-iap`.
   - Solo **llaves SSH** (password auth deshabilitado; con OS Login viene así por defecto).
   Este mismo canal privado sirve para el túnel SSH al agente (`-L 8090:localhost:8090`).
7. **Tailscale en el HOST de la VM** (no como contenedor):
   ```bash
   curl -fsSL https://tailscale.com/install.sh | sh
   sudo tailscale up --accept-routes --authkey <TS_AUTHKEY>
   # Habilitar reenvío para que los contenedores alcancen la LAN de cámaras:
   echo 'net.ipv4.ip_forward=1' | sudo tee /etc/sysctl.d/99-tailscale.conf && sudo sysctl -p /etc/sysctl.d/99-tailscale.conf
   ```
   Verificar que la VM llega a una cámara: `ping 10.0.0.30` (o el rango que anuncie el nodo Tailscale de la LAN).

---

## 2. Configuración (en la VM)

```bash
git clone <repo> sarv-media-server && cd sarv-media-server

# Secretos del backend/compose (genera NUEVOS, distintos de dev):
cp .env.prod.example .env && chmod 600 .env
#   - CLOUDSQL_INSTANCE=project:region:instance
#   - DATABASE_URL=postgres://USUARIO:PASSWORD@cloudsql-proxy:5432/NOMBRE_BD
#   - DB_ENCRYPTION_KEY=$(openssl rand -base64 32)
#   - ADMIN_API_TOKEN=$(openssl rand -hex 32)
#   - CADDY_SITE_ADDRESS=media.carmi.com
#   - CORS_ALLOWED_ORIGINS=^https://app\.carmi\.com$

# Secretos del agente:
cp agent/.env.example agent/.env && chmod 600 agent/.env
#   - GEMINI_API_KEY=...  SLACK_WEBHOOK_URL=...  AGENT_API_TOKEN=$(openssl rand -hex 32)
#   (BACKEND_URL y ADMIN_API_TOKEN los inyecta el compose desde el .env raíz)

# Config de MediaMTX de prod (sin cámaras; solo global + pathDefaults + mosaic + regex).
# El mediamtx.yml está gitignored: cópialo a la VM manualmente (o parte de mediamtx.example.yml).
```

> No se necesita `config/clients.json` en prod: la autenticación vive en la BD.

---

## 3. Primer despliegue

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
```
Qué pasa al arrancar:
- El **esquema** se crea solo (migraciones sqlx al boot). Verificar: `docker compose logs mediamtx-backend | grep -i migrac`.
- **Caddy** obtiene el certificado Let's Encrypt para `media.carmi.com` (necesita DNS + 80/443 ya listos).

Verificación:
```bash
curl https://media.carmi.com/jwks                       # 200 (backend + TLS OK)
curl -H "Authorization: Bearer $ADMIN_API_TOKEN" \
     https://media.carmi.com/admin/cameras              # 200 (admin OK, lista vacía al inicio)
```

---

## 4. Carga inicial de datos (seeding)

Desde una máquina con acceso a la BD ORIGEN (con las cámaras) y al backend de prod:
```bash
SOURCE_DATABASE_URL=postgres://media_server:mediamtx@localhost:15432/media-server \
SOURCE_DB_ENCRYPTION_KEY=<clave-de-ORIGEN> \
TARGET_BACKEND_URL=https://media.carmi.com \
TARGET_ADMIN_TOKEN=<ADMIN_API_TOKEN-de-PROD> \
python3 scripts/seed_cameras.py
```
El destino **re-cifra** con su propia `DB_ENCRYPTION_KEY` y el reconciler sincroniza MediaMTX. Es idempotente (409 = omite).

**Proyectos** (crear con secretos nuevos de prod):
```bash
curl -X POST https://media.carmi.com/admin/projects \
  -H "Authorization: Bearer $ADMIN_API_TOKEN" -H 'Content-Type: application/json' \
  -d '{"client_id":"sigac","secret":"<secreto-nuevo>","all_cameras":true}'
```

---

## 5. Deploy de actualizaciones
```bash
git pull
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
```
Las migraciones nuevas se aplican solas al reiniciar el backend. Sin downtime de datos (la BD es Cloud SQL, externa).

## 6. Rollback
```bash
git checkout <commit-o-tag-anterior>
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d --build
```
> **Migraciones:** son *forward-only*. Un rollback de código NO revierte el esquema. Si una versión agregó columnas, la anterior debe seguir funcionando con ellas (evita cambios de esquema destructivos). Para revertir datos, restaurar un backup de Cloud SQL.

## 7. Operación
- **Estado:** `docker compose -f docker-compose.yml -f docker-compose.prod.yml ps`
- **Logs:** `docker compose ... logs -f mediamtx-backend` (o `mediamtx`, `caddy`, `stream-agent`).
- **Reinicio tras reboot de la VM:** los servicios llevan `restart: unless-stopped`; asegúrate de que Docker arranca al boot (`sudo systemctl enable docker`).
- **Certificados:** Caddy los renueva solo (persisten en el volumen `caddy-data`).
- **Cámaras caídas / diagnóstico:** el agente escribe en el historial (`GET /admin/failures?camera=<path>`).
- **Disparar el agente a mano:** el agente NO es público (Caddy no lo rutea); queda en el loopback de la VM (`127.0.0.1:8090`). Para llamarlo desde tu máquina, túnel SSH:
  ```bash
  ssh -L 8090:localhost:8090 usuario@vm-media
  # en tu local (usa AGENT_API_TOKEN de agent/.env):
  curl -H "Authorization: Bearer $AGENT_API_TOKEN" http://localhost:8090/diagnose   # diagnóstico IA (GET, solo lectura)
  curl -X POST -H "Authorization: Bearer $AGENT_API_TOKEN" http://localhost:8090/run # pipeline + notifica
  ```
  De todas formas el scheduler corre solo cada `SCAN_INTERVAL_SECONDS` (default 300s; `<=0` lo desactiva).
- **Rotación de secretos:** editar `.env`/`agent/.env` y `up -d`. Rotar `DB_ENCRYPTION_KEY` implica re-cifrar las URLs (re-seeding).

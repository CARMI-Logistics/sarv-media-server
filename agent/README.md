# stream-agent

Agente de diagnóstico de cámaras del `sarv-media-server` — **Fase 1: solo lectura**.

Lee el estado (API MediaMTX `9997`) y los logs, **clasifica** la salud de cada
cámara con un motor de reglas (playbook), genera un **diagnóstico** en lenguaje
claro con soluciones vía **Gemini** (LangGraph) y **notifica** por un canal
(Slack). Se ejecuta bajo demanda o de forma automática. **No realiza ningún
cambio** en el servidor ni en las cámaras.

## Estructura (puertos y adaptadores)

```
agent/
  pyproject.toml     # dependencias (fuente única) + extras: ia, sched
  Dockerfile
  .env.example
  app/
    main.py          # FastAPI (endpoints) + lifespan (scheduler)
    config.py        # settings (pydantic-settings)
    deps.py          # composition root (inyección de dependencias)
    graph.py         # LangGraph: collect → classify → diagnose
    state.py         # estado del grafo
    domain/          # modelos puros + ports (Protocols): sin I/O
    infra/           # adaptadores: mediamtx_api, log_reader, gemini, slack, scheduler, ...
    rules/           # motor de reglas del playbook (OCP)
    security/        # redactor de credenciales + autorización
    services/        # casos de uso: collector, classifier, diagnoser, notify, runner
  tests/
```

> Diseño completo en `plans/PLAN-sarv-stream-agent.md`.

## Endpoints

| Método | Ruta | Auth | Descripción |
|--------|------|:----:|-------------|
| GET | `/health` | — | Liveness (healthcheck). |
| GET | `/` | — | Info básica del agente. |
| GET | `/cameras` | 🔒 | Estado consolidado por cámara (crudo). |
| GET | `/report` | 🔒 | Veredicto de salud (reglas, sin IA). |
| GET | `/diagnose` | 🔒 | Diagnóstico completo con IA (grafo). |
| POST | `/run` | 🔒 | Ejecuta el pipeline y **notifica** (on-demand). |

🔒 = requiere `Authorization: Bearer <AGENT_API_TOKEN>`.

## Ejecución

Como parte del stack (recomendado):

```bash
docker compose up --build stream-agent
```

Solo el agente, aislado:

```bash
docker build -t stream-agent ./agent
docker run --rm -p 127.0.0.1:8090:8090 -e AGENT_API_TOKEN=xxx stream-agent
curl http://127.0.0.1:8090/health
curl -H "Authorization: Bearer xxx" http://127.0.0.1:8090/report
```

Tests:

```bash
docker run --rm -v "$PWD/agent:/app" -w /app stream-agent \
  sh -c "pip install -q pytest && python -m pytest -q"
```

## Configuración

Ver `.env.example`. En el `docker-compose.yml`, la config no sensible (URLs
internas) va en `environment`; los **secretos** se ponen en `agent/.env`
(cargado con `env_file`, `required: false`).

| Variable | Uso |
|---|---|
| `AGENT_API_TOKEN` | token de los endpoints de diagnóstico (sin él → 503) |
| `GEMINI_API_KEY` / `GEMINI_MODEL` | IA de diagnóstico (modelo por defecto `gemini-2.5-flash`) |
| `NOTIFY_CHANNEL` / `SLACK_WEBHOOK_URL` | canal de notificación (sin webhook → desactivada) |
| `SCAN_INTERVAL_SECONDS` | ejecución automática (`0` = desactivada) |
| `MEDIAMTX_API_URL` / `MEDIAMTX_LOG_PATH` | fuentes de datos |

## Seguridad (HU 1.3)

- **Solo observa:** el agente nunca modifica el media server. El cliente de la
  API solo tiene métodos de lectura (invariante cubierto por test).
- **No accesible desde internet:** publicado solo en `127.0.0.1:8090`.
- **Autorización:** los endpoints de diagnóstico exigen `AGENT_API_TOKEN`
  (fail-closed: sin token configurado responden 503).
- **Redacción:** las credenciales (URLs RTSP, tokens) se ocultan **antes** de
  salir a servicios externos (Gemini, Slack) y en las respuestas HTTP.
- **Egress controlado:** el agente solo habla con MediaMTX (interno), la API de
  Gemini y el webhook de Slack. Nada más.
- **Secretos:** viven solo en `agent/.env` (gitignored); no se escriben en logs.
```

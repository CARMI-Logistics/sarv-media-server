"""stream-agent — servicio base (HU 1.1).

Fase 1: agente de diagnóstico SOLO LECTURA. Este módulo expone únicamente el
esqueleto del servicio (salud). Las etapas de recolección, clasificación,
diagnóstico IA y notificación se añaden en tareas posteriores del sprint.
"""

import logging
from contextlib import asynccontextmanager
from dataclasses import asdict
from typing import Any

from fastapi import Depends, FastAPI, HTTPException

from app.config import get_settings
from app.deps import (
    get_agent_graph,
    get_agent_runner,
    get_collector,
    get_health_report_service,
    get_redactor,
)
from app.domain.ports import Redactor
from app.infra.gemini import LlmNotConfigured
from app.infra.mediamtx_api import MediaMtxUnavailable
from app.infra.scheduler import AgentScheduler
from app.security.auth import require_auth
from app.services.collector import StateCollector
from app.services.health_report import HealthReportService
from app.services.runner import AgentRunner

settings = get_settings()

logging.basicConfig(level=settings.log_level.upper())
logger = logging.getLogger(settings.service_name)


@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info(
        "%s iniciado (fase 1, solo lectura) — MediaMTX API: %s",
        settings.service_name,
        settings.mediamtx_api_url,
    )
    scheduler = AgentScheduler(get_agent_runner(), settings.scan_interval_seconds)
    scheduler.start()
    try:
        yield
    finally:
        scheduler.shutdown()


app = FastAPI(
    title="sarv stream-agent",
    description="Agente de diagnóstico de cámaras (Fase 1, solo lectura).",
    version="0.1.0",
    lifespan=lifespan,
)


@app.get("/health")
def health() -> dict:
    """Liveness: confirma que el servicio está arriba."""
    return {
        "status": "ok",
        "service": settings.service_name,
        "version": app.version,
    }


@app.get("/")
def root() -> dict:
    """Información básica del agente."""
    return {
        "service": settings.service_name,
        "phase": 1,
        "mode": "read-only",
        "mediamtx_api": settings.mediamtx_api_url,
    }


@app.get("/cameras", dependencies=[Depends(require_auth)])
async def cameras(
    collector: StateCollector = Depends(get_collector),
    redactor: Redactor = Depends(get_redactor),
) -> object:
    """Estado consolidado por cámara (solo lectura).

    Reúne estado (API 9997) + logs recientes. Es la base de la clasificación.
    Se redactan credenciales de los logs (defensa en profundidad, HU 1.3).
    """
    try:
        snapshots = await collector.collect()
    except MediaMtxUnavailable as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc
    payload = {"count": len(snapshots), "cameras": [asdict(s) for s in snapshots]}
    return redactor.redact_obj(payload)


@app.get("/report", dependencies=[Depends(require_auth)])
async def report(
    service: HealthReportService = Depends(get_health_report_service),
    redactor: Redactor = Depends(get_redactor),
) -> object:
    """Veredicto de salud por cámara (estado + hallazgos). Solo lectura.

    Corre el pipeline recolección → clasificación (motor de reglas del playbook,
    sin IA todavía). Se redactan credenciales de la evidencia (HU 1.3).
    """
    try:
        classifications = await service.run()
    except MediaMtxUnavailable as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc

    summary: dict[str, int] = {}
    for classification in classifications:
        key = classification.status.value
        summary[key] = summary.get(key, 0) + 1

    payload = {
        "total": len(classifications),
        "summary": summary,
        "cameras": [asdict(c) for c in classifications],
    }
    return redactor.redact_obj(payload)


@app.get("/diagnose", dependencies=[Depends(require_auth)])
async def diagnose(
    graph: Any = Depends(get_agent_graph),
    redactor: Redactor = Depends(get_redactor),
) -> object:
    """Diagnóstico completo: recolección → clasificación → explicación IA.

    Ejecuta el grafo de LangGraph. Solo lectura. Requiere GEMINI_API_KEY para la
    etapa de IA; la salida se redacta (HU 1.3).
    """
    try:
        result = await graph.ainvoke({})
    except MediaMtxUnavailable as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc
    except LlmNotConfigured as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc

    diagnoses = result.get("diagnoses", [])
    payload = {
        "total": len(diagnoses),
        "diagnoses": [asdict(d) for d in diagnoses],
    }
    return redactor.redact_obj(payload)


@app.post("/run", dependencies=[Depends(require_auth)])
async def run(runner: AgentRunner = Depends(get_agent_runner)) -> dict:
    """Ejecución bajo demanda: corre el pipeline completo y notifica.

    Es POST porque tiene efecto (puede enviar la notificación). El scheduler
    ejecuta este mismo runner de forma automática.
    """
    try:
        result = await runner.run_and_notify()
    except MediaMtxUnavailable as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc
    except LlmNotConfigured as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc

    return {
        "total": result.total,
        "problems": result.problems,
        "notified": result.notified,
    }

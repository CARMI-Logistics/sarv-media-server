"""Composition root: construye e inyecta las dependencias concretas.

Es el ÚNICO lugar que conoce a la vez los puertos y sus implementaciones
concretas (DIP). El resto del código depende solo de abstracciones. FastAPI
resuelve estas factorías vía Depends.
"""

from __future__ import annotations

from functools import lru_cache

from app.config import get_settings
from app.domain.ports import FailureHistory, Notifier
from app.graph import build_graph
from app.infra.failure_history_api import FailureHistoryApi
from app.infra.gemini import GeminiClient
from app.infra.log_reader import FileLogReader
from app.infra.mediamtx_api import MediaMtxApiClient
from app.infra.null_failure_history import NullFailureHistory
from app.infra.null_notifier import NullNotifier
from app.infra.slack import SlackNotifier
from app.rules import default_rules
from app.security.redactor import RegexRedactor
from app.services.classifier import HealthClassifier
from app.services.collector import StateCollector
from app.services.diagnoser import DiagnoserService
from app.services.health_report import HealthReportService
from app.services.notify import NotificationService
from app.services.reporter import ReportFormatter
from app.services.runner import AgentRunner


@lru_cache
def get_collector() -> StateCollector:
    settings = get_settings()
    # MediaMtxApiClient satisface a la vez PathsSource y HlsMuxersSource (ISP).
    api = MediaMtxApiClient(settings.mediamtx_api_url)
    log_reader = FileLogReader(settings.mediamtx_log_path)
    return StateCollector(
        paths_source=api,
        hls_source=api,
        log_source=log_reader,
    )


@lru_cache
def get_classifier() -> HealthClassifier:
    return HealthClassifier(default_rules())


@lru_cache
def get_redactor() -> RegexRedactor:
    return RegexRedactor()


@lru_cache
def get_health_report_service() -> HealthReportService:
    return HealthReportService(get_collector(), get_classifier())


@lru_cache
def get_llm_client() -> GeminiClient:
    settings = get_settings()
    return GeminiClient(settings.gemini_api_key, settings.gemini_model)


@lru_cache
def get_diagnoser() -> DiagnoserService:
    return DiagnoserService(get_llm_client(), get_redactor())


@lru_cache
def get_agent_graph():
    """Grafo LangGraph compilado (collect → classify → diagnose)."""
    return build_graph(get_collector(), get_classifier(), get_diagnoser())


@lru_cache
def get_notifier() -> Notifier:
    settings = get_settings()
    # Slack solo si hay webhook; en cualquier otro caso, canal deshabilitado.
    if settings.notify_channel == "slack" and settings.slack_webhook_url:
        return SlackNotifier(settings.slack_webhook_url)
    return NullNotifier()


@lru_cache
def get_notification_service() -> NotificationService:
    return NotificationService(ReportFormatter(), get_notifier(), get_redactor())


@lru_cache
def get_failure_history() -> FailureHistory:
    settings = get_settings()
    # Con token configurado, persiste en el backend; si no, historial deshabilitado.
    if settings.admin_api_token:
        return FailureHistoryApi(settings.backend_url, settings.admin_api_token)
    return NullFailureHistory()


@lru_cache
def get_agent_runner() -> AgentRunner:
    return AgentRunner(
        get_agent_graph(),
        get_notification_service(),
        get_failure_history(),
        get_redactor(),
    )

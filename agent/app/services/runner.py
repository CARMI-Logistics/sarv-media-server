"""Runner del agente: corre el pipeline completo, persiste y notifica.

Punto único de ejecución reutilizado por el disparo on-demand (POST /run) y por
el scheduler (ejecución automática). Orquesta grafo → historial → notificación.

Deduplicación on-change (HU 4.6): solo se registra y se notifica lo que cambió
respecto al último estado guardado, evitando alertas repetidas. La persistencia
es best-effort: un fallo del historial se registra pero no aborta la corrida.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, replace

from app.domain.models import Diagnosis, HealthStatus
from app.domain.ports import FailureHistory, Redactor
from app.services.notify import NotificationService

logger = logging.getLogger(__name__)


@dataclass(frozen=True)
class RunResult:
    total: int
    problems: int
    notified: bool


class AgentRunner:
    def __init__(
        self,
        graph,
        notification_service: NotificationService,
        history: FailureHistory,
        redactor: Redactor,
    ) -> None:
        self._graph = graph
        self._notification = notification_service
        self._history = history
        self._redactor = redactor

    async def run_and_notify(self) -> RunResult:
        result = await self._graph.ainvoke({})
        diagnoses = result.get("diagnoses", [])

        # Dedup on-change: quedarnos con lo que difiere del historial.
        changed = [d for d in diagnoses if await self._changed(d)]

        # Persistir (redactado) los cambios; best-effort, no aborta la corrida.
        for diagnosis in changed:
            try:
                await self._history.record(self._redact(diagnosis))
            except Exception as exc:  # noqa: BLE001 (best-effort)
                logger.warning("no se pudo registrar '%s': %s", diagnosis.name, exc)

        problems = sum(1 for d in diagnoses if d.status is not HealthStatus.OK)
        # Notificar SOLO lo que cambió → sin alertas repetidas.
        notify_result = await self._notification.notify(changed)
        return RunResult(
            total=len(diagnoses),
            problems=problems,
            notified=notify_result.sent,
        )

    async def _changed(self, diagnosis: Diagnosis) -> bool:
        try:
            return await self._history.has_changed(diagnosis)
        except Exception as exc:  # noqa: BLE001 (best-effort: no perder alertas)
            logger.warning(
                "historial no disponible para '%s' (%s); se trata como cambiado",
                diagnosis.name,
                exc,
            )
            return True

    def _redact(self, diagnosis: Diagnosis) -> Diagnosis:
        """Copia del diagnóstico con texto redactado antes de salir al backend."""
        return replace(
            diagnosis,
            explanation=self._redactor.redact(diagnosis.explanation),
            suggested_solutions=tuple(
                self._redactor.redact(s) for s in diagnosis.suggested_solutions
            ),
        )

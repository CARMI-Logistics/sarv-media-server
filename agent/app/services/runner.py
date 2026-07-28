"""Runner del agente: corre el pipeline completo, persiste y notifica.

Punto único de ejecución reutilizado por el disparo on-demand (POST /run) y por
el scheduler (ejecución automática). Orquesta grafo → historial → notificación.

Deduplicación on-change (HU 4.6): solo se registra y se notifica lo que cambió
respecto al último estado guardado, evitando alertas repetidas. Se notifica
tanto una caída (ALERT) como una recuperación (RECOVERY). La persistencia es
best-effort: un fallo del historial se registra pero no aborta la corrida.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, replace

from app.domain.models import Diagnosis, HealthStatus, Transition
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

        # Quedarnos con las transiciones que valen la pena: caídas (ALERT) y
        # recuperaciones (RECOVERY). Lo que no cambió (UNCHANGED) se ignora.
        reportable = [
            d for d in diagnoses
            if await self._classify(d) in (Transition.ALERT, Transition.RECOVERY)
        ]

        # Persistir (redactado) lo reportado; best-effort, no aborta la corrida.
        # Registrar la recuperación fija el nuevo baseline OK.
        for diagnosis in reportable:
            try:
                await self._history.record(self._redact(diagnosis))
            except Exception as exc:  # noqa: BLE001 (best-effort)
                logger.warning("no se pudo registrar '%s': %s", diagnosis.name, exc)

        problems = sum(1 for d in diagnoses if d.status is not HealthStatus.OK)
        # Notificar solo las transiciones → sin alertas repetidas.
        notify_result = await self._notification.notify(reportable)
        return RunResult(
            total=len(diagnoses),
            problems=problems,
            notified=notify_result.sent,
        )

    async def _classify(self, diagnosis: Diagnosis) -> Transition:
        try:
            return await self._history.classify(diagnosis)
        except Exception as exc:  # noqa: BLE001 (best-effort: no perder alertas)
            logger.warning(
                "historial no disponible para '%s' (%s); se alertan problemas",
                diagnosis.name,
                exc,
            )
            # Sin historial: alertar problemas (no perderlos); OK sin baseline no
            # es recuperación.
            if diagnosis.status is HealthStatus.OK:
                return Transition.UNCHANGED
            return Transition.ALERT

    def _redact(self, diagnosis: Diagnosis) -> Diagnosis:
        """Copia del diagnóstico con texto redactado antes de salir al backend."""
        return replace(
            diagnosis,
            explanation=self._redactor.redact(diagnosis.explanation),
            suggested_solutions=tuple(
                self._redactor.redact(s) for s in diagnosis.suggested_solutions
            ),
        )

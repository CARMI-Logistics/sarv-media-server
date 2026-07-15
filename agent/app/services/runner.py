"""Runner del agente: corre el pipeline completo y notifica.

Punto único de ejecución reutilizado por el disparo on-demand (POST /run) y por
el scheduler (ejecución automática). Orquesta grafo → notificación; no contiene
lógica de negocio propia.
"""

from __future__ import annotations

from dataclasses import dataclass

from app.domain.models import HealthStatus
from app.services.notify import NotificationService


@dataclass(frozen=True)
class RunResult:
    total: int
    problems: int
    notified: bool


class AgentRunner:
    def __init__(self, graph, notification_service: NotificationService) -> None:
        self._graph = graph
        self._notification = notification_service

    async def run_and_notify(self) -> RunResult:
        result = await self._graph.ainvoke({})
        diagnoses = result.get("diagnoses", [])
        problems = sum(1 for d in diagnoses if d.status is not HealthStatus.OK)
        notify_result = await self._notification.notify(diagnoses)
        return RunResult(
            total=len(diagnoses),
            problems=problems,
            notified=notify_result.sent,
        )

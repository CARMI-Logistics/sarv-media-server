"""Caso de uso: reporte de salud (recolección → clasificación).

Orquesta el pipeline de la Fase 1 hasta la clasificación. Mantiene los
endpoints (y luego el scheduler) delgados: la orquestación vive aquí, no en el
controlador. En tareas siguientes este pipeline se extiende con diagnóstico IA
y notificación.
"""

from __future__ import annotations

from app.domain.models import CameraClassification
from app.services.classifier import HealthClassifier
from app.services.collector import StateCollector


class HealthReportService:
    def __init__(self, collector: StateCollector, classifier: HealthClassifier) -> None:
        self._collector = collector
        self._classifier = classifier

    async def run(self) -> list[CameraClassification]:
        snapshots = await self._collector.collect()
        return self._classifier.classify_all(snapshots)

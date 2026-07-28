"""Historial deshabilitado (Null Object, HU 4.6).

Se usa cuando el backend/token no están configurados: no persiste nada. Sin
baseline no se puede saber si una cámara "se recuperó", así que solo alerta los
problemas (para no perderlos) y trata los OK como sin cambio (degradación
elegante, igual que NullNotifier).
"""

from __future__ import annotations

from app.domain.models import Diagnosis, HealthStatus, Transition


class NullFailureHistory:
    async def record(self, diagnosis: Diagnosis) -> None:  # noqa: ARG002
        return None

    async def classify(self, diagnosis: Diagnosis) -> Transition:
        if diagnosis.status is HealthStatus.OK:
            return Transition.UNCHANGED  # sin baseline no hay "recuperación"
        return Transition.ALERT  # problema -> alertar (no perderlo)

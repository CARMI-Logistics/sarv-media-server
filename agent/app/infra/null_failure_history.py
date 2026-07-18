"""Historial deshabilitado (Null Object, HU 4.6).

Se usa cuando el backend/token no están configurados: no persiste nada y trata
todo como "cambiado", preservando el comportamiento previo de notificar en cada
corrida (degradación elegante, igual que NullNotifier).
"""

from __future__ import annotations

from app.domain.models import Diagnosis


class NullFailureHistory:
    async def record(self, diagnosis: Diagnosis) -> None:  # noqa: ARG002
        return None

    async def has_changed(self, diagnosis: Diagnosis) -> bool:  # noqa: ARG002
        return True

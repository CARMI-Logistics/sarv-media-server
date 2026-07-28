"""Servicio de notificación: formatea el reporte y lo envía por el canal.

Depende de los puertos Notifier y Redactor (DIP). 🔴 Redacta el mensaje ANTES
de enviarlo (el canal es externo). Recibe ya filtradas las cámaras que ameritan
aviso (caídas y recuperaciones); si la lista está vacía, no envía nada.
"""

from __future__ import annotations

from dataclasses import dataclass

from app.domain.models import Diagnosis
from app.domain.ports import Notifier, Redactor
from app.services.reporter import ReportFormatter


@dataclass(frozen=True)
class NotifyResult:
    sent: bool
    message: str


class NotificationService:
    def __init__(
        self,
        formatter: ReportFormatter,
        notifier: Notifier,
        redactor: Redactor,
    ) -> None:
        self._formatter = formatter
        self._notifier = notifier
        self._redactor = redactor

    async def notify(self, diagnoses: list[Diagnosis]) -> NotifyResult:
        # Sin cambios que reportar -> no se envía (anti-ruido).
        if not diagnoses:
            return NotifyResult(sent=False, message="")

        message = self._formatter.format(diagnoses)
        safe_message = self._redactor.redact(message)  # 🔴 antes de salir al canal
        await self._notifier.send(safe_message)
        return NotifyResult(sent=True, message=safe_message)

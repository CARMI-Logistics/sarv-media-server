"""Servicio de notificación: formatea el reporte y lo envía por el canal.

Depende de los puertos Notifier y Redactor (DIP). 🔴 Redacta el mensaje ANTES
de enviarlo (Slack es un servicio externo). Por defecto solo notifica cuando hay
problemas, para no generar ruido.
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
        only_on_problems: bool = True,
    ) -> None:
        self._formatter = formatter
        self._notifier = notifier
        self._redactor = redactor
        self._only_on_problems = only_on_problems

    async def notify(self, diagnoses: list[Diagnosis]) -> NotifyResult:
        message = self._formatter.format(diagnoses)

        if self._only_on_problems and not self._formatter.has_problems(diagnoses):
            return NotifyResult(sent=False, message=message)

        safe_message = self._redactor.redact(message)  # 🔴 antes de salir a Slack
        await self._notifier.send(safe_message)
        return NotifyResult(sent=True, message=safe_message)

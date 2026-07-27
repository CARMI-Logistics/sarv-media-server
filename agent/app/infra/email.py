"""Adaptador de notificación por Email (endpoint SendGrid del servicio de correo).

Implementa el puerto Notifier. El endpoint envía a UN destinatario por llamada,
así que iteramos la lista de destinatarios. Es *best-effort*: un correo que
falle se registra pero no impide los demás; solo se relanza si fallan TODOS
(canal roto). El asunto se toma de la primera línea del mensaje (el resumen del
reporte); el cuerpo va como texto plano.
"""

from __future__ import annotations

import logging

import httpx

logger = logging.getLogger("stream-agent")

_DEFAULT_SUBJECT = "Reporte de cámaras (stream-agent)"


class NotifierError(RuntimeError):
    """Fallo al enviar la notificación por email."""


class EmailNotifier:
    def __init__(
        self,
        api_url: str,
        recipients: list[str],
        timeout: float = 5.0,
        transport: httpx.AsyncBaseTransport | None = None,
    ) -> None:
        self._api_url = api_url
        self._recipients = recipients
        self._timeout = timeout
        # Inyectable solo para pruebas (httpx.MockTransport); en prod queda None.
        self._transport = transport

    async def send(self, message: str) -> None:
        subject = self._subject_from(message)
        errors: list[str] = []

        async with httpx.AsyncClient(
            timeout=self._timeout, transport=self._transport
        ) as client:
            for recipient in self._recipients:
                try:
                    response = await client.post(
                        self._api_url,
                        json={"to": recipient, "subject": subject, "text": message},
                    )
                    response.raise_for_status()
                except httpx.HTTPError as exc:
                    errors.append(f"{recipient}: {exc}")
                    logger.warning("Fallo enviando email a %s: %s", recipient, exc)

        # Se relanza solo si NINGÚN destinatario recibió (canal roto); un fallo
        # parcial queda logueado pero no aborta la corrida.
        if errors and len(errors) == len(self._recipients):
            raise NotifierError(f"Fallo enviando email a todos: {'; '.join(errors)}")

    @staticmethod
    def _subject_from(message: str) -> str:
        """Primera línea no vacía del mensaje (el resumen del reporte)."""
        for line in message.splitlines():
            if line.strip():
                return line.strip()
        return _DEFAULT_SUBJECT

"""Adaptador de notificación por Slack (Incoming Webhook).

Implementa el puerto Notifier. Única responsabilidad: enviar un mensaje al
webhook de Slack.
"""

from __future__ import annotations

import httpx


class NotifierError(RuntimeError):
    """Fallo al enviar la notificación."""


class SlackNotifier:
    def __init__(self, webhook_url: str, timeout: float = 5.0) -> None:
        self._webhook_url = webhook_url
        self._timeout = timeout

    async def send(self, message: str) -> None:
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                response = await client.post(self._webhook_url, json={"text": message})
                response.raise_for_status()
        except httpx.HTTPError as exc:
            raise NotifierError(f"Fallo enviando a Slack: {exc}") from exc

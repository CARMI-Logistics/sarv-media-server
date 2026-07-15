"""Notificador deshabilitado (canal apagado).

Implementa el puerto Notifier sin enviar nada. Es el default cuando no hay canal
configurado, y permite "desactivar" la notificación (criterio de HU 1.2).
"""

from __future__ import annotations

import logging

logger = logging.getLogger("stream-agent")


class NullNotifier:
    async def send(self, message: str) -> None:
        logger.info("Notificación deshabilitada (NullNotifier): mensaje no enviado.")

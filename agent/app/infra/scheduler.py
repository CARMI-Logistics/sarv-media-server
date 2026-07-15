"""Ejecución automática del agente vía APScheduler (AsyncIO).

Dispara el AgentRunner cada N segundos. El job atrapa cualquier error (MediaMTX
o Gemini caídos) para que una corrida fallida NO tumbe el scheduler.
Si el intervalo es <= 0, queda desactivado.
"""

from __future__ import annotations

import logging

from apscheduler.schedulers.asyncio import AsyncIOScheduler

from app.services.runner import AgentRunner

logger = logging.getLogger("stream-agent")


class AgentScheduler:
    def __init__(self, runner: AgentRunner, interval_seconds: int) -> None:
        self._runner = runner
        self._interval = interval_seconds
        self._scheduler = AsyncIOScheduler()

    def start(self) -> None:
        if self._interval <= 0:
            logger.info("Scheduler desactivado (SCAN_INTERVAL_SECONDS <= 0).")
            return
        self._scheduler.add_job(
            self._job,
            trigger="interval",
            seconds=self._interval,
            id="diagnose",
            max_instances=1,
            coalesce=True,
        )
        self._scheduler.start()
        logger.info("Scheduler activo: diagnóstico automático cada %ss.", self._interval)

    def shutdown(self) -> None:
        if self._scheduler.running:
            self._scheduler.shutdown(wait=False)

    async def _job(self) -> None:
        try:
            result = await self._runner.run_and_notify()
            logger.info(
                "Corrida automática: %s cámaras, %s con problema, notificado=%s",
                result.total,
                result.problems,
                result.notified,
            )
        except Exception as exc:  # noqa: BLE001 - resiliencia: nunca tumbar el scheduler
            logger.warning("Corrida automática falló (se reintenta al próximo ciclo): %s", exc)

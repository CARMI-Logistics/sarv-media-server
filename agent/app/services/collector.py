"""Servicio de recolección: consolida el estado de cada cámara.

Orquesta las fuentes (paths, muxers HLS, logs) y produce un CameraSnapshot por
cámara. Depende de los PUERTOS del dominio (DIP), no de adaptadores concretos:
así se prueba con fuentes falsas (LSP) y se pueden añadir fuentes nuevas sin
tocar esta clase (OCP).

Aquí vive la ÚNICA responsabilidad de consolidar/correlacionar; la correlación
log↔cámara se hace por el nombre de path, que solo conoce este servicio.
"""

from __future__ import annotations

from datetime import datetime, timezone

from app.domain.models import CameraSnapshot, LogEvent
from app.domain.ports import HlsMuxersSource, LogSource, PathsSource


class StateCollector:
    def __init__(
        self,
        paths_source: PathsSource,
        hls_source: HlsMuxersSource,
        log_source: LogSource,
        log_limit: int = 500,
    ) -> None:
        self._paths = paths_source
        self._hls = hls_source
        self._logs = log_source
        self._log_limit = log_limit

    async def collect(self) -> list[CameraSnapshot]:
        paths = await self._paths.list_paths()
        muxers = {m.path: m for m in await self._hls.list_hls_muxers()}
        logs = self._logs.read_recent(self._log_limit)
        now = datetime.now(timezone.utc)

        return [
            CameraSnapshot(
                name=path.name,
                path=path,
                hls=muxers.get(path.name),
                recent_logs=tuple(self._logs_for(path.name, logs)),
                collected_at=now,
            )
            for path in paths
        ]

    @staticmethod
    def _logs_for(camera: str, logs: list[LogEvent]) -> list[LogEvent]:
        """Correlaciona líneas de log con una cámara por su nombre de path."""
        if not camera:
            return []
        return [event for event in logs if camera in event.message]

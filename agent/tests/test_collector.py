"""Prueba del servicio de recolección con fuentes falsas (sin red).

Demuestra DIP/LSP: el colector no distingue una fuente real de una falsa,
mientras cumpla el puerto. Los datos imitan casos reales del playbook
(cam1 con H265 y error DeltaPocS0; cam2 caída).
"""

import asyncio

from app.domain.models import HlsMuxerState, LogEvent, LogLevel, PathState
from app.services.collector import StateCollector


class FakePaths:
    def __init__(self, paths: list[PathState]) -> None:
        self._paths = paths

    async def list_paths(self) -> list[PathState]:
        return self._paths


class FakeMuxers:
    def __init__(self, muxers: list[HlsMuxerState]) -> None:
        self._muxers = muxers

    async def list_hls_muxers(self) -> list[HlsMuxerState]:
        return self._muxers


class FakeLogs:
    def __init__(self, events: list[LogEvent]) -> None:
        self._events = events

    def read_recent(self, limit: int = 500) -> list[LogEvent]:
        return self._events


def _event(level: LogLevel, message: str) -> LogEvent:
    return LogEvent(timestamp=None, level=level, message=message, raw=message)


def test_collect_consolidates_state_hls_and_logs_per_camera() -> None:
    paths = [
        PathState(name="cam1", ready=True, tracks=("H265",)),
        PathState(name="cam2", ready=False),
    ]
    muxers = [HlsMuxerState(path="cam1", bytes_sent=100)]
    logs = [
        _event(LogLevel.ERROR, "[path cam1] DeltaPocS0 != 0"),
        _event(LogLevel.INFO, "[path cam2] is not receiving data"),
        _event(LogLevel.DEBUG, "línea sin cámara"),
    ]

    collector = StateCollector(FakePaths(paths), FakeMuxers(muxers), FakeLogs(logs))
    snapshots = asyncio.run(collector.collect())

    assert len(snapshots) == 2
    by_name = {s.name: s for s in snapshots}

    cam1 = by_name["cam1"]
    assert cam1.path is not None and cam1.path.tracks == ("H265",)
    assert cam1.hls is not None and cam1.hls.bytes_sent == 100
    assert len(cam1.recent_logs) == 1
    assert cam1.recent_logs[0].level is LogLevel.ERROR

    cam2 = by_name["cam2"]
    assert cam2.hls is None  # sin muxer HLS
    assert len(cam2.recent_logs) == 1  # solo su propia línea


def test_collect_returns_empty_when_no_paths() -> None:
    collector = StateCollector(FakePaths([]), FakeMuxers([]), FakeLogs([]))
    assert asyncio.run(collector.collect()) == []

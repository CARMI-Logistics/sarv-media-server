"""Adaptador de lectura de logs de MediaMTX desde el archivo compartido.

Única responsabilidad (SRP): leer las últimas líneas del log y parsearlas a
LogEvent. NO correlaciona con cámaras (eso es del servicio colector).
Implementa el puerto LogSource.
"""

from __future__ import annotations

import re
from collections import deque
from datetime import datetime
from pathlib import Path

from app.domain.models import LogEvent, LogLevel

# Formato por defecto de MediaMTX: "2026/07/14 16:25:01 INF <mensaje>"
_LINE_RE = re.compile(
    r"^(?P<ts>\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2})\s+(?P<lvl>\w{3})\s+(?P<msg>.*)$"
)
_LEVEL_MAP = {
    "DEB": LogLevel.DEBUG,
    "INF": LogLevel.INFO,
    "WAR": LogLevel.WARN,
    "ERR": LogLevel.ERROR,
}


class FileLogReader:
    """Lee el log de MediaMTX desde un archivo (volumen compartido)."""

    def __init__(self, log_path: str) -> None:
        self._log_path = Path(log_path)

    def read_recent(self, limit: int = 500) -> list[LogEvent]:
        if not self._log_path.exists():
            return []
        with self._log_path.open("r", encoding="utf-8", errors="replace") as fh:
            lines = deque(fh, maxlen=limit)
        return [self._parse(line.rstrip("\n")) for line in lines]

    @staticmethod
    def _parse(line: str) -> LogEvent:
        match = _LINE_RE.match(line)
        if not match:
            return LogEvent(
                timestamp=None, level=LogLevel.UNKNOWN, message=line, raw=line
            )
        try:
            ts: datetime | None = datetime.strptime(
                match.group("ts"), "%Y/%m/%d %H:%M:%S"
            )
        except ValueError:
            ts = None
        level = _LEVEL_MAP.get(match.group("lvl").upper(), LogLevel.UNKNOWN)
        return LogEvent(timestamp=ts, level=level, message=match.group("msg"), raw=line)

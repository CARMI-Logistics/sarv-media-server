"""Modelos de dominio del agente.

Value objects puros e inmutables: sin I/O, sin dependencias de framework.
Representan el "qué" (el estado observado), no el "cómo" se obtiene.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from enum import Enum


class LogLevel(str, Enum):
    """Nivel de una línea de log de MediaMTX."""

    DEBUG = "debug"
    INFO = "info"
    WARN = "warn"
    ERROR = "error"
    UNKNOWN = "unknown"


@dataclass(frozen=True, slots=True)
class PathState:
    """Estado de un path (cámara) según la API de MediaMTX (/v3/paths/list)."""

    name: str
    ready: bool
    source_type: str | None = None
    tracks: tuple[str, ...] = ()
    readers: int = 0
    bytes_received: int = 0
    ready_since: str | None = None


@dataclass(frozen=True, slots=True)
class HlsMuxerState:
    """Estado del muxer HLS de un path (/v3/hlsmuxers/list)."""

    path: str
    created: str | None = None
    last_request: str | None = None
    bytes_sent: int = 0


@dataclass(frozen=True, slots=True)
class LogEvent:
    """Una línea de log ya parseada a estructura."""

    timestamp: datetime | None
    level: LogLevel
    message: str
    raw: str


@dataclass(frozen=True, slots=True)
class CameraSnapshot:
    """Estado consolidado de una cámara en un instante.

    Es la entrada de la etapa de clasificación (tarea siguiente). Reúne el
    estado del path, su muxer HLS (si existe) y los logs recientes asociados.
    """

    name: str
    path: PathState | None
    hls: HlsMuxerState | None
    recent_logs: tuple[LogEvent, ...]
    collected_at: datetime


class HealthStatus(str, Enum):
    """Veredicto de salud de una cámara."""

    OK = "ok"
    DEGRADED = "degraded"
    DOWN = "down"
    UNKNOWN = "unknown"


class IssueType(str, Enum):
    """Tipo de problema reconocido por el playbook."""

    H265_BREAKS_HLS = "h265_breaks_hls"
    PACKETIZATION_MODE0 = "packetization_mode0_needs_tcp"
    NETWORK_UNREACHABLE = "network_unreachable"
    STREAM_NOT_READY = "stream_not_ready"
    UNKNOWN = "unknown"


@dataclass(frozen=True, slots=True)
class Finding:
    """Un hallazgo emitido por una regla: qué problema, qué tan grave y con qué evidencia."""

    issue: IssueType
    severity: HealthStatus
    summary: str
    evidence: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class CameraClassification:
    """Veredicto consolidado de una cámara (salida de la clasificación)."""

    name: str
    status: HealthStatus
    findings: tuple[Finding, ...]


@dataclass(frozen=True, slots=True)
class Diagnosis:
    """Diagnóstico en lenguaje claro con soluciones sugeridas (salida de la IA)."""

    name: str
    status: HealthStatus
    explanation: str
    suggested_solutions: tuple[str, ...]
    issues: tuple[IssueType, ...]

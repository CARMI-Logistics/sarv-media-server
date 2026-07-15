"""Reglas del playbook de diagnóstico (motor determinista).

Cada regla tiene UNA responsabilidad: reconocer un patrón conocido y, si lo
detecta, emitir un Finding. Son detectores puros: no saben unas de otras ni
deciden el estado global (eso lo hace el HealthClassifier).

Añadir un patrón nuevo = nueva clase + registrarla en default_rules(). No se
toca el clasificador (OCP).
"""

from __future__ import annotations

from app.domain.models import (
    CameraSnapshot,
    Finding,
    HealthStatus,
    IssueType,
)
from app.domain.ports import HealthRule

_H265_CODECS = ("h265", "hevc", "h.265")


def _has_h265(snapshot: CameraSnapshot) -> bool:
    tracks = snapshot.path.tracks if snapshot.path else ()
    return any(any(codec in t.lower() for codec in _H265_CODECS) for t in tracks)


def _logs_matching(snapshot: CameraSnapshot, needles: tuple[str, ...]) -> tuple[str, ...]:
    return tuple(
        event.raw
        for event in snapshot.recent_logs
        if any(needle in event.message.lower() for needle in needles)
    )


class H265BreaksHlsRule:
    """H.265/HEVC rompe el muxer HLS (el stream sigue por RTSP/WebRTC)."""

    def evaluate(self, snapshot: CameraSnapshot) -> Finding | None:
        if not _has_h265(snapshot):
            return None
        evidence = (f"tracks={list(snapshot.path.tracks)}",) if snapshot.path else ()
        evidence += _logs_matching(snapshot, ("deltapocs0", "hls"))
        return Finding(
            issue=IssueType.H265_BREAKS_HLS,
            severity=HealthStatus.DEGRADED,
            summary=(
                "La cámara publica en H.265/HEVC, que rompe el muxer HLS. "
                "El video sigue disponible por RTSP/WebRTC, no por HLS."
            ),
            evidence=evidence,
        )


class PacketizationMode0Rule:
    """Fuente H264 con packetization-mode=0: solo funciona leyendo por TCP."""

    _NEEDLES = ("packetization-mode=0", "only supported when reading with tcp")

    def evaluate(self, snapshot: CameraSnapshot) -> Finding | None:
        hits = _logs_matching(snapshot, self._NEEDLES)
        if not hits:
            return None
        return Finding(
            issue=IssueType.PACKETIZATION_MODE0,
            severity=HealthStatus.DEGRADED,
            summary=(
                "Fuente H264 con packetization-mode=0: requiere leer el RTSP con "
                "transporte TCP (rtspTransport: tcp)."
            ),
            evidence=hits,
        )


class NetworkUnreachableRule:
    """La cámara no responde en la red: problema de red, no de software."""

    _NEEDLES = ("i/o timeout", "no route to host", "connection refused", "dial tcp")

    def evaluate(self, snapshot: CameraSnapshot) -> Finding | None:
        hits = _logs_matching(snapshot, self._NEEDLES)
        if not hits:
            return None
        return Finding(
            issue=IssueType.NETWORK_UNREACHABLE,
            severity=HealthStatus.DOWN,
            summary=(
                "La cámara no responde en la red (timeout / host inalcanzable). "
                "Es un problema de red o de la cámara, no del media server."
            ),
            evidence=hits,
        )


def default_rules() -> list[HealthRule]:
    """Reglas activas, de más específica/grave a más general."""
    return [
        NetworkUnreachableRule(),
        PacketizationMode0Rule(),
        H265BreaksHlsRule(),
    ]

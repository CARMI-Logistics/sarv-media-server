"""Pruebas del motor de reglas + clasificador.

Cada caso imita un síntoma real del playbook. Se construyen snapshots a mano
(sin red) y se verifica el veredicto y el tipo de hallazgo.
"""

from datetime import datetime, timezone

from app.domain.models import (
    CameraSnapshot,
    HealthStatus,
    HlsMuxerState,
    IssueType,
    LogEvent,
    LogLevel,
    PathState,
)
from app.rules import default_rules
from app.services.classifier import HealthClassifier


def _snapshot(
    name: str,
    *,
    ready: bool = True,
    tracks: tuple[str, ...] = ("H264",),
    logs: tuple[str, ...] = (),
) -> CameraSnapshot:
    events = tuple(
        LogEvent(timestamp=None, level=LogLevel.ERROR, message=msg, raw=msg)
        for msg in logs
    )
    return CameraSnapshot(
        name=name,
        path=PathState(name=name, ready=ready, tracks=tracks),
        hls=HlsMuxerState(path=name) if ready else None,
        recent_logs=events,
        collected_at=datetime.now(timezone.utc),
    )


def _classifier() -> HealthClassifier:
    return HealthClassifier(default_rules())


def test_healthy_camera_is_ok_without_findings() -> None:
    result = _classifier().classify(_snapshot("cam-ok"))
    assert result.status is HealthStatus.OK
    assert result.findings == ()


def test_h265_camera_is_degraded() -> None:
    result = _classifier().classify(_snapshot("cam-h265", tracks=("H265",)))
    assert result.status is HealthStatus.DEGRADED
    assert result.findings[0].issue is IssueType.H265_BREAKS_HLS


def test_network_unreachable_is_down() -> None:
    result = _classifier().classify(
        _snapshot(
            "cam-net",
            ready=False,
            logs=("dial tcp 10.0.0.222:554: i/o timeout",),
        )
    )
    assert result.status is HealthStatus.DOWN
    assert result.findings[0].issue is IssueType.NETWORK_UNREACHABLE


def test_packetization_mode0_is_degraded() -> None:
    result = _classifier().classify(
        _snapshot(
            "cam-pack",
            logs=("H264 packetization-mode=0 is only supported when reading with TCP",),
        )
    )
    assert result.status is HealthStatus.DEGRADED
    assert result.findings[0].issue is IssueType.PACKETIZATION_MODE0


def test_not_ready_without_cause_falls_back_to_down() -> None:
    result = _classifier().classify(_snapshot("cam-dead", ready=False))
    assert result.status is HealthStatus.DOWN
    assert result.findings[0].issue is IssueType.STREAM_NOT_READY


def test_worst_severity_wins_when_multiple_findings() -> None:
    # H265 (degraded) + red inalcanzable (down) -> gana DOWN
    result = _classifier().classify(
        _snapshot(
            "cam-mix",
            ready=False,
            tracks=("H265",),
            logs=("connection refused",),
        )
    )
    assert result.status is HealthStatus.DOWN
    issues = {f.issue for f in result.findings}
    assert IssueType.NETWORK_UNREACHABLE in issues
    assert IssueType.H265_BREAKS_HLS in issues

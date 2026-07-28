"""Pruebas del formateo y el servicio de notificación (con notifier falso)."""

import asyncio

from app.domain.models import Diagnosis, HealthStatus
from app.security.redactor import RegexRedactor
from app.services.notify import NotificationService
from app.services.reporter import ReportFormatter


class FakeNotifier:
    def __init__(self) -> None:
        self.messages: list[str] = []

    async def send(self, message: str) -> None:
        self.messages.append(message)


def _diag(name: str, status: HealthStatus, explanation: str = "", solutions=()) -> Diagnosis:
    return Diagnosis(
        name=name,
        status=status,
        explanation=explanation,
        suggested_solutions=tuple(solutions),
        issues=(),
    )


def _service(notifier: FakeNotifier) -> NotificationService:
    return NotificationService(ReportFormatter(), notifier, RegexRedactor())


# --- formateo ---

def test_formatter_lists_problems_with_solutions() -> None:
    msg = ReportFormatter().format(
        [_diag("cam-h265", HealthStatus.DEGRADED, "Usa H265", ("Cambiar a H264",))]
    )
    assert "1 con problema(s)" in msg
    assert "cam-h265" in msg
    assert "Cambiar a H264" in msg


def test_formatter_lists_recovered_cameras() -> None:
    msg = ReportFormatter().format([_diag("cam-ok", HealthStatus.OK)])
    assert "1 recuperada(s)" in msg
    assert "Recuperadas" in msg
    assert "cam-ok" in msg


def test_formatter_mixes_problems_and_recoveries() -> None:
    diagnoses = [
        _diag("cam-down", HealthStatus.DOWN, "caída"),
        _diag("cam-ok", HealthStatus.OK),
    ]
    msg = ReportFormatter().format(diagnoses)
    assert "1 con problema(s), 1 recuperada(s)" in msg
    assert "cam-down" in msg
    assert "cam-ok" in msg


# --- servicio de notificación ---

def test_notify_sends_when_there_are_problems() -> None:
    notifier = FakeNotifier()
    result = asyncio.run(
        _service(notifier).notify([_diag("cam", HealthStatus.DOWN, "caída")])
    )
    assert result.sent is True
    assert len(notifier.messages) == 1


def test_notify_sends_on_recovery() -> None:
    notifier = FakeNotifier()
    result = asyncio.run(_service(notifier).notify([_diag("cam", HealthStatus.OK)]))
    assert result.sent is True
    assert "recuperada" in notifier.messages[0].lower()


def test_notify_skips_when_nothing_to_report() -> None:
    notifier = FakeNotifier()
    result = asyncio.run(_service(notifier).notify([]))
    assert result.sent is False
    assert notifier.messages == []


def test_message_is_redacted_before_sending() -> None:
    # 🔴 el canal es externo: el mensaje sale redactado.
    diag = _diag(
        "cam",
        HealthStatus.DOWN,
        explanation="revisar fuente rtsp://zeus:zeus@10.0.0.1/stream",
        solutions=("cambiar credencial",),
    )
    notifier = FakeNotifier()
    asyncio.run(_service(notifier).notify([diag]))
    sent = notifier.messages[0]
    assert "zeus:zeus" not in sent
    assert "***:***@10.0.0.1" in sent

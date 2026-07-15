"""Prueba del AgentRunner: corre el grafo (falso) y notifica."""

import asyncio

from app.domain.models import Diagnosis, HealthStatus
from app.security.redactor import RegexRedactor
from app.services.notify import NotificationService
from app.services.reporter import ReportFormatter
from app.services.runner import AgentRunner


class FakeGraph:
    def __init__(self, diagnoses: list[Diagnosis]) -> None:
        self._diagnoses = diagnoses

    async def ainvoke(self, _state: dict) -> dict:
        return {"diagnoses": self._diagnoses}


class FakeNotifier:
    def __init__(self) -> None:
        self.messages: list[str] = []

    async def send(self, message: str) -> None:
        self.messages.append(message)


def _diag(name: str, status: HealthStatus) -> Diagnosis:
    return Diagnosis(name, status, "explicación", (), ())


def _runner(graph: FakeGraph, notifier: FakeNotifier) -> AgentRunner:
    service = NotificationService(ReportFormatter(), notifier, RegexRedactor())
    return AgentRunner(graph, service)


def test_run_and_notify_sends_when_problems() -> None:
    notifier = FakeNotifier()
    diagnoses = [
        _diag("cam-ok", HealthStatus.OK),
        _diag("cam-down", HealthStatus.DOWN),
    ]
    result = asyncio.run(_runner(FakeGraph(diagnoses), notifier).run_and_notify())

    assert result.total == 2
    assert result.problems == 1
    assert result.notified is True
    assert len(notifier.messages) == 1


def test_run_and_notify_skips_notification_when_all_ok() -> None:
    notifier = FakeNotifier()
    diagnoses = [_diag("cam-ok", HealthStatus.OK)]
    result = asyncio.run(_runner(FakeGraph(diagnoses), notifier).run_and_notify())

    assert result.total == 1
    assert result.problems == 0
    assert result.notified is False
    assert notifier.messages == []

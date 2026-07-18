"""Prueba del AgentRunner: corre el grafo (falso) y notifica."""

import asyncio

from app.domain.models import Diagnosis, HealthStatus
from app.infra.null_failure_history import NullFailureHistory
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


class FakeHistory:
    """Historial falso: controla qué cámaras 'cambiaron' y registra las guardadas."""

    def __init__(self, changed_names: set[str]) -> None:
        self._changed = changed_names
        self.recorded: list[str] = []

    async def has_changed(self, diagnosis: Diagnosis) -> bool:
        return diagnosis.name in self._changed

    async def record(self, diagnosis: Diagnosis) -> None:
        self.recorded.append(diagnosis.name)


def _runner(graph: FakeGraph, notifier: FakeNotifier) -> AgentRunner:
    service = NotificationService(ReportFormatter(), notifier, RegexRedactor())
    return AgentRunner(graph, service, NullFailureHistory(), RegexRedactor())


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


def test_dedup_records_and_notifies_only_changed() -> None:
    notifier = FakeNotifier()
    history = FakeHistory(changed_names={"cam-a"})  # solo cam-a cambió
    diagnoses = [
        _diag("cam-a", HealthStatus.DOWN),
        _diag("cam-b", HealthStatus.DOWN),  # también caída, pero SIN cambio
    ]
    service = NotificationService(ReportFormatter(), notifier, RegexRedactor())
    runner = AgentRunner(FakeGraph(diagnoses), service, history, RegexRedactor())

    result = asyncio.run(runner.run_and_notify())

    assert history.recorded == ["cam-a"]  # solo se registra lo que cambió
    assert result.problems == 2  # estado actual: ambas caídas
    assert result.notified is True  # cam-a cambió y es problema → notifica
    assert "cam-a" in notifier.messages[0]
    assert "cam-b" not in notifier.messages[0]  # cam-b no se re-alerta

"""Prueba del grafo de LangGraph end-to-end con fuentes/LLM falsos (sin red)."""

import asyncio
from datetime import datetime, timezone

from app.domain.models import CameraSnapshot, PathState
from app.graph import build_graph
from app.rules import default_rules
from app.security.redactor import RegexRedactor
from app.services.classifier import HealthClassifier
from app.services.diagnoser import DiagnoserService


class FakeCollector:
    def __init__(self, snapshots: list[CameraSnapshot]) -> None:
        self._snapshots = snapshots

    async def collect(self) -> list[CameraSnapshot]:
        return self._snapshots


class FakeLlm:
    def __init__(self, response: str) -> None:
        self.response = response

    async def generate(self, prompt: str) -> str:
        return self.response


def _snapshot(name: str, tracks: tuple[str, ...]) -> CameraSnapshot:
    return CameraSnapshot(
        name=name,
        path=PathState(name=name, ready=True, tracks=tracks),
        hls=None,
        recent_logs=(),
        collected_at=datetime.now(timezone.utc),
    )


def test_graph_runs_collect_classify_diagnose() -> None:
    snapshots = [_snapshot("cam-ok", ("H264",)), _snapshot("cam-h265", ("H265",))]
    graph = build_graph(
        collector=FakeCollector(snapshots),
        classifier=HealthClassifier(default_rules()),
        diagnoser=DiagnoserService(
            FakeLlm('{"explanation": "explicada", "solutions": ["arreglo"]}'),
            RegexRedactor(),
        ),
    )

    result = asyncio.run(graph.ainvoke({}))

    diagnoses = result["diagnoses"]
    assert len(diagnoses) == 2
    by_name = {d.name: d for d in diagnoses}
    # cam OK: no pasó por el LLM
    assert by_name["cam-ok"].explanation == "Sin problemas detectados."
    # cam H265: diagnosticada por el LLM (falso)
    assert by_name["cam-h265"].explanation == "explicada"
    assert by_name["cam-h265"].suggested_solutions == ("arreglo",)

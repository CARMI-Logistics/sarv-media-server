"""Clasificador de salud: agrega los hallazgos de las reglas en un veredicto.

Depende del puerto HealthRule (DIP): recibe las reglas inyectadas, no las
conoce en concreto. Su única responsabilidad es correr las reglas y decidir el
estado global de cada cámara (política de agregación + baseline).
"""

from __future__ import annotations

from app.domain.models import (
    CameraClassification,
    CameraSnapshot,
    Finding,
    HealthStatus,
    IssueType,
)
from app.domain.ports import HealthRule

# Orden de gravedad para elegir el peor estado entre varios hallazgos.
_SEVERITY_ORDER = {
    HealthStatus.OK: 0,
    HealthStatus.UNKNOWN: 1,
    HealthStatus.DEGRADED: 2,
    HealthStatus.DOWN: 3,
}


class HealthClassifier:
    def __init__(self, rules: list[HealthRule]) -> None:
        self._rules = rules

    def classify(self, snapshot: CameraSnapshot) -> CameraClassification:
        findings = tuple(
            finding
            for rule in self._rules
            if (finding := rule.evaluate(snapshot)) is not None
        )

        # Baseline: si ninguna regla explicó el problema pero el path no está
        # listo, lo marcamos como caído sin causa conocida.
        if not findings and not self._is_ready(snapshot):
            findings = (self._not_ready_finding(snapshot),)

        return CameraClassification(
            name=snapshot.name,
            status=self._overall_status(snapshot, findings),
            findings=findings,
        )

    def classify_all(
        self, snapshots: list[CameraSnapshot]
    ) -> list[CameraClassification]:
        return [self.classify(snapshot) for snapshot in snapshots]

    @staticmethod
    def _is_ready(snapshot: CameraSnapshot) -> bool:
        return snapshot.path is not None and snapshot.path.ready

    def _overall_status(
        self, snapshot: CameraSnapshot, findings: tuple[Finding, ...]
    ) -> HealthStatus:
        if findings:
            return max(
                (finding.severity for finding in findings),
                key=lambda severity: _SEVERITY_ORDER[severity],
            )
        return HealthStatus.OK if self._is_ready(snapshot) else HealthStatus.DOWN

    @staticmethod
    def _not_ready_finding(snapshot: CameraSnapshot) -> Finding:
        return Finding(
            issue=IssueType.STREAM_NOT_READY,
            severity=HealthStatus.DOWN,
            summary="El path no está listo (sin datos entrando) y sin causa conocida en logs.",
            evidence=tuple(event.raw for event in snapshot.recent_logs[-3:]),
        )

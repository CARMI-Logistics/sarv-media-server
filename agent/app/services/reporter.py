"""Formateo del reporte: convierte diagnósticos en un mensaje legible.

Única responsabilidad: dar forma al texto. No decide el canal ni envía (eso es
del NotificationService). Solo resalta las cámaras con problema.
"""

from __future__ import annotations

from app.domain.models import Diagnosis, HealthStatus

_STATUS_EMOJI = {
    HealthStatus.DOWN: "🔴",
    HealthStatus.DEGRADED: "🟠",
    HealthStatus.UNKNOWN: "⚪",
}


class ReportFormatter:
    def has_problems(self, diagnoses: list[Diagnosis]) -> bool:
        return any(d.status is not HealthStatus.OK for d in diagnoses)

    def format(self, diagnoses: list[Diagnosis]) -> str:
        total = len(diagnoses)
        problems = [d for d in diagnoses if d.status is not HealthStatus.OK]

        if not problems:
            return f"✅ Todas las cámaras OK ({total})."

        lines = [f"⚠️ Reporte de cámaras: {len(problems)}/{total} con problemas", ""]
        for diagnosis in problems:
            emoji = _STATUS_EMOJI.get(diagnosis.status, "⚪")
            lines.append(f"{emoji} {diagnosis.name} [{diagnosis.status.value}]")
            lines.append(f"   {diagnosis.explanation}")
            for solution in diagnosis.suggested_solutions:
                lines.append(f"   • {solution}")
            lines.append("")
        return "\n".join(lines).strip()

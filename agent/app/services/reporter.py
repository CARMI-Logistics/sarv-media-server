"""Formateo del reporte: convierte diagnósticos en un mensaje legible.

Única responsabilidad: dar forma al texto. No decide el canal ni envía (eso es
del NotificationService). Recibe las cámaras que ameritan aviso y las agrupa en
dos secciones: las que tienen problema y las que se recuperaron (ahora OK).
"""

from __future__ import annotations

from app.domain.models import Diagnosis, HealthStatus

_STATUS_EMOJI = {
    HealthStatus.DOWN: "🔴",
    HealthStatus.DEGRADED: "🟠",
    HealthStatus.UNKNOWN: "⚪",
}


class ReportFormatter:
    def format(self, diagnoses: list[Diagnosis]) -> str:
        problems = [d for d in diagnoses if d.status is not HealthStatus.OK]
        recovered = [d for d in diagnoses if d.status is HealthStatus.OK]

        if not problems and not recovered:
            return "✅ Sin cambios que reportar."

        header = []
        if problems:
            header.append(f"{len(problems)} con problema(s)")
        if recovered:
            header.append(f"{len(recovered)} recuperada(s)")
        lines = [f"📋 Cámaras: {', '.join(header)}", ""]

        for diagnosis in problems:
            emoji = _STATUS_EMOJI.get(diagnosis.status, "⚪")
            lines.append(f"{emoji} {diagnosis.name} [{diagnosis.status.value}]")
            lines.append(f"   {diagnosis.explanation}")
            for solution in diagnosis.suggested_solutions:
                lines.append(f"   • {solution}")
            lines.append("")

        if recovered:
            lines.append("Recuperadas (ahora OK):")
            for diagnosis in recovered:
                lines.append(f"🟢 {diagnosis.name}")
            lines.append("")

        return "\n".join(lines).strip()

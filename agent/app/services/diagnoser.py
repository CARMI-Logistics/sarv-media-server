"""Servicio de diagnóstico: convierte un veredicto en explicación + soluciones.

Depende de los puertos LlmClient y Redactor (DIP): en tests se inyecta un LLM
falso y no se necesita red ni API key.

🔴 Seguridad (HU 1.3): el prompt pasa por el Redactor ANTES de llegar al LLM.
Grounding: a Gemini solo se le pide explicar/redactar lo que la regla ya
identificó (issue + evidencia); las cámaras OK no gastan llamada al LLM.
"""

from __future__ import annotations

import json

from app.domain.models import CameraClassification, Diagnosis, HealthStatus
from app.domain.ports import LlmClient, Redactor

_INSTRUCTION = (
    "Eres un asistente que explica fallos de cámaras de video (servidor MediaMTX) "
    "a un operador que NO es experto en streaming. Con base en el estado y los "
    "hallazgos, responde EXCLUSIVAMENTE con un JSON válido con esta forma:\n"
    '{"explanation": "<explicación breve y clara en español>", '
    '"solutions": ["<solución 1>", "<solución 2>"]}\n'
    "No agregues texto fuera del JSON."
)


class DiagnoserService:
    def __init__(self, llm: LlmClient, redactor: Redactor) -> None:
        self._llm = llm
        self._redactor = redactor

    async def diagnose(self, classification: CameraClassification) -> Diagnosis:
        issues = tuple(finding.issue for finding in classification.findings)

        # Las cámaras sanas no gastan llamada al LLM.
        if classification.status is HealthStatus.OK:
            return Diagnosis(
                name=classification.name,
                status=classification.status,
                explanation="Sin problemas detectados.",
                suggested_solutions=(),
                issues=issues,
            )

        prompt = self._build_prompt(classification)
        safe_prompt = self._redactor.redact(prompt)  # 🔴 redacción antes del LLM
        raw = await self._llm.generate(safe_prompt)
        return self._parse(classification, issues, raw)

    async def diagnose_all(
        self, classifications: list[CameraClassification]
    ) -> list[Diagnosis]:
        return [await self.diagnose(c) for c in classifications]

    @staticmethod
    def _build_prompt(classification: CameraClassification) -> str:
        lines = [
            _INSTRUCTION,
            "",
            f"Cámara: {classification.name}",
            f"Estado: {classification.status.value}",
            "Hallazgos:",
        ]
        for finding in classification.findings:
            lines.append(f"- [{finding.issue.value}] {finding.summary}")
            for evidence in finding.evidence:
                lines.append(f"    evidencia: {evidence}")
        return "\n".join(lines)

    @staticmethod
    def _parse(
        classification: CameraClassification,
        issues: tuple,
        raw: str,
    ) -> Diagnosis:
        explanation = raw.strip()
        solutions: tuple[str, ...] = ()

        start, end = raw.find("{"), raw.rfind("}")
        if start != -1 and end != -1 and end > start:
            try:
                data = json.loads(raw[start : end + 1])
                explanation = str(data.get("explanation", explanation)).strip()
                solutions = tuple(str(s) for s in data.get("solutions", []))
            except (ValueError, TypeError):
                pass  # se queda con el texto crudo como explicación

        return Diagnosis(
            name=classification.name,
            status=classification.status,
            explanation=explanation,
            suggested_solutions=solutions,
            issues=issues,
        )

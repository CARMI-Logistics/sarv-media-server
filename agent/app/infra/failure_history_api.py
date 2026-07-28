"""Adaptador HTTP al historial de diagnósticos del backend (HU 4.6).

Única responsabilidad (SRP): hablar con `POST/GET /admin/failures` del backend y
traducir el estado del dominio (HealthStatus) a la severidad del backend. Es la
capa anticorrupción entre el vocabulario del agente y el del backend.
"""

from __future__ import annotations

import httpx

from app.domain.models import Diagnosis, HealthStatus, Transition

# Traducción del estado del agente a la severidad del backend.
_SEVERITY: dict[HealthStatus, str] = {
    HealthStatus.OK: "ok",
    HealthStatus.DEGRADED: "warn",
    HealthStatus.DOWN: "error",
    HealthStatus.UNKNOWN: "warn",
}
# Severidades del backend que representan un problema (no-OK).
_PROBLEM_SEVERITIES = {"warn", "error"}


class FailureHistoryApi:
    """Cliente del historial en el backend (bearer ADMIN_API_TOKEN)."""

    def __init__(self, base_url: str, token: str, timeout: float = 5.0) -> None:
        self._base_url = base_url.rstrip("/")
        self._token = token
        self._timeout = timeout

    def _headers(self) -> dict[str, str]:
        return {"Authorization": f"Bearer {self._token}"}

    async def record(self, diagnosis: Diagnosis) -> None:
        payload = {
            "camera_path": diagnosis.name,
            "severity": _SEVERITY.get(diagnosis.status, "warn"),
            "diagnosis": diagnosis.explanation,
            "raw": {
                "issues": [issue.value for issue in diagnosis.issues],
                "suggested_solutions": list(diagnosis.suggested_solutions),
            },
        }
        async with httpx.AsyncClient(timeout=self._timeout) as client:
            resp = await client.post(
                f"{self._base_url}/admin/failures",
                json=payload,
                headers=self._headers(),
            )
            resp.raise_for_status()

    async def classify(self, diagnosis: Diagnosis) -> Transition:
        current = _SEVERITY.get(diagnosis.status, "warn")
        async with httpx.AsyncClient(timeout=self._timeout) as client:
            resp = await client.get(
                f"{self._base_url}/admin/failures",
                params={"camera": diagnosis.name, "limit": 1},
                headers=self._headers(),
            )
            resp.raise_for_status()
            items = resp.json()
        last = items[0]["severity"] if items else None

        if last == current:
            return Transition.UNCHANGED  # mismo estado -> dedup
        if current == "ok":
            # Recuperación solo si ANTES había un problema registrado; un OK sin
            # baseline previo no es "recuperación" (evita falsos en el 1er arranque).
            return Transition.RECOVERY if last in _PROBLEM_SEVERITIES else Transition.UNCHANGED
        return Transition.ALERT  # cambió a/entre estados de problema

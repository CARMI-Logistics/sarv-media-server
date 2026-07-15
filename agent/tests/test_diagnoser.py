"""Pruebas del DiagnoserService con un LLM falso (sin red ni API key).

Incluye la prueba 🔴 de seguridad: el prompt que recibe el LLM va redactado.
"""

import asyncio

import pytest

from app.domain.models import (
    CameraClassification,
    Finding,
    HealthStatus,
    IssueType,
)
from app.infra.gemini import GeminiClient, LlmNotConfigured
from app.security.redactor import RegexRedactor
from app.services.diagnoser import DiagnoserService


class FakeLlm:
    def __init__(self, response: str) -> None:
        self.response = response
        self.prompts: list[str] = []

    async def generate(self, prompt: str) -> str:
        self.prompts.append(prompt)
        return self.response


def _svc(llm: FakeLlm) -> DiagnoserService:
    return DiagnoserService(llm, RegexRedactor())


def test_ok_camera_skips_the_llm() -> None:
    llm = FakeLlm("{}")
    result = asyncio.run(
        _svc(llm).diagnose(CameraClassification("cam-ok", HealthStatus.OK, ()))
    )
    assert result.explanation == "Sin problemas detectados."
    assert result.suggested_solutions == ()
    assert llm.prompts == []  # no se gastó llamada al LLM


def test_diagnose_parses_json_explanation_and_solutions() -> None:
    llm = FakeLlm(
        '{"explanation": "La cámara publica en H265.", '
        '"solutions": ["Cambiar a H264", "Consumir por WebRTC"]}'
    )
    finding = Finding(
        IssueType.H265_BREAKS_HLS, HealthStatus.DEGRADED, "H265 rompe HLS", ()
    )
    result = asyncio.run(
        _svc(llm).diagnose(
            CameraClassification("cam-h265", HealthStatus.DEGRADED, (finding,))
        )
    )
    assert "H265" in result.explanation
    assert result.suggested_solutions == ("Cambiar a H264", "Consumir por WebRTC")
    assert IssueType.H265_BREAKS_HLS in result.issues


def test_diagnose_handles_json_wrapped_in_markdown() -> None:
    llm = FakeLlm('```json\n{"explanation": "x", "solutions": ["y"]}\n```')
    finding = Finding(IssueType.STREAM_NOT_READY, HealthStatus.DOWN, "sin datos", ())
    result = asyncio.run(
        _svc(llm).diagnose(
            CameraClassification("cam", HealthStatus.DOWN, (finding,))
        )
    )
    assert result.explanation == "x"
    assert result.suggested_solutions == ("y",)


def test_prompt_sent_to_llm_is_redacted() -> None:
    # 🔴 HU 1.3: la evidencia trae una URL RTSP con credenciales.
    evidence = (
        "source rtsp://zeus:zeus@10.0.0.222:554/rtsp/defaultPrimary-0 failed: i/o timeout",
    )
    finding = Finding(
        IssueType.NETWORK_UNREACHABLE, HealthStatus.DOWN, "no responde", evidence
    )
    llm = FakeLlm('{"explanation": "x", "solutions": []}')
    asyncio.run(
        _svc(llm).diagnose(
            CameraClassification("cam-net", HealthStatus.DOWN, (finding,))
        )
    )
    sent = llm.prompts[0]
    assert "zeus:zeus" not in sent
    assert "***:***@10.0.0.222" in sent


def test_gemini_client_without_key_raises_not_configured() -> None:
    client = GeminiClient(api_key="", model="gemini-2.5-flash")
    with pytest.raises(LlmNotConfigured):
        asyncio.run(client.generate("hola"))

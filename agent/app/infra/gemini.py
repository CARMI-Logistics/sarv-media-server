"""Adaptador del LLM Gemini (langchain-google-genai).

Implementa el puerto LlmClient. Única responsabilidad: hablar con Gemini.

- Import perezoso: la librería pesada solo se carga al usarse.
- Tolera key vacía: si no hay GEMINI_API_KEY, lanza LlmNotConfigured (el
  servicio decide cómo reportarlo) en vez de romper al arrancar. Así el agente
  levanta sin la clave y solo falla el /diagnose hasta que la configures.
"""

from __future__ import annotations


class LlmNotConfigured(RuntimeError):
    """No hay GEMINI_API_KEY configurada."""


class GeminiClient:
    def __init__(self, api_key: str | None, model: str) -> None:
        self._api_key = api_key
        self._model = model
        self._client = None  # se crea perezosamente

    async def generate(self, prompt: str) -> str:
        if not self._api_key:
            raise LlmNotConfigured(
                "GEMINI_API_KEY no configurada: define la clave en el .env del agente."
            )
        client = self._ensure_client()
        message = await client.ainvoke(prompt)
        content = getattr(message, "content", message)
        if isinstance(content, list):  # algunos modelos devuelven partes
            return " ".join(str(part) for part in content)
        return str(content)

    def _ensure_client(self):
        if self._client is None:
            from langchain_google_genai import ChatGoogleGenerativeAI

            self._client = ChatGoogleGenerativeAI(
                model=self._model,
                google_api_key=self._api_key,
            )
        return self._client

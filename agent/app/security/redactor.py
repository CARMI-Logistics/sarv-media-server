"""Redacción de datos sensibles antes de enviarlos a un servicio externo (Gemini).

🔴 HU 1.3: ningún secreto (credenciales en URLs RTSP, tokens, contraseñas) debe
salir hacia el asistente externo de IA.

Motor de reglas de texto: cada regla es una tupla (patrón, reemplazo). Añadir un
secreto nuevo = añadir una tupla; no se toca el motor (OCP). `redact_obj` recorre
estructuras (dict/list/tuple/str) para ofrecer un único punto de estrangulamiento
antes del LLM: así no se puede "olvidar" un campo.

Diseño: NO enmascara IPs ni nombres de path (se necesitan para diagnosticar) ni
query no sensible; solo credenciales y secretos.
"""

from __future__ import annotations

import re

_MASK = "***"

# Reglas por defecto, aplicadas en orden. Añadir un secreto = añadir una tupla.
_DEFAULT_RULES: tuple[tuple[re.Pattern[str], str], ...] = (
    # 1) Credenciales en el userinfo de una URL: scheme://user:pass@host
    (re.compile(r"([a-zA-Z][a-zA-Z0-9+.\-]*://)[^/\s:@]+:[^/\s@]+@"), rf"\1{_MASK}:{_MASK}@"),
    # 2) Secretos en query string: ?password=... &token=...
    (
        re.compile(
            r"(?i)([?&](?:password|passwd|pwd|token|apikey|api[_-]?key|secret|auth)=)[^&#\s]+"
        ),
        rf"\1{_MASK}",
    ),
    # 3) Cabecera Authorization: enmascara todo el valor (incluye 'Bearer xxx')
    (re.compile(r"(?i)(authorization\s*[:=]\s*).+"), rf"\1{_MASK}"),
    # 4) Token tipo 'Bearer xxx' suelto
    (re.compile(r"(?i)\b(bearer)\s+\S+"), rf"\1 {_MASK}"),
    # 5) Pares clave-valor con secretos: password: xxx, token=xxx, api_key: xxx
    #    El valor para en espacios y en separadores de query (& #) para no
    #    tragarse el resto de una URL.
    (
        re.compile(
            r"(?i)\b(password|passwd|pwd|token|api[_-]?key|secret)\b(\s*[:=]\s*)[^\s&#]+"
        ),
        rf"\1\2{_MASK}",
    ),
)


class RegexRedactor:
    """Redactor basado en reglas regex. Implementa el puerto Redactor."""

    def __init__(
        self, rules: tuple[tuple[re.Pattern[str], str], ...] | None = None
    ) -> None:
        self._rules = rules if rules is not None else _DEFAULT_RULES

    def redact(self, text: str) -> str:
        if not text:
            return text
        for pattern, replacement in self._rules:
            text = pattern.sub(replacement, text)
        return text

    def redact_obj(self, obj: object) -> object:
        """Redacta recursivamente strings dentro de dict/list/tuple."""
        if isinstance(obj, str):
            return self.redact(obj)
        if isinstance(obj, dict):
            return {key: self.redact_obj(value) for key, value in obj.items()}
        if isinstance(obj, (list, tuple)):
            return type(obj)(self.redact_obj(item) for item in obj)
        return obj

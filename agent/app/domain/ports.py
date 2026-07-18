"""Puertos (interfaces) del dominio.

Los servicios dependen de estas abstracciones, no de implementaciones concretas
(DIP). Cada puerto es pequeño y enfocado (ISP): un consumidor depende solo de lo
que usa, aunque un mismo adaptador implemente varios.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from app.domain.models import (
    CameraSnapshot,
    Diagnosis,
    Finding,
    HlsMuxerState,
    LogEvent,
    PathState,
)


@runtime_checkable
class PathsSource(Protocol):
    """Fuente del estado de los paths (cámaras)."""

    async def list_paths(self) -> list[PathState]: ...


@runtime_checkable
class HlsMuxersSource(Protocol):
    """Fuente del estado de los muxers HLS."""

    async def list_hls_muxers(self) -> list[HlsMuxerState]: ...


@runtime_checkable
class LogSource(Protocol):
    """Fuente de eventos de log recientes."""

    def read_recent(self, limit: int = 500) -> list[LogEvent]: ...


@runtime_checkable
class HealthRule(Protocol):
    """Regla de clasificación: reconoce un patrón y emite un Finding (o None)."""

    def evaluate(self, snapshot: CameraSnapshot) -> Finding | None: ...


@runtime_checkable
class Redactor(Protocol):
    """Oculta datos sensibles antes de que un texto salga a un servicio externo."""

    def redact(self, text: str) -> str: ...

    def redact_obj(self, obj: object) -> object: ...


@runtime_checkable
class LlmClient(Protocol):
    """Cliente de un modelo de lenguaje: recibe un prompt y devuelve texto."""

    async def generate(self, prompt: str) -> str: ...


@runtime_checkable
class Notifier(Protocol):
    """Canal de notificación: envía un mensaje ya formateado."""

    async def send(self, message: str) -> None: ...


@runtime_checkable
class FailureHistory(Protocol):
    """Historial de diagnósticos (persistido en el backend).

    `record` guarda un diagnóstico; `has_changed` indica si el estado de la
    cámara difiere del último registrado (para deduplicar alertas). El mapeo del
    estado a la severidad del backend es responsabilidad del adaptador.
    """

    async def record(self, diagnosis: Diagnosis) -> None: ...

    async def has_changed(self, diagnosis: Diagnosis) -> bool: ...

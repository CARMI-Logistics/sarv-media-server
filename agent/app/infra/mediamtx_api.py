"""Adaptador HTTP a la API REST de MediaMTX (puerto 9997).

Única responsabilidad (SRP): hablar con la API y mapear su JSON a modelos de
dominio. Implementa los puertos PathsSource y HlsMuxersSource (ver domain.ports).
"""

from __future__ import annotations

import httpx

from app.domain.models import HlsMuxerState, PathState


class MediaMtxUnavailable(RuntimeError):
    """La API de MediaMTX no está accesible o respondió con error."""


class MediaMtxApiClient:
    """Cliente de solo lectura para la API v3 de MediaMTX."""

    def __init__(self, base_url: str, timeout: float = 5.0) -> None:
        self._base_url = base_url.rstrip("/")
        self._timeout = timeout

    async def list_paths(self) -> list[PathState]:
        data = await self._get("/v3/paths/list")
        return [self._to_path_state(item) for item in data.get("items", [])]

    async def list_hls_muxers(self) -> list[HlsMuxerState]:
        data = await self._get("/v3/hlsmuxers/list")
        return [self._to_hls_state(item) for item in data.get("items", [])]

    async def _get(self, path: str) -> dict:
        url = f"{self._base_url}{path}"
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                resp = await client.get(url)
                resp.raise_for_status()
                return resp.json()
        except (httpx.HTTPError, ValueError) as exc:
            raise MediaMtxUnavailable(f"Fallo consultando {url}: {exc}") from exc

    @staticmethod
    def _to_path_state(item: dict) -> PathState:
        source = item.get("source") or {}
        return PathState(
            name=item.get("name", ""),
            ready=bool(item.get("ready", False)),
            source_type=source.get("type"),
            tracks=tuple(item.get("tracks") or ()),
            readers=len(item.get("readers") or []),
            bytes_received=int(item.get("bytesReceived", 0) or 0),
            ready_since=item.get("readyTime"),
        )

    @staticmethod
    def _to_hls_state(item: dict) -> HlsMuxerState:
        return HlsMuxerState(
            path=item.get("path", ""),
            created=item.get("created"),
            last_request=item.get("lastRequest"),
            bytes_sent=int(item.get("bytesSent", 0) or 0),
        )

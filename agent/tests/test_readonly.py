"""Invariante de solo-lectura del agente (HU 1.3, tarea 8).

Guarda de regresión: el cliente de la API de MediaMTX no debe exponer ningún
método que modifique el servidor. Si alguien agrega un método mutante, este test
falla y obliga a revisarlo (en Fase 1 el agente SOLO observa).
"""

import inspect

from app.infra.mediamtx_api import MediaMtxApiClient

_MUTATING_VERBS = (
    "add",
    "patch",
    "delete",
    "create",
    "update",
    "remove",
    "post",
    "put",
    "kick",
    "set",
)


def _public_methods(cls) -> list[str]:
    return [
        name
        for name, _ in inspect.getmembers(cls, predicate=inspect.isfunction)
        if not name.startswith("_")
    ]


def test_mediamtx_client_exposes_only_read_methods() -> None:
    methods = _public_methods(MediaMtxApiClient)
    # Debe tener las lecturas esperadas...
    assert "list_paths" in methods
    assert "list_hls_muxers" in methods
    # ...y ningún método mutante.
    for name in methods:
        assert not any(verb in name.lower() for verb in _MUTATING_VERBS), (
            f"método potencialmente mutante en el cliente de MediaMTX: {name}"
        )

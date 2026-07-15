"""Pruebas de la autorización de endpoints (HU 1.3).

Se prueba la dependencia directamente, parcheando la config del token.
"""

import asyncio
from types import SimpleNamespace

import pytest
from fastapi import HTTPException

import app.security.auth as auth


def _patch_token(monkeypatch, token) -> None:
    monkeypatch.setattr(
        auth, "get_settings", lambda: SimpleNamespace(agent_api_token=token)
    )


def test_fails_closed_when_token_not_configured(monkeypatch) -> None:
    _patch_token(monkeypatch, None)
    with pytest.raises(HTTPException) as exc:
        asyncio.run(auth.require_auth(authorization="Bearer whatever"))
    assert exc.value.status_code == 503


def test_valid_token_is_accepted(monkeypatch) -> None:
    _patch_token(monkeypatch, "s3cret")
    # No debe lanzar
    asyncio.run(auth.require_auth(authorization="Bearer s3cret"))


def test_wrong_token_is_rejected(monkeypatch) -> None:
    _patch_token(monkeypatch, "s3cret")
    with pytest.raises(HTTPException) as exc:
        asyncio.run(auth.require_auth(authorization="Bearer nope"))
    assert exc.value.status_code == 401


def test_missing_header_is_rejected(monkeypatch) -> None:
    _patch_token(monkeypatch, "s3cret")
    with pytest.raises(HTTPException) as exc:
        asyncio.run(auth.require_auth(authorization=None))
    assert exc.value.status_code == 401

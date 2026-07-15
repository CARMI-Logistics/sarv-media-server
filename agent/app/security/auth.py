"""Autorización de los endpoints de diagnóstico (HU 1.3).

Token compartido por cabecera `Authorization: Bearer <token>`. Proporcional para
un servicio interno. Comparación en tiempo constante (evita timing attacks).

Fail-closed: si no hay AGENT_API_TOKEN configurado, los endpoints protegidos
responden 503 en lugar de quedar abiertos.
"""

from __future__ import annotations

import secrets

from fastapi import Header, HTTPException, status

from app.config import get_settings


async def require_auth(authorization: str | None = Header(default=None)) -> None:
    token = get_settings().agent_api_token

    if not token:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Autorización no configurada: define AGENT_API_TOKEN.",
        )

    expected = f"Bearer {token}"
    if not authorization or not secrets.compare_digest(authorization, expected):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="No autorizado.",
            headers={"WWW-Authenticate": "Bearer"},
        )

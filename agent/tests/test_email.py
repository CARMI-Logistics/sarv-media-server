"""Pruebas del adaptador de email (EmailNotifier) con transporte simulado.

Usa httpx.MockTransport (sin dependencias nuevas): captura las requests y
responde por destinatario para ejercitar el éxito, el fallo parcial y el total.
"""

import asyncio
import json

import httpx
import pytest

from app.infra.email import EmailNotifier, NotifierError

URL = "http://email-svc/email/sendgrid"


def _transport(status_by_recipient: dict[str, int] | None = None):
    """MockTransport que registra los bodies y responde según el destinatario."""
    status_by_recipient = status_by_recipient or {}
    sent: list[dict] = []

    def handler(request: httpx.Request) -> httpx.Response:
        body = json.loads(request.content)
        sent.append(body)
        return httpx.Response(status_by_recipient.get(body["to"], 201))

    return httpx.MockTransport(handler), sent


def test_sends_one_email_per_recipient() -> None:
    transport, sent = _transport()
    notifier = EmailNotifier(URL, ["a@x.com", "b@y.com"], transport=transport)
    message = "⚠️ Reporte de cámaras: 1/2 con problemas\n\n🔴 cam [down]"

    asyncio.run(notifier.send(message))

    assert [m["to"] for m in sent] == ["a@x.com", "b@y.com"]
    # Asunto = primera línea; cuerpo = mensaje completo.
    assert sent[0]["subject"] == "⚠️ Reporte de cámaras: 1/2 con problemas"
    assert sent[0]["text"] == message


def test_partial_failure_does_not_raise() -> None:
    transport, sent = _transport({"b@y.com": 500})
    notifier = EmailNotifier(URL, ["a@x.com", "b@y.com"], transport=transport)

    # No lanza: a@x.com sí recibió (best-effort), pero se intentó con ambos.
    asyncio.run(notifier.send("resumen\ncuerpo"))
    assert [m["to"] for m in sent] == ["a@x.com", "b@y.com"]


def test_raises_when_all_recipients_fail() -> None:
    transport, _ = _transport({"a@x.com": 500, "b@y.com": 503})
    notifier = EmailNotifier(URL, ["a@x.com", "b@y.com"], transport=transport)

    with pytest.raises(NotifierError):
        asyncio.run(notifier.send("resumen\ncuerpo"))


def test_subject_falls_back_when_message_blank() -> None:
    transport, sent = _transport()
    notifier = EmailNotifier(URL, ["a@x.com"], transport=transport)

    asyncio.run(notifier.send("   \n  "))
    assert sent[0]["subject"] == "Reporte de cámaras (stream-agent)"

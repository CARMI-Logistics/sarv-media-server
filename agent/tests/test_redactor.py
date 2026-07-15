"""Pruebas del redactor de datos sensibles (🔴 HU 1.3).

Casos reales: URLs RTSP con credenciales tal como aparecen en el mediamtx.yml
y en los logs de MediaMTX. El objetivo es que NINGUNA credencial sobreviva.
"""

from app.security.redactor import RegexRedactor

R = RegexRedactor()


def test_rtsp_url_credentials_are_masked() -> None:
    src = "rtsp://zeus:zeus@10.0.0.45/defaultPrimary?mtu=1440&streamType=m"
    out = R.redact(src)
    assert "zeus:zeus@" not in out
    assert out == "rtsp://***:***@10.0.0.45/defaultPrimary?mtu=1440&streamType=m"


def test_rtsp_url_in_log_line_is_masked() -> None:
    line = "2026/07/14 16:25:01 ERR [path cam] source rtsp://root:dynamics8249@10.0.0.30/axis-media/media.amp failed"
    out = R.redact(line)
    assert "dynamics8249" not in out
    assert "root:dynamics8249" not in out
    # se conservan IP y contexto para diagnosticar
    assert "10.0.0.30" in out
    assert "failed" in out


def test_ip_host_and_port_without_credentials_is_preserved() -> None:
    src = "rtsp://10.0.0.222:554/rtsp/defaultPrimary-0?streamType=m"
    assert R.redact(src) == src  # no hay credenciales que ocultar


def test_internal_service_url_is_preserved() -> None:
    assert R.redact("http://mediamtx:9997/v3/paths/list") == "http://mediamtx:9997/v3/paths/list"


def test_query_string_secret_is_masked() -> None:
    assert R.redact("stream?token=abc123&streamType=m") == "stream?token=***&streamType=m"


def test_authorization_header_is_masked() -> None:
    out = R.redact("Authorization: Bearer eyJhbGciOi.super.secret")
    assert "eyJhbGciOi" not in out
    assert out == "Authorization: ***"


def test_key_value_password_is_masked() -> None:
    assert R.redact("password: hunter2") == "password: ***"
    assert R.redact("api_key=SECRETKEY") == "api_key=***"


def test_non_secret_text_is_untouched() -> None:
    text = "cam entrance-guardhouse-bottom is not receiving data (H265)"
    assert R.redact(text) == text


def test_redact_obj_walks_nested_structures() -> None:
    payload = {
        "cameras": [
            {
                "name": "cam1",
                "recent_logs": (
                    "source rtsp://zeus:zeus@10.0.0.46/defaultPrimary failed",
                ),
            }
        ]
    }
    out = R.redact_obj(payload)
    dumped = str(out)
    assert "zeus:zeus" not in dumped
    assert "***:***@10.0.0.46" in dumped
    # preserva tipos y estructura
    assert isinstance(out["cameras"][0]["recent_logs"], tuple)


def test_redaction_is_idempotent() -> None:
    src = "rtsp://root:dynamics8249@10.0.0.34/axis-media/media.amp"
    once = R.redact(src)
    assert R.redact(once) == once

"""Configuración del agente (pydantic-settings).

Fuente: variables de entorno / archivo .env. Ver .env.example.
"""

from functools import lru_cache

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # --- Servicio ---
    service_name: str = "stream-agent"
    host: str = "0.0.0.0"
    port: int = 8090
    log_level: str = "info"

    # --- Autorización (HU 1.3): token para los endpoints de diagnóstico ---
    agent_api_token: str | None = None

    # --- MediaMTX ---
    mediamtx_api_url: str = "http://mediamtx:9997"
    mediamtx_hls_url: str = "http://mediamtx:8888"
    mediamtx_log_path: str = "/shared-logs/mediamtx.log"

    # --- Gemini (tarea de diagnóstico IA, HU 1.1) ---
    gemini_api_key: str | None = None
    gemini_model: str = "gemini-2.5-flash"

    # --- Programación (HU 1.2) ---
    scan_interval_seconds: int = 300

    # --- Notificación (HU 1.2) ---
    notify_channel: str = "slack"
    slack_webhook_url: str | None = None


@lru_cache
def get_settings() -> Settings:
    return Settings()

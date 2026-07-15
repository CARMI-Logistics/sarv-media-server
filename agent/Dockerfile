# =============================================================================
# stream-agent - Agente de diagnóstico de cámaras (Fase 1, solo lectura)
# =============================================================================
FROM python:3.12-slim

ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PIP_NO_CACHE_DIR=1

WORKDIR /app

# Instalar dependencias + la app (fuente única de deps en pyproject.toml).
# Extras: 'ia' (LangGraph + Gemini, diagnóstico) y 'sched' (APScheduler,
# ejecución automática).
COPY pyproject.toml ./
COPY app ./app
RUN pip install --no-cache-dir ".[ia,sched]"

EXPOSE 8090

# Healthcheck sin curl (la imagen slim no lo incluye)
HEALTHCHECK --interval=10s --timeout=5s --retries=3 --start-period=10s \
  CMD python -c "import urllib.request,sys; sys.exit(0 if urllib.request.urlopen('http://localhost:8090/health').status==200 else 1)"

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8090"]

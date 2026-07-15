"""Estado compartido del grafo de LangGraph."""

from __future__ import annotations

from typing import TypedDict

from app.domain.models import CameraClassification, CameraSnapshot, Diagnosis


class AgentState(TypedDict, total=False):
    snapshots: list[CameraSnapshot]
    classifications: list[CameraClassification]
    diagnoses: list[Diagnosis]

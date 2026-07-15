"""Grafo de LangGraph: orquesta el pipeline collect → classify → diagnose.

LangGraph vive SOLO en el borde (orquestación). Cada nodo es un envoltorio
delgado que delega en un service; la lógica de negocio no depende de LangGraph.
Las dependencias se inyectan (DIP) al construir el grafo.
"""

from __future__ import annotations

from langgraph.graph import END, StateGraph

from app.services.classifier import HealthClassifier
from app.services.collector import StateCollector
from app.services.diagnoser import DiagnoserService
from app.state import AgentState


def build_graph(
    collector: StateCollector,
    classifier: HealthClassifier,
    diagnoser: DiagnoserService,
):
    async def collect_node(state: AgentState) -> AgentState:
        return {"snapshots": await collector.collect()}

    async def classify_node(state: AgentState) -> AgentState:
        return {"classifications": classifier.classify_all(state["snapshots"])}

    async def diagnose_node(state: AgentState) -> AgentState:
        return {"diagnoses": await diagnoser.diagnose_all(state["classifications"])}

    graph = StateGraph(AgentState)
    graph.add_node("collect", collect_node)
    graph.add_node("classify", classify_node)
    graph.add_node("diagnose", diagnose_node)
    graph.set_entry_point("collect")
    graph.add_edge("collect", "classify")
    graph.add_edge("classify", "diagnose")
    graph.add_edge("diagnose", END)
    return graph.compile()

"""Phago - Self-evolving knowledge substrates through biological computing primitives.

Phago maps cellular biology mechanisms to computational operations for
autonomous knowledge graph construction.

Example:
    >>> from phago import Colony, Position
    >>> colony = Colony()
    >>> colony.ingest_document("Biology 101", "The cell membrane controls transport.")
    >>> colony.run(50)
    >>> results = colony.query("cell membrane")
    >>> for r in results:
    ...     print(f"{r.label}: {r.score:.3f}")
"""

from phago._phago import (
    Colony,
    ColonyConfig,
    ColonyStats,
    Position,
    QueryResult,
)

__all__ = [
    "Colony",
    "ColonyConfig",
    "ColonyStats",
    "Position",
    "QueryResult",
]

__version__ = "0.2.0"

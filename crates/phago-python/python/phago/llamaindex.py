"""LlamaIndex integration for Phago.

This module provides a LlamaIndex-compatible knowledge store that uses
Phago's biological knowledge graph for document storage and retrieval.

Example:
    >>> from phago.llamaindex import PhagoKnowledgeStore
    >>> from llama_index.core import VectorStoreIndex
    >>>
    >>> store = PhagoKnowledgeStore()
    >>> store.add_document("title", "content about cells and proteins")
    >>> results = store.query("cells")
"""

from typing import Any, Dict, List, Optional, Sequence

try:
    from llama_index.core.schema import Document, NodeWithScore, TextNode
    from llama_index.core.storage.docstore.types import BaseDocumentStore
except ImportError:
    raise ImportError(
        "LlamaIndex is required for this module. "
        "Install it with: pip install 'phago[llamaindex]'"
    )

from phago import Colony, Position


class PhagoKnowledgeStore(BaseDocumentStore):
    """LlamaIndex document store backed by Phago biological knowledge graph.

    This store uses Phago's colony system to store and retrieve documents,
    allowing the knowledge graph to grow and evolve organically.

    Example:
        >>> store = PhagoKnowledgeStore()
        >>> doc = Document(text="Cells are the basic unit of life", id_="doc1")
        >>> store.add_documents([doc])
        >>> retrieved = store.get_all_document_hashes()
    """

    def __init__(self, colony: Optional[Colony] = None):
        """Initialize the knowledge store.

        Args:
            colony: Existing Colony instance, or None to create new one
        """
        self.colony = colony or Colony()
        self._document_map: Dict[str, Dict] = {}  # id -> doc metadata
        self._node_count = 0

    @classmethod
    def class_name(cls) -> str:
        return "PhagoKnowledgeStore"

    def add_documents(
        self,
        docs: Sequence[Document],
        allow_update: bool = True,
        **kwargs
    ) -> None:
        """Add documents to the store.

        Args:
            docs: Sequence of Document objects
            allow_update: Whether to allow updating existing docs
        """
        for i, doc in enumerate(docs):
            doc_id = doc.id_ or f"doc_{self._node_count}"
            self._node_count += 1

            # Position in 2D space
            pos = Position(float(i * 2), 0.0)

            # Extract title from metadata or use id
            title = doc.metadata.get("title", doc_id) if doc.metadata else doc_id

            # Ingest into colony
            self.colony.ingest_document(title, doc.text, pos)

            # Store metadata
            self._document_map[doc_id] = {
                "text": doc.text,
                "metadata": doc.metadata or {},
                "title": title,
            }

        # Process all documents
        self.colony.run(len(docs) * 10)

    def delete_document(self, doc_id: str, raise_error: bool = True) -> None:
        """Delete a document by ID.

        Note: Phago's knowledge graph doesn't support direct deletion,
        so this only removes from the local mapping.
        """
        if doc_id in self._document_map:
            del self._document_map[doc_id]
        elif raise_error:
            raise ValueError(f"Document {doc_id} not found")

    def get_document(self, doc_id: str, raise_error: bool = True) -> Optional[Document]:
        """Get a document by ID."""
        if doc_id in self._document_map:
            data = self._document_map[doc_id]
            return Document(
                text=data["text"],
                id_=doc_id,
                metadata=data["metadata"],
            )
        elif raise_error:
            raise ValueError(f"Document {doc_id} not found")
        return None

    def document_exists(self, doc_id: str) -> bool:
        """Check if document exists."""
        return doc_id in self._document_map

    def get_all_document_hashes(self) -> Dict[str, str]:
        """Get all document hashes (IDs)."""
        return {doc_id: doc_id for doc_id in self._document_map}

    def set_document_hash(self, doc_id: str, doc_hash: str) -> None:
        """Set document hash (no-op for Phago)."""
        pass

    def get_document_hash(self, doc_id: str) -> Optional[str]:
        """Get document hash."""
        if doc_id in self._document_map:
            return doc_id
        return None

    def query(
        self,
        query: str,
        max_results: int = 10,
        alpha: float = 0.5
    ) -> List[NodeWithScore]:
        """Query the knowledge graph.

        Args:
            query: Search query
            max_results: Maximum number of results
            alpha: Balance between TF-IDF (0) and graph (1) scores

        Returns:
            List of NodeWithScore objects
        """
        results = self.colony.query(query, alpha=alpha, max_results=max_results)

        nodes = []
        for r in results:
            node = TextNode(text=r.label)
            node_with_score = NodeWithScore(
                node=node,
                score=r.score,
            )
            nodes.append(node_with_score)

        return nodes

    @property
    def stats(self) -> Dict[str, Any]:
        """Get colony statistics."""
        s = self.colony.stats()
        return {
            "tick": s.tick,
            "nodes": s.graph_nodes,
            "edges": s.graph_edges,
            "documents": s.documents_total,
            "documents_digested": s.documents_digested,
            "agents": s.agents_alive,
            "stored_documents": len(self._document_map),
        }


class PhagoRetriever:
    """Simple retriever compatible with LlamaIndex query engines.

    Example:
        >>> from phago.llamaindex import PhagoRetriever
        >>> retriever = PhagoRetriever()
        >>> retriever.add_texts(["Cells are fundamental units", "DNA encodes proteins"])
        >>> nodes = retriever.retrieve("cells")
    """

    def __init__(
        self,
        colony: Optional[Colony] = None,
        k: int = 4,
        alpha: float = 0.5
    ):
        """Initialize retriever.

        Args:
            colony: Existing colony or None to create new
            k: Number of results to retrieve
            alpha: Balance between TF-IDF and graph scores
        """
        self.colony = colony or Colony()
        self.k = k
        self.alpha = alpha
        self._doc_count = 0

    def add_texts(
        self,
        texts: List[str],
        titles: Optional[List[str]] = None
    ) -> None:
        """Add text documents.

        Args:
            texts: List of text content
            titles: Optional list of titles
        """
        titles = titles or [f"doc_{i}" for i in range(len(texts))]

        for i, (title, text) in enumerate(zip(titles, texts)):
            pos = Position(float(self._doc_count * 2), 0.0)
            self.colony.ingest_document(title, text, pos)
            self._doc_count += 1

        self.colony.run(len(texts) * 10)

    def retrieve(self, query: str) -> List[NodeWithScore]:
        """Retrieve nodes matching query.

        Args:
            query: Search query

        Returns:
            List of NodeWithScore objects
        """
        results = self.colony.query(query, alpha=self.alpha, max_results=self.k)

        nodes = []
        for r in results:
            node = TextNode(
                text=r.label,
                metadata={
                    "score": r.score,
                    "tfidf_score": r.tfidf_score,
                    "graph_score": r.graph_score,
                }
            )
            node_with_score = NodeWithScore(node=node, score=r.score)
            nodes.append(node_with_score)

        return nodes

    async def aretrieve(self, query: str) -> List[NodeWithScore]:
        """Async retrieve (delegates to sync version)."""
        return self.retrieve(query)

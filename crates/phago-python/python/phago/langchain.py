"""LangChain integration for Phago.

This module provides a LangChain-compatible memory class that uses
Phago's biological knowledge graph for persistent memory.

Example:
    >>> from langchain.chains import ConversationChain
    >>> from langchain_openai import ChatOpenAI
    >>> from phago.langchain import PhagoMemory
    >>>
    >>> memory = PhagoMemory()
    >>> llm = ChatOpenAI()
    >>> chain = ConversationChain(llm=llm, memory=memory)
    >>> response = chain.run("Tell me about cells")
"""

from typing import Any, Dict, List, Optional

try:
    from langchain.memory.chat_memory import BaseChatMemory
    from langchain.schema import BaseMessage, HumanMessage, AIMessage
except ImportError:
    raise ImportError(
        "LangChain is required for this module. "
        "Install it with: pip install 'phago[langchain]'"
    )

from phago import Colony, Position


class PhagoMemory(BaseChatMemory):
    """LangChain memory backed by Phago biological knowledge graph.

    This memory class stores conversation history in a Phago colony,
    allowing the knowledge graph to grow and evolve with each interaction.

    Attributes:
        colony: The Phago Colony instance
        human_prefix: Prefix for human messages (default: "Human")
        ai_prefix: Prefix for AI messages (default: "AI")
        memory_key: Key to use in memory dict (default: "history")
        input_key: Key for input in chain (default: "input")
        output_key: Key for output in chain (default: "output")
    """

    colony: Colony = None
    human_prefix: str = "Human"
    ai_prefix: str = "AI"
    memory_key: str = "history"
    input_key: str = "input"
    output_key: str = "output"
    return_messages: bool = False

    # Internal tracking
    _message_count: int = 0
    _ticks_per_message: int = 5

    class Config:
        arbitrary_types_allowed = True

    def __init__(
        self,
        colony: Optional[Colony] = None,
        human_prefix: str = "Human",
        ai_prefix: str = "AI",
        memory_key: str = "history",
        input_key: str = "input",
        output_key: str = "output",
        return_messages: bool = False,
        ticks_per_message: int = 5,
        **kwargs
    ):
        """Initialize PhagoMemory.

        Args:
            colony: Existing Colony instance, or None to create new one
            human_prefix: Prefix for human messages
            ai_prefix: Prefix for AI messages
            memory_key: Key to use when returning memory
            input_key: Key for input in chain dict
            output_key: Key for output in chain dict
            return_messages: Whether to return Message objects or string
            ticks_per_message: Number of simulation ticks per message ingestion
        """
        super().__init__(**kwargs)
        self.colony = colony or Colony()
        self.human_prefix = human_prefix
        self.ai_prefix = ai_prefix
        self.memory_key = memory_key
        self.input_key = input_key
        self.output_key = output_key
        self.return_messages = return_messages
        self._message_count = 0
        self._ticks_per_message = ticks_per_message

    @property
    def memory_variables(self) -> List[str]:
        """Return memory variables."""
        return [self.memory_key]

    def load_memory_variables(self, inputs: Dict[str, Any]) -> Dict[str, Any]:
        """Load memory variables for chain.

        This queries the Phago colony for relevant concepts based on input.
        """
        # Get input text
        input_text = inputs.get(self.input_key, "")

        # Query the knowledge graph for relevant concepts
        if input_text:
            results = self.colony.query(input_text, max_results=5, alpha=0.5)
            concepts = [f"{r.label} (score: {r.score:.2f})" for r in results]
        else:
            concepts = []

        # Build context from concepts
        if self.return_messages:
            # Return as messages
            messages = []
            if concepts:
                context = f"Relevant knowledge: {', '.join(concepts)}"
                messages.append(AIMessage(content=context))
            return {self.memory_key: messages}
        else:
            # Return as string
            if concepts:
                return {self.memory_key: f"Relevant concepts: {', '.join(concepts)}"}
            return {self.memory_key: ""}

    def save_context(self, inputs: Dict[str, Any], outputs: Dict[str, Any]) -> None:
        """Save context from this conversation turn.

        Ingests both input and output into the Phago colony.
        """
        input_text = inputs.get(self.input_key, "")
        output_text = outputs.get(self.output_key, "")

        # Position messages in 2D space based on time
        x = self._message_count * 2.0
        y = 0.0

        # Ingest human message
        if input_text:
            self.colony.ingest_document(
                f"{self.human_prefix} message {self._message_count}",
                input_text,
                Position(x, y)
            )

        # Ingest AI response
        if output_text:
            self.colony.ingest_document(
                f"{self.ai_prefix} response {self._message_count}",
                output_text,
                Position(x + 1.0, y)
            )

        # Run simulation to process new information
        self.colony.run(self._ticks_per_message)
        self._message_count += 1

    def clear(self) -> None:
        """Clear memory by creating a new colony."""
        self.colony = Colony()
        self._message_count = 0

    @property
    def stats(self) -> dict:
        """Get current colony statistics."""
        s = self.colony.stats()
        return {
            "tick": s.tick,
            "nodes": s.graph_nodes,
            "edges": s.graph_edges,
            "documents": s.documents_total,
            "agents": s.agents_alive,
        }

    def query(self, query: str, max_results: int = 10) -> List[dict]:
        """Query the knowledge graph directly.

        Args:
            query: Search query
            max_results: Maximum results to return

        Returns:
            List of result dicts with label, score, tfidf_score, graph_score
        """
        results = self.colony.query(query, max_results=max_results)
        return [
            {
                "label": r.label,
                "score": r.score,
                "tfidf_score": r.tfidf_score,
                "graph_score": r.graph_score,
            }
            for r in results
        ]


class PhagoRetriever:
    """Simple retriever for use with LangChain RAG chains.

    Example:
        >>> from phago.langchain import PhagoRetriever
        >>> retriever = PhagoRetriever()
        >>> retriever.add_document("Biology 101", "Cells are the basic unit of life...")
        >>> docs = retriever.get_relevant_documents("cells")
    """

    def __init__(self, colony: Optional[Colony] = None, k: int = 4):
        """Initialize retriever.

        Args:
            colony: Existing colony or None to create new
            k: Number of documents to retrieve
        """
        self.colony = colony or Colony()
        self.k = k

    def add_document(self, title: str, content: str, position: Optional[Position] = None) -> None:
        """Add a document to the knowledge base.

        Args:
            title: Document title
            content: Document content
            position: Optional position in 2D space
        """
        self.colony.ingest_document(title, content, position)
        self.colony.run(15)  # Process the document

    def add_documents(self, documents: List[Dict[str, str]]) -> None:
        """Add multiple documents.

        Args:
            documents: List of dicts with 'title' and 'content' keys
        """
        for i, doc in enumerate(documents):
            pos = Position(float(i * 2), 0.0)
            self.colony.ingest_document(doc["title"], doc["content"], pos)
        self.colony.run(len(documents) * 10)

    def get_relevant_documents(self, query: str) -> List[Dict[str, Any]]:
        """Get documents relevant to query.

        Args:
            query: Search query

        Returns:
            List of document dicts with 'content' and 'metadata' keys
        """
        results = self.colony.query(query, max_results=self.k)
        return [
            {
                "content": r.label,
                "metadata": {
                    "score": r.score,
                    "tfidf_score": r.tfidf_score,
                    "graph_score": r.graph_score,
                }
            }
            for r in results
        ]

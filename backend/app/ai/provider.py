"""Provider-independent messages and streamed events."""
import json
from collections.abc import AsyncIterator
from dataclasses import dataclass
from typing import Protocol

from ..contracts import ModelHealth


@dataclass(frozen=True)
class Text:
    value: str


@dataclass(frozen=True)
class Call:
    id: str
    name: str
    arguments: str

    def wire(self) -> dict:
        # Jinja must receive a valid argument object, even for a rejected call.
        # The executor still validates the original unmodified model input.
        try:
            parsed = json.loads(self.arguments)
        except (ValueError, RecursionError):
            parsed = None
        arguments = self.arguments if isinstance(parsed, dict) else "{}"
        return {"id": self.id, "type": "function",
                "function": {"name": self.name, "arguments": arguments}}


@dataclass(frozen=True)
class Finished:
    calls: tuple[Call, ...] = ()
    reasoning: str = ""


class ModelProvider(Protocol):
    async def health(self) -> ModelHealth: ...
    async def capacity(self) -> int: ...
    async def prompt_tokens(self, messages: list[dict], tools: list[dict]) -> int: ...
    def stream(self, messages: list[dict], tools: list[dict]) -> AsyncIterator[Text | Finished]: ...

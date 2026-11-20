from typing import Annotated, Any, Literal
from uuid import UUID

from pydantic import BaseModel, ConfigDict, Field, StringConstraints


class Input(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)


class ChatRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    conversation_id: UUID
    message: Annotated[str, StringConstraints(strip_whitespace=True, min_length=1, max_length=8192)]


class Conversation(BaseModel):
    id: str
    title: str
    created_at: str
    updated_at: str


TurnState = Literal["generating", "complete", "interrupted", "error"]


class ToolActivity(BaseModel):
    call_id: str
    name: str
    ok: bool
    result: dict[str, Any]


class Turn(BaseModel):
    seq: int
    id: str
    created_at: str
    user: str
    assistant: str
    status: TurnState
    error: str | None
    tools: list[ToolActivity]


class ConversationDetail(Conversation):
    turns: list[Turn]
    next_before: int | None


class ModelHealth(BaseModel):
    model: str
    model_server: Literal["online", "offline", "loading", "misconfigured"]
    diagnostic: str


class SystemStatus(ModelHealth):
    backend: Literal["online"] = "online"
    status: Literal["ready", "degraded"]
    version: str
    context_tokens: int
    max_output_tokens: int


class ToolInfo(BaseModel):
    name: str
    description: str
    permission: Literal["read-only"] = "read-only"
    parameters: dict[str, Any]


class ManualRun(Input):
    arguments: dict[str, Any] = Field(default_factory=dict)


class ToolResult(BaseModel):
    ok: bool
    result: dict[str, Any]


class PublicError(Exception):
    """A stable, user-readable error safe to send across the API."""

    def __init__(self, code: str, message: str, status: int = 400):
        super().__init__(message)
        self.code = code
        self.message = message
        self.status = status

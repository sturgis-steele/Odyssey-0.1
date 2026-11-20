import json
import logging
import sqlite3
import time
from collections.abc import AsyncIterator
from contextlib import aclosing

from ..config import Settings
from ..contracts import PublicError
from ..database import Store
from ..tools.registry import Registry
from .provider import Finished, ModelProvider, Text

log = logging.getLogger("odyssey")


class TurnRun:
    def __init__(self, settings: Settings, store: Store, model: ModelProvider, registry: Registry,
                 cid: str, tid: str, message: str):
        self.settings, self.store, self.model, self.registry = settings, store, model, registry
        self.cid, self.tid = cid, tid
        self.transcript = [{"role": "user", "content": message}]
        self.activity: list[dict] = []
        self.text = ""
        self.closed = False

    def save(self, status: str, error: str | None = None):
        # Reasoning may be needed inside a tool round. Never expose or store it.
        stored = [{k: v for k, v in msg.items() if k != "reasoning_content"} for msg in self.transcript]
        self.store.checkpoint(self.tid, self.text, self.activity, stored, status, error)
        self.closed = status != "generating"

    def interrupt(self):
        if not self.closed:
            self.save("interrupted", "Generation was interrupted. Send another message to continue.")

    async def fit(self, history: list[list[dict]], tools: list[dict], capacity: int) -> list[dict]:
        while True:
            messages = [{"role": "system", "content": self.settings.system_prompt}]
            for turn in history:
                messages.extend(turn)
            messages.extend(self.transcript)
            size = await self.model.prompt_tokens(messages, tools)
            if size + self.settings.max_output_tokens + 8 <= capacity:
                return messages
            if not history:
                raise PublicError("context_limit", "This turn exceeds the available context. Try a shorter message.", 422)
            history.pop(0)

    async def events(self) -> AsyncIterator[tuple[str, dict]]:
        yield "message_start", {"turn_id": self.tid, "conversation_id": self.cid}
        try:
            history = self.store.history(self.cid, self.settings.history_turns)
            tools = self.registry.definitions()
            capacity = min(self.settings.context_tokens, await self.model.capacity())
            last_save = time.monotonic()
            for iteration in range(self.settings.max_tool_iterations + 1):
                messages = await self.fit(history, tools, capacity)
                round_text = ""
                finish = None
                async with aclosing(self.model.stream(messages, tools)) as stream:
                    async for part in stream:
                        if isinstance(part, Text):
                            round_text += part.value
                            self.text += part.value
                            if time.monotonic() - last_save >= 1:
                                self.save("generating")
                                last_save = time.monotonic()
                            yield "text_delta", {"text": part.value}
                        elif isinstance(part, Finished):
                            finish = part
                if finish is None:
                    raise PublicError("incomplete", "The model stopped before completing its response.", 502)
                message = {"role": "assistant", "content": round_text}
                if not finish.calls:
                    if not round_text.strip() or any(marker in round_text for marker in ("<tool_call>", "<|tool_call_start|>")):
                        raise PublicError("tool_template", "Expected a structured model response. Check the tool-aware chat template.", 502)
                    self.transcript.append(message)
                    self.save("complete")
                    yield "message_complete", {"turn_id": self.tid}
                    return
                if iteration == self.settings.max_tool_iterations:
                    raise PublicError("tool_limit", "The model reached the tool-call limit. Try a more specific request.", 422)
                message["tool_calls"] = [call.wire() for call in finish.calls]
                if finish.reasoning:
                    message["reasoning_content"] = finish.reasoning
                self.transcript.append(message)
                for call in finish.calls:
                    ref = {"call_id": call.id, "name": call.name}
                    yield "tool_start", ref
                    outcome = self.registry.run(call.name, call.arguments).model_dump()
                    self.transcript.append({"role": "tool", "tool_call_id": call.id, "name": call.name,
                                            "content": json.dumps(outcome, ensure_ascii=False)})
                    activity = {**ref, **outcome}
                    self.activity.append(activity)
                    self.save("generating")
                    yield "tool_result", activity
                if round_text:
                    self.text += "\n\n"
                    yield "text_delta", {"text": "\n\n"}
        except PublicError as exc:
            code, details = exc.code, exc.message
        except sqlite3.Error:
            code, details = "storage", "Local storage is temporarily unavailable. Check disk space and retry."
        except Exception as exc:
            log.error("turn_exception", extra={"kind": type(exc).__name__})
            code, details = "internal", "The response could not be completed. Try again."
        log.warning("turn_failed", extra={"kind": code})
        try:
            self.save("error", details)
        except sqlite3.Error:
            log.error("storage_write_failed")
        yield "error", {"turn_id": self.tid, "code": code, "message": details}

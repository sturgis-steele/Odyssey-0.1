"""Only this adapter knows llama.cpp's endpoints and OpenAI stream format."""
import json
from collections.abc import AsyncIterator
from uuid import uuid4

import httpx

from ..config import Settings
from ..contracts import ModelHealth, PublicError
from .provider import Call, Finished, Text


def malformed() -> PublicError:
    return PublicError("model_protocol", "The model returned an incomplete or invalid response. Check its tool-aware chat template.", 502)


def transport_error(exc: httpx.HTTPError) -> PublicError:
    if isinstance(exc, httpx.TimeoutException):
        return PublicError("model_timeout", "The local model timed out. Try a shorter request.", 504)
    return PublicError("model_offline", "The local model connection was lost. Check llama-server and try again.", 503)


def check_response(response: httpx.Response):
    if response.is_success:
        return
    try:
        detail = str(response.json().get("error", "")).lower()
    except (ValueError, AttributeError):
        detail = ""
    if "context" in detail or "exceed_context_size_error" in detail:
        raise PublicError("context_limit", "The model context is full. Shorten the message or start a new conversation.", 422)
    if response.status_code == 503:
        raise PublicError("model_loading", "The local model is loading or temporarily unavailable.", 503)
    raise PublicError("model_request", "llama-server rejected the request. Check the model alias, server version and tool template.", 502)


class LlamaProvider:
    def __init__(self, settings: Settings, client: httpx.AsyncClient):
        self.config = settings
        self.client = client
        self.base = settings.llama_base_url
        self.root = self.base.removesuffix("/v1")

    async def health(self) -> ModelHealth:
        name = self.config.llama_model

        def state(status, diagnostic):
            return ModelHealth(model=name, model_server=status, diagnostic=diagnostic)

        try:
            response = await self.client.get(self.root + "/health", timeout=2)
            if response.status_code == 503:
                return state("loading", "The model server is loading or temporarily unavailable.")
            if not response.is_success or response.json().get("status") != "ok":
                return state("offline", "llama-server did not report ready.")
            response = await self.client.get(self.base + "/models", timeout=2)
            if response.is_success and any(name == item.get("id") or name in (item.get("aliases") or [])
                                           for item in response.json()["data"]):
                return state("online", "Local inference is ready.")
            return state("misconfigured", "The configured model alias is unavailable. Check LLAMA_MODEL.")
        except (httpx.HTTPError, ValueError, KeyError, TypeError, AttributeError):
            return state("offline", "Start llama-server on the configured local address.")

    async def capacity(self) -> int:
        health = await self.health()
        if health.model_server != "online":
            raise PublicError("model_" + health.model_server, health.diagnostic, 503)
        try:
            response = await self.client.get(self.root + "/props", timeout=3)
            check_response(response)
            props = response.json()
            caps = props.get("chat_template_caps", {})
            if caps.get("supports_tools") is False or caps.get("supports_tool_calls") is False:
                raise PublicError("tool_template", "The loaded template does not support tools. Select a tool-aware GGUF or template.", 422)
            value = props["default_generation_settings"]["n_ctx"]
            if type(value) is not int or value <= 0:
                raise malformed()
            return value
        except httpx.HTTPError as exc:
            raise transport_error(exc) from exc
        except (ValueError, TypeError, KeyError, AttributeError) as exc:
            raise malformed() from exc

    def payload(self, messages: list[dict], tools: list[dict]) -> dict:
        return {"model": self.config.llama_model, "messages": messages, "tools": tools,
                "tool_choice": "auto", "parallel_tool_calls": False,
                "chat_template_kwargs": {"enable_thinking": self.config.enable_thinking}}

    async def prompt_tokens(self, messages: list[dict], tools: list[dict]) -> int:
        try:
            rendered = await self.client.post(self.root + "/apply-template", json=self.payload(messages, tools))
            check_response(rendered)
            prompt = rendered.json()["prompt"]
            if not isinstance(prompt, str):
                raise malformed()
            response = await self.client.post(self.root + "/tokenize",
                                              json={"content": prompt, "parse_special": True, "add_special": True})
            check_response(response)
            tokens = response.json()["tokens"]
            if not isinstance(tokens, list):
                raise malformed()
            return len(tokens)
        except httpx.HTTPError as exc:
            raise transport_error(exc) from exc
        except (ValueError, KeyError, TypeError) as exc:
            raise malformed() from exc

    async def stream(self, messages: list[dict], tools: list[dict]) -> AsyncIterator[Text | Finished]:
        payload = {**self.payload(messages, tools), "stream": True, "temperature": self.config.temperature,
                   "max_tokens": self.config.max_output_tokens}
        fragments: dict[int, dict[str, str]] = {}
        finish = None
        reasoning = []
        try:
            async with self.client.stream("POST", self.base + "/chat/completions", json=payload) as response:
                if not response.is_success:
                    await response.aread()
                    check_response(response)
                async for line in response.aiter_lines():
                    if not line or line.startswith(":"):
                        continue
                    if not line.startswith("data:"):
                        raise malformed()
                    data = line[5:].strip()
                    if data == "[DONE]":
                        break
                    packet = json.loads(data)
                    if "error" in packet:
                        raise malformed()
                    choices = packet["choices"]
                    if choices == [] and "usage" in packet:
                        continue
                    choice = choices[0]
                    delta = choice.get("delta") or {}
                    if finish is not None or not isinstance(delta, dict):
                        raise malformed()
                    content = delta.get("content")
                    if content:
                        if not isinstance(content, str):
                            raise malformed()
                        yield Text(content)
                    thought = delta.get("reasoning_content") or delta.get("reasoning")
                    if thought:
                        if not isinstance(thought, str):
                            raise malformed()
                        reasoning.append(thought)
                    for part in delta.get("tool_calls") or []:
                        self.collect(fragments, part)
                    finish = choice.get("finish_reason")
            if finish == "length":
                raise PublicError("output_limit", "The response reached the output token limit. Try a shorter question.", 422)
            if finish not in ("stop", "tool_calls") or (finish == "tool_calls" and not fragments):
                raise malformed()
            calls = tuple(Call(p["id"] or "call_" + uuid4().hex, p["name"], p["arguments"])
                          for _, p in sorted(fragments.items()))
            if any(not call.name for call in calls) or len({call.id for call in calls}) != len(calls):
                raise malformed()
            yield Finished(calls, "".join(reasoning))
        except httpx.HTTPError as exc:
            raise transport_error(exc) from exc
        except (ValueError, IndexError, KeyError, TypeError, AttributeError) as exc:
            raise malformed() from exc

    @staticmethod
    def collect(parts: dict, part: dict):
        index = part.get("index", 0)
        if type(index) is not int or not 0 <= index < 4 or part.get("type", "function") != "function":
            raise malformed()
        slot = parts.setdefault(index, {"id": "", "name": "", "arguments": ""})
        function = part.get("function") or {}
        for field, value in (("id", part.get("id")), ("name", function.get("name")),
                             ("arguments", function.get("arguments"))):
            if value is None:
                continue
            if field == "arguments" and isinstance(value, dict):
                value = json.dumps(value)
            if not isinstance(value, str):
                raise malformed()
            slot[field] += value
            if len(slot[field]) > (2048 if field == "arguments" else 160):
                raise malformed()

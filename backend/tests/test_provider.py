import asyncio
import json

import httpx
import pytest

from app.ai.llama import LlamaProvider
from app.ai.provider import Finished, Text
from app.config import Settings
from app.contracts import PublicError


def run(transport, handler):
    async def scenario():
        async with httpx.AsyncClient(transport=httpx.MockTransport(transport)) as client:
            adapter = LlamaProvider(Settings(_env_file=None), client)
            return await handler(adapter)
    return asyncio.run(scenario())


@pytest.mark.parametrize(("result", "expected"), [
    (httpx.Response(503, json={"error": {"message": "Loading model"}}), "loading"),
    (httpx.Response(200, json={"status": "wrong"}), "offline"),
    (httpx.Response(200, text="{"), "offline"),
])
def test_health_checks_response(result, expected):
    assert run(lambda _: result, lambda provider: provider.health()).model_server == expected


def test_alias_mismatch():
    def handler(req):
        data = {"status": "ok"} if req.url.path == "/health" else {"data": [{"id": "different"}]}
        return httpx.Response(200, json=data)
    assert run(handler, lambda p: p.health()).model_server == "misconfigured"


def test_tokenizer_receives_template_and_tools():
    def handler(req):
        data = json.loads(req.content)
        if req.url.path == "/apply-template":
            assert data["tools"] == [{}] and data["chat_template_kwargs"]["enable_thinking"] is False
            return httpx.Response(200, json={"prompt": "formatted template with tools"})
        assert req.url.path == "/tokenize" and data["content"] == "formatted template with tools"
        return httpx.Response(200, json={"tokens": [1, 2, 3, 4]})
    assert run(handler, lambda p: p.prompt_tokens([], [{}])) == 4


def packet(delta, finish=None):
    return "data: " + json.dumps({"choices": [{"index": 0, "delta": delta, "finish_reason": finish}]}) + "\n\n"


async def collect(provider):
    return [chunk async for chunk in provider.stream([{"role": "user", "content": "hi"}], [])]


def test_fragmented_tool_wire():
    body = (packet({"role": "assistant", "reasoning_content": "private"})
            + packet({"tool_calls": [{"index": 0, "id": "call_a", "function": {"name": "calcu", "arguments": '{"ex'}}]})
            + packet({"tool_calls": [{"index": 0, "function": {"name": "lator", "arguments": 'pression":"2+2"}'}}]})
            + packet({}, "tool_calls") + "data: [DONE]\n\n")
    def handler(req):
        assert req.url.path == "/v1/chat/completions" and json.loads(req.content)["stream"] is True
        return httpx.Response(200, text=body)
    finish, = run(handler, collect)
    assert isinstance(finish, Finished) and finish.reasoning == "private"
    call, = finish.calls
    assert (call.id, call.name, json.loads(call.arguments)) == ("call_a", "calculator", {"expression": "2+2"})


def test_object_arguments_missing_id_and_text_stream():
    parts = run(lambda _: httpx.Response(200, text=packet({"content": "hello"}) + packet({}, "stop")), collect)
    assert parts == [Text("hello"), Finished()]
    response = packet({"tool_calls": [{"function": {"name": "get_current_datetime", "arguments": {}}}]}, "stop")
    finish, = run(lambda _: httpx.Response(200, text=response), collect)
    call, = finish.calls
    assert call.id.startswith("call_") and call.arguments == "{}"


@pytest.mark.parametrize(("body", "code"), [
    ("data: bad\n\n", "model_protocol"),
    (packet({"content": "partial"}) + "data: [DONE]\n\n", "model_protocol"),
    (packet({}, "tool_calls"), "model_protocol"),
    (packet({}, "length"), "output_limit"),
    (packet({"content": ["bad"]}, "stop"), "model_protocol"),
    (packet({"tool_calls": [{"index": 200}]}, "tool_calls"), "model_protocol"),
])
def test_incomplete_model_output(body, code):
    with pytest.raises(PublicError) as failure:
        run(lambda _: httpx.Response(200, text=body), collect)
    assert failure.value.code == code


def test_timeout_context_and_template_errors():
    def timeout(_):
        raise httpx.ReadTimeout("internal-sensitive-address")
    with pytest.raises(PublicError) as failure:
        run(timeout, collect)
    assert failure.value.code == "model_timeout" and "internal" not in failure.value.message
    with pytest.raises(PublicError) as failure:
        run(lambda _: httpx.Response(400, json={"error": {"type": "exceed_context_size_error"}}), collect)
    assert failure.value.code == "context_limit"

    def no_tools(req):
        responses = {"/health": {"status": "ok"}, "/v1/models": {"data": [{"id": "odyssey-local"}]},
                     "/props": {"chat_template_caps": {"supports_tools": False}}}
        return httpx.Response(200, json=responses[req.url.path])
    with pytest.raises(PublicError) as failure:
        run(no_tools, lambda p: p.capacity())
    assert failure.value.code == "tool_template"

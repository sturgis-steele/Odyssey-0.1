import asyncio
import json
import sqlite3

import pytest
from fastapi.testclient import TestClient
from pydantic import ValidationError

from app.ai.provider import Call, Text, Finished
from app.ai.orchestrator import TurnRun
from app.config import Settings
from app.contracts import ModelHealth, PublicError
from app.database import Store
from app.main import create_app
from app.tools.registry import device_registry


class FixtureModel:
    def __init__(self):
        self.rounds = []
        self.requests = []
        self.online = True
        self.token_count = lambda messages: 128

    async def health(self):
        return ModelHealth(model="fixture", model_server="online" if self.online else "offline", diagnostic="Fixture state.")

    async def capacity(self):
        if not self.online:
            raise PublicError("model_offline", "Model is offline.", 503)
        return 4096

    async def prompt_tokens(self, messages, tools):
        return self.token_count(messages)

    async def stream(self, messages, tools):
        self.requests.append(messages.copy())
        for part in self.rounds.pop(0):
            if isinstance(part, Exception):
                raise part
            yield part


@pytest.fixture
def setup(tmp_path):
    return Settings(_env_file=None, app_env="test", database_path=tmp_path / "test.db"), FixtureModel()


def client(cfg, model):
    return TestClient(create_app(cfg, model), base_url="http://127.0.0.1")


def chat(web, cid, question="Question"):
    response = web.post("/api/chat/stream", json={"conversation_id": cid, "message": question})
    assert response.status_code == 200
    return [(lines[0][7:], json.loads(lines[1][6:])) for event in response.text.strip().split("\n\n")
            if (lines := event.splitlines())]


@pytest.mark.parametrize("values", [
    {"backend_host": "0.0.0.0"}, {"llama_base_url": "https://example.com/v1"},
    {"llama_base_url": "http://name@localhost:8080/v1"}, {"llama_base_url": "http://localhost:99999/v1"},
    {"max_tool_iterations": 0}, {"context_tokens": 1024, "max_output_tokens": 1024},
])
def test_configuration(values):
    with pytest.raises(ValidationError):
        Settings(_env_file=None, **values)


def test_status_and_offline_tools(setup):
    cfg, model = setup
    model.online = False
    with client(cfg, model) as web:
        assert web.get("/api/health").json()["backend"] == "online"
        assert web.get("/api/system/status").json()["status"] == "degraded"
        assert web.get("/api/models/current").json()["model_server"] == "offline"
        assert len(web.get("/api/tools").json()) == 3
        assert web.post("/api/tools/calculator/run", json={"arguments": {"expression": "6*7"}}).json()["result"] == {"value": 42}
        assert web.post("/api/tools/unknown/run", json={}).status_code == 404
        assert web.post("/api/conversations", headers={"Origin": "https://external.example"}).status_code == 403
        assert web.get("/api/conversations", headers={"Host": "external.example"}).status_code == 400
        model.online = True
        assert web.get("/api/system/status").json()["status"] == "ready"


def test_real_tools_and_conversation_lifecycle(setup):
    cfg, model = setup
    model.rounds = [[Finished((Call("one", "calculator", '{"expression":"7*6"}'),), "private reasoning")],
                    [Text("That is "), Text("42."), Finished()], [Text("Yes."), Finished()]]
    with client(cfg, model) as web:
        response = web.post("/api/conversations")
        assert response.status_code == 201
        cid = response.json()["id"]
        events = chat(web, cid, "Multiply seven by six")
        assert [kind for kind, _ in events] == ["message_start", "tool_start", "tool_result", "text_delta", "text_delta", "message_complete"]
        tool_response = model.requests[1][-1]
        assert tool_response["role"] == "tool" and tool_response["tool_call_id"] == "one"
        assert json.loads(tool_response["content"])["result"] == {"value": 42}
        assert model.requests[1][-2]["reasoning_content"] == "private reasoning"
        assert web.get("/api/conversations").json()[0]["title"] == "Multiply seven by six"
    with client(cfg, model) as web:
        turn = web.get("/api/conversations/" + cid).json()["turns"][0]
        assert turn["assistant"] == "That is 42." and turn["tools"][0]["call_id"] == "one"
        chat(web, cid, "Do you remember?")
        assert [m["role"] for m in model.requests[-1]] == ["system", "user", "assistant", "tool", "assistant", "user"]
        assert "private reasoning" not in str(model.requests[-1])
        page = web.get(f"/api/conversations/{cid}?limit=1").json()
        assert page["turns"][0]["user"] == "Do you remember?"
        assert web.get(f"/api/conversations/{cid}?before={page['next_before']}").json()["turns"][0]["id"] == turn["id"]
        assert web.delete("/api/conversations/" + cid).status_code == 204
        assert web.get("/api/conversations/" + cid).status_code == 404


def test_model_failure_partial_recovery_and_budget(setup):
    cfg, model = setup
    with client(cfg, model) as web:
        cid = web.post("/api/conversations").json()["id"]
        model.online = False
        assert chat(web, cid)[-1][1]["code"] == "model_offline"
        model.online = True
        model.rounds = [[Text("partial"), PublicError("model_timeout", "Timed out.")], [Text("whole"), Finished()]]
        assert chat(web, cid)[-1][1]["code"] == "model_timeout"
        assert web.get("/api/conversations/" + cid).json()["turns"][1]["assistant"] == "partial"
        chat(web, cid, "Another")
        assert len(model.requests[-1]) == 2
        model.token_count = lambda messages: 5000 if len(messages) > 2 else 100
        model.rounds = [[Text("Truncated history"), Finished()]]
        chat(web, cid)
        assert len(model.requests[-1]) == 2
        model.token_count = lambda messages: 5000
        assert chat(web, cid)[-1][1]["code"] == "context_limit"


def test_unknown_invalid_and_repeated_tool_calls(setup):
    cfg, model = setup
    cfg.max_tool_iterations = 2
    model.rounds = [[Finished((Call("a","unknown","{}"),))], [Finished((Call("b","calculator","oops"),))],
                    [Finished((Call("c","calculator","{}"),))]]
    with client(cfg, model) as web:
        cid = web.post("/api/conversations").json()["id"]
        events = chat(web, cid)
        assert [v["ok"] for k,v in events if k == "tool_result"] == [False, False]
        assert events[-1][1]["code"] == "tool_limit"
        assert all(json.loads(r[-1]["content"])["ok"] is False for r in model.requests[1:])


def test_crash_recovery_and_exclusive_generation(setup):
    cfg, model = setup
    store = Store(cfg.database_path)
    with store.ownership():
        with pytest.raises(sqlite3.OperationalError):
            with Store(cfg.database_path).ownership():
                pass
        cid = store.create()["id"]
        tid = store.begin(cid, "Interrupted question")
        with pytest.raises(PublicError, match="running"):
            store.begin(cid, "Overlap")
        with pytest.raises(PublicError):
            store.delete(cid)
        store.checkpoint(tid, "part", [], [], "generating")
    with client(cfg, model) as web:
        turn = web.get("/api/conversations/" + cid).json()["turns"][0]
        assert (turn["status"], turn["assistant"]) == ("interrupted", "part")
        model.rounds = [[Text("Recovered"), Finished()]]
        assert chat(web, cid)[-1][0] == "message_complete"


def test_cancel_preserves_partial_and_releases_slot(setup):
    cfg, model = setup
    store = Store(cfg.database_path)
    store.initialize()
    cid = store.create()["id"]
    tid = store.begin(cid, "Cancel")
    run = TurnRun(cfg, store, model, device_registry(), cid, tid, "Cancel")
    model.rounds = [[Text("Partial answer"), Text("later"), Finished()]]

    async def cancel():
        events = run.events()
        assert (await anext(events))[0] == "message_start"
        assert (await anext(events))[0] == "text_delta"
        await events.aclose()
        run.interrupt()

    asyncio.run(cancel())
    assert store.detail(cid, 1, None)["turns"][0]["assistant"] == "Partial answer"
    assert store.detail(cid, 1, None)["turns"][0]["status"] == "interrupted"
    assert store.begin(cid, "Continue")


def test_temporarily_locked_database_recovers(setup):
    cfg, model = setup
    with client(cfg, model) as web, sqlite3.connect(cfg.database_path) as blocker:
        blocker.execute("BEGIN IMMEDIATE")
        result = web.post("/api/conversations")
        assert result.status_code == 503 and result.json()["code"] == "storage"
        assert str(cfg.database_path) not in result.text
        assert web.get("/api/health").status_code == 200
        blocker.rollback()
        assert web.post("/api/conversations").status_code == 201


def test_input_validation_does_not_echo_content(setup):
    cfg, model = setup
    with client(cfg, model) as web:
        response = web.post("/api/chat/stream", json={"conversation_id": "bad", "message": "private input"})
        assert response.status_code == 422 and "private input" not in response.text

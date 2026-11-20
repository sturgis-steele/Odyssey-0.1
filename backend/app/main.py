import json
import logging
import sqlite3
from contextlib import asynccontextmanager, aclosing
from typing import Annotated
from uuid import UUID

import httpx
from fastapi import FastAPI, Query, Request, Response
from fastapi.exceptions import RequestValidationError
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, StreamingResponse
from fastapi.staticfiles import StaticFiles
from starlette.middleware.trustedhost import TrustedHostMiddleware

from . import VERSION
from .ai.llama import LlamaProvider
from .ai.orchestrator import TurnRun
from .ai.provider import ModelProvider
from .config import ROOT, Settings
from .contracts import (ChatRequest, Conversation, ConversationDetail, ManualRun, ModelHealth,
                        PublicError, SystemStatus, ToolInfo, ToolResult)
from .database import Store
from .tools.registry import device_registry

log = logging.getLogger("odyssey")
LIMIT = Annotated[int, Query(ge=1, le=100)]


class Stream(StreamingResponse):
    """Release the model slot even if the client disappears before the first byte."""
    def __init__(self, run: TurnRun, release):
        self.run, self.release = run, release

        async def encode():
            async with aclosing(run.events()) as events:
                async for name, data in events:
                    yield f"event: {name}\ndata: {json.dumps(data, ensure_ascii=False)}\n\n"

        super().__init__(encode(), media_type="text/event-stream",
                         headers={"Cache-Control": "no-store", "X-Accel-Buffering": "no"})

    async def __call__(self, scope, receive, send):
        try:
            await super().__call__(scope, receive, send)
        finally:
            try:
                await self.body_iterator.aclose()
                self.run.interrupt()
            except sqlite3.Error:
                log.error("storage_cleanup_failed")
            finally:
                self.release()


def create_app(settings: Settings | None = None, provider: ModelProvider | None = None) -> FastAPI:
    cfg = settings or Settings()
    store = Store(cfg.database_path)
    registry = device_registry()
    busy = False
    pending_cleanup: TurnRun | None = None

    @asynccontextmanager
    async def lifespan(app):
        with store.ownership():
            async with httpx.AsyncClient(timeout=httpx.Timeout(cfg.model_timeout_seconds, connect=3),
                                         trust_env=False, follow_redirects=False) as client:
                app.state.model = provider or LlamaProvider(cfg, client)
                yield

    app = FastAPI(title=cfg.app_name, version=VERSION, lifespan=lifespan, docs_url=None, redoc_url=None)
    allowed = {f"http://{host}:{port}" for host in ("127.0.0.1", "localhost", "[::1]")
               for port in (5173, cfg.backend_port)}
    app.add_middleware(CORSMiddleware, allow_origins=sorted(allowed), allow_methods=["GET", "POST", "DELETE"],
                       allow_headers=["Content-Type"])
    app.add_middleware(TrustedHostMiddleware, allowed_hosts=["127.0.0.1", "localhost", "[::1]"])

    @app.middleware("http")
    async def local_access(request: Request, next_call):
        origin = request.headers.get("origin")
        if origin and origin not in allowed:
            return JSONResponse({"detail": "This origin cannot access Odyssey."}, status_code=403)
        response = await next_call(request)
        if request.url.path.startswith("/api/"):
            response.headers["Cache-Control"] = "no-store"
        response.headers["X-Content-Type-Options"] = "nosniff"
        response.headers["Referrer-Policy"] = "no-referrer"
        response.headers["Content-Security-Policy"] = (
            "default-src 'self'; script-src 'self'; style-src 'self'; font-src 'self'; "
            "img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"
        )
        return response

    @app.exception_handler(PublicError)
    async def public_error(_, exc):
        return JSONResponse({"code": exc.code, "detail": exc.message}, status_code=exc.status)

    @app.exception_handler(RequestValidationError)
    async def invalid(_, exc):
        return JSONResponse({"code": "invalid_input", "detail": "Check the request fields and input length."}, status_code=422)

    @app.exception_handler(sqlite3.Error)
    async def storage_error(_, exc):
        log.error("storage_error", extra={"kind": type(exc).__name__})
        return JSONResponse({"code": "storage", "detail": "Local storage is temporarily unavailable. Retry shortly."}, status_code=503)

    @app.get("/api/health")
    async def health():
        return {"backend": "online", "version": VERSION}

    @app.get("/api/models/current", response_model=ModelHealth)
    async def current_model():
        return await app.state.model.health()

    @app.get("/api/system/status", response_model=SystemStatus)
    async def status():
        model = await app.state.model.health()
        return SystemStatus(**model.model_dump(), status="ready" if model.model_server == "online" else "degraded",
                            version=VERSION, context_tokens=cfg.context_tokens, max_output_tokens=cfg.max_output_tokens)

    @app.get("/api/tools", response_model=list[ToolInfo])
    def tools():
        return [tool.info() for tool in registry.tools.values()]

    @app.post("/api/tools/{name}/run", response_model=ToolResult)
    def manual(name: str, run: ManualRun):
        if name not in registry.tools:
            raise PublicError("unknown_tool", "This tool is unavailable.", 404)
        return registry.run(name, run.arguments)

    @app.post("/api/conversations", status_code=201, response_model=Conversation)
    def create():
        return store.create()

    @app.get("/api/conversations", response_model=list[Conversation])
    def conversations(limit: LIMIT = 30, offset: Annotated[int, Query(ge=0)] = 0):
        return store.listing(limit, offset)

    @app.get("/api/conversations/{cid}", response_model=ConversationDetail)
    def conversation(cid: UUID, limit: LIMIT = 20, before: Annotated[int | None, Query(ge=1, le=2**63-1)] = None):
        return store.detail(str(cid), limit, before)

    @app.delete("/api/conversations/{cid}", status_code=204)
    def delete(cid: UUID):
        store.delete(str(cid))
        return Response(status_code=204)

    @app.post("/api/chat/stream", response_class=StreamingResponse,
              responses={200: {"content": {"text/event-stream": {}}}})
    async def chat(body: ChatRequest):
        nonlocal busy, pending_cleanup
        if busy:
            raise PublicError("busy", "A response is already running on this device.", 409)
        if pending_cleanup:
            pending_cleanup.interrupt()
            pending_cleanup = None
        cid = str(body.conversation_id)
        tid = store.begin(cid, body.message)
        run = TurnRun(cfg, store, app.state.model, registry, cid, tid, body.message)
        busy = True

        def release():
            nonlocal busy, pending_cleanup
            busy = False
            if not run.closed:
                pending_cleanup = run

        return Stream(run, release)

    dist = ROOT / "frontend/dist"
    if (dist / "index.html").is_file():
        app.mount("/", StaticFiles(directory=dist, html=True), name="console")
    return app


app = create_app()

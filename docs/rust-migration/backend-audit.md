# Stage 1 backend audit

The reference is commit `cba9b73`. The primary checkout was clean on `main` and had one worktree. Inspection covered tracked source, the registered FastAPI routes, emitted tool schemas, the TypeScript fetch client, tests and a read-only Stage 1 validation database.

| Source | Implemented behavior |
| --- | --- |
| `backend/run.py`, `app/main.py` | Single Uvicorn worker, structured application logs, lifespan owns SQLite and one HTTP client. Loopback Host and Origin allowlists, local CORS, CSP, API cache prevention, optional React dist mount and OpenAPI JSON. |
| `config.py`, `contracts.py` | Pydantic settings from environment over `backend/.env`, paths relative to checkout, local HTTP inference, token and iteration bounds, strict public input and explicit output models. |
| `database.py` | Schema v1, sidecar SQLite ownership lock, WAL, immediate write transactions, atomic turn snapshots, completed-history replay and orphan recovery. |
| `ai/llama.py`, `ai/provider.py` | Health and model-alias probes, slot size and template capability checks, server-side template application and tokenization, OpenAI chat stream, bounded multi-call fragments, reasoning kept for the current tool round. |
| `ai/orchestrator.py` | One generation, whole-turn history eviction, actual function dispatch, tool-result replay, checkpoint roughly once per second while text arrives and after every tool. Partial, failed and cancelled answers remain visible. |
| `tools/registry.py`, `calculator.py` | Exactly three read-only tools. Bounded real arithmetic including power, floor division, Python modulo and sqrt. Local date including UTC offset. OS/release/architecture/application version. |
| `frontend/src/services/api.ts`, hooks and pages | Same-origin relative fetch, fetch-based POST SSE, abort on Stop, strict event ordering, paged history, manual tools, readiness and degraded operation. Vite proxies /api. |

No runtime Kiwix, ZIM, search, notes, maps, speech, calendar, embeddings or Knowledge Builder code appears in the reference. Model management is an external process. The Python model launcher/validator and original backend can be used independently.

The reference pytest suite passed all 52 cases in an isolated temporary directory. It covers HTTP, SQLite, providers and tools. The nine Playwright fixture cases cover SSE, React and responsive behavior. Three opt-in live cases cover GGUF, cancellation and stopped-model behavior. Migration run results are recorded in [testing](testing.md).

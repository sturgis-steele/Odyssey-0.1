# Development

Use the repository root as the working directory. The Python and npm lock files describe the tested environment. Python supports Windows and Linux. A compatible browser, native inference build and GGUF are separate installations. The setup and startup sequence is in the [README](../README.md).

## Common commands

~~~powershell
.\.venv\Scripts\python.exe -m pip install -r backend\requirements-dev.txt
npm.cmd --prefix frontend ci
.\.venv\Scripts\python.exe -m pytest -q
npm.cmd --prefix frontend run typecheck
npm.cmd --prefix frontend run build
npm.cmd --prefix frontend test
~~~

Install Playwright's Chromium once with `npx.cmd playwright install chromium` from `frontend`. For an existing Chromium browser, set `PLAYWRIGHT_CHROMIUM_EXECUTABLE` to its executable path. The fixture suite starts its own loopback Vite instance on port 4176 and uses isolated browser contexts. It serves synthetic HTTP responses for predictable UI states. Pytest uses a fake provider, an HTTPX transport fixture for actual llama.cpp payload parsing, and temporary real SQLite databases.

If Windows restricts the default pytest temp directory, use a dedicated disposable directory such as `python -m pytest --basetemp=.local/pytest-run` after creating `.local`. Pytest owns the selected base directory for that run.

### Live integration

Run the test backend in a separate terminal with its own database to keep validation conversations outside your normal app history.

~~~powershell
$env:BACKEND_PORT = '8019'
$env:DATABASE_PATH = '.local/validation.db'
.\.venv\Scripts\python.exe backend\run.py
~~~

With a production frontend build and a running model, run the validation commands in another terminal.

~~~powershell
.\.venv\Scripts\python.exe scripts\validate-model.py --port 8019
$env:ODYSSEY_LIVE_PORT = '8019'
npm.cmd --prefix frontend run test:live -- --grep-invert 'stopped llama-server'
~~~

The browser uses the real backend, SQLite and model. The calculation test rejects non-loopback browser requests, verifies the tool result, reloads the conversation and asks a follow-up. The cancellation test checks concurrent-request rejection, stops an active generation, inspects its durable terminal state and sends another message. Screenshots are written to Playwright's ignored test results directory.

To repeat the process-isolated offline check, start the test backend with the same environment and the following entry point. Its Python audit hook rejects outbound TCP connections and hostname lookups beyond loopback. It checks the restriction before running the real backend. Use llama-server's `--offline` mode and run the same API and browser tests.

~~~powershell
.\.venv\Scripts\python.exe backend\tests\loopback_server.py
~~~

Stop the test model process and run the outage case.

~~~powershell
$env:ODYSSEY_MODEL_STOPPED = '1'
npm.cmd --prefix frontend run test:live -- --grep 'stopped llama-server'
Remove-Item Env:ODYSSEY_MODEL_STOPPED
~~~

The outage test expects at least one saved conversation from the preceding run. Restart llama-server and refresh System to restore chat. The complete manual procedure includes a backend restart and a disconnected network test. [Acceptance procedure](model-setup.md)

## Configuration

Process environment variables override `backend/.env`. Relative database paths resolve against the repository root regardless of the launch directory. Settings are validated at startup. Restart affected processes after editing them.

| Setting | Default | Purpose |
| --- | --- | --- |
| `APP_NAME` / `APP_ENV` | Odyssey / development | API identity and environment label |
| `BACKEND_HOST` / `BACKEND_PORT` | 127.0.0.1 / 8000 | Single-worker local listener |
| `LLAMA_BASE_URL` | http://127.0.0.1:8080/v1 | Loopback HTTP endpoint |
| `LLAMA_MODEL` | odyssey-local | Alias returned by the model list |
| `DATABASE_PATH` | ./data/odyssey.db | Conversation database |
| `MAX_TOOL_ITERATIONS` | 3 | Maximum completed tool-call batches per turn |
| `MAX_OUTPUT_TOKENS` | 512 | Per-round model output allowance |
| `CONTEXT_TOKENS` / `HISTORY_TURNS` | 4096 / 8 | Context ceiling and maximum recent complete turns |
| `MODEL_TIMEOUT_SECONDS` | 120 | HTTP read/write inactivity timeout during inference |
| `TEMPERATURE` / `ENABLE_THINKING` | 0.1 / false | Sampling temperature and model-template setting |
| `SYSTEM_PROMPT` | Built-in local assistant instructions | Optional concise override |

## Debugging

1. Request `/api/health` first. A database startup failure usually means the data directory cannot be written, another process owns it, or the schema requires a newer application.
2. Request `/api/system/status`. A healthy backend can report an offline model, a loading model or a mismatched alias. Use the model guide to check the direct llama.cpp endpoints.
3. Inspect the Network panel for the `POST /api/chat/stream` response. Check `tool_start` and `tool_result`, then the final event. Development ports are 5173 and 8000 by default. The built UI uses one origin.
4. Use backend JSON logs for tool names, success flags and error codes. Prompts, results, reasoning and database paths are excluded from normal application events.
5. For SQLite backups, close the backend or use a SQLite-aware online backup. The WAL and shared-memory files may contain uncheckpointed data. Only one Odyssey process may own a database at a time.

## Conventions

Keep API validation and error translation in the backend. New providers implement the small protocol in `ai/provider.py`. New tools declare a schema, permission class, handler and registry entry. Perform validation before any side effect. Add regression coverage for the new behavior. Database schema changes get an ordered migration and restart tests. Keep native device-specific code behind an interface. The `.in` files declare dependency ranges and the `.txt` files pin tested resolutions. Refresh pins intentionally and test them on the target systems.

## Dependencies and commercial distribution

The npm build emits `frontend/dist/THIRD-PARTY-NOTICES.md` with licenses for bundled frontend packages and fonts. Preserve it with the shipped UI. React, Vite, react-markdown, FastAPI and Pydantic carry MIT terms. HTTPX and Uvicorn carry BSD-3-Clause terms. The icon bundle includes ISC and inherited Feather MIT notices. Barlow and IBM Plex Mono use SIL OFL 1.1. See the installed packages' exact license texts and the generated frontend notices. Upstream Python, SQLite and the native binary have separate terms. The Python dependency tree includes the OpenTelemetry API through FastAPI. Odyssey configures no telemetry SDK, exporter or remote endpoint. [SQLite copyright](https://sqlite.org/copyright.html) · [llama.cpp license](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE)

The tested LFM3 weights carry Liquid's LFM 1.0 license, including commercial-use conditions tied to a defined revenue threshold and redistribution obligations. Record the exact artifact, license, template, model source and revision before choosing a shipping configuration. The Qwen3 1.7B publisher lists Apache-2.0. Verify the specific quantization and its accompanying notices. A successful local validation does not settle commercial distribution rights. [LFM3 license](https://huggingface.co/LiquidAI/LFM3-1.7B-GGUF/blob/main/LICENSE) · [Qwen3 GGUF publisher](https://huggingface.co/Qwen/Qwen3-1.7B-GGUF)

The generated terrain illustration is bundled as one WebP. Its source and prompt are recorded in the [artwork note](artwork.md). The project owner retains the decision about Odyssey's own license.

# Odyssey

Odyssey is an offline personal assistant for a portable knowledge device. Stage 1 runs on a Windows laptop with a touchscreen-oriented React console, a FastAPI application, local SQLite storage and a separate CPU-only llama.cpp server. Normal operation uses loopback connections and locally bundled assets.

## What works

- Stream assistant answers, stop generation, read Markdown and inspect actual tool results.
- Ask the model to calculate, read the local date/time or inspect device information. Run the same read-only tools directly in Tools while inference is unavailable.
- Create, reopen, page through and delete conversations. SQLite retains partial responses and tool activity across restarts.
- See independent backend and model status, the model alias and configuration. Enter fullscreen from the top bar.

## Windows quick start

Use Python 3.11 or later, Node.js 22.12 or later and npm. Inference also needs a recent tool-capable `llama-server` build and a local GGUF with a matching Jinja chat template. The release tested on the development laptop used Python 3.15.2, Node 24.18.0 and LFM3 1.7B Q4_K_M. Run the commands from the repository root.

### One-time setup

~~~powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r backend\requirements-dev.txt
npm.cmd --prefix frontend ci
if (!(Test-Path .\backend\.env)) { Copy-Item .\backend\.env.example .\backend\.env }
npm.cmd --prefix frontend run build
~~~

The runtime-only Python installation uses `backend/requirements.txt`. Dependency installation needs package access or an offline cache. Choose and obtain model weights separately. [Model selection and setup](docs/model-setup.md)

### Start the model

On the development laptop, use the existing local Ollama model file and bundled llama.cpp executable. This script launches llama-server directly.

~~~powershell
.\scripts\start-model.ps1 -OllamaModel 'lfm3:latest'
~~~

On a fresh machine, put `llama-server.exe` on PATH and pass the path to your existing model file.

~~~powershell
.\scripts\start-model.ps1 -ModelPath .\models\assistant.gguf
~~~

The default model alias is `odyssey-local` and the server listens at `127.0.0.1:8080`. To change models, choose another file in the launcher. Keep its `-Alias`, `-Port` and `-Context` consistent with `LLAMA_MODEL`, `LLAMA_BASE_URL` and `CONTEXT_TOKENS` in `backend/.env`.

### Start FastAPI and open Odyssey

Launch the application in a second terminal.

~~~powershell
.\.venv\Scripts\python.exe backend\run.py
~~~

Open the built application at [Odyssey on localhost](http://127.0.0.1:8000/). FastAPI serves the compiled React assets as well as the API. The interface, saved conversations and manual tools work with the model stopped.

### Start React for development

Keep FastAPI running and launch Vite in a third terminal.

~~~powershell
npm.cmd --prefix frontend run dev
~~~

Open the [Vite development console](http://127.0.0.1:5173/). Vite forwards only the local API requests to FastAPI. Ctrl+C stops each foreground service.

## Verify a first conversation

Send **What is 157 multiplied by 243?** in Assistant. Expand Calculator when its status changes to Completed. Its value and the model's final answer should both equal **38,151**. Repeat with the local time and operating system prompts. Open Conversations to retrieve each saved turn. The following command runs the four real-model prompts against the active backend.

~~~powershell
.\.venv\Scripts\python.exe scripts\validate-model.py
~~~

For deterministic tests, run pytest and the browser suite. Install its Chromium browser once or point `PLAYWRIGHT_CHROMIUM_EXECUTABLE` at an existing compatible Chromium executable.

~~~powershell
.\.venv\Scripts\python.exe -m pytest -q
Push-Location frontend
npx.cmd playwright install chromium
npm.cmd test
Pop-Location
~~~

## Layout

~~~text
backend/app/         FastAPI, configuration, API schemas and SQLite
backend/app/ai/      Model interface, llama.cpp adapter and turn orchestration
backend/app/tools/   Validated registry, calculator, system clock and device information
backend/tests/       Deterministic API, provider, persistence and tool tests
backend/run.py       Validated single-worker launcher and structured logging
frontend/src/       React shell, pages, components, hooks, API client and local assets
frontend/tests/     Chromium UI and stream-contract tests using fixtures
frontend/tests-live/ Opt-in browser tests against actual local processes
scripts/            Windows model launcher and live model validation
docs/               Development, architecture, validation and device handoff
~~~

## Scope and limitations

Stage 1 supplies an assistant and conversation archive. Offline reference collections, document retrieval, long-term memory, calendar, voice and maps are future stages. Conversations are plaintext local SQLite data. The device supports one generation at a time. Model and template quality affect tool selection. Physical Pi deployment, battery performance and an isolated physical-network test remain to be verified.

The development environment's resolved dependency versions are pinned. Rights to distribute each model, template and future dataset require a separate review. Odyssey's product license remains an owner decision. [Development and licensing](docs/development.md) · [Stage roadmap](docs/roadmap.md)

## Documentation

- Data model, endpoints and the SSE contract are in the [architecture guide](docs/architecture.md).
- Setup, configuration, testing, debugging and dependency notices are in the [development guide](docs/development.md).
- Model setup, template verification and the A–G acceptance procedure are in the [model guide](docs/model-setup.md).
- Linux commands and Chromium kiosk setup are in the [Pi deployment notes](docs/deployment-notes.md).
- Measured Windows results and remaining hardware checks are in the [Stage 1 validation record](docs/validation.md).

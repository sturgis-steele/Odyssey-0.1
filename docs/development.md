# Development

The Rust launcher, toolchain and current test commands are in the [README](../README.md). The notes below cover the development workflow and the configuration table.

Use the repository root as the working directory. The npm and Cargo lock files describe the tested environment. A compatible browser, native inference build and GGUF are separate installations. The setup and startup sequence is in the [README](../README.md).

## Common commands

~~~powershell
npm.cmd --prefix frontend ci
npm.cmd --prefix frontend run typecheck
npm.cmd --prefix frontend run build
npm.cmd --prefix frontend test
~~~

Install Playwright's Chromium once with `npx.cmd playwright install chromium` from `frontend`. For an existing Chromium browser, set `PLAYWRIGHT_CHROMIUM_EXECUTABLE` to its executable path. The fixture suite starts its own loopback Vite instance on port 4176 and uses isolated browser contexts. It serves synthetic HTTP responses for predictable UI states. The Rust integration tests use a mock HTTP model server and temporary real SQLite databases.

### Live integration

Run the test backend in a separate terminal with its own database to keep validation conversations outside your normal app history.

~~~powershell
$env:BACKEND_PORT = '8019'
$env:DATABASE_PATH = '.local/validation.db'
cargo run --locked --manifest-path backend-rust\Cargo.toml
~~~

With a production frontend build and a running model, run the validation commands in another terminal. The validation script needs any Python 3 installation with HTTPX.

~~~powershell
python scripts\validate-model.py --port 8019
$env:ODYSSEY_LIVE_PORT = '8019'
npm.cmd --prefix frontend run test:live -- --grep-invert 'stopped llama-server'
~~~

The browser uses the real backend, SQLite and model. The calculation test rejects non-loopback browser requests, verifies the tool result, reloads the conversation and asks a follow-up. The cancellation test checks concurrent-request rejection, stops an active generation, inspects its durable terminal state and sends another message. Screenshots are written to Playwright's ignored test results directory.

To repeat the process-isolated offline check, run `scripts/validate-offline.sh` against the backend inside a network namespace containing only loopback, with llama-server started in `--offline` mode, then run the same API and browser tests.

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
| `ODYSSEY_ROOT` | Launch root | Rust deployment root, supplied in the process environment |
| `ODYSSEY_TIMEZONE` | Device zone | Optional IANA zone for the Rust clock tool |
| `LOG_LEVEL` | info | Rust structured application log level |

## Debugging

1. Request `/api/health` first. A database startup failure usually means the data directory cannot be written, another process owns it, or the schema requires a newer application.
2. Request `/api/system/status`. A healthy backend can report an offline model, a loading model or a mismatched alias. Use the model guide to check the direct llama.cpp endpoints.
3. Inspect the Network panel for the `POST /api/chat/stream` response. Check `tool_start` and `tool_result`, then the final event. Development ports are 5173 and 8000 by default. The built UI uses one origin.
4. Use backend JSON logs for tool names, success flags and error codes. Prompts, results, reasoning and database paths are excluded from normal application events.
5. For SQLite backups, close the backend or use a SQLite-aware online backup. The WAL and shared-memory files may contain uncheckpointed data. Only one Odyssey process may own a database at a time.

## Conventions

Keep API validation and error translation in the backend. New providers implement the provider interface in `backend-rust`. New tools declare a schema, permission class, handler and registry entry. Perform validation before any side effect. Add regression coverage for the new behavior. Database schema changes get an ordered migration and restart tests. Keep native device-specific code behind an interface. Cargo.lock pins the tested Rust dependency resolutions; refresh pins intentionally and test them on the target systems.

## Dependencies and commercial distribution

The npm build emits `frontend/dist/THIRD-PARTY-NOTICES.md` with licenses for bundled frontend packages and fonts. Preserve it with the shipped UI. React, Vite and react-markdown carry MIT terms. The icon bundle includes ISC and inherited Feather MIT notices. Barlow and IBM Plex Mono use SIL OFL 1.1. See the installed packages' exact license texts and the generated frontend notices. SQLite, the Rust crate tree and llama.cpp have separate terms, inventoried in the [licensing notes](licensing/rust-dependencies.md). [SQLite copyright](https://sqlite.org/copyright.html) · [llama.cpp license](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE)

The tested LFM3 weights carry Liquid's LFM 1.0 license, including commercial-use conditions tied to a defined revenue threshold and redistribution obligations. Record the exact artifact, license, template, model source and revision before choosing a shipping configuration. The Qwen3 1.7B publisher lists Apache-2.0. Verify the specific quantization and its accompanying notices. A successful local validation does not settle commercial distribution rights. [LFM3 license](https://huggingface.co/LiquidAI/LFM3-1.7B-GGUF/blob/main/LICENSE) · [Qwen3 GGUF publisher](https://huggingface.co/Qwen/Qwen3-1.7B-GGUF)

The generated terrain illustration is bundled as one WebP. Its source and prompt are recorded in the [artwork note](artwork.md). The project owner retains the decision about Odyssey's own license.

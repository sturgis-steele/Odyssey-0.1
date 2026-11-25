# Odyssey Rust backend migration

The migration branch runs the existing React console against a standalone Axum/Tokio backend. All ten application operations, the six-event POST chat stream, the three read-only tools and SQLite schema v1 are implemented. The release passed end-to-end acceptance on the Windows laptop using local WSL2. Windows application control blocked native execution of the unsigned Windows build. ARM64 passed in emulation. Physical Raspberry Pi acceptance remains open.

## Implementation and compatibility

| Area | Outcome |
| --- | --- |
| HTTP and frontend | Same paths, status codes used by the console, JSON names and SSE event order. React production source and styles are unchanged. The live browser test now verifies visibly growing text before Stop. Axum serves dist and the existing Vite proxy remains available. |
| Status and access | Process liveness, a real database read, llama.cpp reachability and alias readiness are evaluated separately. System status adds database, model_reachable and model_ready. Loopback-only binding, local browser origins, CSP and no-store response handling are preserved. |
| Database | A dedicated worker owns rusqlite and bundled SQLite. Transactions, foreign keys, sidecar ownership, orphan recovery, timestamps, pagination and complete transcript replay retain the v1 contract. Both implementations reopened test copies. All 14 archived conversations and 19 turns matched through the public API and retained their stored values. |
| Inference and tools | Reqwest implements model list/health checks, slot capacity, template rendering, token accounting and both streaming and non-streaming completions. Streaming assembles genuine indexed function calls and sends ordered results back to llama.cpp. The calculator preserves large integers, Python-style floor/modulo and bounded powers. The clock uses local timezone data. Device information is restricted to four fields. |
| Cancellation and failures | The bounded event queue provides backpressure. Disconnect cancels the upstream read and persists partial text. A locked terminal write reports storage failure to the client and retries database cleanup before releasing the permit. Failed and interrupted turns are excluded from subsequent prompts. |
| Scope | Python source stays available under backend/ for comparison. The normal release needs no Python runtime. Stage 1 had no Kiwix runtime, ZIM retrieval or Knowledge Builder implementation in this checkout. The migration introduces no content-preparation pipeline. |

The migration adds a 256 KiB JSON envelope limit and bounded upstream JSON, line, per-round text and function-fragment sizes. The original chat character limit and tool schemas fit these bounds. The machine-readable OpenAPI starts from the reference schema. Actual public validation errors and runtime statuses are documented in the [HTTP contract](api-contracts.md). The [architecture guide](architecture.md), [database contract](database-compatibility.md) and [source audit](backend-audit.md) explain the implementation and design decisions.

## Verification and measurements

Stable Rust 1.100.0 passed formatting, compilation, warnings-as-errors Clippy, release build and all 11 integration cases on Linux. The same cases passed under ARM64 QEMU. The Windows target passed cross-compilation and platform-specific Clippy. All 52 original Python tests, 9 browser fixture cases and 3 real browser cases passed. The frontend type check and production build passed.

In a Linux network namespace containing only loopback, the real LFM3 1.7B model answered the purpose prompt and called calculator, clock and system information. The multiplication tool returned 38151 and the final answer included it. Restarting Axum reproduced the saved response and IDs. Stopping llama.cpp left the history and manual tools working. Windows Chromium verified the 1024 × 600 and 800 × 480 layouts, incremental rendering, tool activity, cancellation and slot reuse. [Commands and test record](testing.md)

Across three paired WSL2 runs, median warm backend RSS was 8.23 MiB for Rust and 57.47 MiB for Python. Median startup to first health response was 12.44 ms and 661.05 ms respectively. Both measurements exclude llama.cpp. The real-model run validates function and stream behavior. The performance harness uses a mock model for comparability. [Measurements and methodology](performance.md)

## Commercial distribution

The [inventory](../licensing/rust-dependencies.md) lists 152 external crate versions across the lockfile, including target-specific and development dependencies. A reusable audit utility collects their packaged notices. The release uses Jiff after the maintenance audit identified an archived timezone dependency. The reviewed Windows and ARM64 backend graphs include permissive licenses with attribution requirements. The actual product image also needs the precise model/template rights, native-library notices, font/icon notices, OS distribution obligations and Odyssey's own licensing decision. The LFM Open License has a commercial-use revenue condition. [Distribution review](../licensing/distribution-considerations.md)

## Launch and release artifacts

In two PowerShell terminals in the migration worktree, use the following commands on this laptop.

~~~powershell
# Model terminal
.\scripts\start-model.ps1 -WslDistro OdysseyRustDev -OllamaModel 'lfm3:latest' -Port 18581
~~~

~~~powershell
# Application terminal
$env:BACKEND_PORT = '18580'
$env:LLAMA_BASE_URL = 'http://127.0.0.1:18581/v1'
.\scripts\start-odyssey.ps1 -WslDistro OdysseyRustDev
~~~

Open [Odyssey](http://127.0.0.1:18580/). These ports keep the existing 8000 and 8080 listeners available. The setup uses the existing local GGUF, the dedicated Debian WSL distro and its local CPU-only llama-server. The distro disk and validation outputs live under the worktree's ignored .local directory. The WSL distro registration is local to the laptop. Keep its data disk if retaining this prepared development environment. The shipped backend and the device installation have no WSL dependency. The [README](../../README.md) includes native Windows builds, Linux deployment, Vite and Python comparison commands.

The binaries below are local build artifacts. Cargo.lock and the source are tracked in Git.

| Target | Worktree-relative release path | SHA-256 |
| --- | --- | --- |
| Windows x86_64 GNU | backend-rust/target/release/odyssey-backend.exe | 775a560d5e6d00e77d48ce46e86e69c50a7b3531bed46dd19ccfb5b0e8dfc755 |
| Linux x86_64 | backend-rust/target/linux-x86_64/release/odyssey-backend | 5ea0c8c49439a6b7da22171823bae8a9d0a3d952cbd29dcbea12310eb88b0cec |
| Linux ARM64 GNU | backend-rust/target/aarch64-unknown-linux-gnu/release/odyssey-backend | 25ef658680abe4f9f2ff475c90c9cafd9ab4c2a7dbed1230bc7940e0be91f59c |

## Handoff and open acceptance

The sibling worktree is `C:\Users\Sturgis\Code\odyssey-rust` on `feat/rust-backend`, based on `main` at `cba9b73`. Runtime/test sources are at `2fddeb6`; the handoff commit adds the launchers, reports and licensing audit. Use `git log -1 --oneline` in the worktree to identify the documentation tip. Work remained local and unmerged throughout validation.

Native Windows acceptance needs a build and executable approved by the device's signing policy. The current Windows platform code has compile and lint coverage. The next physical Pi acceptance needs the chosen 64-bit OS image, local GGUF, touchscreen, storage and power system. Measure real startup, thermal limits, inference latency and behavior during unexpected power cuts on that hardware. Other models and chat templates each need their own live tool checks. Commercial release requires review of the exact staged distribution.

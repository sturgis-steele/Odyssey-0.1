# Odyssey

Odyssey is an offline personal assistant for a portable knowledge device. The React console talks to a local Rust backend built with Axum and Tokio. SQLite stores conversations and tool activity. A separate llama.cpp process runs a GGUF chosen by the device owner.

The console streams Markdown answers, stops generation, reopens saved and interrupted turns, and displays real tool results. Calculator, local time, device information and the archive work with inference offline. Stage 1 provides one generation slot. Knowledge collections, retrieval, maps, calendar, persistent memory and speech are future stages.

## Run on this Windows laptop

The verified setup uses the local Debian WSL distro `OdysseyRustDev`, Windows Chromium and the existing LFM3 GGUF. The compiled console and release binary are present in the migration worktree. Open two PowerShell terminals there. Ports 8000 and 8080 already had local listeners during handoff, so these commands use 18580 and 18581. In the first terminal, start the model.

~~~powershell
Set-Location C:\Users\Sturgis\Code\odyssey-rust
.\scripts\start-model.ps1 -WslDistro OdysseyRustDev -OllamaModel 'lfm3:latest' -Port 18581
~~~

In the second, start the application.

~~~powershell
Set-Location C:\Users\Sturgis\Code\odyssey-rust
$env:BACKEND_PORT = '18580'
$env:LLAMA_BASE_URL = 'http://127.0.0.1:18581/v1'
.\scripts\start-odyssey.ps1 -WslDistro OdysseyRustDev
~~~

Open [Odyssey](http://127.0.0.1:18580/). The application command also works by itself for saved conversations and manual tools. The model launcher starts llama-server directly using the file already installed on disk. `-ModelPath` accepts another local GGUF. Ctrl+C stops a foreground service.

For React development on this laptop, open a third terminal. IPv6 loopback port 5173 was available during validation.

~~~powershell
Set-Location C:\Users\Sturgis\Code\odyssey-rust
$env:BACKEND_PORT = '18580'
npm.cmd --prefix frontend run dev -- --host ::1
~~~

Open the [local Vite console](http://[::1]:5173/).

## Native Windows development

Use stable Rust 1.100 or newer and a matching C toolchain for bundled SQLite. The MSVC host uses Visual Studio C++ Build Tools and a Windows SDK. A GNU Rust host uses MinGW-w64. Node 22.12 or newer and npm build the UI. Dependency installation can use the network or populated local caches. The native Windows release cross-compiles successfully. This laptop's application-control policy blocks execution of the unsigned native binary. The WSL commands above were exercised here. [Rust installation](https://rust-lang.org/tools/install/)

~~~powershell
rustup toolchain install 1.100.0 --profile minimal --component rustfmt --component clippy
cargo +1.100.0 fetch --locked --manifest-path backend-rust/Cargo.toml
cargo +1.100.0 build --locked --release --manifest-path backend-rust/Cargo.toml
npm.cmd --prefix frontend ci
npm.cmd --prefix frontend run build
if (!(Test-Path backend\.env)) { Copy-Item backend\.env.example backend\.env }
~~~

On a Windows system that permits the locally built executable, use these in separate terminals. The launcher can rebuild after edits with `-Build`.

~~~powershell
.\scripts\start-model.ps1 -ModelPath .\models\assistant.gguf
~~~

~~~powershell
.\scripts\start-odyssey.ps1
~~~

For live React editing, keep the model and either Rust launcher running and start Vite in a third terminal. Its proxy uses the same backend settings. Open the [development console](http://127.0.0.1:5173/).

~~~powershell
npm.cmd --prefix frontend run dev
~~~

## Linux and Raspberry Pi OS

Use a 64-bit OS and stable Rust 1.100 or newer. On Debian or Raspberry Pi OS, the native build needs a C compiler and linker. A built frontend/dist can be copied from the development machine. A deployed device needs the backend executable, built assets, writable data directory and any chosen external model/runtime. The normal application runtime requires neither Python nor Node.

~~~bash
sudo apt-get update
sudo apt-get install build-essential
cargo build --locked --release --manifest-path backend-rust/Cargo.toml
# When building assets locally with Node 22.12+ installed
npm --prefix frontend ci
npm --prefix frontend run build
# Start the API and compiled console from the checkout
./backend-rust/target/release/odyssey-backend
~~~

Start local inference in another terminal with a tool-aware llama.cpp build and the model's embedded Jinja template.

~~~bash
llama-server -m /srv/odyssey/models/assistant.gguf --alias odyssey-local \
  --host 127.0.0.1 --port 8080 -c 4096 -np 1 -ngl 0 --jinja --offline \
  --no-ui --no-context-shift --cors-origins localhost
~~~

For an installation outside the checkout, set ODYSSEY_ROOT to the directory containing frontend/dist and the optional backend/.env. Use an explicit writable DATABASE_PATH when installing in a read-only application directory. Run the device service as its ordinary local account. The [testing record](docs/rust-migration/testing.md) lists target toolchains, binary requirements and physical hardware checks.

## Configuration and data

Process settings override backend/.env. Default ports are 8000 for the API and 8080 for llama.cpp. Set LLAMA_BASE_URL to the loopback HTTP /v1 endpoint and LLAMA_MODEL to the server alias. The launcher accepts -Alias, -Port and -Context. Context and output budgets, history turns, thinking, sampling and timeouts retain the Stage 1 environment names. LOG_LEVEL controls structured logs. ODYSSEY_TIMEZONE optionally sets an IANA zone. [Configuration table](docs/development.md) · [Runtime architecture](docs/rust-migration/architecture.md)

The default database is data/odyssey.db under the runtime root. Rust reads and writes the original version-one schema. Back up through SQLite or stop the owning process before copying. A .lock database prevents concurrent ownership. The production API and UI use loopback and local assets. Local processes under the device account share access to the plaintext store. [Database compatibility](docs/rust-migration/database-compatibility.md)

## Test and compare

~~~powershell
Push-Location backend-rust
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked
Pop-Location
npm.cmd --prefix frontend run typecheck
npm.cmd --prefix frontend run build
npm.cmd --prefix frontend test
~~~

Playwright needs its installed Chromium or PLAYWRIGHT_CHROMIUM_EXECUTABLE pointing to a compatible existing browser. The integration tests use private databases and a mock HTTP model server. The acceptance suite exercises a real GGUF, a disconnected network namespace, persistence, both target screen sizes and cancellation. [Procedures and platform results](docs/rust-migration/testing.md) · [Measured performance](docs/rust-migration/performance.md)

The original Python backend was removed after the migration completed; its final state remains in Git history for reference.

## Migration and distribution

The [migration report](docs/rust-migration/completion-report.md) links the implementation, validation and remaining work. The [HTTP and SSE contract](docs/rust-migration/api-contracts.md) and [backend audit](docs/rust-migration/backend-audit.md) describe compatibility with Stage 1. The [crate inventory](docs/licensing/rust-dependencies.md) and [distribution review](docs/licensing/distribution-considerations.md) cover backend dependencies, model weights, front-end assets, and obligations for an eventual device image.

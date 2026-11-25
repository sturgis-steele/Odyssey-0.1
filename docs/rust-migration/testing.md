# Validation record

Testing used the Windows development laptop and its isolated Debian 13 WSL2 builder. The Rust compiler was stable 1.100.0, commit 0ef1bc386. The Linux host used glibc 2.41. Windows ran Node 24.18.0, Vite 8.3.8, Playwright 1.65.1 and Brave Chromium. Each application process used a dedicated validation database. The Python reference on Windows passed all 52 pytest cases.

| Check | Result and evidence |
| --- | --- |
| Linux x86_64 | Formatting check, cargo check, Clippy on all targets/features with warnings denied, all 11 Rust integration tests and an optimized release build passed. |
| Linux ARM64 | Cross-build, target-specific Clippy and the 11 Rust integration tests under QEMU user-mode with a Debian ARM64 sysroot. The optimized release also booted under QEMU, reported aarch64 and persisted a conversation while inference was offline. The executable is ELF64 AArch64 and its highest recorded glibc symbol version is 2.34. Physical Pi tests remain open. |
| Windows x86_64 GNU | Cross-built PE release and target-specific Clippy. Imports resolve to Windows system libraries. Native Windows compilation stopped when Code Integrity blocked Cargo's generated serde build script. A native acceptance launch also received OS error 4551 for the unsigned executable. The verified laptop workflow runs Linux binaries in WSL and uses the Windows browser. |
| Reference parity | Both HTTP implementations ran against the same loopback model fixture and separate SQLite backups. All 14 archived conversations and 19 turns matched the Python reader, including tool results and IDs. The harness compared new turn events and errors, reopened Rust-written transcripts with Python, checked ownership exclusion, restarted both implementations, and checked row values, foreign keys and integrity. The same comparison passed with both databases on the Windows-mounted worktree filesystem. |
| Frontend | TypeScript, the production build and nine Playwright fixture cases passed. The unchanged Vite proxy served React, returned ready status and calculated sqrt(144) = 12 through Rust on IPv6 loopback port 5173. An existing listener occupied the IPv4 development port. |
| Live browser | Windows Chromium accessed the Rust-served production build over localhost. It showed the actual calculator result, reopened the saved answer, handled a follow-up and displayed increasing text before Stop. Overlapping generation and active deletion returned 409. Cancellation released the slot. With the model stopped, the browser opened history and ran a manual calculation. Screenshots and viewport assertions covered 1024 × 600 and 800 × 480. |
| Real, network-isolated inference | CPU llama.cpp build b12669, commit 30ec35ae2, used the existing LFM3 1.7B Q4_K_M weights, embedded Jinja template, one slot, 4096 context, 512 output tokens, temperature 0.1 and thinking off. All four prompts passed in a Linux namespace with only IPv4/IPv6 loopback and no external route. The actual calls returned 38151, the local device timestamp and Linux device information. A backend restart reproduced the same saved turn. Stopping only llama.cpp left storage, HTTP and direct tools usable. |

## Deterministic coverage

`tests/http.rs` drives real TCP listeners, a real Reqwest client and temporary SQLite databases. It checks local access policies, statuses, public validation errors, CRUD, pagination, actual upstream chat payloads, multiple tool calls, invalid arguments, unknown tools, iteration limits, partial responses and output/context limits. It times a partial frame before upstream termination, aborts the HTTP stream, tests contention and verifies terminal storage recovery.

`tests/provider.rs` splits UTF-8 and SSE across network reads and checks standard non-streaming completions, duplicate call IDs, fragment bounds and invalid wire data. `tests/storage_tools.rs` exercises schema rollback, newer-version refusal, exclusive ownership, crash recovery, foreign keys, locked writes, exact integer arithmetic, expression limits, timezone overrides and safe device output.

Run the ordinary Rust and frontend commands from the [README](../../README.md). To compare against Python, use a reference venv with backend/requirements-dev.txt installed. Supply a quiescent copy of a Stage 1 validation database. Each process receives its own disposable backup.

~~~powershell
.\.venv-reference\Scripts\python.exe scripts\compare-backends.py --binary backend-rust\target\release\odyssey-backend.exe --legacy-db C:\odyssey-tests\stage1-copy.db
~~~

The equivalent Linux invocation accepts a Linux executable. For an OS-isolated offline acceptance run, install the validation venv, iproute2, util-linux and curl. Both binaries and the GGUF must already be local. Run the following from the checkout. Namespace creation needs the relevant Linux privilege. The script owns and removes its temporary database.

~~~bash
sudo unshare --net bash scripts/validate-offline.sh \
  ./backend-rust/target/release/odyssey-backend /opt/llama/llama-server \
  /srv/odyssey/models/assistant.gguf ./.venv-reference/bin/python
~~~

To run the real browser cases, point a test backend at its own database and use the same alias and port as its test model. Run the online cases, stop only that test model, then run the offline case with the backend still running.

~~~powershell
$env:ODYSSEY_LIVE_PORT = '18580'
$env:PLAYWRIGHT_CHROMIUM_EXECUTABLE = 'C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe'
npm.cmd --prefix frontend run test:live -- --grep-invert 'stopped llama-server'
$env:ODYSSEY_MODEL_STOPPED = '1'
npm.cmd --prefix frontend run test:live -- --grep 'stopped llama-server'
Remove-Item Env:ODYSSEY_MODEL_STOPPED
~~~

## ARM64 reproduction

Build natively on a 64-bit Pi with build-essential and tzdata, or on Debian x86_64 with the cross C compiler, linker and ARM64 sysroot. SQLite is compiled into the binary. The target Linux image supplies the dynamic loader, libc, libm, libgcc_s and system timezone data. Use a sysroot matching the oldest OS image you intend to ship.

~~~bash
sudo apt-get install gcc-aarch64-linux-gnu libc6-dev-arm64-cross qemu-user
rustup target add aarch64-unknown-linux-gnu
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER='qemu-aarch64 -L /usr/aarch64-linux-gnu'
cd backend-rust
cargo test --locked --target aarch64-unknown-linux-gnu
cargo build --locked --release --target aarch64-unknown-linux-gnu
~~~

The device itself still needs display/input, power-loss under real storage, battery/thermal and sustained inference measurements. The OS-level network namespace disconnected the test processes from external networks while the laptop remained connected. Alternate GGUFs and native Windows execution on an approved signing configuration remain separate acceptance runs.

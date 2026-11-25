# Runtime architecture

A request from React goes to the same-origin Axum server. Tokio runs two asynchronous workers. Shared state owns typed configuration, a bounded SQLite task queue, a reusable Reqwest client, the immutable tool registry and one generation permit. The console uses its existing TypeScript fetch client and POST event stream. A production build serves frontend/dist from Axum. Vite retains its local development proxy.

## A conversation turn

1. Validate and trim the request, acquire the device permit and ask the database worker to create the turn transactionally. An acknowledgement precedes the HTTP stream. The worker owns cleanup even if a browser closes before reading headers.
2. Read recent complete transcripts. Probe llama.cpp /health and /v1/models, then /props for the actual slot capacity and template capabilities. Use the lower of device and configured context limits.
3. Ask the server to apply its own Jinja template and tokenize the rendered prompt, including registered tools. Evict the oldest complete turn until the prompt plus output allowance and eight tokens of margin fits. The archive keeps all turns.
4. Read /v1/chat/completions incrementally. Buffer one protocol line and bounded tool fragments, including incremental UTF-8. Emit public text into an eight-event queue. A bounded queue propagates browser backpressure. The provider also supports a standard non-streaming completion.
5. Collect up to four indexed function calls. Validate IDs, name, arguments and terminal reason. Give the model a structured outcome for each call, retaining its ID and order. Missing IDs receive a local UUID. Invalid arguments get a failed tool outcome and a valid empty argument object for template replay. Reasoning can return to the server within the turn. The store removes it from disk.
6. Checkpoint public text about once a second during generation and after each tool. Persist a terminal snapshot and release the generation slot. Client disconnect drops the upstream request and records interrupted text. On a temporary write lock, retain the slot and retry the final snapshot with capped backoff. On process loss, startup converts orphan generating rows to interrupted.

The provider uses HTTP over loopback, with redirects and environment proxies disabled. Network and protocol failures have stable public codes. The catalog contains three fixed read-only operations. Arithmetic uses bounded integer and floating-point operations with a grammar limited to the reference operators. Registry and device operations run outside the async workers. Linux reads the kernel release from procfs. Windows reads the OS build from the registry. Time follows the device zone with an optional IANA override.

## Configuration and trust

Existing variables and defaults are listed in [development](../development.md). Process settings override backend/.env. Start in the repository root or backend-rust, or set ODYSSEY_ROOT to an absolute deployment directory. DATABASE_PATH resolves under that directory. ODYSSEY_TIMEZONE accepts a zone such as America/Chicago. LOG_LEVEL accepts a tracing level and defaults to info. Startup rejects non-loopback hosts, non-local model URLs, invalid ranges and generation settings that exhaust the prompt budget.

GET /api/health checks the process. System status reads the conversations table and separately reports database availability, server reachability and model alias readiness. A loading or mismatched model yields degraded status. Stage 1 has no knowledge service to probe.

The default listener is 127.0.0.1:8000. Browser origins and Host values are restricted to the local console. The API sets no-store and content security headers. There is no application authentication between local OS processes. Another process running as the device user can call the API and access plaintext SQLite files. Protect the OS account, database directory and backups. Normal backend events contain registered tool names, success flags and public error categories. Model-server logging is configured independently.

The SQLite worker owns one data connection and one sidecar ownership connection for the process lifetime. It sleeps on a 32-job queue. There is no idle database poller, inference poller or telemetry task. The React console requests status on focus and periodically when visible. See [database compatibility](database-compatibility.md).

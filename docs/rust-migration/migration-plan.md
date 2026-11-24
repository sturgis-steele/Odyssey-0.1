# Migration sequence

1. Audit the Stage 1 source, observable routes, UI stream parser, persistence and tests at `cba9b73`. Use the sibling worktree on `feat/rust-backend`.
2. Implement typed configuration, the HTTP boundary and the v1 SQLite store. Prefer rusqlite's bundled SQLite and a dedicated bounded worker because one device owns one database and generation slot. SQLx's async pool and extra connections offer little here.
3. Port template/token accounting, streaming, cancellation, transcript handling and the three validated tools. Keep llama.cpp external and preserve the six SSE events.
4. Compare HTTP, stream and database behavior with fixtures, a legacy database backup and a real local model. Build and exercise the existing console at both target sizes.
5. Measure comparable standalone processes, audit the resolved dependency graph, document installation and verification, then select the default launcher according to measured parity. Keep reference source and existing user data intact.

# Backend performance on the development laptop

Three paired runs compared the original single-worker FastAPI service with the Rust release from commit `2fddeb6`. Both ran in Debian 13 x86_64 under WSL2 on the same Windows laptop. Each process used its own SQLite backup on the Linux temporary filesystem, starting from the same 14-conversation, 19-turn validation database. The model fixture ran in the driver process and returned the same calculator request and final text to both backends.

| Measurement | Python/FastAPI | Rust/Axum |
| --- | ---: | ---: |
| Startup to first successful health response | 661.05 ms | 12.44 ms |
| Warm idle resident memory | 57.47 MiB | 8.23 MiB |
| Idle CPU, percentage of one core | 0.0% | 0.0% |
| GET health | 0.433 ms | 0.155 ms |
| GET conversation detail | 1.196 ms | 0.339 ms |
| POST create conversation | 1.168 ms | 0.337 ms |
| POST manual calculator | 1.188 ms | 0.167 ms |
| Stream request to first public text, one fixture tool round | 49.699 ms | 43.085 ms |
| Stream request through terminal frame and EOF | 51.078 ms | 43.112 ms |

Values are the medians across the three runs. Each run used 20 timed requests per HTTP operation after a warmup. Stream measurements used five timed new conversations after a warmup. The stream figures include loopback HTTP, the model fixture, tool dispatch and SQLite snapshots. The cold process startup probe sleeps 10 ms between attempts. The three startup readings were 1084.29, 558.13 and 661.05 ms for Python, and 13.48, 12.44 and 11.49 ms for Rust. Python ran first in each pair, so filesystem caching and scheduling can affect startup.

The driver sampled the backend process's resident set and CPU time from procfs after the workload. Python RSS ranged from 57.26 to 57.47 MiB and Rust from 7.98 to 8.36 MiB. During two-second idle windows, Linux recorded one 0.5% CPU sample for Python. Other samples rounded to zero. This measurement excludes the browser, driver and model server. Peak allocations inside a single tool call and real-model tokens per second were outside the benchmark.

A separate compatibility run placed both temporary databases on the Windows-mounted worktree filesystem used by the laptop quick start. Archive reads, ownership, writes, restarts and integrity checks passed. In that run median create latency was 14.322 ms for Python and 5.698 ms for Rust. These filesystem-specific results are separate from the table above.

The final stripped backend files were 6,453,224 bytes on Linux x86_64, 6,117,424 bytes on Linux ARM64 and 6,560,256 bytes on Windows x86_64. CPU cost, resident memory, battery life and flash-write behavior on the physical Pi require device measurements.

Reproduce with the [comparison harness](../../scripts/compare-backends.py) and the commands in [testing](testing.md). The local raw results are in `.local/release-comparison.txt` in the migration worktree.

# Stage 1 validation record

Windows validation took place on November 20, 2026. Test conversations used a dedicated SQLite database, separate from the application's default database.

## Environment

| Component | Tested configuration |
| --- | --- |
| Host | Windows 11, AMD64, CPU-only inference |
| Application | Odyssey 0.1.0, Python 3.15.2, Node.js 24.18.0, npm 11.16.0 |
| Build and browser | Vite 8.3.8, Playwright 1.65.1 with installed Brave Chromium |
| Inference | Local llama-server 0.9.0-dev, build 1, commit `6c01da627` |
| Model | LFM3 1.7B Q4_K_M, 1,099,627,808-byte pre-existing GGUF |
| File SHA-256 | `3793e1e6e2266919d6e015287221b41100175a98eddaba92e111aed3b05c93c4`, checked against the local model manifest |
| Inference settings | Embedded Jinja template, one slot, 4,096 context tokens, 512 output tokens, temperature 0.1, thinking disabled |

## Automated results

| Coverage | Result |
| --- | --- |
| Backend pytest suite | **52 passed**. Exercises real SQLite, HTTP endpoints, time and device tools, parser limits, strict schemas, invalid and unknown tools, maximum rounds, cancellation, storage contention, startup recovery, context pruning, loading, stale aliases, malformed model output, timeouts and fragmented tool calls |
| Frontend fixture suite | **9 passed**. Exercises input, progressive output, Markdown, tool activity, status distinction, conversation navigation and deletion, recovery after a temporary read failure, malformed streams, fullscreen and responsive layouts |
| Live browser suite | **3 cases passed** across running-model and stopped-model sessions. Covers a real tool call and follow-up after reload, concurrent request/deletion rejection, cancellation and slot reuse, and actual model loss |
| Build and dependency checks | TypeScript and Vite production build passed. Python dependency inspection passed. A clean `npm ci --offline` succeeded from the populated npm cache |
| Development proxy | Actual Vite listener returned the UI, proxied a ready system status and returned 12 for `sqrt(144)` from the FastAPI tool endpoint |

The fixture suites are deterministic software tests. The following observations were made with real Windows processes and the identified GGUF.

## Acceptance results

| Case | Observed result |
| --- | --- |
| A · Conversation | The model streamed a normal explanation of its purpose and available local tools |
| B · Calculator | A structured `calculator` call returned **38151**. The model's final answer included **38,151**. The browser rendered the activity, saved answer and follow-up correctly |
| C · Date/time | `get_current_datetime` returned a timezone-aware local instant. The final answer included the matching date and time |
| D · Device | `get_system_information` reported Windows 11 and AMD64. The final answer used the returned operating system |
| E · Persistence | Terminated and restarted FastAPI with the same validation database. Reopening the original turn returned the same UUID, completed state and answer. SQLite integration tests also verify preserved tool-call/result IDs |
| F · Model loss | Stopped the actual llama-server while FastAPI stayed up. Chromium showed the model offline and backend online, opened the conversation list and calculated `2 ** 10 = 1024` directly. Restarting llama-server restored readiness |
| G · Offline operation | Ran the four real-model prompts with offline-mode llama-server and a backend test process that denied non-loopback TCP connections and hostname lookups. The browser integration test blocked non-loopback page requests and verified a complete calculation and persisted follow-up |

## Display and hardware

Browser checks covered 1024 × 600, 800 × 480 and 390 × 700. The tests checked on-screen input, Send, minimum navigation height, horizontal page fit and fullscreen. Desktop and both landscape target sizes also produced screenshots of the built UI. See the browser integration test's output directory after a live run.

The Windows network adapters stayed enabled during the process-isolated offline test. A physical disconnected-network run, a hardware touchscreen/keyboard test, ARM64 installation, thermal/power measurements and Raspberry Pi performance remain open. Other GGUFs and templates require their own live validation. [Manual acceptance procedure](model-setup.md) · [Pi deployment checklist](deployment-notes.md)

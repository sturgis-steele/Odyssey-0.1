# Stage 1 architecture

Odyssey has one browser interface, one application process and one inference process. All three run on the device.

~~~text
Touchscreen browser
       | same-origin /api, JSON and SSE
FastAPI application ─────── SQLite conversations and turns
       |
       ├── Tool registry ── bounded calculator, device clock, device information
       |
       └── TurnRun ── ModelProvider ── local llama-server ── GGUF
~~~

## Frontend

The React shell owns navigation and fullscreen state. The Assistant, Conversations, Tools and System views share typed contracts. The API client handles same-origin fetch requests, UTF-8 decoding, fragmented SSE frames and terminal events. The chat hook owns draft submission, cancellation, loading earlier turns and reconciliation with SQLite. The status hook checks on focus and every 30 seconds while visible. One pending request per device matches the single CPU inference slot.

The initial screen is an empty draft. Sending its first message creates a conversation. Quick prompts fill the editable composer. Saved conversations are loaded on selection, 20 turns at a time. Markdown is rendered into React elements. HTML, linked images and active links are disabled. Build-time fonts, icons and terrain artwork are local files. The production backend enforces a same-origin content security policy.

## Backend and inference

`app/config.py` reads environment variables and `backend/.env`. The FastAPI lifespan owns a shared HTTPX client with proxy inheritance and redirects disabled. Each request uses a validated loopback URL. The API suppresses request contents in validation errors. The launcher emits structured event, tool and outcome fields and disables access logs.

`ai/provider.py` defines model health, capacity, prompt token counting and a stream of text or completed call batches. `ai/llama.py` handles llama.cpp's model list, health, properties, template application, tokenizer and chat completion endpoints. The adapter assembles fragmented call IDs, names and arguments by call index. It accepts JSON argument strings and objects. A missing call ID receives a local unique ID. Incomplete streams, malformed calls and output limits become explicit errors.

`ai/orchestrator.py` starts with the system prompt, recent complete turns and the new user message. Before **each** inference round it renders the actual template with tools and counts tokens with the loaded model. It reserves the configured output allowance and eight extra tokens within the smaller of the configured budget and the server's slot capacity. The oldest complete turns drop out until the request fits. An oversized current turn returns a context error. Every turn remains in SQLite.

On a structured call, the orchestrator publishes activity, executes the registry entry, appends the original assistant call and a matching `tool` response, and calls the model again. Invalid arguments yield an error result that the model may correct. The wire transcript normalizes malformed argument JSON to an object for Jinja and retains the explicit rejection. A repeated call after the iteration limit fails the turn. Intermediate reasoning is used only in memory when required by the template.

## Device tools

`tools/registry.py` holds each public name, schema, description, permission class and callable. Both model requests and manual tool runs pass through the same strict Pydantic validation. A new tool can be added to this registry without changing the model adapter. All three Stage 1 tools are read-only. Their work is bounded and synchronous. Future blocking tools will need deadlines and an execution boundary appropriate to their permissions.

The calculator walks a limited expression AST. It bounds input length, node count, exponent and intermediate magnitude. Date/time reads the system clock with its local UTC offset. The injectable `DeviceInformation` interface returns OS, release, architecture and application version.

## Storage and recovery

`database.py` uses SQLite foreign keys, short transactions, WAL journaling and schema migrations under `PRAGMA user_version`. Version 1 has two domain tables.

| Table | Contents |
| --- | --- |
| `conversations` | UUID, title derived from the first message, creation and modification times in UTC |
| `turns` | UUID and chronological sequence, user and assistant text, state, diagnostic, tool activity and role-tagged model transcript |

A turn transaction records the user input before streaming. Completed calls and roughly one-second partial text checkpoints are durable. The final transcript and terminal state commit together. Completed transcripts preserve call IDs, argument JSON and matching tool responses. Interrupted and failed turns remain readable and stay out of subsequent model history.

A separate SQLite exclusive lock is held for the lifetime of the backend. The OS releases it if the process exits. This permits safe startup recovery of unfinished turns, and prevents a second process from recovering live work. A unique database index and the application slot prevent concurrent generation. Deletes cascade, and deletion of a generating conversation returns HTTP 409. Close the application before making a filesystem backup, or use the SQLite online backup API.

## HTTP API

The machine-readable schema is available locally at `/openapi.json`.

| Method | Path | Behavior |
| --- | --- | --- |
| GET | `/api/health` | Backend liveness and version |
| GET | `/api/system/status` | Verified model state, alias, diagnostic, version and token configuration |
| GET | `/api/models/current` | Model health and alias, including loading and mismatched aliases |
| GET | `/api/tools` | Approved names, permissions and JSON schemas |
| POST | `/api/tools/{name}/run` | Body `{"arguments":{...}}`. Returns `ok` and a structured result |
| POST | `/api/conversations` | Create an empty conversation, HTTP 201 |
| GET | `/api/conversations?limit=30&offset=0` | Recent conversations first |
| GET | `/api/conversations/{id}?limit=20&before=123` | Chronological page of turns. Pass `next_before` to retrieve the preceding page |
| DELETE | `/api/conversations/{id}` | Remove conversation and turns, HTTP 204 |
| POST | `/api/chat/stream` | Body with `conversation_id` and `message`, followed by an SSE response |

A missing conversation returns 404, an occupied device returns 409, invalid input returns 422 and temporarily unavailable storage returns 503. An error before streaming uses a JSON `detail` field. After headers have been sent, errors use terminal SSE events.

## Streaming contract

Frames use UTF-8 `event: NAME` and `data: JSON` fields followed by a blank line. One request has one start, zero or more content/tool events, then one terminal event when the transport remains connected.

| Event | Data | Meaning |
| --- | --- | --- |
| `message_start` | `conversation_id, turn_id` | User turn reserved and persisted |
| `text_delta` | `text` | Append the fragment verbatim |
| `tool_start` | `call_id, name` | Requested action is entering validation/execution |
| `tool_result` | `call_id, name, ok, result` | Success or rejection from the executor |
| `message_complete` | `turn_id` | Complete response committed |
| `error` | `turn_id, code, message` | Terminal failure with a user-readable diagnostic |

The Stop control aborts the request. Disconnection closes the upstream HTTP stream, saves the partial turn and releases the slot. Reopening the conversation retrieves its durable state. There is no automatic replay of a POST stream.

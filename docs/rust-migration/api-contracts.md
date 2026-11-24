# Stage 1 HTTP and event contracts

All API paths are relative to the backend. Local processes are trusted. Browsers must use an allowed loopback Host and, when supplied, an allowed HTTP Origin at port 5173 or the backend port. There are no accounts or bearer tokens. API responses use no-store. Public application errors use `{code,detail}`. Invalid body, UUID or query values produce 422 without echoing input. Unavailable storage produces 503. Invalid origins return 403 and invalid Host headers return 400.

| Method and path | Input | Success and consumers |
| --- | --- | --- |
| GET /api/health | None | 200 `{backend:"online",version}`. Process probe and diagnostics. |
| GET /api/models/current | None | 200 `{model,model_server,diagnostic}`. Model status probe. States online, offline, loading, misconfigured. |
| GET /api/system/status | None | 200 model fields plus backend, status (ready/degraded), version, context_tokens, max_output_tokens. React useSystem, status/header and System. Rust also exposes database, model_reachable and model_ready. |
| GET /api/tools | None | 200 ordered array of name, description, permission and JSON Schema parameters. Tools page. |
| POST /api/tools/{name}/run | Object with optional object arguments, default {} | 200 `{ok,result}`, including validated execution failures with result.error. Unknown name 404, invalid envelope 422. Tools page. |
| POST /api/conversations | No body | 201 summary with id, title, created_at, updated_at. Assistant creates its conversation on demand. |
| GET /api/conversations | limit 1–100 (30), offset ≥0 (0) | 200 summary array, newest update first with id as tie-breaker. Conversations page. |
| GET /api/conversations/{cid} | UUID, limit 1–100 (20), optional before 1–9223372036854775807 | 200 summary plus turns and nullable next_before. Turns are oldest first within the newest matching page. 404 missing. Archive and Assistant. |
| DELETE /api/conversations/{cid} | UUID | 204 empty, 404 missing or 409 while generating in that conversation. Archive. |
| POST /api/chat/stream | Exactly conversation_id (UUID) and message (trimmed, 1–8192 characters) | 200 text/event-stream. 404 missing, 409 device busy, 422 invalid, 503 storage before streaming. Used by useChat. |

Each turn contains seq, id, created_at, user, assistant, status, nullable error and tools. Each tool activity contains call_id, name, ok and result. Complete transcripts are private to storage and inference. GET/HEAD /openapi.json exposes the machine-readable schema. A built frontend supplies GET/HEAD / and local static paths.

Each chat frame has an event line, a JSON data line and a blank line. No event IDs are needed. `message_start {turn_id,conversation_id}` occurs once. `text_delta {text}` arrives progressively. Each `tool_start {call_id,name}` precedes its `tool_result {call_id,name,ok,result}`. A completed turn ends with `message_complete {turn_id}`. An in-stream failure ends with `error {turn_id,code,message}` under HTTP 200. A disconnected client can reopen history to see an interrupted turn. The frontend validates framing, order and terminal events.

Generation error codes include model_offline, model_loading, model_misconfigured, model_request, model_timeout, model_protocol, tool_template, output_limit, tool_limit, context_limit and storage. The reference orchestrator also has incomplete and internal fallbacks. Errors retain partial public text. Tool rejections feed a structured failure back to the model.

# Local model setup and validation

Use a single-model llama.cpp server with a local GGUF. Install the server binary for the laptop's CPU architecture. Windows release archives need their adjacent runtime DLLs. Check `llama-server --version` and `--help` against the flags below. The exact runtime verified on this laptop was the bundled Windows AMD64 server, version `0.9.0-dev`, commit `6c01da627`. [Official releases](https://github.com/ggml-org/llama.cpp/releases) · [Server options and endpoints](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)

## Select a model

The existing LFM3 1.7B Q4_K_M file on the development laptop passed the Stage 1 tool prompts. Its embedded Jinja template advertises tools, structured calls and object arguments. Liquid's template accepts tool definitions on each round and needs current-turn reasoning preserved for some configurations. Odyssey defaults to thinking disabled and preserves returned reasoning in memory for tool continuations. [LFM3 model card](https://huggingface.co/LiquidAI/LFM3-1.7B) · [Official GGUF files](https://huggingface.co/LiquidAI/LFM3-1.7B-GGUF/tree/main)

For a new installation, choose a specific GGUF file from the publisher, record its revision, verify its published SHA-256 and place it under `models/` or another local directory. You can name the local file `assistant.gguf` for the examples. Keep installation media separate from source control. The Windows script can also read a single existing GGUF layer from a local Ollama manifest.

A Qwen3 1.7B 4-bit quantization is another CPU candidate. Qwen documents tool-call parsing with llama.cpp. Verify the exact file, chat template and server build using the tests below. Qwen3 0.6B has a smaller footprint, and its tool reliability requires separate measurement. [Qwen llama.cpp guide](https://github.com/QwenLM/Qwen3/blob/main/docs/source/run_locally/llama.cpp.md) · [Qwen3 1.7B model card](https://huggingface.co/Qwen/Qwen3-1.7B-GGUF) · [ggml-org Q4_K_M file](https://huggingface.co/ggml-org/Qwen3-1.7B-GGUF/blob/main/Qwen3-1.7B-Q4_K_M.gguf)

## Start llama-server on Windows

From the project root, use this laptop's existing model and runtime.

~~~powershell
.\scripts\start-model.ps1 -OllamaModel 'lfm3:latest'
~~~

For a standalone installation with `llama-server.exe` on PATH, use the file directly.

~~~powershell
.\scripts\start-model.ps1 -ModelPath .\models\assistant.gguf
~~~

Pass `-ServerPath` for an executable elsewhere. `-Port`, `-Alias` and `-Context` override the defaults. You can also invoke the server directly.

~~~powershell
llama-server.exe -m .\models\assistant.gguf --alias odyssey-local --host 127.0.0.1 --port 8080 -c 4096 -np 1 -ngl 0 --jinja --offline --no-webui --no-context-shift --cors-origins localhost
~~~

## Start llama-server on Linux

Start the ARM64 native executable with the local model file.

~~~bash
llama-server -m ./models/assistant.gguf --alias odyssey-local --host 127.0.0.1 --port 8080 -c 4096 -np 1 -ngl 0 --jinja --offline --no-webui --no-context-shift --cors-origins localhost
~~~

The CPU-only configuration uses one slot and a 4,096-token context. The served alias must match `LLAMA_MODEL`. Set `LLAMA_BASE_URL=http://127.0.0.1:8080/v1` and `CONTEXT_TOKENS=4096` in `backend/.env`. Restart FastAPI after changing configuration.

## Verify the template and connection

~~~powershell
Invoke-RestMethod http://127.0.0.1:8080/health
(Invoke-RestMethod http://127.0.0.1:8080/v1/models).data | Select-Object id
$props = Invoke-RestMethod http://127.0.0.1:8080/props
$props.chat_template_caps
$props.default_generation_settings.n_ctx
Invoke-RestMethod http://127.0.0.1:8000/api/system/status
~~~

The server should return `ok`, the chosen alias and a usable slot capacity. Tool-capable builds may expose `supports_tools` and `supports_tool_calls`. Older builds may expose only template text. Inspect the embedded template's handling of `tools`, `tool_calls` and the `tool` role. The capability check and successful health probe are preliminary. The call, result and grounded final answer in an actual conversation validate the full path.

Use `--jinja` explicitly. Model families encode calls differently. llama-server translates those representations to structured OpenAI-compatible deltas, including argument fragments. If a GGUF embeds an unsuitable template, use `--chat-template-file` with a matching, tool-aware template from the publisher. Retest after every change to weights, quantization, template or runtime. [llama.cpp function calling](https://github.com/ggml-org/llama.cpp/blob/master/docs/function-calling.md)

## Acceptance checks

Run the API-level probe with both services running. It creates four conversations and leaves them available for inspection.

~~~powershell
.\.venv\Scripts\python.exe scripts\validate-model.py
~~~

The same prompts can be sent in Assistant. Expand each tool activity and compare its returned value to the final answer.

| Test | Action | Expected observation |
| --- | --- | --- |
| A | Send “Hello. What is your purpose?” | A real, progressively generated assistant answer |
| B | Send “What is 157 multiplied by 243?” | `calculator` completes with `38151` and the answer contains 38,151 |
| C | Send “What is the current local date and time?” | `get_current_datetime` returns the machine's current local time and offset. Compare the answer with that result |
| D | Send “What operating system is this device running?” | `get_system_information` returns the OS and architecture. Compare the answer with that result |
| E | Record the conversation title, stop FastAPI with Ctrl+C, restart it with the same database and reopen the chat | Text, tool results and ordering survive |
| F | Stop the test llama-server with Ctrl+C, leave FastAPI running, open System and select Refresh status | Backend online, model offline. Open a saved conversation and run `2 ** 10` in Tools. Restart the model and refresh |
| G | After installation, run the built UI, disconnect Wi-Fi and Ethernet, reload the loopback URL and repeat A–D | Assets load and chat/tools work on localhost. Restore the network afterwards |

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Backend offline | Start `backend/run.py`, check `/api/health` and confirm Vite is using the configured port |
| Model loading | Wait for `/health` to return 200. A 503 can occur while the model is loading |
| Alias unavailable | Compare `LLAMA_MODEL` against `/v1/models` and the launcher's `--alias` |
| Server rejects token counting | Use a build exposing `/props`, `/apply-template` and `/tokenize`, in addition to streaming chat completions |
| Plain call markup, no activity or invalid arguments | Inspect template capabilities, enable Jinja and check supported function formats. Re-run B–D |
| Output stops early | Inspect the displayed limit or timeout and the saved partial response. Adjust `MAX_OUTPUT_TOKENS` or `MODEL_TIMEOUT_SECONDS` within the context budget |
| Context exhausted | Shorten the current turn or raise both server and application context settings within available memory |
| Repeated tool requests | Inspect rejected results and reduce ambiguity. Odyssey enforces a bounded round count |

#!/usr/bin/env bash
# Run inside a disposable Linux network namespace. Only test-owned processes use these ports.
set -euo pipefail
if (( $# != 4 )); then
    echo 'Usage: validate-offline.sh BACKEND LLAMA_SERVER GGUF VALIDATION_PYTHON' >&2
    exit 2
fi
backend=$(realpath "$1")
server=$(realpath "$2")
gguf=$(realpath "$3")
python=$(realpath -s "$4")
root=$(cd "$(dirname "$0")/.." && pwd)
if [[ $(ip -o link show | wc -l) != 1 ]] || ! ip link show lo >/dev/null; then
    echo 'This test requires a new network namespace containing only loopback.' >&2
    exit 2
fi
ip link set lo up
ip -brief addr
work=$(mktemp -d)
model_pid=''
api_pid=''
cleanup() {
    if [[ -n "$api_pid" ]]; then kill "$api_pid" 2>/dev/null || true; wait "$api_pid" 2>/dev/null || true; fi
    if [[ -n "$model_pid" ]]; then kill "$model_pid" 2>/dev/null || true; wait "$model_pid" 2>/dev/null || true; fi
    rm -r -- "$work"
}
trap cleanup EXIT
export ODYSSEY_ROOT="$root" BACKEND_HOST=127.0.0.1 BACKEND_PORT=18580 DATABASE_PATH="$work/offline.db"
export LLAMA_BASE_URL=http://127.0.0.1:18581/v1 LLAMA_MODEL=odyssey-local
"$server" -m "$gguf" --alias "$LLAMA_MODEL" --host 127.0.0.1 --port 18581 -c 4096 -np 1 -t 4 -ngl 0 --jinja --offline --no-ui --no-context-shift --cors-origins localhost >"$work/model.log" 2>&1 &
model_pid=$!
ready() {
    local url=$1 pid=$2 log=$3
    for ((i=0; i<180; i++)); do
        if curl --noproxy '*' -fsS "$url" >/dev/null 2>&1; then return; fi
        if ! kill -0 "$pid" 2>/dev/null; then tail -20 "$log" >&2; return 1; fi
        sleep .5
    done
    tail -20 "$log" >&2
    return 1
}
start_backend() {
    "$backend" >"$work/backend.log" 2>&1 &
    api_pid=$!
    ready http://127.0.0.1:18580/api/health "$api_pid" "$work/backend.log"
}
ready http://127.0.0.1:18581/health "$model_pid" "$work/model.log"
start_backend
"$python" "$root/scripts/validate-model.py" --port 18580
api=http://127.0.0.1:18580/api
cid=$(curl -fsS "$api/conversations" | "$python" -c 'import json,sys; print(json.load(sys.stdin)[0]["id"])')
curl -fsS "$api/conversations/$cid" >"$work/before.json"
kill "$api_pid"
wait "$api_pid"
api_pid=''
start_backend
curl -fsS "$api/conversations/$cid" >"$work/after.json"
cmp "$work/before.json" "$work/after.json"
kill "$model_pid"
wait "$model_pid" || true
model_pid=''
curl -fsS "$api/system/status" | "$python" -c 'import json,sys; s=json.load(sys.stdin); assert (s["backend"],s["database"],s["model_server"])==("online","online","offline")'
curl -fsS -H 'Content-Type: application/json' -d '{"arguments":{"expression":"2**10"}}' "$api/tools/calculator/run" | "$python" -c 'import json,sys; assert json.load(sys.stdin)["result"]["value"]==1024'
echo 'Loopback-only inference, persistence after restart, model loss and offline tools passed.'

"""Compare Stage 1 and Rust on temporary database copies and an HTTP model fixture."""
import argparse
import contextlib
import http.client
import json
import os
from pathlib import Path
import socket
import sqlite3
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "backend"))
from app.database import Store  # The original Python reader and process ownership protocol.


class Model(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def reply(self, data, mime="application/json"):
        body = (json.dumps(data) if mime == "application/json" else data).encode()
        self.send_response(200)
        self.send_header("Content-Type", mime)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        self.reply({"/health": {"status": "ok"}, "/v1/models": {"data": [{"id": "fixture"}]},
                    "/props": {"default_generation_settings": {"n_ctx": 4096}}}[self.path])

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path == "/apply-template":
            self.reply({"prompt": "rendered"})
        elif self.path == "/tokenize":
            self.reply({"tokens": [1, 2]})
        else:
            assert self.path == "/v1/chat/completions" and body["stream"]
            messages = body["messages"]
            if messages[-1]["role"] == "user":
                delta = {"tool_calls": [{"index": 0, "id": "call_fixture", "type": "function",
                         "function": {"name": "calculator", "arguments": '{"expression":"157*243"}'}}]}
                finish = "tool_calls"
            else:
                assert messages[-1]["tool_call_id"] == "call_fixture"
                assert json.loads(messages[-1]["content"])["result"]["value"] == 38151
                delta, finish = {"content": "38151"}, "stop"
            packets = [{"choices": [{"delta": delta, "finish_reason": None}]},
                       {"choices": [{"delta": {}, "finish_reason": finish}]}]
            self.reply("".join("data: " + json.dumps(p) + "\n\n" for p in packets) + "data: [DONE]\n\n",
                       "text/event-stream")


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


@contextlib.contextmanager
def backend(command, db, model_port, port):
    env = dict(os.environ, DATABASE_PATH=str(db), LLAMA_BASE_URL=f"http://127.0.0.1:{model_port}/v1",
               LLAMA_MODEL="fixture", BACKEND_HOST="127.0.0.1", BACKEND_PORT=str(port), APP_ENV="test",
               ODYSSEY_ROOT=str(ROOT), PYTHONDONTWRITEBYTECODE="1")
    # Direct interpreter PID makes working-set and CPU samples comparable on Windows.
    if command[0] == sys.executable and os.name == "nt":
        command = [sys._base_executable, *command[1:]]
        env["PYTHONPATH"] = os.pathsep.join(p for p in sys.path if p.endswith("site-packages"))
    start = time.perf_counter()
    with tempfile.TemporaryFile() as logs, subprocess.Popen(command, cwd=ROOT, env=env, stdout=logs, stderr=logs) as p:
        try:
            for _ in range(600):
                if p.poll() is not None:
                    logs.seek(0)
                    raise RuntimeError("Startup failed: " + logs.read(1500).decode(errors="replace"))
                try:
                    with contextlib.closing(http.client.HTTPConnection("127.0.0.1", port, timeout=1)) as probe:
                        probe.request("GET", "/api/health")
                        if probe.getresponse().status == 200:
                            break
                except OSError:
                    pass
                time.sleep(.01)
            else:
                raise RuntimeError("Backend did not start")
            yield p.pid, round((time.perf_counter() - start) * 1000, 2)
        finally:
            p.terminate()
            p.wait(timeout=15)


def process_sample(pid):
    if os.name == "nt":
        import ctypes
        from ctypes import wintypes as w

        class Counters(ctypes.Structure):
            _fields_ = [("cb", w.DWORD), ("faults", w.DWORD)] + [(field, ctypes.c_size_t) for field in
                ("peak", "rss", "paged_peak", "paged", "nonpaged_peak", "nonpaged", "pagefile", "pagefile_peak")]

        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.restype = w.HANDLE
        handle = kernel.OpenProcess(0x0410, False, pid)
        if not handle:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            mem = Counters()
            mem.cb = ctypes.sizeof(mem)
            times = [w.FILETIME() for _ in range(4)]
            if not ctypes.windll.psapi.GetProcessMemoryInfo(w.HANDLE(handle), ctypes.byref(mem), mem.cb):
                raise ctypes.WinError()
            if not kernel.GetProcessTimes(w.HANDLE(handle), *(ctypes.byref(t) for t in times)):
                raise ctypes.WinError()
            cpu = sum((t.dwHighDateTime << 32) | t.dwLowDateTime for t in times[2:]) / 10000000
            return mem.rss, cpu
        finally:
            kernel.CloseHandle(w.HANDLE(handle))
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return int(fields[21]) * os.sysconf("SC_PAGE_SIZE"), (int(fields[11]) + int(fields[12])) / os.sysconf("SC_CLK_TCK")


def sql_rows(db):
    with sqlite3.connect(db) as conn:
        return {name: {r[0]: r for r in conn.execute(f"SELECT * FROM {name}")}
                for name in ("conversations", "turns")}


def check(command, db, model_port, previous, port):
    def req(method, path, body=None, expected=200):
        conn.request(method, "/api/" + path, None if body is None else json.dumps(body),
                     {} if body is None else {"Content-Type": "application/json"})
        res = conn.getresponse()
        raw = res.read()
        assert res.status == expected, (path, res.status, expected)
        if not raw:
            return None
        return raw.decode() if "event-stream" in res.getheader("Content-Type", "") else json.loads(raw)

    legacy = Store(db)
    expected = {cid: legacy.detail(cid, 100, None) for cid in previous.get("conversations", {})}
    with backend(command, db, model_port, port) as (pid, startup), contextlib.closing(
        http.client.HTTPConnection("127.0.0.1", port, timeout=15)
    ) as conn:
        try:
            with Store(db).ownership():
                raise AssertionError("A second Stage 1 owner acquired a live Rust/Python database")
        except sqlite3.OperationalError as error:
            assert "locked" in str(error)
        for old_id, old_turns in expected.items():
            assert req("GET", f"conversations/{old_id}?limit=100") == old_turns
        health = req("GET", "health")
        status = req("GET", "system/status")
        for field in ("database", "model_reachable", "model_ready"):
            status.pop(field, None)
        catalog = req("GET", "tools")
        cid = req("POST", "conversations", expected=201)["id"]
        stream = req("POST", "chat/stream", {"conversation_id": cid, "message": "test calculation"})
        events = []
        for frame in stream.strip().split("\n\n"):
            name, data = frame.splitlines()
            value = json.loads(data.removeprefix("data: "))
            value.pop("turn_id", None)
            value.pop("conversation_id", None)
            events.append((name, value))
        assert events[-1][0] == "event: message_complete"
        turn = req("GET", "conversations/" + cid)["turns"][0]
        saved = {k: turn[k] for k in ("user", "assistant", "status", "error", "tools")}
        assert saved["tools"][0]["result"]["value"] == 38151
        invalid = req("POST", "chat/stream", {"message": "private"}, expected=422)
        missing = req("POST", "tools/unknown/run", {}, expected=404)
        latencies = {}
        for metric, action in (("health", lambda: req("GET", "health")),
                               ("detail", lambda: req("GET", "conversations/" + cid)),
                               ("create", lambda: req("POST", "conversations", expected=201)),
                               ("tool", lambda: req("POST", "tools/calculator/run", {"arguments": {"expression": "6*7"}}))):
            samples = []
            for _ in range(21):
                start = time.perf_counter_ns()
                action()
                samples.append((time.perf_counter_ns() - start) / 1000000)
            latencies[metric + "_median_ms"] = round(statistics.median(samples[1:]), 3)
        stream_samples = []
        for _ in range(6):
            scratch = req("POST", "conversations", expected=201)["id"]
            body = json.dumps({"conversation_id": scratch, "message": "test calculation"})
            start = time.perf_counter_ns()
            conn.request("POST", "/api/chat/stream", body, {"Content-Type": "application/json"})
            response = conn.getresponse()
            assert response.status == 200
            first_text = None
            terminal = False
            for line in response:
                if line.rstrip() == b"event: text_delta" and first_text is None:
                    first_text = (time.perf_counter_ns() - start) / 1000000
                if line.rstrip() == b"event: message_complete":
                    terminal = True
            assert first_text is not None and terminal
            stream_samples.append((first_text, (time.perf_counter_ns() - start) / 1000000))
        latencies["mock_first_text_ms"], latencies["mock_complete_ms"] = (
            round(statistics.median(v), 3) for v in zip(*stream_samples[1:])
        )
        rss, cpu = process_sample(pid)
        start = time.monotonic()
        time.sleep(2)
        _, cpu_end = process_sample(pid)
        perf = dict(startup_ms=startup, working_set_MiB=round(rss / 2**20, 2),
                    idle_core_percent=round(100 * (cpu_end-cpu)/(time.monotonic()-start), 2), **latencies)
    with legacy.ownership():
        assert legacy.detail(cid, 20, None)["turns"][0] == turn
        transcript, = legacy.history(cid, 1)
        assert [m["role"] for m in transcript] == ["user", "assistant", "tool", "assistant"]
        assert transcript[1]["tool_calls"][0]["id"] == transcript[2]["tool_call_id"] == "call_fixture"
    with backend(command, db, model_port, port), contextlib.closing(http.client.HTTPConnection("127.0.0.1", port)) as conn:
        again = req("GET", "conversations/" + cid)["turns"][0]
        assert again == turn
        req("DELETE", "conversations/" + cid, expected=204)
        req("GET", "conversations/" + cid, expected=404)
    rows = sql_rows(db)
    for table, originals in previous.items():
        assert all(rows[table].get(k) == v for k, v in originals.items()), "Existing records changed"
    with sqlite3.connect(db) as conn:
        assert conn.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
        assert conn.execute("PRAGMA foreign_key_check").fetchall() == []
    return (health, status, catalog, events, saved, invalid, missing), perf


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--legacy-db", type=Path, help="Optional quiescent Stage 1 validation database to back up read-only")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as temp, ThreadingHTTPServer(("127.0.0.1", 0), Model) as model:
        threading.Thread(target=model.serve_forever, daemon=True).start()
        baseline = None
        chosen_ports = []
        for name, command in (("python", [sys.executable, str(ROOT / "backend/run.py")]), ("rust", [str(args.binary.resolve())])):
            db = Path(temp) / (name + ".db")
            previous = {}
            if args.legacy_db:
                with contextlib.closing(sqlite3.connect(args.legacy_db.resolve().as_uri() + "?mode=ro", uri=True)) as src, contextlib.closing(sqlite3.connect(db)) as dest:
                    src.backup(dest)
                previous = sql_rows(db)
            port = free_port()
            while port in chosen_ports:
                port = free_port()
            chosen_ports.append(port)
            facts, perf = check(command, db, model.server_port, previous, port)
            if baseline is not None:
                assert facts == baseline, "Public contract differs from the reference"
            baseline = facts
            print(name, json.dumps(perf), flush=True)
        model.shutdown()
    print("Equivalent HTTP, SSE, saved activity, restart, deletion and database integrity.")


if __name__ == "__main__":
    main()

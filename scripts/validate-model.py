"""Exercise the real backend and GGUF. Leaves four named conversations for inspection."""
import argparse
import json
import re
from datetime import datetime

import httpx


CASES = [
    ("Hello. What is your purpose?", None),
    ("What is 157 multiplied by 243?", "calculator"),
    ("What is the current local date and time?", "get_current_datetime"),
    ("What operating system is this device running?", "get_system_information"),
]


def check_answer(tool: str | None, result: dict, reply: str):
    if tool == "calculator":
        assert result["value"] == 38151 and "38151" in re.sub(r"[,\s]", "", reply), "Incorrect product."
    if tool == "get_current_datetime":
        stamp = datetime.fromisoformat(result["iso_datetime"])
        assert str(stamp.year) in reply, "Check the date in the final answer."
        times = [stamp.strftime(fmt).lstrip("0") for fmt in ("%H:%M", "%I:%M")]
        assert any(t in reply for t in times), "Check the local time in the final answer."
    if tool == "get_system_information":
        assert result["operating_system"].lower() in reply.lower(), "Missing operating system in the reply."


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8000)
    args = parser.parse_args()
    with httpx.Client(base_url=f"http://127.0.0.1:{args.port}/api/", timeout=180, trust_env=False) as client:
        status = client.get("system/status").raise_for_status().json()
        if status["status"] != "ready":
            raise SystemExit(status["diagnostic"])
        for prompt, tool in CASES:
            cid = client.post("conversations").raise_for_status().json()["id"]
            print(f"\n{prompt}\nConversation {cid}", flush=True)
            with client.stream("POST", "chat/stream", json={"conversation_id": cid, "message": prompt}) as stream:
                stream.raise_for_status()
                events, name = [], ""
                for line in stream.iter_lines():
                    if line.startswith("event: "):
                        name = line[7:]
                    if line.startswith("data: "):
                        events.append((name, json.loads(line[6:])))
            reply = "".join(data["text"] for kind, data in events if kind == "text_delta")
            print(reply, flush=True)
            assert events and events[-1][0] == "message_complete", events[-1:]
            assert reply.strip(), "Empty reply."
            results = [data for kind, data in events if kind == "tool_result"]
            if tool:
                matches = [r for r in results if r["name"] == tool and r["ok"]]
                assert matches, f"Expected a successful structured call to {tool}."
                print(tool, matches[-1]["result"], flush=True)
                check_answer(tool, matches[-1]["result"], reply)
            detail = client.get("conversations/" + cid).raise_for_status().json()["turns"][-1]
            assert detail["assistant"] == reply and detail["status"] == "complete"
        print("\nAll four real-model interactions passed.")


if __name__ == "__main__":
    main()

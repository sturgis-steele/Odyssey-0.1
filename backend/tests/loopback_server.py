"""Launch the real backend with an audit hook for the offline integration test."""
import ipaddress
import runpy
import socket
import sys
from pathlib import Path

BACKEND = Path(__file__).resolve().parents[1]


def require_loopback(event, args):
    if event == "socket.connect":
        sock, address = args
        if sock.family not in (socket.AF_INET, socket.AF_INET6):
            return
        host = address[0]
    elif event in ("socket.getaddrinfo", "socket.gethostbyname", "socket.gethostbyaddr"):
        host = args[0]
    else:
        return
    if host is None or host == "localhost":
        return
    try:
        if ipaddress.ip_address(host).is_loopback:
            return
    except ValueError:
        pass
    raise PermissionError("Offline verification blocked a non-loopback network operation.")


if __name__ == "__main__":
    sys.addaudithook(require_loopback)
    # Confirm the test restriction before running the unmodified application.
    try:
        socket.getaddrinfo("offline-probe.invalid", 80)
    except PermissionError:
        print("Offline check active: backend TCP and hostname lookups restricted to loopback.", flush=True)
    else:
        raise RuntimeError("Offline verification was not activated.")
    sys.path.insert(0, str(BACKEND))
    runpy.run_path(str(BACKEND / "run.py"), run_name="__main__")

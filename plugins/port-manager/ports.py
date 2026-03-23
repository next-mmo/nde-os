"""Port Manager plugin — prevents port conflicts, auto-assigns ports, shows port map."""

import json
import os
import socket
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
from threading import Lock

# Port allocation state
allocations = {}  # app_id -> port
lock = Lock()

PORT_RANGE_START = int(os.environ.get("PORT_RANGE_START", "7860"))
PORT_RANGE_END = int(os.environ.get("PORT_RANGE_END", "7960"))


def is_port_free(port: int) -> bool:
    """Check if a TCP port is available on localhost."""
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(0.5)
            s.bind(("127.0.0.1", port))
            return True
    except OSError:
        return False


def find_free_port(preferred: int = 0) -> int:
    """Find a free port, preferring the given one if available."""
    if preferred and is_port_free(preferred):
        return preferred
    for port in range(PORT_RANGE_START, PORT_RANGE_END):
        if port not in allocations.values() and is_port_free(port):
            return port
    raise RuntimeError(f"No free ports in range {PORT_RANGE_START}-{PORT_RANGE_END}")


def allocate(app_id: str, preferred: int = 0) -> int:
    """Allocate a port for an app."""
    with lock:
        if app_id in allocations:
            return allocations[app_id]
        port = find_free_port(preferred)
        allocations[app_id] = port
        return port


def release(app_id: str):
    """Release a port allocation."""
    with lock:
        allocations.pop(app_id, None)


def get_port_map() -> list[dict]:
    """Get full port map: allocated + detected listeners."""
    result = []
    with lock:
        for app_id, port in allocations.items():
            in_use = not is_port_free(port)
            result.append({
                "port": port,
                "app_id": app_id,
                "allocated": True,
                "listening": in_use,
            })
    # Also scan the range for unknown listeners
    with lock:
        allocated_ports = set(allocations.values())
    for port in range(PORT_RANGE_START, PORT_RANGE_END):
        if port not in allocated_ports and not is_port_free(port):
            result.append({
                "port": port,
                "app_id": None,
                "allocated": False,
                "listening": True,
            })
    result.sort(key=lambda x: x["port"])
    return result


def handle_pre_launch(payload: dict) -> dict:
    """Middleware intercept: assign a port before app launch."""
    app_id = payload.get("app_id", "")
    env_vars = payload.get("env_vars", {})
    requested_port = int(env_vars.get("PORT", 0))

    port = allocate(app_id, requested_port)
    env_vars["PORT"] = str(port)
    payload["env_vars"] = env_vars
    return payload


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass

    def _json(self, data, status=200):
        body = json.dumps(data).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_body(self):
        length = int(self.headers.get("Content-Length", 0))
        return json.loads(self.rfile.read(length)) if length else {}

    def do_GET(self):
        if self.path == "/map" or self.path == "/api/plugins/port-manager/map":
            self._json({
                "ports": get_port_map(),
                "range": {"start": PORT_RANGE_START, "end": PORT_RANGE_END},
                "allocated_count": len(allocations),
            })
        elif self.path == "/health":
            self._json({"status": "ok"})
        else:
            self._json({"error": "not found"}, 404)

    def do_POST(self):
        if self.path == "/allocate":
            body = self._read_body()
            app_id = body.get("app_id", "")
            preferred = int(body.get("preferred_port", 0))
            if not app_id:
                self._json({"error": "app_id is required"}, 400)
                return
            try:
                port = allocate(app_id, preferred)
                self._json({"app_id": app_id, "port": port})
            except RuntimeError as e:
                self._json({"error": str(e)}, 503)
        elif self.path == "/release":
            body = self._read_body()
            app_id = body.get("app_id", "")
            release(app_id)
            self._json({"released": app_id})
        else:
            self._json({"error": "not found"}, 404)


def main():
    port = int(os.environ.get("PLUGIN_PORT", "9103"))

    # If invoked as middleware (CLI arg), process and return modified payload
    if len(sys.argv) > 1:
        try:
            event = json.loads(sys.argv[1])
            if event.get("type") == "pre_launch":
                result = handle_pre_launch(event.get("payload", {}))
                print(json.dumps(result))
        except (json.JSONDecodeError, KeyError):
            pass
        return

    server = HTTPServer(("127.0.0.1", port), Handler)
    print(json.dumps({"plugin": "port-manager", "port": port, "status": "running"}))
    sys.stdout.flush()
    server.serve_forever()


if __name__ == "__main__":
    main()

"""Disk Cleaner plugin — scan and clean uv caches, venvs, logs, orphaned workspaces."""

import json
import os
import shutil
import sys
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path

BASE_DIR = Path(os.environ.get("AI_LAUNCHER_DIR", os.path.expanduser("~/.ai-launcher")))


def get_dir_size(path: Path) -> int:
    """Recursively compute directory size in bytes."""
    total = 0
    try:
        for entry in path.rglob("*"):
            if entry.is_file():
                total += entry.stat().st_size
    except (PermissionError, OSError):
        pass
    return total


def scan():
    """Scan for cleanable items. Returns a list of findings."""
    findings = []

    # 1. uv cache
    uv_cache = Path(os.environ.get("UV_CACHE_DIR", Path.home() / ".cache" / "uv"))
    if uv_cache.exists():
        size = get_dir_size(uv_cache)
        if size > 0:
            findings.append({
                "id": "uv-cache",
                "label": "uv package cache",
                "path": str(uv_cache),
                "size_bytes": size,
                "safe_to_delete": True,
            })

    # 2. Unused .venv dirs inside app workspaces
    if BASE_DIR.exists():
        for app_dir in BASE_DIR.iterdir():
            if not app_dir.is_dir():
                continue
            workspace = app_dir / "workspace"
            venv = workspace / ".venv"
            if venv.exists():
                size = get_dir_size(venv)
                findings.append({
                    "id": f"venv-{app_dir.name}",
                    "label": f"venv for {app_dir.name}",
                    "path": str(venv),
                    "size_bytes": size,
                    "safe_to_delete": True,
                })

    # 3. Old log files (> 7 days)
    logs_dir = BASE_DIR / "logs"
    if logs_dir.exists():
        cutoff = time.time() - 7 * 86400
        for f in logs_dir.rglob("*.log"):
            try:
                if f.stat().st_mtime < cutoff:
                    findings.append({
                        "id": f"log-{f.name}",
                        "label": f"Old log: {f.name}",
                        "path": str(f),
                        "size_bytes": f.stat().st_size,
                        "safe_to_delete": True,
                    })
            except OSError:
                pass

    # 4. Plugin .venvs
    plugins_dir = BASE_DIR.parent / "plugins" if not (BASE_DIR / "plugins").exists() else BASE_DIR / "plugins"
    # Also check sibling plugins/ dir
    for pdir in [BASE_DIR / "plugins", BASE_DIR.parent / "plugins"]:
        if not pdir.exists():
            continue
        for plugin in pdir.iterdir():
            venv = plugin / ".venv"
            if venv.exists():
                size = get_dir_size(venv)
                findings.append({
                    "id": f"plugin-venv-{plugin.name}",
                    "label": f"Plugin venv: {plugin.name}",
                    "path": str(venv),
                    "size_bytes": size,
                    "safe_to_delete": True,
                })

    # 5. __pycache__ dirs
    for pycache in BASE_DIR.rglob("__pycache__"):
        size = get_dir_size(pycache)
        if size > 0:
            findings.append({
                "id": f"pycache-{pycache.parent.name}",
                "label": f"__pycache__ in {pycache.parent.name}",
                "path": str(pycache),
                "size_bytes": size,
                "safe_to_delete": True,
            })

    return findings


def clean(item_ids: list[str], findings: list[dict]) -> list[dict]:
    """Delete the selected items. Returns results."""
    results = []
    lookup = {f["id"]: f for f in findings}
    for item_id in item_ids:
        item = lookup.get(item_id)
        if not item:
            results.append({"id": item_id, "status": "not_found"})
            continue
        path = Path(item["path"])
        try:
            if path.is_dir():
                shutil.rmtree(path)
            elif path.is_file():
                path.unlink()
            results.append({"id": item_id, "status": "deleted", "freed_bytes": item["size_bytes"]})
        except Exception as e:
            results.append({"id": item_id, "status": "error", "error": str(e)})
    return results


# Keep last scan results for the clean endpoint
_last_scan = []


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
        global _last_scan
        if self.path == "/scan" or self.path == "/api/plugins/disk-cleaner/scan":
            _last_scan = scan()
            total = sum(f["size_bytes"] for f in _last_scan)
            self._json({"items": _last_scan, "total_bytes": total, "count": len(_last_scan)})
        elif self.path == "/health":
            self._json({"status": "ok"})
        else:
            self._json({"error": "not found"}, 404)

    def do_POST(self):
        if self.path == "/clean" or self.path == "/api/plugins/disk-cleaner/clean":
            body = self._read_body()
            ids = body.get("ids", [])
            if not ids:
                self._json({"error": "ids is required"}, 400)
                return
            results = clean(ids, _last_scan)
            freed = sum(r.get("freed_bytes", 0) for r in results)
            self._json({"results": results, "freed_bytes": freed})
        else:
            self._json({"error": "not found"}, 404)


def main():
    port = int(os.environ.get("PLUGIN_PORT", "9102"))
    server = HTTPServer(("127.0.0.1", port), Handler)
    print(json.dumps({"plugin": "disk-cleaner", "port": port, "status": "running"}))
    sys.stdout.flush()
    server.serve_forever()


if __name__ == "__main__":
    main()

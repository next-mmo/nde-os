"""Model Downloader plugin — background downloads with progress tracking."""

import json
import os
import sys
import time
import hashlib
import threading
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from urllib.parse import urlparse, parse_qs

try:
    from huggingface_hub import hf_hub_download, HfApi
    HAS_HF = True
except ImportError:
    HAS_HF = False

try:
    import requests
    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False

# In-memory download state
downloads = {}  # id -> {url, dest, progress, total, status, error, filename}
lock = threading.Lock()

MODELS_DIR = Path(os.environ.get("MODELS_DIR", os.path.expanduser("~/.ai-launcher/models")))
MODELS_DIR.mkdir(parents=True, exist_ok=True)


def download_id(url: str) -> str:
    return hashlib.sha256(url.encode()).hexdigest()[:12]


def download_direct(url: str, dest: Path, did: str):
    """Download a file from a direct URL with progress."""
    try:
        resp = requests.get(url, stream=True, timeout=30)
        resp.raise_for_status()
        total = int(resp.headers.get("content-length", 0))
        with lock:
            downloads[did]["total"] = total
            downloads[did]["status"] = "downloading"
        downloaded = 0
        with open(dest, "wb") as f:
            for chunk in resp.iter_content(chunk_size=8192):
                f.write(chunk)
                downloaded += len(chunk)
                with lock:
                    downloads[did]["progress"] = downloaded
        with lock:
            downloads[did]["status"] = "complete"
    except Exception as e:
        with lock:
            downloads[did]["status"] = "error"
            downloads[did]["error"] = str(e)


def download_hf(repo_id: str, filename: str, did: str):
    """Download a model file from HuggingFace Hub."""
    try:
        with lock:
            downloads[did]["status"] = "downloading"
        token = os.environ.get("HF_TOKEN")
        path = hf_hub_download(
            repo_id=repo_id,
            filename=filename,
            local_dir=str(MODELS_DIR / repo_id.replace("/", "_")),
            token=token,
        )
        with lock:
            downloads[did]["status"] = "complete"
            downloads[did]["dest"] = path
    except Exception as e:
        with lock:
            downloads[did]["status"] = "error"
            downloads[did]["error"] = str(e)


def start_download(url: str, filename: str = ""):
    """Start a background download. Returns download ID."""
    did = download_id(url)
    with lock:
        if did in downloads and downloads[did]["status"] == "downloading":
            return did  # already in progress

    # Determine type
    is_hf = "huggingface.co" in url or url.startswith("hf:")
    if not filename:
        filename = url.split("/")[-1] or "model.bin"

    dest = MODELS_DIR / filename
    with lock:
        downloads[did] = {
            "id": did,
            "url": url,
            "filename": filename,
            "dest": str(dest),
            "progress": 0,
            "total": 0,
            "status": "queued",
            "error": None,
        }

    if is_hf and HAS_HF:
        # Parse hf:org/repo/file or huggingface URL
        parts = url.replace("hf:", "").strip("/").split("/")
        if len(parts) >= 3:
            repo_id = f"{parts[0]}/{parts[1]}"
            hf_file = "/".join(parts[2:])
        else:
            repo_id = "/".join(parts[:2]) if len(parts) >= 2 else parts[0]
            hf_file = filename
        t = threading.Thread(target=download_hf, args=(repo_id, hf_file, did), daemon=True)
    elif HAS_REQUESTS:
        t = threading.Thread(target=download_direct, args=(url, dest, did), daemon=True)
    else:
        with lock:
            downloads[did]["status"] = "error"
            downloads[did]["error"] = "No download backend (install requests or huggingface-hub)"
        return did

    t.start()
    return did


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
        if self.path.startswith("/progress") or self.path.startswith("/api/plugins/model-downloader/progress"):
            with lock:
                self._json(list(downloads.values()))
        elif self.path == "/health":
            self._json({"status": "ok"})
        else:
            self._json({"error": "not found"}, 404)

    def do_POST(self):
        if self.path == "/download" or self.path == "/api/plugins/model-downloader/download":
            body = self._read_body()
            url = body.get("url", "")
            filename = body.get("filename", "")
            if not url:
                self._json({"error": "url is required"}, 400)
                return
            did = start_download(url, filename)
            with lock:
                self._json(downloads.get(did, {}))
        else:
            self._json({"error": "not found"}, 404)


def handle_hook(event_type: str, payload: dict):
    """Called as a hook on post_install — auto-download models declared in manifest."""
    manifest = payload.get("manifest", {})
    models = manifest.get("models", [])
    for model in models:
        url = model.get("url", "")
        filename = model.get("filename", "")
        if url:
            start_download(url, filename)


def main():
    port = int(os.environ.get("PLUGIN_PORT", "9101"))

    # If invoked as a hook (CLI arg), handle it and exit
    if len(sys.argv) > 1:
        try:
            event = json.loads(sys.argv[1])
            handle_hook(event.get("type", ""), event.get("payload", {}))
        except (json.JSONDecodeError, KeyError):
            pass
        return

    # Otherwise run as a daemon with HTTP API
    server = HTTPServer(("127.0.0.1", port), Handler)
    print(json.dumps({"plugin": "model-downloader", "port": port, "status": "running"}))
    sys.stdout.flush()
    server.serve_forever()


if __name__ == "__main__":
    main()

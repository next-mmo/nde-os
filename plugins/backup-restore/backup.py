"""Backup & Restore plugin — export/import app configs, models, workspace snapshots."""

import json
import os
import shutil
import sys
import tarfile
import tempfile
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path

BASE_DIR = Path(os.environ.get("AI_LAUNCHER_DIR", os.path.expanduser("~/.ai-launcher")))
BACKUP_DIR = BASE_DIR / "backups"
BACKUP_DIR.mkdir(parents=True, exist_ok=True)

# Optional S3 support
try:
    import boto3
    HAS_S3 = True
except ImportError:
    HAS_S3 = False


def create_backup(app_ids: list[str] = None, include_models: bool = False) -> dict:
    """Create a tarball backup of selected apps (or all)."""
    timestamp = time.strftime("%Y%m%d-%H%M%S")
    backup_name = f"backup-{timestamp}.tar.gz"
    backup_path = BACKUP_DIR / backup_name

    with tarfile.open(backup_path, "w:gz") as tar:
        # Registry
        registry_path = BASE_DIR / "registry.json"
        if registry_path.exists():
            tar.add(registry_path, arcname="registry.json")

        # App workspaces
        for app_dir in BASE_DIR.iterdir():
            if not app_dir.is_dir():
                continue
            if app_ids and app_dir.name not in app_ids:
                continue
            # Skip non-app dirs
            workspace = app_dir / "workspace"
            if not workspace.exists():
                continue
            # Add config files from workspace (not the whole venv)
            for f in workspace.iterdir():
                if f.name in (".venv", "node_modules", "__pycache__", ".git"):
                    continue
                tar.add(f, arcname=f"apps/{app_dir.name}/{f.name}")

        # Models (optional, can be large)
        if include_models:
            models_dir = BASE_DIR / "models"
            if models_dir.exists():
                tar.add(models_dir, arcname="models")

    size = backup_path.stat().st_size
    return {
        "name": backup_name,
        "path": str(backup_path),
        "size_bytes": size,
        "timestamp": timestamp,
        "app_ids": app_ids or "all",
        "include_models": include_models,
    }


def restore_backup(backup_path: str) -> dict:
    """Restore from a backup tarball."""
    path = Path(backup_path)
    if not path.exists():
        return {"error": f"Backup not found: {backup_path}"}

    restore_dir = BASE_DIR
    extracted = []

    with tarfile.open(path, "r:gz") as tar:
        # Safety: reject paths that escape the target directory
        for member in tar.getmembers():
            resolved = (restore_dir / member.name).resolve()
            if not str(resolved).startswith(str(restore_dir.resolve())):
                return {"error": f"Unsafe path in archive: {member.name}"}

        for member in tar.getmembers():
            if member.name == "registry.json":
                tar.extract(member, restore_dir)
                extracted.append("registry.json")
            elif member.name.startswith("apps/"):
                parts = member.name.split("/", 2)
                if len(parts) >= 3:
                    app_id = parts[1]
                    dest_dir = restore_dir / app_id / "workspace"
                    dest_dir.mkdir(parents=True, exist_ok=True)
                    tar.extract(member, restore_dir)
                    extracted.append(member.name)
            elif member.name.startswith("models/"):
                tar.extract(member, restore_dir)
                extracted.append(member.name)

    return {"restored": len(extracted), "files": extracted[:50]}


def list_backups() -> list[dict]:
    """List available local backups."""
    backups = []
    for f in sorted(BACKUP_DIR.glob("backup-*.tar.gz"), reverse=True):
        stat = f.stat()
        backups.append({
            "name": f.name,
            "path": str(f),
            "size_bytes": stat.st_size,
            "created": stat.st_mtime,
        })
    return backups


def upload_to_s3(backup_path: str, bucket: str, prefix: str = "ai-launcher-backups/") -> dict:
    """Upload a backup to S3."""
    if not HAS_S3:
        return {"error": "boto3 not installed"}
    path = Path(backup_path)
    key = prefix + path.name
    s3 = boto3.client("s3")
    s3.upload_file(str(path), bucket, key)
    return {"bucket": bucket, "key": key, "size_bytes": path.stat().st_size}


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
        if self.path == "/list" or self.path == "/api/plugins/backup-restore/list":
            self._json({"backups": list_backups()})
        elif self.path == "/health":
            self._json({"status": "ok"})
        else:
            self._json({"error": "not found"}, 404)

    def do_POST(self):
        if self.path == "/export" or self.path == "/api/plugins/backup-restore/export":
            body = self._read_body()
            app_ids = body.get("app_ids")
            include_models = body.get("include_models", False)
            try:
                result = create_backup(app_ids, include_models)
                # Optionally upload to S3
                s3_bucket = body.get("s3_bucket") or os.environ.get("S3_BUCKET")
                if s3_bucket:
                    s3_result = upload_to_s3(result["path"], s3_bucket)
                    result["s3"] = s3_result
                self._json(result)
            except Exception as e:
                self._json({"error": str(e)}, 500)
        elif self.path == "/import" or self.path == "/api/plugins/backup-restore/import":
            body = self._read_body()
            backup_path = body.get("path", "")
            if not backup_path:
                self._json({"error": "path is required"}, 400)
                return
            result = restore_backup(backup_path)
            status = 500 if "error" in result else 200
            self._json(result, status)
        else:
            self._json({"error": "not found"}, 404)


def main():
    port = int(os.environ.get("PLUGIN_PORT", "9104"))
    server = HTTPServer(("127.0.0.1", port), Handler)
    print(json.dumps({"plugin": "backup-restore", "port": port, "status": "running"}))
    sys.stdout.flush()
    server.serve_forever()


if __name__ == "__main__":
    main()

"""
NDE Actor SDK — Local storage implementations.

Provides JSONL-based Dataset and file-based Key-Value Store
that mirror the Apify storage APIs for local (NDE-OS) usage.
"""
import json
import os
from typing import Optional


class LocalDataset:
    """Append-only JSONL dataset.

    Each call to push() appends JSON lines to the file.
    Compatible with the Apify Dataset API pattern.
    """

    def __init__(self, path: str):
        self.path = path
        # Ensure parent directory exists
        parent = os.path.dirname(path)
        if parent:
            os.makedirs(parent, exist_ok=True)

    def push(self, items: list[dict]):
        """Append items to the dataset."""
        with open(self.path, "a", encoding="utf-8") as f:
            for item in items:
                f.write(json.dumps(item, ensure_ascii=False) + "\n")

    def get_items(self, offset: int = 0, limit: int = 0) -> list[dict]:
        """Read items with pagination."""
        if not os.path.exists(self.path):
            return []

        items = []
        with open(self.path, "r", encoding="utf-8") as f:
            for i, line in enumerate(f):
                line = line.strip()
                if not line:
                    continue
                if i < offset:
                    continue
                if limit > 0 and len(items) >= limit:
                    break
                try:
                    items.append(json.loads(line))
                except json.JSONDecodeError:
                    pass

        return items

    def count(self) -> int:
        """Count total items in the dataset."""
        if not os.path.exists(self.path):
            return 0

        count = 0
        with open(self.path, "r", encoding="utf-8") as f:
            for line in f:
                if line.strip():
                    count += 1
        return count

    def export_json(self) -> list[dict]:
        """Export all items as a list."""
        return self.get_items()


class LocalKvStore:
    """File-based key-value store.

    Each key is stored as a file in the store directory.
    Content types are tracked in companion .content-type files.
    Compatible with the Apify Key-Value Store API pattern.
    """

    def __init__(self, directory: str):
        self.directory = directory
        os.makedirs(directory, exist_ok=True)

    def _sanitize_key(self, key: str) -> str:
        """Sanitize key for filesystem safety."""
        return "".join(
            c if c.isalnum() or c in "-_." else "_"
            for c in key
        )

    def set(self, key: str, value: bytes, content_type: str = "application/octet-stream"):
        """Store a value under the given key."""
        safe_key = self._sanitize_key(key)
        value_path = os.path.join(self.directory, safe_key)
        meta_path = os.path.join(self.directory, f"{safe_key}.content-type")

        with open(value_path, "wb") as f:
            f.write(value if isinstance(value, bytes) else value.encode("utf-8"))

        with open(meta_path, "w") as f:
            f.write(content_type)

    def get(self, key: str) -> Optional[bytes]:
        """Retrieve a value by key. Returns None if not found."""
        safe_key = self._sanitize_key(key)
        value_path = os.path.join(self.directory, safe_key)

        if not os.path.exists(value_path):
            return None

        with open(value_path, "rb") as f:
            return f.read()

    def get_content_type(self, key: str) -> Optional[str]:
        """Get the content type for a stored key."""
        safe_key = self._sanitize_key(key)
        meta_path = os.path.join(self.directory, f"{safe_key}.content-type")

        if not os.path.exists(meta_path):
            return None

        with open(meta_path, "r") as f:
            return f.read().strip()

    def set_json(self, key: str, value: dict):
        """Store a JSON value."""
        data = json.dumps(value, indent=2, ensure_ascii=False).encode("utf-8")
        self.set(key, data, "application/json")

    def get_json(self, key: str) -> Optional[dict]:
        """Retrieve a JSON value."""
        data = self.get(key)
        if data is None:
            return None
        return json.loads(data)

    def list_keys(self) -> list[str]:
        """List all keys (excluding metadata files)."""
        if not os.path.exists(self.directory):
            return []

        return sorted(
            f for f in os.listdir(self.directory)
            if not f.endswith(".content-type")
        )

    def delete(self, key: str):
        """Delete a key from the store."""
        safe_key = self._sanitize_key(key)
        value_path = os.path.join(self.directory, safe_key)
        meta_path = os.path.join(self.directory, f"{safe_key}.content-type")

        if os.path.exists(value_path):
            os.remove(value_path)
        if os.path.exists(meta_path):
            os.remove(meta_path)

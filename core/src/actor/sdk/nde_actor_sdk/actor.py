"""
NDE Actor SDK — Dual-runtime Actor class.

Detects whether running on NDE-OS (via NDE_ACTOR env var)
or Apify (via APIFY_TOKEN env var) and routes storage/browser
calls accordingly. Same actor code works in both environments.

Usage:
    from nde_actor_sdk import Actor

    await Actor.init()
    input_data = await Actor.get_input()
    browser = await Actor.get_browser()
    # ... do work ...
    await Actor.push_data([{"url": "...", "title": "..."}])
    await Actor.exit()
"""
import json
import os
import sys
from typing import Any, Optional

from .storage import LocalDataset, LocalKvStore


class Actor:
    """Apify-compatible Actor SDK that works on both NDE-OS and Apify."""

    _dataset: Optional[LocalDataset] = None
    _kv_store: Optional[LocalKvStore] = None
    _browser: Any = None
    _is_nde: bool = False
    _input: Optional[dict] = None

    @classmethod
    async def init(cls):
        """Initialize the actor runtime.

        On NDE-OS: sets up local Dataset and KV Store from env vars.
        On Apify: delegates to apify.Actor.init().
        """
        cls._is_nde = bool(os.environ.get("NDE_ACTOR"))

        if cls._is_nde:
            dataset_path = os.environ.get("NDE_DATASET_PATH", "dataset.jsonl")
            kv_path = os.environ.get("NDE_KV_STORE_PATH", "key-value-store")
            cls._dataset = LocalDataset(dataset_path)
            cls._kv_store = LocalKvStore(kv_path)
            print(f"[NDE Actor SDK] Initialized (run_id={os.environ.get('NDE_RUN_ID', 'unknown')})")
        else:
            # Apify runtime — try to import and delegate
            try:
                from apify import Actor as ApifyActor
                await ApifyActor.init()
            except ImportError:
                # Standalone mode — use local storage
                cls._dataset = LocalDataset("dataset.jsonl")
                cls._kv_store = LocalKvStore("key-value-store")
                print("[NDE Actor SDK] Initialized in standalone mode")

    @classmethod
    async def get_input(cls) -> dict:
        """Get the actor's input configuration.

        On NDE-OS: reads from NDE_INPUT_PATH env var or stdin.
        On Apify: delegates to apify.Actor.getInput().
        """
        if cls._input is not None:
            return cls._input

        if cls._is_nde:
            input_path = os.environ.get("NDE_INPUT_PATH")
            if input_path and os.path.exists(input_path):
                with open(input_path, "r") as f:
                    cls._input = json.load(f)
            else:
                # Try reading from KV store INPUT key
                if cls._kv_store:
                    data = cls._kv_store.get("INPUT")
                    if data:
                        cls._input = json.loads(data)

            if cls._input is None:
                cls._input = {}

        else:
            try:
                from apify import Actor as ApifyActor
                cls._input = await ApifyActor.get_input() or {}
            except ImportError:
                cls._input = {}

        return cls._input

    @classmethod
    async def push_data(cls, items: list[dict]):
        """Push data items to the dataset.

        On NDE-OS: appends to local JSONL file.
        On Apify: delegates to apify.Actor.push_data().
        """
        if cls._is_nde:
            if cls._dataset:
                cls._dataset.push(items)
        else:
            try:
                from apify import Actor as ApifyActor
                await ApifyActor.push_data(items)
            except ImportError:
                if cls._dataset:
                    cls._dataset.push(items)

    @classmethod
    async def set_value(cls, key: str, value: bytes, content_type: str = "application/octet-stream"):
        """Store a value in the key-value store.

        On NDE-OS: writes to local file.
        On Apify: delegates to default KV store.
        """
        if cls._is_nde:
            if cls._kv_store:
                cls._kv_store.set(key, value, content_type)
        else:
            try:
                from apify import Actor as ApifyActor
                store = await ApifyActor.open_key_value_store()
                await store.set_value(key, value, content_type=content_type)
            except ImportError:
                if cls._kv_store:
                    cls._kv_store.set(key, value, content_type)

    @classmethod
    async def get_value(cls, key: str) -> Optional[bytes]:
        """Retrieve a value from the key-value store."""
        if cls._is_nde:
            if cls._kv_store:
                return cls._kv_store.get(key)
            return None
        else:
            try:
                from apify import Actor as ApifyActor
                store = await ApifyActor.open_key_value_store()
                return await store.get_value(key)
            except ImportError:
                if cls._kv_store:
                    return cls._kv_store.get(key)
                return None

    @classmethod
    async def get_browser(cls):
        """Get a browser instance.

        On NDE-OS: connects to Shield browser via CDP endpoint.
        On Apify: launches a new Playwright browser.
        """
        if cls._browser is not None:
            return cls._browser

        from .browser import get_browser
        cls._browser = await get_browser()
        return cls._browser

    @classmethod
    async def exit(cls, status: str = "SUCCEEDED"):
        """Graceful shutdown.

        On NDE-OS: just logs completion.
        On Apify: delegates to apify.Actor.exit().
        """
        if cls._browser:
            try:
                await cls._browser.close()
            except Exception:
                pass
            cls._browser = None

        if cls._is_nde:
            items = cls._dataset.count() if cls._dataset else 0
            print(f"[NDE Actor SDK] Exit: status={status}, dataset_items={items}")
        else:
            try:
                from apify import Actor as ApifyActor
                await ApifyActor.exit()
            except ImportError:
                items = cls._dataset.count() if cls._dataset else 0
                print(f"[NDE Actor SDK] Exit: status={status}, dataset_items={items}")

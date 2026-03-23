"""GPU Monitor plugin — polls NVIDIA GPU stats via pynvml and serves them as JSON."""

import json
import sys
import time
from collections import deque
from http.server import HTTPServer, BaseHTTPRequestHandler
from threading import Thread

try:
    import pynvml
    HAS_NVML = True
except ImportError:
    HAS_NVML = False

HISTORY_MAX = 720  # ~1h at 5s interval
history = deque(maxlen=HISTORY_MAX)
latest = {}


def read_gpu_stats():
    """Read stats from GPU 0 (extend for multi-GPU later)."""
    if not HAS_NVML:
        return {"error": "pynvml not installed"}

    try:
        count = pynvml.nvmlDeviceGetCount()
    except pynvml.NVMLError:
        return {"error": "No NVIDIA GPU detected"}

    gpus = []
    for i in range(count):
        handle = pynvml.nvmlDeviceGetHandleByIndex(i)
        util = pynvml.nvmlDeviceGetUtilizationRates(handle)
        mem = pynvml.nvmlDeviceGetMemoryInfo(handle)
        try:
            temp = pynvml.nvmlDeviceGetTemperature(handle, pynvml.NVML_TEMPERATURE_GPU)
        except pynvml.NVMLError:
            temp = -1
        try:
            power = pynvml.nvmlDeviceGetPowerUsage(handle) / 1000.0  # mW -> W
        except pynvml.NVMLError:
            power = -1
        try:
            name = pynvml.nvmlDeviceGetName(handle)
            if isinstance(name, bytes):
                name = name.decode()
        except pynvml.NVMLError:
            name = f"GPU {i}"

        gpus.append({
            "index": i,
            "name": name,
            "gpu_util_percent": util.gpu,
            "mem_util_percent": util.memory,
            "mem_used_mb": mem.used // 1048576,
            "mem_total_mb": mem.total // 1048576,
            "temperature_c": temp,
            "power_w": round(power, 1),
        })
    return {"gpus": gpus, "count": count}


def poll_loop(interval: int):
    """Background thread that polls GPU stats on a timer."""
    global latest
    if HAS_NVML:
        pynvml.nvmlInit()
    while True:
        stats = read_gpu_stats()
        stats["timestamp"] = time.time()
        latest = stats
        history.append(stats)
        time.sleep(interval)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass  # silence request logs

    def _json(self, data, status=200):
        body = json.dumps(data).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/stats" or self.path == "/api/plugins/gpu-monitor/stats":
            self._json(latest or {"error": "no data yet"})
        elif self.path == "/history" or self.path == "/api/plugins/gpu-monitor/history":
            self._json(list(history))
        elif self.path == "/health":
            self._json({"status": "ok"})
        else:
            self._json({"error": "not found"}, 404)


def main():
    import os
    interval = int(os.environ.get("PLUGIN_REFRESH_RATE", "5"))
    port = int(os.environ.get("PLUGIN_PORT", "9100"))

    # Start polling thread
    t = Thread(target=poll_loop, args=(interval,), daemon=True)
    t.start()

    server = HTTPServer(("127.0.0.1", port), Handler)
    print(json.dumps({"plugin": "gpu-monitor", "port": port, "status": "running"}))
    sys.stdout.flush()
    server.serve_forever()


if __name__ == "__main__":
    main()

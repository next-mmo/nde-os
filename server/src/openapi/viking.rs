use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/viking/status":{
            "get":{
                "tags":["viking"],
                "summary":"OpenViking server status",
                "operationId":"vikingStatus",
                "description":"Returns connectivity status for the OpenViking context-database server running on localhost:1933. Reports both HTTP reachability and whether NDE-OS is managing the process.",
                "responses":{
                    "200":{
                        "description":"Status report (connected or not)",
                        "content":{"application/json":{"schema":{"$ref":"#/components/schemas/VikingStatusResponse"}}}
                    }
                }
            }
        },
        "/api/viking/install":{
            "post":{
                "tags":["viking"],
                "summary":"Install OpenViking via uv/pip",
                "operationId":"vikingInstall",
                "description":"Installs the `openviking` Python package into the system or active venv using `uv pip install` (with pip fallback). Safe to call multiple times — skips if already installed.",
                "responses":{
                    "200":{
                        "description":"Installation succeeded",
                        "content":{"application/json":{"schema":{"$ref":"#/components/schemas/VikingInstallResponse"}}}
                    },
                    "500":{
                        "description":"Installation failed — Python or uv/pip not available",
                        "content":{"application/json":{"schema":{"$ref":"#/components/schemas/ApiResponse"}}}
                    }
                }
            }
        },
        "/api/viking/start":{
            "post":{
                "tags":["viking"],
                "summary":"Start OpenViking server",
                "operationId":"vikingStart",
                "description":"Writes `ov.conf` / `ovcli.conf` and spawns `openviking-server` as a managed subprocess on port 1933. Waits up to 15 s for the health endpoint to respond before returning.",
                "responses":{
                    "200":{
                        "description":"Server started (or already running)",
                        "content":{"application/json":{"schema":{"$ref":"#/components/schemas/VikingStartResponse"}}}
                    },
                    "500":{
                        "description":"Failed to spawn process",
                        "content":{"application/json":{"schema":{"$ref":"#/components/schemas/ApiResponse"}}}
                    }
                }
            }
        },
        "/api/viking/stop":{
            "post":{
                "tags":["viking"],
                "summary":"Stop OpenViking server",
                "operationId":"vikingStop",
                "description":"Sends SIGKILL to the managed `openviking-server` subprocess. No-op if not currently running.",
                "responses":{
                    "200":{
                        "description":"Server stopped",
                        "content":{"application/json":{"schema":{"$ref":"#/components/schemas/VikingStopResponse"}}}
                    }
                }
            }
        }
    })
}

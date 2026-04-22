use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/apps":{
            "get":{"tags":["apps"],"summary":"List installed apps","operationId":"listApps","responses":{"200":{"description":"Installed apps"}}},
            "post":{"tags":["apps"],"summary":"Install app into sandbox","operationId":"installApp",
                "description":"Creates sandboxed workspace, verifies security, creates uv venv with pinned Python version, installs pip deps via uv (10-100x faster than pip).",
                "requestBody":{"required":true,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/InstallRequest"},
                    "example":{"manifest":{"id":"gradio-demo","name":"Gradio Demo","description":"Test app","author":"ai-launcher","python_version":"3","needs_gpu":false,"pip_deps":["gradio"],"launch_cmd":"python3 app.py","port":7860,"env":[],"disk_size":"~200MB","tags":["demo"]}}}}},
                "responses":{"201":{"description":"Installed"},"400":{"description":"Failed"},"409":{"description":"Already installed"}}}
        },
        "/api/apps/{app_id}":{
            "get":{"tags":["apps"],"summary":"Get app details","operationId":"getApp","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"App details"},"404":{"description":"Not found"}}},
            "delete":{"tags":["apps"],"summary":"Uninstall app and remove workspace","operationId":"uninstallApp","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Uninstalled"},"404":{"description":"Not found"}}}
        },
        "/api/apps/{app_id}/launch":{"post":{"tags":["apps"],"summary":"Launch app in sandbox","description":"Spawns process inside uv venv with jailed HOME, TMPDIR, TEMP, USERPROFILE, PYTHONPATH, APPDATA, VIRTUAL_ENV (cross-platform)","operationId":"launchApp","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Launched","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ApiResponse"}}}},"404":{"description":"Not installed"},"409":{"description":"Already running"}}}},
        "/api/apps/{app_id}/stop":{"post":{"tags":["apps"],"summary":"Stop running app","operationId":"stopApp","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Stopped"},"404":{"description":"Not running"}}}}
    })
}

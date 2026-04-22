use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/sandbox/{app_id}/verify":{"get":{"tags":["sandbox"],"summary":"Verify sandbox security","description":"Tests: path traversal (Unix+Windows paths), absolute escape, symlink escape, valid path resolution","operationId":"verifySandbox","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Verification results"}}}},
        "/api/sandbox/{app_id}/disk":{"get":{"tags":["sandbox"],"summary":"Workspace disk usage","operationId":"getDiskUsage","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Disk usage"}}}}
    })
}

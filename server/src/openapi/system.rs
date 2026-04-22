use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/health":{"get":{"tags":["system"],"summary":"Health check","operationId":"healthCheck","responses":{"200":{"description":"Healthy"}}}},
        "/api/system":{"get":{"tags":["system"],"summary":"System info (OS, Python, GPU)","operationId":"getSystemInfo","responses":{"200":{"description":"System details"}}}},
        "/api/system/resources":{"get":{"tags":["system"],"summary":"Live RAM and disk usage","operationId":"getSystemResources","responses":{"200":{"description":"Live resource usage","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ResourceUsage"}}}}}}}
    })
}

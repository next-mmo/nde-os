use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/catalog":{"get":{"tags":["catalog"],"summary":"List available apps","operationId":"getCatalog","responses":{"200":{"description":"App catalog"}}}},
        "/api/store/upload":{"post":{"tags":["store"],"summary":"Upload app to store","operationId":"storeUpload",
            "description":"Upload an app via local folder, ZIP file, or Git URL. Validates structure, reads manifest.json, performs a trial install. Only accepts the app if install succeeds.",
            "requestBody":{"required":true,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/StoreUploadRequest"},
                "examples":{
                    "folder":{"summary":"Upload from folder","value":{"source_type":"folder","source_path":"/path/to/my-app"}},
                    "zip":{"summary":"Upload from ZIP","value":{"source_type":"zip","source_path":"/path/to/my-app.zip"}},
                    "git":{"summary":"Upload from Git URL","value":{"source_type":"git_url","git_url":"https://github.com/user/my-app.git"}}
                }}}},
            "responses":{
                "201":{"description":"App uploaded and installed successfully","content":{"application/json":{"schema":{"$ref":"#/components/schemas/StoreUploadResult"}}}},
                "400":{"description":"Validation or install failed","content":{"application/json":{"schema":{"$ref":"#/components/schemas/StoreUploadResult"}}}},
                "500":{"description":"Internal error"}
            }}}
    })
}

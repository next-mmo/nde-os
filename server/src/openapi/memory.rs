use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/memory/status": {
            "get": {
                "tags": ["memory"],
                "summary": "Get memory substrate status",
                "responses": {
                    "200": {
                        "description": "Status retrieved",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "status": { "type": "string" },
                                        "database": { "type": "string" },
                                        "row_counts": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        "/api/memory/remember": {
            "post": {
                "tags": ["memory"],
                "summary": "Store a memory fragment",
                "requestBody": {
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "content": { "type": "string" },
                                    "source": { "type": "string" }
                                },
                                "required": ["content"]
                            }
                        }
                    }
                },
                "responses": {
                    "200": { "description": "Memory stored" }
                }
            }
        },
        "/api/memory/recall": {
            "post": {
                "tags": ["memory"],
                "summary": "Recall memories by query",
                "requestBody": {
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string" },
                                    "limit": { "type": "integer" }
                                },
                                "required": ["query"]
                            }
                        }
                    }
                },
                "responses": {
                    "200": { "description": "Memories recalled" }
                }
            }
        }
    })
}

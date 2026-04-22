use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/agent/chat":{"post":{"tags":["agent"],"summary":"Send message to agent","operationId":"agentChat",
            "description":"Send a user message to the agent and get a response. Creates or continues a conversation.",
            "requestBody":{"required":true,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/ChatRequest"},
                "example":{"message":"What tools do you have?"}}}},
            "responses":{
                "200":{"description":"Agent response","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ChatResponse"}}}},
                "502":{"description":"LLM provider error"}
            }}},
        "/api/agent/conversations":{"get":{"tags":["agent"],"summary":"List conversations","operationId":"agentConversations",
            "responses":{"200":{"description":"List of conversations","content":{"application/json":{"schema":{"type":"array","items":{"$ref":"#/components/schemas/ConversationSummary"}}}}}}}},
        "/api/agent/conversations/{conv_id}/messages":{"get":{"tags":["agent"],"summary":"Get conversation messages","operationId":"agentMessages",
            "parameters":[{"name":"conv_id","in":"path","required":true,"schema":{"type":"string"}}],
            "responses":{"200":{"description":"Messages in conversation","content":{"application/json":{"schema":{"type":"array","items":{"$ref":"#/components/schemas/StoredMessage"}}}}}}}},
        "/api/agent/config":{"get":{"tags":["agent"],"summary":"Get agent configuration","operationId":"agentConfig",
            "responses":{"200":{"description":"Current agent config","content":{"application/json":{"schema":{"$ref":"#/components/schemas/AgentConfigInfo"}}}}}}}
    })
}

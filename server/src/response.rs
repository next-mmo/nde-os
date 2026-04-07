use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::io::Cursor;
use tiny_http::{Header, Request, Response};

// ── Type alias to reduce noise ──────────────────────────────────────────────

pub type HttpResponse = Response<Cursor<Vec<u8>>>;

// ── JSON helpers ────────────────────────────────────────────────────────────

/// Create a JSON response with CORS headers (compact serialization).
pub fn json_resp(status: u16, body: &Value) -> HttpResponse {
    let data = serde_json::to_string(body).unwrap_or_default();
    Response::from_data(data.into_bytes())
        .with_status_code(tiny_http::StatusCode(status))
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
}

pub fn ok<T: serde::Serialize>(msg: &str, data: T) -> HttpResponse {
    json_resp(200, &json!({"success":true,"message":msg,"data":data}))
}

pub fn created<T: serde::Serialize>(msg: &str, data: T) -> HttpResponse {
    json_resp(201, &json!({"success":true,"message":msg,"data":data}))
}

pub fn err(status: u16, msg: &str) -> HttpResponse {
    json_resp(status, &json!({"success":false,"message":msg,"data":null}))
}

pub fn html(body: &str) -> HttpResponse {
    Response::from_data(body.as_bytes().to_vec())
        .with_header(Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap())
}

// ── Body parsing ────────────────────────────────────────────────────────────

/// Read the raw request body as a String.
pub fn read_body(req: &mut Request) -> Option<String> {
    let mut buf = String::new();
    req.as_reader().read_to_string(&mut buf).ok()?;
    Some(buf)
}

/// Read + deserialize JSON body in one step.
/// Returns either the parsed value or an HTTP 400 error response.
pub fn parse_body<T: DeserializeOwned>(req: &mut Request) -> Result<T, HttpResponse> {
    let body = read_body(req).ok_or_else(|| err(400, "Missing request body"))?;
    serde_json::from_str::<T>(&body).map_err(|e| err(400, &format!("Invalid JSON: {}", e)))
}

// ── SSE helpers ─────────────────────────────────────────────────────────────

/// Wrap raw SSE bytes into a properly-headered event-stream response.
pub fn sse_response(data: Vec<u8>) -> HttpResponse {
    Response::from_data(data)
        .with_header(Header::from_bytes("Content-Type", "text/event-stream").unwrap())
        .with_header(Header::from_bytes("Cache-Control", "no-cache").unwrap())
        .with_header(Header::from_bytes("Connection", "keep-alive").unwrap())
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
}

/// Wrap raw SSE bytes into an error event-stream response (no cache/keepalive).
pub fn sse_error_response(data: Vec<u8>) -> HttpResponse {
    Response::from_data(data)
        .with_header(Header::from_bytes("Content-Type", "text/event-stream").unwrap())
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
}

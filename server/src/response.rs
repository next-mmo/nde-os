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

pub fn ok_msg(msg: &str) -> HttpResponse {
    json_resp(200, &json!({"success":true,"message":msg,"data":null}))
}

pub fn err(status: u16, msg: &str) -> HttpResponse {
    json_resp(status, &json!({"success":false,"message":msg,"data":null}))
}

pub fn html(body: &str) -> HttpResponse {
    Response::from_data(body.as_bytes().to_vec())
        .with_header(Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap())
}

// ── Body parsing ────────────────────────────────────────────────────────────

/// Parse a simple multipart/form-data body.
/// Returns a list of `(field_name, field_bytes)` pairs.
pub fn parse_multipart(body: &[u8], boundary: &str) -> Vec<(String, Vec<u8>)> {
    let mut fields = Vec::new();
    let delimiter = format!("--{}", boundary).into_bytes();

    let parts = body
        .windows(delimiter.len())
        .enumerate()
        .filter(|(_, w)| *w == delimiter.as_slice())
        .map(|(i, _)| i)
        .collect::<Vec<_>>();

    for i in 0..parts.len().saturating_sub(1) {
        let start = parts[i] + delimiter.len();
        let end = parts[i + 1];

        let mut part = &body[start..end];

        if part.starts_with(b"\r\n") {
            part = &part[2..];
        }
        if part.ends_with(b"\r\n") {
            part = &part[..part.len() - 2];
        }

        if let Some(header_end) = part.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&part[..header_end]);
            let data = part[header_end + 4..].to_vec();

            if let Some(cd_line) = headers.lines().find(|l| l.starts_with("Content-Disposition")) {
                if let Some(name_start) = cd_line.find("name=\"") {
                    let after = &cd_line[name_start + 6..];
                    if let Some(name_end) = after.find('"') {
                        fields.push((after[..name_end].to_string(), data));
                    }
                }
            }
        }
    }
    fields
}

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

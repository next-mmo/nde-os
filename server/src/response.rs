use serde_json::{json, Value};
use std::io::Cursor;
use tiny_http::{Header, Request, Response};

/// Create a JSON response with CORS headers
pub fn json_resp(status: u16, body: &Value) -> Response<Cursor<Vec<u8>>> {
    let data = serde_json::to_string_pretty(body).unwrap_or_default();
    Response::from_data(data.into_bytes())
        .with_status_code(tiny_http::StatusCode(status))
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
}

pub fn ok<T: serde::Serialize>(msg: &str, data: T) -> Response<Cursor<Vec<u8>>> {
    json_resp(200, &json!({"success":true,"message":msg,"data":data}))
}

pub fn created<T: serde::Serialize>(msg: &str, data: T) -> Response<Cursor<Vec<u8>>> {
    json_resp(201, &json!({"success":true,"message":msg,"data":data}))
}

pub fn err(status: u16, msg: &str) -> Response<Cursor<Vec<u8>>> {
    json_resp(status, &json!({"success":false,"message":msg,"data":null}))
}

pub fn html(body: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(body.as_bytes().to_vec())
        .with_header(Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap())
}

pub fn read_body(req: &mut Request) -> Option<String> {
    let mut buf = String::new();
    req.as_reader().read_to_string(&mut buf).ok()?;
    Some(buf)
}

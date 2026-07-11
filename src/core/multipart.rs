//! Small multipart/form-data helpers shared by file-upload operations.

use http::header::HeaderValue;

use crate::error::{Error, Result};

pub(crate) fn boundary_for(payload: &[u8]) -> String {
    for index in 0u64.. {
        let boundary = format!("blooio-form-boundary-{index}");
        if !contains_bytes(payload, boundary.as_bytes()) {
            return boundary;
        }
    }
    unreachable!("unbounded boundary search must find a value absent from a finite payload")
}

pub(crate) fn part_content_type(content_type: Option<&str>) -> Result<&str> {
    let content_type = content_type.unwrap_or("application/octet-stream");
    HeaderValue::from_str(content_type).map_err(Error::config)?;
    Ok(content_type)
}

pub(crate) fn file_body(
    boundary: &str,
    field_name: &str,
    payload: &[u8],
    filename: Option<&str>,
    content_type: &str,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"");
    body.extend_from_slice(field_name.as_bytes());
    body.extend_from_slice(b"\"");
    if let Some(filename) = filename {
        body.extend_from_slice(b"; filename=\"");
        body.extend_from_slice(escape_header_value(filename).as_bytes());
        body.extend_from_slice(b"\"");
    }
    body.extend_from_slice(b"\r\nContent-Type: ");
    body.extend_from_slice(content_type.as_bytes());
    body.extend_from_slice(b"\r\n\r\n");
    body.extend_from_slice(payload);
    body.extend_from_slice(b"\r\n--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");
    body
}

fn escape_header_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\r' | '\n' => escaped.push('_'),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

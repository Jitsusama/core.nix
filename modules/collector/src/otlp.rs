//! Spans as OpenTelemetry's JSON mapping of an export request gives them.
//!
//! A line is one `ExportTraceServiceRequest`: resources, each with the
//! scopes that made its spans. Ids are hex, times Unix nanoseconds written
//! as strings or numbers, and attributes typed values, which are kept as
//! plain JSON so they read back as they were meant.

use serde_json::{Map, Value};

/// One step a program took, with when it began and ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub trace: String,
    /// The span's own id, which its children name as their parent.
    pub id: String,
    pub parent: Option<String>,
    pub name: String,
    /// The resource's `service.name`, which program sent it.
    pub service: String,
    /// Unix nanoseconds.
    pub start: i64,
    pub end: i64,
    /// `ok` or `error` when the span set its status.
    pub status: Option<String>,
    pub attributes: Map<String, Value>,
}

/// Every span in one export request.
///
/// # Errors
///
/// When the line isn't JSON, or a span lacks its ids, name or times.
pub fn spans(line: &str) -> Result<Vec<Span>, String> {
    let request: Value = serde_json::from_str(line).map_err(|e| format!("not JSON: {e}"))?;
    let mut found = Vec::new();
    for resource in list(&request, "resourceSpans") {
        let described = resource.get("resource").map(attributes).unwrap_or_default();
        let service = described
            .get("service.name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        for scope in list(resource, "scopeSpans") {
            for span in list(scope, "spans") {
                found.push(one(span, service)?);
            }
        }
    }
    Ok(found)
}

fn one(span: &Value, service: &str) -> Result<Span, String> {
    let trace = id(span, "traceId", 32)?.ok_or("a span without its traceId")?;
    let own = id(span, "spanId", 16)?.ok_or("a span without its spanId")?;
    let name = span
        .get("name")
        .and_then(Value::as_str)
        .ok_or("a span without its name")?;
    let start = nanos(span, "startTimeUnixNano")?;
    let end = nanos(span, "endTimeUnixNano")?;
    if end < start {
        return Err(format!("{name} ends before it starts"));
    }
    Ok(Span {
        trace,
        parent: id(span, "parentSpanId", 16)?,
        id: own,
        name: name.to_owned(),
        service: service.to_owned(),
        start,
        end,
        status: status(span),
        attributes: attributes(span),
    })
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A hex id of the given length, lower-cased, or none when it's absent or
/// empty, as a root span's parent is.
fn id(span: &Value, key: &str, length: usize) -> Result<Option<String>, String> {
    match span.get(key).and_then(Value::as_str) {
        None | Some("") => Ok(None),
        Some(hex) if hex.len() == length && hex.bytes().all(|b| b.is_ascii_hexdigit()) => {
            Ok(Some(hex.to_ascii_lowercase()))
        }
        Some(other) => Err(format!("{key} {other:?} isn't {length} hex digits")),
    }
}

fn nanos(span: &Value, key: &str) -> Result<i64, String> {
    match span.get(key) {
        Some(Value::String(text)) => text.parse().map_err(|_| format!("{key} {text:?}")),
        Some(Value::Number(number)) => number.as_i64().ok_or_else(|| format!("{key} {number}")),
        _ => Err(format!("a span without its {key}")),
    }
}

fn status(span: &Value) -> Option<String> {
    match span.get("status")?.get("code")? {
        Value::Number(code) if code.as_u64() == Some(1) => Some("ok".to_owned()),
        Value::Number(code) if code.as_u64() == Some(2) => Some("error".to_owned()),
        Value::String(code) if code == "STATUS_CODE_OK" => Some("ok".to_owned()),
        Value::String(code) if code == "STATUS_CODE_ERROR" => Some("error".to_owned()),
        _ => None,
    }
}

/// An object's `attributes`, each typed value made plain JSON.
fn attributes(value: &Value) -> Map<String, Value> {
    list(value, "attributes")
        .iter()
        .filter_map(|pair| {
            let key = pair.get("key")?.as_str()?;
            Some((key.to_owned(), plain(pair.get("value")?)))
        })
        .collect()
}

fn plain(value: &Value) -> Value {
    let Some(typed) = value.as_object() else {
        return Value::Null;
    };
    if let Some(text) = typed.get("stringValue").or_else(|| typed.get("bytesValue")) {
        return text.clone();
    }
    if let Some(int) = typed.get("intValue") {
        // OTLP writes 64-bit integers as strings, so JSON loses none.
        return match int {
            Value::String(text) => text.parse::<i64>().map_or(Value::Null, Value::from),
            other => other.clone(),
        };
    }
    if let Some(other) = typed.get("doubleValue").or_else(|| typed.get("boolValue")) {
        return other.clone();
    }
    if let Some(array) = typed.get("arrayValue") {
        return Value::Array(list(array, "values").iter().map(plain).collect());
    }
    if let Some(kvlist) = typed.get("kvlistValue") {
        return Value::Object(
            list(kvlist, "values")
                .iter()
                .filter_map(|pair| {
                    Some((
                        pair.get("key")?.as_str()?.to_owned(),
                        plain(pair.get("value")?),
                    ))
                })
                .collect(),
        );
    }
    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUEST: &str = r#"{"resourceSpans":[{"resource":{"attributes":[
        {"key":"service.name","value":{"stringValue":"launcher"}}]},
      "scopeSpans":[{"scope":{"name":"shell"},"spans":[
        {"traceId":"4BF92F3577B34DA6A3CE929D0E0E4736","spanId":"00f067aa0ba902b7",
         "parentSpanId":"","name":"launcher.open","kind":1,
         "startTimeUnixNano":"1793451717000000000","endTimeUnixNano":1793451718300000000,
         "status":{"code":2},
         "attributes":[{"key":"rows","value":{"intValue":"12"}},
           {"key":"first_y","value":{"doubleValue":0.5}},
           {"key":"place","value":{"stringValue":"home"}},
           {"key":"settled","value":{"boolValue":true}},
           {"key":"sizes","value":{"arrayValue":{"values":[{"intValue":1},{"intValue":"2"}]}}}]},
        {"traceId":"4bf92f3577b34da6a3ce929d0e0e4736","spanId":"1111111111111111",
         "parentSpanId":"00f067aa0ba902b7","name":"launcher.layout",
         "startTimeUnixNano":"1793451717100000000","endTimeUnixNano":"1793451717200000000"}]}]}]}"#;

    #[test]
    fn a_request_gives_each_span_with_its_service_and_plain_attributes() {
        let found = spans(REQUEST).unwrap();
        assert_eq!(found.len(), 2);
        let open = &found[0];
        assert_eq!(open.trace, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(open.parent, None, "an empty parent is a root");
        assert_eq!(open.service, "launcher");
        assert_eq!(open.end - open.start, 1_300_000_000);
        assert_eq!(open.status.as_deref(), Some("error"));
        assert_eq!(open.attributes["rows"], 12);
        assert_eq!(open.attributes["first_y"], 0.5);
        assert_eq!(open.attributes["place"], "home");
        assert_eq!(open.attributes["settled"], true);
        assert_eq!(open.attributes["sizes"], serde_json::json!([1, 2]));
        assert_eq!(found[1].parent.as_deref(), Some("00f067aa0ba902b7"));
    }

    #[test]
    fn a_span_without_its_ids_or_times_is_refused_and_says_which() {
        let bad = r#"{"resourceSpans":[{"scopeSpans":[{"spans":[{"traceId":"ab",
            "spanId":"00f067aa0ba902b7","name":"x","startTimeUnixNano":"1","endTimeUnixNano":"2"}]}]}]}"#;
        assert!(spans(bad).unwrap_err().contains("traceId"));
        assert!(spans("{").unwrap_err().contains("not JSON"));
    }

    #[test]
    fn a_resource_that_names_no_service_is_unknown() {
        let anonymous = r#"{"resourceSpans":[{"scopeSpans":[{"spans":[{
            "traceId":"4bf92f3577b34da6a3ce929d0e0e4736","spanId":"00f067aa0ba902b7",
            "name":"x","startTimeUnixNano":"1","endTimeUnixNano":"2"}]}]}]}"#;
        assert_eq!(spans(anonymous).unwrap()[0].service, "unknown");
    }
}

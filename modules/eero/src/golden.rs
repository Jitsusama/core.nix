//! Comparing a record with its golden answer, for the tests that hold the
//! model and the reader to `home-network.json`.

use serde_json::Value;

/// The model's golden answer, drawn from the contract's files.
pub const MODEL: &str = include_str!("../home-network.json");

/// The paths where two values differ, so a failure names the figure rather
/// than printing the whole network. Numbers count as equal by value, since
/// `1` and `1.0` are the same figure.
pub fn differences(left: &Value, right: &Value) -> Vec<String> {
    let mut out = Vec::new();
    walk("", left, right, &mut out);
    out
}

fn walk(path: &str, left: &Value, right: &Value, out: &mut Vec<String>) {
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            for key in l.keys().chain(r.keys().filter(|k| !l.contains_key(*k))) {
                match (l.get(key), r.get(key)) {
                    (Some(a), Some(b)) => walk(&format!("{path}.{key}"), a, b, out),
                    _ => out.push(format!("{path}.{key}: only on one side")),
                }
            }
        }
        (Value::Array(l), Value::Array(r)) if l.len() == r.len() => {
            for (i, (a, b)) in l.iter().zip(r).enumerate() {
                walk(&format!("{path}[{i}]"), a, b, out);
            }
        }
        (Value::Array(l), Value::Array(r)) => {
            out.push(format!("{path}: {} items against {}", l.len(), r.len()));
        }
        (Value::Number(a), Value::Number(b)) if a.as_f64() == b.as_f64() => {}
        (a, b) if a != b => out.push(format!("{path}: {a} against {b}")),
        _ => {}
    }
}

//! How long something waited for a resource, as the kernel's pressure
//! stall information says it in `/proc/pressure/<resource>`: of the last
//! 10 s, minute and 5 minutes, the share of time at least one task waited
//! (`some`) and every task did (`full`), and the microseconds waited since
//! boot.

use serde_json::{Map, Value, json};

/// A pressure file's lines as a record's field, or null where the kernel
/// doesn't keep them.
pub fn parse(text: Option<&str>) -> Value {
    let Some(text) = text else {
        return Value::Null;
    };
    let mut lines = Map::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(which) = fields.next() else { continue };
        let mut said = Map::new();
        for field in fields {
            let Some((name, value)) = field.split_once('=') else {
                continue;
            };
            let key = match name {
                "avg10" => "avg10_pct",
                "avg60" => "avg60_pct",
                "avg300" => "avg300_pct",
                "total" => "total_us",
                _ => continue,
            };
            let value = if name == "total" {
                value.parse::<u64>().map(Value::from).ok()
            } else {
                value.parse::<f64>().ok().map(|v| super::rounded(v, 2))
            };
            if let Some(value) = value {
                said.insert(key.to_owned(), value);
            }
        }
        lines.insert(which.to_owned(), Value::Object(said));
    }
    if lines.is_empty() {
        Value::Null
    } else {
        json!(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_some_and_full_with_their_units() {
        let said = parse(Some(
            "some avg10=43.83 avg60=24.45 avg300=13.24 total=649860568\n\
             full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n",
        ));
        assert_eq!(said["some"]["avg10_pct"], json!(43.83));
        assert_eq!(said["some"]["total_us"], json!(649_860_568));
        assert_eq!(said["full"]["avg300_pct"], json!(0.0));
    }

    #[test]
    fn a_kernel_without_it_says_null() {
        assert_eq!(parse(None), Value::Null);
    }
}

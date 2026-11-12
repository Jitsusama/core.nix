//! The one `windows-open` record: how many windows niri holds, and each
//! program with how many of them, the most first and then A to Z, as the
//! shell orders amounts (Order.js's `amounts`). Lock's drawer says from it
//! whose windows stay running behind the lock, "kitty 5 · Chrome 4", so
//! the shell counts and sorts no windows of its own.

use std::collections::HashMap;

use ready_rows::order::by_name;
use serde_json::{Value, json};

/// The record, worked out from the window records, each of which names
/// its program; a window niri names no program for is counted open but
/// said of no program, as it has no name to say.
#[must_use]
pub fn record(records: &[Value]) -> Value {
    let windows = records.iter().filter(|record| {
        record["key"]
            .as_str()
            .is_some_and(|key| key.starts_with("window:"))
    });
    let mut open = 0_u64;
    let mut counted: HashMap<&str, u64> = HashMap::new();
    for window in windows {
        open += 1;
        let program = window["program"].as_str().unwrap_or("");
        if !program.is_empty() {
            *counted.entry(program).or_default() += 1;
        }
    }
    let mut programs: Vec<(&str, u64)> = counted.into_iter().collect();
    programs.sort_by(|(one, many), (other, more)| more.cmp(many).then_with(|| by_name(one, other)));
    json!({
        "key": "windows-open",
        "open": open,
        "programs": programs
            .iter()
            .map(|(program, count)| json!({"program": program, "count": count}))
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: u64, program: &str) -> Value {
        json!({"key": format!("window:{id}"), "id": id, "program": program})
    }

    #[test]
    fn programs_come_the_most_first_then_a_to_z() {
        let records = [
            json!({"key": "desktop"}),
            window(1, "kitty"),
            window(2, "Google Chrome"),
            window(3, "kitty"),
            window(4, "foot"),
            window(5, "Google Chrome"),
            window(6, "Slack"),
            json!({"key": "workspace:1", "program": "kitty"}),
        ];
        assert_eq!(
            record(&records),
            json!({
                "key": "windows-open",
                "open": 6,
                "programs": [
                    {"program": "Google Chrome", "count": 2},
                    {"program": "kitty", "count": 2},
                    {"program": "foot", "count": 1},
                    {"program": "Slack", "count": 1},
                ],
            })
        );
    }

    #[test]
    fn a_window_of_no_program_is_open_but_named_of_none() {
        let said = record(&[window(1, ""), window(2, "foot")]);
        assert_eq!(said["open"], 2);
        assert_eq!(said["programs"], json!([{"program": "foot", "count": 1}]));
    }

    #[test]
    fn none_open_says_none() {
        assert_eq!(
            record(&[json!({"key": "desktop"})]),
            json!({"key": "windows-open", "open": 0, "programs": []})
        );
    }

    #[test]
    fn names_are_read_case_alike_and_their_numbers_by_value() {
        // The same order every daemon's rows take (ready-rows' by_name).
        let said = record(&[
            window(1, "tab 10"),
            window(2, "Beta"),
            window(3, "Tab 9"),
            window(4, "alpha"),
        ]);
        let named: Vec<&str> = said["programs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|program| program["program"].as_str().unwrap())
            .collect();
        assert_eq!(named, ["alpha", "Beta", "Tab 9", "tab 10"]);
    }
}

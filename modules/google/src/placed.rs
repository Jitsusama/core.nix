//! Where each ready row stands: its `order`, spaced as the ready-rows
//! crate spaces every daemon's, each place's apart. A row keeps the order
//! it was last told where it still rises among its place's, and only one
//! that moved takes a new one, midway between its neighbours, so one
//! meeting, mail or document moving tells the launcher one row rather
//! than every row after it (Y2YBTS's review of #2963).

use std::collections::BTreeMap;

use ready_rows::order::spaced;
use serde_json::{Value, json};

/// The rows, each place's in the order shown, with each one's `order`
/// set from the record last told for its key among `told`.
pub fn placed(mut rows: Vec<Value>, told: &BTreeMap<String, Value>) -> Vec<Value> {
    let mut places: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (at, row) in rows.iter().enumerate() {
        let place = row["place"].as_str().unwrap_or_default().to_owned();
        places.entry(place).or_default().push(at);
    }
    for shown in places.values() {
        let was: Vec<Option<i64>> = shown
            .iter()
            .map(|at| {
                let key = rows[*at]["key"].as_str().unwrap_or_default();
                told.get(key)
                    .and_then(|record| record["row"]["order"].as_i64())
            })
            .collect();
        for (at, order) in shown.iter().zip(spaced(&was)) {
            rows[*at]["order"] = json!(order);
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(place: &str, id: &str) -> Value {
        json!({"key": format!("row:{place}:{id}"), "place": place})
    }

    fn told(rows: &[Value]) -> BTreeMap<String, Value> {
        rows.iter()
            .map(|row| {
                let key = row["key"].as_str().unwrap_or_default().to_owned();
                (key.clone(), json!({"key": key, "kind": "row", "row": row}))
            })
            .collect()
    }

    fn orders(rows: &[Value]) -> Vec<i64> {
        rows.iter()
            .filter_map(|row| row["order"].as_i64())
            .collect()
    }

    #[test]
    fn each_place_is_spaced_apart_from_the_others() {
        let rows = placed(
            vec![row("mail", "a"), row("mail", "b"), row("documents", "c")],
            &BTreeMap::new(),
        );
        assert_eq!(
            orders(&rows),
            [1024, 2048, 1024],
            "each place's rows spaced from the first, whatever another place holds"
        );
    }

    #[test]
    fn a_row_that_moved_alone_takes_a_new_order_and_the_rest_keep_theirs() {
        let first = placed(
            vec![row("mail", "a"), row("mail", "b"), row("mail", "c")],
            &BTreeMap::new(),
        );
        let then = placed(
            vec![row("mail", "a"), row("mail", "c"), row("mail", "b")],
            &told(&first),
        );
        let keys: Vec<&str> = then.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(keys, ["row:mail:a", "row:mail:c", "row:mail:b"]);
        assert_eq!(
            orders(&then),
            [1024, 1536, 2048],
            "c moved up midway between a and b, which kept theirs"
        );
        let again = placed(
            vec![row("mail", "new"), row("mail", "a"), row("mail", "b")],
            &told(&then),
        );
        assert_eq!(
            orders(&again),
            [0, 1024, 2048],
            "a new row first goes before the rest, c gone from between"
        );
    }
}

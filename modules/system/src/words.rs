//! How a count is written, by the parts' lines (parts.rs) and a machine's
//! card in a terminal (said.rs) alike, so the two never say one count two
//! ways. Its own module, since how a number is written is no part's
//! judgement.

/// Digits between two commas, as Canada groups a count's thousands.
const GROUP: usize = 3;

/// A count with its thousands set apart by commas, as Canada writes one.
pub fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(GROUP) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Running cells by how they stand, the busy, the idle and those out of
/// reach, which the node counts apart from both and are still cells: the
/// parked are no part of it, since running means the cells less the
/// parked. A machine's line and the mycelium's say one count this one way.
pub fn running(busy: u64, idle: u64, far: u64) -> String {
    let cells = |count: u64| if count == 1 { "cell" } else { "cells" };
    let reached = match (busy, idle) {
        (0, 0) => None,
        (busy, 0) => Some(format!("{} {} busy", grouped(busy), cells(busy))),
        (0, idle) => Some(format!("{} {} idle", grouped(idle), cells(idle))),
        (busy, idle) => Some(format!(
            "{} busy and {} idle cells",
            grouped(busy),
            grouped(idle)
        )),
    };
    match (reached, far) {
        (None, 0) => "no cell".to_owned(),
        (None, far) => format!("{} {} out of reach", grouped(far), cells(far)),
        (Some(reached), 0) => reached,
        (Some(reached), far) => format!("{reached}, and {} out of reach", grouped(far)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_cells_say_the_out_of_reach_beside_the_busy_and_idle() {
        assert_eq!(running(0, 0, 0), "no cell");
        assert_eq!(running(1, 0, 0), "1 cell busy");
        assert_eq!(running(0, 3, 0), "3 cells idle");
        assert_eq!(running(1_200, 3_400, 0), "1,200 busy and 3,400 idle cells");
        assert_eq!(running(0, 0, 12), "12 cells out of reach");
        assert_eq!(
            running(2, 0, 3),
            "2 cells busy, and 3 out of reach",
            "the out of reach were lost beside the busy"
        );
        assert_eq!(
            running(2, 1, 1),
            "2 busy and 1 idle cells, and 1 out of reach",
            "the out of reach were lost beside the busy and idle"
        );
    }

    #[test]
    fn a_count_groups_its_thousands_from_the_right() {
        let said: Vec<String> = [0, 999, 1_000, 12_345, 100_000, 1_000_000, u64::MAX]
            .into_iter()
            .map(grouped)
            .collect();
        assert_eq!(
            said,
            [
                "0",
                "999",
                "1,000",
                "12,345",
                "100,000",
                "1,000,000",
                "18,446,744,073,709,551,615"
            ],
            "each comma should fall three digits from the next, counted from the right"
        );
    }
}

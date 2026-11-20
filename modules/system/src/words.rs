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

#[cfg(test)]
mod tests {
    use super::*;

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

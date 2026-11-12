//! Someone's name as People says and sorts it (listing.rs): whether a name
//! a source gives is only an address, what to call someone no source
//! names, and the runs a name is compared by, as the shell's Order.js has.

use std::cmp::Ordering;

/// Whether a name is only an address, which a real name always wins over.
pub fn bare(name: &str) -> bool {
    name.is_empty() || name.contains('@')
}

/// What to call someone no source names: their address's part before the
/// `@`, and before any `+` tag, each run between dots, hyphens and
/// underscores a word in Title Case, so `chao.duan@` is Chao Duan. Their
/// row says the address under it.
pub fn by_address(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email);
    let local = local.split('+').next().unwrap_or(local);
    let words: Vec<String> = local
        .split(['.', '-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect();
    if words.is_empty() {
        email.to_owned()
    } else {
        words.join(" ")
    }
}

/// A name's runs of digits and of the rest, the rest in lower case, as
/// Order.js's `pieces` breaks it.
pub fn pieces(name: &str) -> Vec<String> {
    let mut runs: Vec<String> = Vec::new();
    let mut digits = false;
    for c in name.to_lowercase().chars() {
        let digit = c.is_ascii_digit();
        match runs.last_mut() {
            Some(run) if digit == digits => run.push(c),
            _ => runs.push(c.to_string()),
        }
        digits = digit;
    }
    runs
}

/// A to Z, as names are read: numbers by their value, so Tab 9 comes
/// before Tab 10, and case alike, as Order.js's `byPieces`. The rest is
/// compared by character where the shell asks the locale, which differs
/// only for letters with marks.
pub fn by_pieces(one: &[String], other: &[String]) -> Ordering {
    for (a, b) in one.iter().zip(other) {
        let numbers = a.starts_with(|c: char| c.is_ascii_digit())
            && b.starts_with(|c: char| c.is_ascii_digit());
        let said = if numbers {
            let value = |run: &str| run.parse::<f64>().unwrap_or(0.0);
            value(a).partial_cmp(&value(b)).unwrap_or(Ordering::Equal)
        } else {
            a.cmp(b)
        };
        if said != Ordering::Equal {
            return said;
        }
    }
    one.len().cmp(&other.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_no_source_names_is_called_by_their_address() {
        assert_eq!(by_address("bo@example.com"), "Bo");
        assert_eq!(
            by_address("chao.duan+cal@example.com"),
            "Chao Duan",
            "each part of it a word, its tag left out"
        );
        assert_eq!(by_address("@example.com"), "@example.com", "or all of it");
    }

    #[test]
    fn a_name_breaks_into_runs_of_digits_and_the_rest() {
        assert_eq!(
            pieces("Tab 10b"),
            ["tab ", "10", "b"],
            "digits apart, the rest lower case"
        );
        assert_eq!(
            by_pieces(&pieces("tab 9"), &pieces("Tab 10")),
            Ordering::Less,
            "9 before 10, case alike"
        );
    }
}

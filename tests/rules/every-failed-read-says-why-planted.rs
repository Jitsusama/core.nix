// The planted tree's own file, which every-failed-read-says-why.nix sees the
// rule judge before it reads the repository: each function below is one
// shape, and the check's said list names the lines that must go unsaid.
use std::fs;
use std::path::Path;

#[cfg(test)]
const PLANTED: &str = "a test item first never hides what follows it";

// Planted as src/golden.rs; the path is where the formatter finds it here.
#[cfg(test)]
#[path = "every-failed-read-says-why-planted-golden.rs"]
mod golden;

fn bare(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn said(path: &Path) -> String {
    // A file never written yet holds nothing.
    fs::read_to_string(path).unwrap_or_default()
}

fn said_on_its_line(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_default() // Nothing yet is no bytes.
}

fn after_another_statements_comment(path: &Path) -> String {
    let _ = 1; // A note for the line it ends.
    fs::read_to_string(path).unwrap_or(String::new())
}

fn after_a_string_holding_slashes(path: &Path) -> String {
    let _ = "https://example.org/{;";
    fs::read_to_string(path).unwrap_or_else(|_| String::new())
}

fn propagated(path: &Path) -> std::io::Result<String> {
    let text = fs::read_to_string(path)?;
    Ok(text.trim().to_owned())
}

fn loud(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| panic!("no {}", path.display()))
}

fn counted_as_none(path: &Path) -> usize {
    fs::read_to_string(path).map_or(0, |text| text.lines().count())
}

fn judged_as_false(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|text| !text.is_empty())
}

fn dropped(path: &Path) -> Option<usize> {
    let text = fs::read_to_string(path).ok()?;
    Some(text.len())
}

fn let_else(path: &Path) -> usize {
    let Ok(text) = fs::read_to_string(path) else {
        return 0;
    };
    text.len()
}

fn if_let_with_a_default_in_its_body(path: &Path, other: &Path) -> usize {
    if let Ok(text) = fs::read_to_string(path) {
        // Its body's comment says nothing of the read above.
        return text.len() + fs::read(other).unwrap_or_default().len();
    }
    0
}

fn matched(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => String::new(),
    }
}

fn matched_to_its_error(path: &Path, other: &Path) -> std::io::Result<String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text + &fs::read_to_string(other).unwrap_or_default()),
        Err(error) => Err(error),
    }
}

fn in_a_closure(paths: &[&Path]) -> Vec<String> {
    paths
        .iter()
        .map(|path| fs::read_to_string(path).ok())
        .map(Option::unwrap_or_default)
        .collect()
}

fn defaulted_outside_its_closure(path: Option<&Path>) -> String {
    path.map(|path| fs::read_to_string(path))
        .and_then(Result::ok)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_default()
    }
}

fn after_the_tests(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

// A field marked as test code ends at its comma, never hiding what follows.
struct Held {
    #[cfg(test)]
    seen: u8,
    kept: u8,
}

fn after_a_test_field(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn iterated(path: &Path) -> Vec<String> {
    fs::read_to_string(path).into_iter().collect()
}

fn matched_to_a_named_error(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_error) => String::new(),
    }
}

fn matched_to_any_error(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(..) => String::new(),
    }
}

fn matched_to_anything_after_a_block(path: &Path) -> usize {
    match fs::read(path) {
        Ok(bytes) => {
            let count = bytes.len();
            count
        }
        _ => 0,
    }
}

fn matched_inside_an_arm(path: &Path) -> std::io::Result<usize> {
    match fs::read_to_string(path) {
        Ok(text) => match text.trim().parse::<usize>() {
            Ok(count) => Ok(count),
            Err(_) => Ok(0),
        },
        Err(error) => Err(error),
    }
}

fn let_else_failing(path: &Path) -> std::io::Result<usize> {
    let Ok(text) = fs::read_to_string(path) else {
        return Err(std::io::Error::other("unread"));
    };
    Ok(text.len())
}

fn let_else_said_in_its_block(path: &Path) -> usize {
    let Ok(text) = fs::read_to_string(path) else {
        // A file never written yet holds nothing.
        return 0;
    };
    text.len()
}

fn exited_as_fine(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| std::process::exit(0))
}

fn exited_as_failed(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| std::process::exit(1))
}

fn defaulted_in_an_argument(path: &Path, other: &Path) -> std::io::Result<String> {
    fs::read_to_string(path).map(|text| text + &fs::read_to_string(other).unwrap_or_default())
}

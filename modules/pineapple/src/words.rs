//! A message's own words: what its writer said, with what they quoted,
//! fenced off or forwarded taken out, and those words as tokens to match.

/// What of `text` is its writer's own words: block quotes, fenced and
/// inline code, HTML comments and text in quotation marks are taken out,
/// since a phrase there is someone repeating it rather than saying it.
/// Attachments and forwards never reach here: a reader passes only the
/// message's own text.
#[must_use]
pub fn own_words(text: &str) -> String {
    let unfenced = without_between(text, "```", "```");
    let uncommented = without_between(&unfenced, "<!--", "-->");
    let unquoted = uncommented
        .lines()
        .filter(|line| !is_quote_line(line))
        .collect::<Vec<_>>()
        .join("\n");
    let uncoded = without_between(&unquoted, "`", "`");
    let unstraight = without_between(&uncoded, "\"", "\"");
    without_between(&unstraight, "\u{201c}", "\u{201d}")
}

/// Whether a line is a block quote, as markdown writes one and as Slack's
/// API sends one, with its `>` escaped.
fn is_quote_line(line: &str) -> bool {
    let start = line.trim_start();
    start.starts_with('>') || start.starts_with("&gt;")
}

/// `text` with every span from `open` to the next `close` taken out. An
/// unclosed one is left in, since a lone mark is likelier a slip than a
/// quote, and leaving it in leans toward hearing the phrase.
fn without_between(text: &str, open: &str, close: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let after = &rest[start + open.len()..];
        let Some(end) = after.find(close) else {
            break;
        };
        kept.push_str(&rest[..start]);
        kept.push(' ');
        rest = &after[end + close.len()..];
    }
    kept.push_str(rest);
    kept
}

/// The words of `text` in lower case, split at anything not a letter or a
/// digit, so "Pineapple-Override" and "pineapple  override" read alike.
#[must_use]
pub fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Where in `words` the run `run` first stands, word by word.
#[must_use]
pub fn find_run(words: &[String], run: &[&str]) -> Option<usize> {
    if run.is_empty() || words.len() < run.len() {
        return None;
    }
    words
        .windows(run.len())
        .position(|window| window.iter().zip(run).all(|(word, want)| word == want))
}

/// Whether the run `run` stands anywhere in `words`.
#[must_use]
pub fn has_run(words: &[String], run: &[&str]) -> bool {
    find_run(words, run).is_some()
}

//! The definition of good, from `good.md` beside cadence: what each
//! measure the hour card judges is held to, the line of Joel's it comes
//! from, and what being over it costs him. The page is built in, and each
//! hour counts each measure it names in no section or holds to another
//! target, and each section saying less than all four, so a figure can't
//! push or pass work with no line of his behind it.

use super::measures::{Class, Measure};

/// The page, as it was built in.
const PAGE: &str = include_str!("../../good.md");

/// The key the count of measures without a line takes.
pub const KEY: &str = "measures-undefined";

/// Who keeps the page, and writes a new measure's line.
const KEEPER: &str = "1JQPPC";

/// What a section holds its measures to.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Target {
    /// The most the hour must hold each to.
    Most(f64),
    /// Shown and never judged.
    Unjudged,
    /// Set where the measure is read, which the page doesn't repeat; only
    /// where it says so, so a number it can't read isn't taken for one.
    Elsewhere,
}

/// The fields a section's items open with, each said once.
const FIELDS: [&str; 4] = ["Measures:", "Target:", "Joel, ", "Over costs him:"];

/// One section of the page: its title, the keys it names, and what they
/// are held to and why, each none where the section doesn't say, and
/// what it holds that no field reads: a line under no item, an item
/// opening as no field, or a field said again.
#[derive(Debug)]
struct Section {
    title: String,
    keys: Vec<String>,
    target: Option<Target>,
    joel: Option<String>,
    cost: Option<String>,
    misread: Vec<String>,
}

/// The page's sections, each opening at a second-level heading, its
/// fields the list items under it, an item's lines after its first
/// indented under it with no blank line between.
fn sections(page: &str) -> Vec<Section> {
    page.split("\n## ").skip(1).map(section).collect()
}

fn section(text: &str) -> Section {
    let (title, body) = text.split_once('\n').unwrap_or((text, ""));
    let mut items: Vec<String> = Vec::new();
    let mut misread = Vec::new();
    let mut open = false;
    for line in body.lines() {
        let more = line.trim_start();
        let indented = more.len() < line.len();
        if let Some(item) = line.strip_prefix("- ") {
            items.push(item.to_owned());
            open = true;
        } else if more.is_empty() {
            open = false;
        } else if let Some(last) = items.last_mut().filter(|_| open && indented) {
            last.push(' ');
            last.push_str(more);
        } else {
            open = false;
            misread.push(format!("under no item: {}", opening(more)));
        }
    }
    for (at, item) in items.iter().enumerate() {
        match FIELDS.iter().find(|name| item.starts_with(*name)) {
            None => misread.push(format!("no such field: {}", opening(item))),
            Some(name) if items[..at].iter().any(|i| i.starts_with(name)) => {
                let name = name.trim_end_matches([':', ',', ' ']);
                misread.push(format!("{name} said twice"));
            }
            Some(_) => {}
        }
    }
    let field = |name: &str| {
        items
            .iter()
            .find_map(|i| i.strip_prefix(name))
            .map(str::trim)
    };
    Section {
        title: title.trim().to_owned(),
        keys: field("Measures:").map(spans).unwrap_or_default(),
        target: field("Target:").and_then(target),
        joel: items.iter().find(|i| i.starts_with("Joel, ")).cloned(),
        cost: field("Over costs him:").map(str::to_owned),
        misread,
    }
}

/// The first words of a line or item no field reads, enough to find it
/// by.
fn opening(line: &str) -> String {
    line.split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text inside each pair of backticks in `text`.
fn spans(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// What a target's words hold a measure to: the number they open with,
/// unjudged where they open with "none", the source's own where they
/// say it's set where it's read, and none for any other words.
fn target(words: &str) -> Option<Target> {
    if words.starts_with("none") {
        return Some(Target::Unjudged);
    }
    if words.starts_with("set where it's read") {
        return Some(Target::Elsewhere);
    }
    let number = words.split([' ', ',']).next().unwrap_or_default();
    number.parse().ok().map(Target::Most)
}

/// Whether `pattern` names `key`: the key itself, or, ending in `*`,
/// every key opening as it does.
fn names(pattern: &str, key: &str) -> bool {
    pattern
        .strip_suffix('*')
        .map_or(pattern == key, |opening| key.starts_with(opening))
}

/// Whether a line of Joel's says when, and then what he said, in quotes.
fn quoted(joel: &str) -> bool {
    let Some((when, said)) = joel.split_once(": \"") else {
        return false;
    };
    when.chars().any(|c| c.is_ascii_digit()) && said.len() > 1 && said.ends_with('"')
}

/// What each section fails to say, each thing it holds that no field
/// reads, each key holding a space, as a key broken across lines does,
/// and each key two sections name, a key
/// a family names in another section counted as named twice too.
fn unsaid(page: &[Section]) -> Vec<String> {
    let mut faults = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for s in page {
        let lacks: Vec<&str> = [
            (s.keys.is_empty(), "no measure"),
            (s.target.is_none(), "no target"),
            (
                !s.joel.as_deref().is_some_and(quoted),
                "no dated line of Joel's",
            ),
            (
                s.cost.as_deref().is_none_or(str::is_empty),
                "no cost to him",
            ),
        ]
        .into_iter()
        .filter_map(|(lacking, what)| lacking.then_some(what))
        .collect();
        if !lacks.is_empty() {
            faults.push(format!("{}: {}", s.title, lacks.join(", ")));
        }
        for what in &s.misread {
            faults.push(format!("{}: {what}", s.title));
        }
        for key in &s.keys {
            if key.contains(char::is_whitespace) {
                faults.push(format!(
                    "{key}: holds a space, as a key broken across lines does"
                ));
            }
            if seen
                .iter()
                .any(|named| names(named, key) || names(key, named))
            {
                faults.push(format!("{key}: named twice"));
            }
            seen.push(key);
        }
    }
    faults
}

/// What's wrong with how the page holds `measure`: no section naming it,
/// or a target the hour holds it to that the first section naming it
/// doesn't give. A measure left unjudged, as one unread is, can't
/// differ from its target.
fn unheld(measure: &Measure, page: &[Section]) -> Option<String> {
    let key = &measure.key;
    let Some(section) = page.iter().find(|s| s.keys.iter().any(|k| names(k, key))) else {
        return Some(format!("{key}: no section says what good is"));
    };
    let held = measure.target?;
    match section.target? {
        Target::Most(most) if (most - held).abs() > f64::EPSILON => {
            Some(format!("{key}: held to {held} here, {most} on the page"))
        }
        Target::Unjudged => Some(format!("{key}: held to {held} here, unjudged")),
        Target::Most(_) | Target::Elsewhere => None,
    }
}

/// The measures the definition of good gives no line or another target,
/// and the page's own faults, as a measure of their count.
#[must_use]
pub fn undefined(measures: &[Measure]) -> Measure {
    judged(PAGE, measures)
}

fn judged(page: &str, measures: &[Measure]) -> Measure {
    let page = sections(page);
    let mut behind = unsaid(&page);
    behind.extend(
        measures
            .iter()
            .filter(|m| m.key != KEY)
            .filter_map(|m| unheld(m, &page)),
    );
    Measure {
        key: KEY.to_owned(),
        class: Class::Cells,
        says: "measures the definition of good gives no line, or holds to another target"
            .to_owned(),
        now: f64::from(u32::try_from(behind.len()).unwrap_or(u32::MAX)),
        target: Some(0.0),
        owner: KEEPER.to_owned(),
        behind,
    }
}

#[cfg(test)]
mod tests;

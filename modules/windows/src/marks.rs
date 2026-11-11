//! Each program's own mark, the one its row among the launcher's Programs
//! wears, from the marks file a module writes from ProgramGlyphs.js's
//! table and the machine's own: a program's notice wears it (../notices),
//! and so does its row in Sound (../sound), each by the desktop entry its
//! program is ([`crate::entries`]).

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// Each program's own mark by its desktop entry id in lower case: a code
/// point in hex, or `mark:<role>` for one the theme names.
#[derive(Debug, Default)]
pub struct Marks(BTreeMap<String, String>);

const MARK: &str = "mark:";

impl Marks {
    /// The marks a marks file holds, one object of ids to marks.
    pub fn read(text: &str) -> serde_json::Result<Self> {
        serde_json::from_str(text).map(Self)
    }

    /// Marks given outright, as a test gives only those it reads.
    pub fn of(marks: &[(&str, &str)]) -> Self {
        Self(
            marks
                .iter()
                .map(|(id, mark)| ((*id).to_owned(), (*mark).to_owned()))
                .collect(),
        )
    }

    /// A program's mark as a row's glyph: its role, or its code point;
    /// a browser's window of its own wears the browser's. None where no
    /// mark is known for it.
    pub fn glyph(&self, id: &str) -> Option<Value> {
        let key = id.to_lowercase();
        let found = self.0.get(&key).or_else(|| {
            chrome_site(&key)
                .then(|| self.0.get("google-chrome"))
                .flatten()
        })?;
        Some(match found.strip_prefix(MARK) {
            Some(role) => json!(role),
            None => json!({ "code": found }),
        })
    }
}

/// Whether an id is a Chrome window of a page's own, `chrome-<site>-<profile>`.
fn chrome_site(id: &str) -> bool {
    id.strip_prefix("chrome-")
        .and_then(|rest| rest.rsplit_once('-'))
        .is_some_and(|(site, profile)| !site.is_empty() && !profile.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_browser_page_of_its_own_wears_the_browser() {
        let marks = Marks::of(&[("google-chrome", "f268")]);
        assert_eq!(
            marks.glyph("chrome-mail.google.com__-Default"),
            Some(json!({ "code": "f268" }))
        );
        assert_eq!(marks.glyph("chromeless"), None);
    }

    #[test]
    fn a_mark_the_theme_names_is_its_role() {
        let marks = Marks::of(&[("pi", "mark:agent"), ("slack", "f198")]);
        assert_eq!(marks.glyph("Pi"), Some(json!("agent")));
        assert_eq!(marks.glyph("slack"), Some(json!({ "code": "f198" })));
    }
}

//! The file kinds shared-mime-info knows, along the data folders: each
//! type's globs from `mime/globs2`, and its description from the type's
//! own `mime/<type>.xml`. A folder earlier in the list wins for a type it
//! names, as the specification says.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

/// What's been read of the kinds, read as first asked and again once
/// [`Mime::forget`] says a folder changed.
#[derive(Debug, Default)]
pub struct Mime {
    folders: Vec<PathBuf>,
    globs: Option<HashMap<String, Vec<(u32, String)>>>,
    comments: HashMap<String, Option<String>>,
}

impl Mime {
    /// The kinds under these data folders, nearest first.
    pub fn new(folders: Vec<PathBuf>) -> Self {
        Self {
            folders,
            globs: None,
            comments: HashMap::new(),
        }
    }

    /// Forgets what was read, for a folder that changed.
    pub fn forget(&mut self) {
        self.globs = None;
        self.comments.clear();
    }

    /// A type's globs, heaviest first and in the file's order between
    /// those of one weight, each once.
    pub fn globs(&mut self, kind: &str) -> Vec<String> {
        let folders = &self.folders;
        let all = self.globs.get_or_insert_with(|| read_globs(folders));
        let mut found = all.get(kind).cloned().unwrap_or_default();
        found.sort_by_key(|(weight, _)| std::cmp::Reverse(*weight));
        let mut globs: Vec<String> = Vec::new();
        for (_, glob) in found {
            if !globs.contains(&glob) {
                globs.push(glob);
            }
        }
        globs
    }

    /// What shared-mime-info calls a type, in English, or null.
    pub fn comment(&mut self, kind: &str) -> Value {
        if !self.comments.contains_key(kind) {
            // An entry's MimeType is its writer's words, so only a type's
            // own shape is read as a file's name, never `../..`.
            let said = self
                .folders
                .iter()
                .filter(|_| well_formed(kind))
                .find_map(|folder| {
                    std::fs::read_to_string(folder.join(format!("mime/{kind}.xml"))).ok()
                })
                .and_then(|xml| comment_in(&xml));
            self.comments.insert(kind.to_owned(), said);
        }
        self.comments[kind]
            .clone()
            .map_or(Value::Null, Value::String)
    }
}

/// Every glob each folder's `globs2` names, by type. A type named in a
/// nearer folder keeps only that folder's globs.
fn read_globs(folders: &[PathBuf]) -> HashMap<String, Vec<(u32, String)>> {
    let mut all: HashMap<String, Vec<(u32, String)>> = HashMap::new();
    for folder in folders {
        let Ok(text) = std::fs::read_to_string(folder.join("mime/globs2")) else {
            continue;
        };
        let mut here: HashMap<String, Vec<(u32, String)>> = HashMap::new();
        for (weight, kind, glob) in text.lines().filter_map(glob_line) {
            here.entry(kind).or_default().push((weight, glob));
        }
        for (kind, globs) in here {
            all.entry(kind).or_insert(globs);
        }
    }
    all
}

/// One line of `globs2`, `weight:type:glob[:flags]`, or none for a
/// comment or a line that isn't one.
pub fn glob_line(line: &str) -> Option<(u32, String, String)> {
    if line.starts_with('#') {
        return None;
    }
    let mut parts = line.splitn(4, ':');
    let weight = parts.next()?.parse().ok()?;
    let kind = parts.next()?.to_owned();
    let glob = parts.next()?.to_owned();
    (!kind.is_empty() && !glob.is_empty()).then_some((weight, kind, glob))
}

/// Whether a type reads as a type does, `application/pdf`: two parts
/// of letters, digits and `+-._`, neither a `.` or `..` of its own, so
/// it names a file in the mime folder and nothing outside it.
pub fn well_formed(kind: &str) -> bool {
    let part = |part: &str| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-._".contains(c))
    };
    kind.split_once('/')
        .is_some_and(|(group, name)| part(group) && part(name))
}

/// A type's `<comment>` with no language, its XML escapes read.
pub fn comment_in(xml: &str) -> Option<String> {
    let start = xml.find("<comment>")? + "<comment>".len();
    let end = start + xml[start..].find("</comment>")?;
    let said = xml[start..end]
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    (!said.is_empty()).then_some(said)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_types_own_shape_names_its_file() {
        assert!(well_formed("application/vnd.oasis.opendocument.text"));
        assert!(well_formed("image/svg+xml"));
        for bad in [
            "../../etc/passwd",
            "text/..",
            "text",
            "a/b/c",
            "/pdf",
            "text/x y",
        ] {
            assert!(!well_formed(bad), "{bad}");
        }
    }

    #[test]
    fn a_glob_line_says_its_weight_type_and_glob() {
        assert_eq!(
            glob_line("50:application/pdf:*.pdf"),
            Some((50, "application/pdf".into(), "*.pdf".into()))
        );
        assert_eq!(
            glob_line("50:text/x-makefile:Makefile:cs"),
            Some((50, "text/x-makefile".into(), "Makefile".into()))
        );
        assert_eq!(glob_line("# GLib"), None);
    }

    #[test]
    fn the_comment_without_a_language_is_the_one() {
        let xml = concat!(
            r#"<mime-type type="application/pdf">"#,
            r#"<comment xml:lang="fr">Document PDF</comment>"#,
            "<comment>PDF document</comment></mime-type>"
        );
        assert_eq!(comment_in(xml).as_deref(), Some("PDF document"));
    }
}

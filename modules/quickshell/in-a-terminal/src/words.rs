//! What `launcher` says of its own, from the launcher's words.json
//! (`LAUNCHER_WORDS`), its `terminal` section, so every word the desktop
//! says is said once. Each is a sentence with `{name}`s filled in. A word
//! the file lacks is said as its key, so a missing one shows rather than
//! being written a second time here.

use std::ffi::OsStr;
use std::fs;

use serde_json::Value;

/// The `terminal` section of words.json, and home's name, the field's.
#[derive(Debug, Clone)]
pub struct Words {
    said: Value,
    home: String,
}

impl Words {
    /// The words its install gives.
    pub fn read(path: Option<&OsStr>) -> Self {
        let all = path
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or(Value::Null);
        Self::from_value(&all)
    }

    /// The words a whole words.json holds.
    pub fn from_value(all: &Value) -> Self {
        Self {
            said: all.get("terminal").cloned().unwrap_or(Value::Null),
            home: all
                .pointer("/field/home")
                .and_then(Value::as_str)
                .unwrap_or("home")
                .to_string(),
        }
    }

    /// Home's name, as the launcher's field says it with nothing typed.
    pub fn home(&self) -> &str {
        &self.home
    }

    /// A word by its key, its `{name}`s filled from `filled`.
    fn word(&self, key: &str, filled: &[(&str, &str)]) -> String {
        let mut said = self
            .said
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or(key)
            .to_string();
        for (name, value) in filled {
            said = said.replace(&format!("{{{name}}}"), value);
        }
        said
    }

    /// How the program is used, kept in words.json as its lines.
    pub fn usage(&self) -> String {
        let mut usage = match self.said.get("usage").and_then(Value::as_array) {
            Some(lines) => lines
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n"),
            None => self.word("usage", &[]),
        };
        if !usage.ends_with('\n') {
            usage.push('\n');
        }
        usage
    }

    /// What a command does, as zsh's completion describes it.
    pub fn offered(&self, command: &str) -> String {
        self.said
            .get("offers")
            .and_then(|offers| offers.get(command))
            .and_then(Value::as_str)
            .unwrap_or(command)
            .to_string()
    }

    /// The shell not answering.
    pub fn silent(&self) -> String {
        self.word("silent", &[])
    }

    /// The shell failing at the act it was asked, as a refusal naming the
    /// act and where its log says why (VKCF3Z, 2026-11-08).
    pub fn failed(&self, name: &str) -> String {
        self.word("failed", &[("name", name)])
    }

    /// A place asked open that the launcher didn't get into, as Passwords
    /// refuses a walk from outside, said as a refusal (Y2YBTS, 2026-11-13).
    pub fn unopened(&self, name: &str) -> String {
        self.word("unopened", &[("name", name)])
    }

    /// An argument it doesn't take.
    pub fn unknown(&self, what: &str) -> String {
        self.word("unknown", &[("what", what)])
    }

    /// An act that asks first, with no terminal to answer on.
    pub fn unsure(&self, question: &str) -> String {
        self.word("unsure", &[("question", question)])
    }

    /// An act that takes words, given none, and what it asks for; one
    /// that asks nothing in words of its own is refused without it, never
    /// with an empty item between two dots (VKCF3Z, 2026-11-08).
    pub fn takes(&self, name: &str, ask: &str) -> String {
        if ask.trim().is_empty() {
            return self.word("takesWords", &[("name", name)]);
        }
        self.word("takes", &[("name", name), ("ask", ask)])
    }

    /// `do` given no steps.
    pub fn nothing_to_do(&self) -> String {
        self.word("nothingToDo", &[])
    }

    /// `do` given a blank step, which names no row.
    pub fn blank_step(&self) -> String {
        self.word("blankStep", &[])
    }

    /// A value the shell hasn't read yet, as the drawer's placeholder.
    pub fn waiting(&self) -> String {
        self.word("waiting", &[])
    }

    /// A step that named nothing, where it was looked for (the steps'
    /// names before it, none at home), and the names it might have meant.
    pub fn missing(&self, step: &str, within: &[String], near: &[String]) -> String {
        let missing = match within.last() {
            Some(place) => self.word("missingIn", &[("step", step), ("place", place)]),
            None => self.word("missing", &[("step", step)]),
        };
        if near.is_empty() {
            return missing;
        }
        let meant = self.word("meant", &[("names", &self.either(near))]);
        format!("{missing} {} {meant}", self.word("apart", &[]))
    }

    /// Names said as one of them: "Programs or People", "A, B or C".
    fn either(&self, names: &[String]) -> String {
        match names {
            [] => String::new(),
            [one] => one.clone(),
            [first @ .., last] => format!("{} {} {last}", first.join(", "), self.word("or", &[])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::from_value(
            &serde_json::from_str(
                r#"{"terminal":{"missing":"nothing called {step}",
                    "missingIn":"nothing called {step} in {place}",
                    "meant":"did you mean {names}?","or":"or","apart":"\u00b7",
                    "silent":"the shell isn't answering","usage":"launcher show",
                    "takes":"{name} takes words \u00b7 {ask} \u00b7 give them after --",
                    "takesWords":"{name} not done \u00b7 give its words after --",
                    "failed":"{name} not done \u00b7 the shell failed \u00b7 its log says why"}}"#,
            )
            .expect("the words parse"),
        )
    }

    fn names(of: &[&str]) -> Vec<String> {
        of.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn an_act_that_asks_nothing_of_its_own_is_said_without_it() {
        assert_eq!(
            words().takes("Talk", "what to say?"),
            "Talk takes words \u{b7} what to say? \u{b7} give them after --"
        );
        assert_eq!(
            words().takes("Talk", ""),
            "Talk not done \u{b7} give its words after --"
        );
    }

    #[test]
    fn a_failure_is_a_refusal_naming_the_act_and_its_log() {
        assert_eq!(
            words().failed("Reboot"),
            "Reboot not done \u{b7} the shell failed \u{b7} its log says why"
        );
    }

    #[test]
    fn a_miss_says_what_it_might_have_meant() {
        assert_eq!(
            words().missing("mahcine", &[], &names(&["Machine"])),
            "nothing called mahcine \u{b7} did you mean Machine?"
        );
    }

    #[test]
    fn a_miss_inside_a_place_says_the_place() {
        assert_eq!(
            words().missing("dsk", &names(&["Machine"]), &[]),
            "nothing called dsk in Machine"
        );
    }

    #[test]
    fn several_names_are_said_as_one_of_them() {
        assert_eq!(
            words().missing("p", &[], &names(&["Programs", "People", "Passwords"])),
            "nothing called p \u{b7} did you mean Programs, People or Passwords?"
        );
    }

    #[test]
    fn a_word_missing_from_the_file_is_said_as_its_key() {
        assert_eq!(Words::from_value(&Value::Null).silent(), "silent");
        assert_eq!(Words::from_value(&Value::Null).home(), "home");
        assert_eq!(words().usage(), "launcher show\n");
    }

    #[test]
    fn usage_kept_as_lines_is_said_as_them() {
        let lined = Words::from_value(&serde_json::json!({
            "terminal": { "usage": ["launcher show", "", "Shows a place."] }
        }));
        assert_eq!(lined.usage(), "launcher show\n\nShows a place.\n");
    }
}

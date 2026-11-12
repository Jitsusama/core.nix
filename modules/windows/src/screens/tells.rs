//! What a lit screen's settings and Turn Every Screen Off say in their
//! drawers, a little more than their rows do, since every row but a bare
//! verb inside a thing has a drawer (Joel, 2026-11-11): what each does,
//! what a setting offers, and how the : line types it, so the drawer
//! teaches the line as the row is rested on.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::settings::{Setting, option};
use super::{APART, Words};

/// The settings and the act whose drawer says what it does, each by its
/// part's id after `displays:`.
const TOLD: [&str; 6] = [
    "scale",
    "refresh",
    "resolution",
    "rotation",
    "variable",
    "sleep",
];

/// What the drawers are said in.
#[derive(Clone, Debug, PartialEq)]
pub struct Tells {
    /// What each does, a sentence, by its part's id.
    does: BTreeMap<&'static str, String>,
    /// The names of what a setting offers, of how the : line types it,
    /// and of the screens Turn Every Screen Off turns off.
    offers: String,
    typed: String,
    turns_off: String,
    /// The place's own name, over Turn Every Screen Off's drawer.
    place: String,
    /// How the : line types a command and its argument,
    /// ":{command} {argument}", and a switch's option turned off,
    /// "no{option}", as Vim's are.
    line: String,
    not: String,
    /// A setting's command and argument, as its choices' verb fills
    /// them: "setlocal", "{setting}={value}".
    set: (String, String),
    /// Turn Every Screen Off's command and argument.
    sleep: (String, String),
}

impl Tells {
    /// The drawers' words, as `word` reads a thing's from the vocabulary.
    pub fn read(word: &dyn Fn(&str, &[&str], &str) -> String) -> Tells {
        let typed = |key: &str| word("displays:typed", &["said", key], key);
        let command = |id: &str| {
            let said = |path: &str| word(id, &[path], path);
            (said("command"), said("argument"))
        };
        Tells {
            does: TOLD
                .iter()
                .map(|id| {
                    (
                        *id,
                        word(&format!("displays:{id}"), &["tells", "does"], "does"),
                    )
                })
                .collect(),
            offers: word("displays:offers", &["name"], "displays:offers"),
            typed: word("displays:typed", &["name"], "displays:typed"),
            turns_off: word("displays:turns-off", &["name"], "displays:turns-off"),
            place: word("displays", &["name"], "displays"),
            line: typed("line"),
            not: typed("not"),
            set: command("verb:display-set"),
            sleep: command("displays:sleep"),
        }
    }

    /// A command and its argument as the : line types them.
    fn typed(&self, command: &str, argument: &str) -> String {
        self.line
            .replace("{command}", command)
            .replace("{argument}", argument)
    }
}

/// A setting's drawer: the screen it's of, named as its row is, over the
/// setting's name; what it does; what it offers, where a step into it
/// offers two at least; and how the : line types it as it's set now.
pub fn setting(screen: &str, each: &Setting, words: &Words) -> Value {
    let tells = &words.tells;
    let named = words.told.part(each.id);
    let (command, argument) = &tells.set;
    let argument = argument
        .replace("{setting}", &option(&named))
        .replace("{value}", &each.typed);
    let line = tells.typed(command, &argument);
    let mut facts = Vec::new();
    if each.choices.len() > 1 {
        let offers: Vec<&str> = each
            .choices
            .iter()
            .map(|choice| choice.name.as_str())
            .collect();
        facts.push(json!({"name": tells.offers, "said": offers.join(APART)}));
    }
    facts.push(json!({"name": tells.typed, "said": line, "identifies": true}));
    drawn(screen, &named, each.id, &facts, words)
}

/// Variable Refresh's drawer: as a setting's, its : line both ways, since
/// a switch is typed on or off rather than at a value.
pub fn switch(screen: &str, words: &Words) -> Value {
    let tells = &words.tells;
    let named = words.told.part("variable");
    let on = option(&named);
    let off = tells.not.replace("{option}", &on);
    let command = &tells.set.0;
    let line = [tells.typed(command, &on), tells.typed(command, &off)].join(APART);
    let facts = [json!({"name": tells.typed, "said": line, "identifies": true})];
    drawn(screen, &named, "variable", &facts, words)
}

/// Turn Every Screen Off's drawer: the place over its name, what it does,
/// the lit screens it turns off, by `lit`'s names, and its : line.
pub fn sleep(lit: &[String], words: &Words) -> Value {
    let tells = &words.tells;
    let (command, argument) = &tells.sleep;
    let line = tells.typed(command, argument);
    json!({
        "eyebrow": tells.place,
        "title": words.sleep.name,
        "body": tells.does["sleep"],
        "facts": [
            {"name": tells.turns_off, "said": lit.join(APART)},
            {"name": tells.typed, "said": line, "identifies": true},
        ],
    })
}

/// A setting's drawer as it's laid: the screen's name, as written, since
/// it's a name from the world, over the setting's, then what it does,
/// then `facts`.
fn drawn(screen: &str, named: &str, id: &str, facts: &[Value], words: &Words) -> Value {
    json!({
        "eyebrow": screen,
        "eyebrowNames": true,
        "title": named,
        "body": words.tells.does.get(id).cloned().unwrap_or_default(),
        "facts": facts,
    })
}

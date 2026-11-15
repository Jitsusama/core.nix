//! Each ask as the launcher draws it: a ready row in Asks, its drawer
//! holding everything the line shows, whole, and its two acts carrying
//! the digest of all of it, so a yes is for exactly what was read.

use std::fmt::Write;
use std::path::Path;

use ready_rows::time::{self, Form};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::ask::{Ask, VERSION};
use crate::tree::{Checkout, Tree};
use crate::words::Words;

/// The place the rows are in.
pub const PLACE: &str = "asks";
/// The summary's key.
pub const SUMMARY: &str = "summary:asks";
/// How long a row stays once its ask is answered, in milliseconds.
pub const STAYS_MS: i64 = 300_000;

/// Where an ask stands.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Waiting,
    /// Said yes to, the value being read and the command run.
    Releasing,
    Ran {
        exit: Option<i32>,
    },
    /// Said yes to, but the value couldn't be read or the command run.
    Failed {
        why: String,
    },
    Refused,
    /// Left unanswered past its ten minutes.
    Ended,
}

impl State {
    /// Its name, as a row and an answer say it.
    pub fn name(&self) -> &'static str {
        match self {
            State::Waiting => "waiting",
            State::Releasing => "releasing",
            State::Ran { .. } => "ran",
            State::Failed { .. } => "failed",
            State::Refused => "refused",
            State::Ended => "ended",
        }
    }
}

/// An ask as it's held: what its file said, its tree as read, where it
/// stands and since when.
#[derive(Clone, Debug)]
pub struct Held {
    pub ask: Ask,
    pub tree: Tree,
    pub state: State,
    pub since_ms: i64,
}

/// The digest of everything the line shows of an ask, and the tree as it
/// was read: SHA-256 of one JSON array, its objects' keys sorted and
/// every number whole, so it reads the same however it's made.
pub fn digest(ask: &Ask, tree: &Tree) -> String {
    digest_lasting(ask, tree, None)
}

/// The digest of an ask whose line says a yes lasts, as a grant, for
/// any tree of the repository `lasts` names, so a yes is to that too.
pub fn digest_lasting(ask: &Ask, tree: &Tree, lasts: Option<&str>) -> String {
    let quest = ask
        .quest
        .as_ref()
        .map(|(id, title)| json!({"id": id, "title": title}));
    let mut said = json!([
        VERSION,
        ask.id,
        ask.asked_ms,
        ask.session,
        quest,
        ask.item,
        ask.why,
        ask.argv,
        ask.tree,
        tree.commit,
        tree.uncommitted,
        ask.given,
        ask.minutes
    ]);
    // Where a release can't commit is said in the row, so a yes is to it;
    // a tree that commits adds nothing, keeping the digests it had.
    if let (Some(kept), Value::Array(said)) = (uncommitting(&tree.checkout), &mut said) {
        said.push(json!(kept));
    }
    // So too what a yes lasts for, where the line says it lasts; a line
    // that doesn't keeps the digests it had.
    if let (Some(repository), Value::Array(said)) = (lasts, &mut said) {
        said.push(json!({"lasts": repository}));
    }
    // And whether a standing yes would cover it but for its code being
    // off main, where that was asked.
    if let (Some(landed), Value::Array(said)) = (tree.landed, &mut said) {
        said.push(json!({"landed": landed}));
    }
    let hash = Sha256::digest(said.to_string().as_bytes());
    hash.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// The word saying a release in a tree kept as `checkout` can't commit,
/// or none where it can or there's no history to write.
fn uncommitting(checkout: &Checkout) -> Option<&'static str> {
    match checkout {
        // Its packed refs and the top beside its settings stay read-only
        // (ISXEG8, 14 November), so it tags but can't commit.
        Checkout::Own { .. } => Some("tagsOnly"),
        Checkout::Unvouched { .. } => Some("noHistory"),
        Checkout::None | Checkout::Worktree { .. } => None,
    }
}

/// A word of the command, quoted as a POSIX shell would need it, so a
/// space inside one argument reads as one.
fn quoted(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_./=:,+@%".contains(&byte));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// A command as the line says it, each word quoted as a shell needs it.
pub fn command(argv: &[String]) -> String {
    argv.iter()
        .map(|word| quoted(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The tree's path as Joel reads it, his home folder as `~`.
fn homed(path: &str, home: &str) -> String {
    match path.strip_prefix(home) {
        Some(rest) if !home.is_empty() && (rest.is_empty() || rest.starts_with('/')) => {
            format!("~{rest}")
        }
        _ => path.to_owned(),
    }
}

/// The tree's name as Joel calls it, its folder's last part, `core.nix`
/// for `~/src/core.nix`.
fn tree_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// A row's side: when it was asked, after what came of it once answered,
/// since a row away from the cursor says its side alone (VKCF3Z, 15
/// November).
fn side(held: &Held, words: &Words) -> Value {
    let came = time::at(held.ask.asked_ms, Form::Came);
    let stands = match held.state {
        State::Waiting => return json!([came]),
        State::Releasing => "side:releasing",
        State::Ran { .. } => "side:released",
        State::Failed { .. } => "side:failed",
        State::Refused => "side:refused",
        State::Ended => "side:ended",
    };
    json!([{"said": words.fill(stands, &[])}, came])
}

/// The fact saying what came of an answered ask, or none while it waits.
fn outcome(held: &Held, words: &Words) -> Option<Value> {
    let name = words.fill("fact:outcome", &[]);
    let said = match &held.state {
        State::Waiting => return None,
        State::Releasing => words.fill("releasing", &[]),
        State::Ran { exit: Some(code) } => words.fill("ran", &[("exit", &code.to_string())]),
        State::Ran { exit: None } => words.fill("stopped", &[]),
        State::Failed { why } => words.fill("failed", &[("why", why)]),
        State::Refused => words.fill("refused", &[]),
        State::Ended => words.fill("ended", &[]),
    };
    let warns = matches!(held.state, State::Failed { .. } | State::Ran { exit: None })
        || matches!(held.state, State::Ran { exit: Some(code) } if code != 0);
    let mut fact = json!({"name": name, "said": said});
    if warns {
        fact["hue"] = json!("warning");
    }
    Some(fact)
}

/// The row for an ask, without its order, which the kept rows give it.
/// `says_commits` has it say where a release can't commit, for a releaser
/// whose command writes a worktree's history alone.
pub fn row(held: &Held, home: &str, words: &Words, says_commits: bool) -> Value {
    row_lasting(held, home, words, says_commits, None)
}

/// The row for an ask whose yes lasts, as a grant, for any tree of the
/// repository `lasts` names, the line saying so; as [`row`] where none.
pub fn row_lasting(
    held: &Held,
    home: &str,
    words: &Words,
    says_commits: bool,
    lasts: Option<&str>,
) -> Value {
    let ask = &held.ask;
    let digest = digest_lasting(ask, &held.tree, lasts);
    let key = format!("row:{PLACE}:{}", ask.id);
    // An act names the ask alone, by its thing, never by its row's key
    // too, so a forged act has one form to match, not two.
    let thing = format!("ask:{}", ask.id);
    let eyebrow = match &ask.quest {
        Some((_, title)) => words.fill("from", &[("session", &ask.session), ("quest", title)]),
        None => ask.session.clone(),
    };
    let path = homed(&ask.tree, home);
    let short = held
        .tree
        .commit
        .as_deref()
        .map(|commit| &commit[..commit.len().min(12)]);
    let place = match short {
        Some(commit) => words.fill("at", &[("path", &path), ("commit", commit)]),
        None => words.fill("notGit", &[("path", &path)]),
    };
    let mut facts = vec![
        // Each identifies what runs, so the drawer lays it whole, never cut.
        json!({
            "name": words.fill("fact:runs", &[]),
            "said": command(&ask.argv),
            "identifies": true,
        }),
        json!({"name": words.fill("fact:in", &[]), "said": place, "identifies": true}),
    ];
    // What a yes lasts for decides the press, so it comes before the rest,
    // above the fold at 1440x900 without going in (VKCF3Z, 15 November).
    if let Some(repository) = lasts {
        facts.push(json!({
            "name": words.fill("fact:lasts", &[]),
            "said": words.fill("lasts", &[("repository", repository)]),
        }));
    }
    // A standing yes waits on code off main, and he's told why it asks.
    if held.tree.landed == Some(false) {
        facts.push(json!({
            "name": words.fill("fact:standing", &[]),
            "said": words.fill("unlanded", &[]),
        }));
    }
    // A yes is for the command in its tree, so the tree's own code
    // runs with the secret too (ISXEG8, 14 November), and he's told.
    facts.push(json!({
        "name": words.fill("fact:trusts", &[]),
        "said": words.fill("trusts", &[("tree", &tree_name(&ask.tree))]),
    }));
    if let Some(kept) = uncommitting(&held.tree.checkout).filter(|_| says_commits) {
        facts.push(json!({"name": words.fill("fact:commits", &[]), "said": words.fill(kept, &[])}));
    }
    let changes = match held.tree.uncommitted {
        Some(0) => None,
        Some(count) => Some(words.fill("uncommitted", &[("count", &count.to_string())])),
        None => Some(words.fill("unknown", &[])),
    };
    if let Some(said) = changes.filter(|_| held.tree.commit.is_some()) {
        facts
            .push(json!({"name": words.fill("fact:changes", &[]), "said": said, "hue": "warning"}));
    }
    let minutes = ask.minutes.to_string();
    facts.push(json!({
        "name": words.fill("fact:given", &[]),
        "said": words.fill("given", &[("name", &ask.given), ("minutes", &minutes)]),
    }));
    // How long the ask waits, named apart from a grant's Ends, so it
    // never reads as when a yes ends (VKCF3Z, 15 November).
    if held.state == State::Waiting {
        facts.push(json!({
            "name": words.fill("fact:waits", &[]),
            "at": ask.ends_ms(), "as": Form::Moment.name(),
        }));
    }
    facts.extend(outcome(held, words));
    let acts = if held.state == State::Waiting {
        let act = |verb: &str| {
            json!({"key": format!("verb:{verb}"),
                   "ask": {"ask": verb, "key": thing, "digest": digest}})
        };
        json!([act("release"), act("refuse")])
    } else {
        json!([])
    };
    json!({
        "key": key,
        "thing": thing,
        "place": PLACE,
        "kind": "ask",
        // Only a waiting ask says its state, which the cursor's row says
        // beside its side; an answered one says it in its side.
        "state": (held.state == State::Waiting).then(|| held.state.name()),
        "name": ask.item,
        "glyph": "passwords",
        "hue": if held.state == State::Waiting { "action" } else { "machine" },
        "note": eyebrow,
        "side": side(held, words),
        "attention": (held.state == State::Waiting).then_some(ask.asked_ms),
        "digest": digest,
        "acts": acts,
        "drawer": {"eyebrow": eyebrow, "title": ask.item, "body": ask.why, "facts": facts},
    })
}

/// The summary beside the rows: how many wait, or that none does.
pub fn summary(held: &[&Held], words: &Words) -> Value {
    let waiting = held
        .iter()
        .filter(|one| one.state == State::Waiting)
        .count();
    let said = match waiting {
        0 => words.fill("none", &[]),
        1 => words.fill("one", &[]),
        count => words.fill("waiting", &[("count", &count.to_string())]),
    };
    json!({"key": SUMMARY, "kind": "summary", "summary": {"place": PLACE, "said": said}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn held(state: State, uncommitted: Option<u64>) -> Held {
        Held {
            ask: Ask {
                id: "3f0c9e2a7b5d41e8a6c2f9d01b7e4a35".to_owned(),
                asked_ms: 1_794_458_400_000,
                session: "S1".to_owned(),
                quest: Some(("Q-1".to_owned(), "Try the Line".to_owned())),
                item: "op://Private/Release Test/password".to_owned(),
                why: "To try the line.".to_owned(),
                argv: vec!["secret-asks".to_owned(), "received".to_owned()],
                tree: "/home/joel/src/tree".to_owned(),
                given: "RELEASED".to_owned(),
                minutes: 1,
            },
            tree: Tree {
                commit: Some("4f2a91c03b1d7e6a5b4c3d2e1f0a9b8c7d6e5f4a".to_owned()),
                uncommitted,
                checkout: Checkout::None,
                landed: None,
            },
            state,
            since_ms: 1_794_458_400_000,
        }
    }

    fn words() -> Words {
        Words::of(&[
            ("from", "{session} \u{b7} {quest}"),
            ("fact:runs", "Runs"),
            ("fact:in", "In"),
            ("at", "{path} at {commit}"),
            ("fact:changes", "Changes"),
            ("uncommitted", "{count} not committed"),
            ("fact:given", "Given As"),
            ("given", "{name}, for {minutes} min"),
            ("fact:outcome", "Outcome"),
            ("ran", "ran, exit {exit}"),
            ("fact:trusts", "Also Runs"),
            (
                "trusts",
                "{tree}'s own code, so its hooks and scripts get the secret too",
            ),
            ("fact:commits", "Commits"),
            ("tagsOnly", "can't from here, only tags"),
            ("noHistory", "can't from here, nor tags"),
            ("fact:lasts", "Yes Lasts"),
            ("lasts", "a week, from any tree of {repository} on main"),
            ("fact:waits", "Waits Until"),
            ("fact:ends", "Ends"),
            ("side:released", "released"),
            ("side:refused", "refused"),
        ])
    }

    /// The names of a row's drawer's facts, in order.
    fn named(row: &Value) -> Vec<String> {
        let facts = row["drawer"]["facts"].as_array().expect("facts");
        facts
            .iter()
            .map(|fact| fact["name"].as_str().unwrap_or("").to_owned())
            .collect()
    }

    #[test]
    fn a_waiting_ask_says_what_a_yes_lasts_before_what_else_runs() {
        let held = held(State::Waiting, Some(0));
        let row = row_lasting(&held, "", &words(), true, Some("core.nix"));
        // Yes Lasts decides the press, so it shows above the fold at
        // 1440x900 without going in (VKCF3Z, 15 November).
        assert_eq!(
            named(&row),
            [
                "Runs",
                "In",
                "Yes Lasts",
                "Also Runs",
                "Given As",
                "Waits Until"
            ],
            "the facts' order"
        );
    }

    #[test]
    fn an_asks_own_end_is_said_as_how_long_it_waits() {
        let row = row(&held(State::Waiting, Some(0)), "", &words(), true);
        // Ends is the grant's alone, so an ask's minute never reads as
        // when a yes ends (VKCF3Z, 15 November).
        assert!(
            !named(&row).contains(&"Ends".to_owned()),
            "the ask's end is named as a grant's: {:?}",
            named(&row)
        );
        assert_eq!(named(&row).last().map(String::as_str), Some("Waits Until"));
    }

    #[test]
    fn an_answered_asks_row_says_how_it_stands_beside_when() {
        let said = |state: State| {
            let row = row(&held(state, Some(0)), "", &words(), true);
            let side = row["side"].as_array().expect("a side").clone();
            (
                side.first().map(|one| one["said"].clone()),
                side.len(),
                row["state"].clone(),
            )
        };
        // A row away from the cursor says only its side, so an answered
        // ask's says what came of it there (VKCF3Z, 15 November), and
        // serves no state the cursor's row would say beside it again.
        let (word, items, state) = said(State::Ran { exit: Some(0) });
        assert_eq!(
            (word, items),
            (Some(json!("released")), 2),
            "a ran ask's side"
        );
        assert_eq!(state, Value::Null, "a ran ask says its state twice");
        let (word, items, _) = said(State::Refused);
        assert_eq!(
            (word, items),
            (Some(json!("refused")), 2),
            "a refused ask's side"
        );
        let (_, items, state) = said(State::Waiting);
        assert_eq!(
            (items, state),
            (1, json!("waiting")),
            "a waiting ask's side"
        );
    }

    /// An ask whose tree's history is kept as `checkout` says.
    fn kept_in(checkout: Checkout) -> Held {
        let mut held = held(State::Waiting, Some(0));
        held.tree.checkout = checkout;
        held
    }

    /// What the row's Commits fact says, if it has one, and where.
    fn commits(row: &Value) -> Option<(usize, Value)> {
        let facts = row["drawer"]["facts"].as_array().expect("facts");
        let at = facts.iter().position(|fact| fact["name"] == "Commits")?;
        Some((at, facts[at]["said"].clone()))
    }

    #[test]
    fn an_ask_from_a_repositorys_own_checkout_says_it_can_tag_but_not_commit() {
        let dot_git = PathBuf::from("/home/joel/src/tree/.git");
        let own = kept_in(Checkout::Own { dot_git });
        let said = row(&own, "", &words(), true);
        let (at, commits) = commits(&said).expect("no fact says it can't commit");
        assert_eq!(commits, "can't from here, only tags", "the fact's words");
        let facts = said["drawer"]["facts"].as_array().expect("facts");
        assert_eq!(
            facts[at - 1]["name"],
            "Also Runs",
            "it doesn't follow the fact saying what else runs"
        );
        let elsewhere = row(&held(State::Waiting, Some(0)), "", &words(), true);
        assert_eq!(
            commits_of(&elsewhere),
            None,
            "a tree that commits said it can't"
        );
        assert_ne!(
            said["digest"], elsewhere["digest"],
            "the digest missed where its history is kept"
        );
        let unsaid = row(&own, "", &words(), false);
        assert_eq!(
            commits_of(&unsaid),
            None,
            "a releaser that writes no history said where it can't commit"
        );
    }

    /// The Commits fact's words alone.
    fn commits_of(row: &Value) -> Option<Value> {
        commits(row).map(|(_, said)| said)
    }

    #[test]
    fn an_ask_whose_git_file_no_worktree_vouches_for_says_it_writes_no_history() {
        let dot_git = PathBuf::from("/home/joel/src/tree/.git");
        let unvouched = row(
            &kept_in(Checkout::Unvouched { dot_git }),
            "",
            &words(),
            true,
        );
        assert_eq!(
            commits_of(&unvouched),
            Some(json!("can't from here, nor tags")),
            "the fact's words"
        );
        let worktree = kept_in(Checkout::Worktree {
            dot_git: PathBuf::from("/home/joel/src/tree/.git"),
            common: PathBuf::from("/home/joel/src/repo/.git"),
            own: PathBuf::from("/home/joel/src/repo/.git/worktrees/tree"),
        });
        assert_eq!(
            commits_of(&row(&worktree, "", &words(), true)),
            None,
            "a worktree git vouches for said it can't commit"
        );
    }

    #[test]
    fn an_ask_says_its_trees_own_code_gets_the_secret_too() {
        let row = row(&held(State::Waiting, Some(0)), "/home/joel", &words(), true);
        let facts = row["drawer"]["facts"].as_array().expect("facts");
        let trusts = facts
            .iter()
            .position(|fact| fact["name"] == "Also Runs")
            .expect("no fact says the tree's code runs too");
        assert_eq!(
            facts[trusts]["said"], "tree's own code, so its hooks and scripts get the secret too",
            "the fact didn't name the tree"
        );
        assert_eq!(
            facts[trusts - 1]["name"],
            "In",
            "it doesn't follow the tree's fact"
        );
    }

    #[test]
    fn a_waiting_ask_offers_both_acts_with_its_digest() {
        let row = row(&held(State::Waiting, Some(0)), "/home/joel", &words(), true);
        let digest = row["digest"].as_str().expect("a digest");
        assert_eq!(digest.len(), 64, "the digest isn't SHA-256 in hex");
        assert_eq!(
            row["acts"][0]["ask"]["ask"], "release",
            "release isn't first"
        );
        assert_eq!(
            row["acts"][1]["ask"]["digest"], digest,
            "refuse lacks the digest"
        );
        assert_eq!(
            row["drawer"]["facts"][1]["said"],
            "~/src/tree at 4f2a91c03b1d"
        );
    }

    #[test]
    fn uncommitted_changes_are_warned_of_and_change_the_digest() {
        let clean = row(&held(State::Waiting, Some(0)), "", &words(), true);
        let dirty = row(&held(State::Waiting, Some(2)), "", &words(), true);
        assert_ne!(
            clean["digest"], dirty["digest"],
            "the digest missed the changes"
        );
        let facts = dirty["drawer"]["facts"].as_array().expect("facts");
        let warned = facts
            .iter()
            .find(|fact| fact["name"] == "Changes")
            .expect("a fact");
        assert_eq!(warned["said"], "2 not committed", "the count wasn't said");
        assert_eq!(warned["hue"], "warning", "the changes weren't warned of");
        let calm = clean["drawer"]["facts"].as_array().expect("facts");
        assert!(
            calm.iter().all(|fact| fact["name"] != "Changes"),
            "a clean tree warned"
        );
    }

    #[test]
    fn an_answered_ask_offers_nothing_and_says_what_came_of_it() {
        let row = row(
            &held(State::Ran { exit: Some(0) }, Some(0)),
            "",
            &words(),
            true,
        );
        assert_eq!(row["acts"], json!([]), "an answered ask still offered acts");
        let facts = row["drawer"]["facts"].as_array().expect("facts");
        assert_eq!(facts.last().expect("a fact")["said"], "ran, exit 0");
    }

    #[test]
    fn a_word_with_a_space_is_quoted_as_one() {
        assert_eq!(
            quoted("plain-word"),
            "plain-word",
            "a plain word was quoted"
        );
        assert_eq!(
            quoted("two words"),
            "'two words'",
            "a spaced word wasn't quoted"
        );
        assert_eq!(
            quoted("it's"),
            r"'it'\''s'",
            "a quote inside wasn't escaped"
        );
    }
}

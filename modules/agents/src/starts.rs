//! The starts Joel gave Start New Work, each kept on disk from the ask
//! until the shell says it told him how it ended, so a start outlives
//! the shell that asked and the daemon that waited on the node: a shell
//! started again is told it on its watch, and one a restart cut off is
//! ended as one that may yet begin, since the node may have started it
//! and asking again blind could start it twice (Joel, 18 November:
//! "things I send in must be seen through").
//!
//! Only the shell's token for a start is kept, its moment and its end,
//! never Joel's words, which nothing told needs.

use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::served;

/// The file beside the daemon's others the starts are kept in.
pub const FILE: &str = "starts.json";

/// The record kind each ended start is served as.
pub const KIND: &str = "start";

/// The shell's ask saying which starts it told Joel the end of.
pub const TOLD: &str = "sprout-told";

/// What a start a restart cut off is ended as, coded as a silent node's
/// is (sprout.rs), since either may still begin.
pub const CUT_OFF: &str = "a restart cut it off before the node answered; it may still start \
     it, so look in Agents before asking again";

/// How many starts are kept at most, the oldest let go first: a shell
/// that never says what it told would otherwise grow the file for good.
const KEPT_AT_MOST: usize = 20;

/// How long an untold start is kept, a day, after which telling it would
/// say nothing Agents hasn't.
const KEPT_FOR_MS: u128 = 24 * 60 * 60 * 1000;

/// The longest token the shell names a start by.
const TOKEN_AT_MOST: usize = 64;

/// One start: the shell's token for it, when it was asked, and how it
/// ended, none while the node is still asked.
#[derive(Clone, Debug, PartialEq)]
pub struct Start {
    pub token: String,
    pub at: u128,
    pub ended: Option<Value>,
}

/// Every start kept, oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Starts(Vec<Start>);

/// The token the shell named its start by in `said`, none where it named
/// none, or one that isn't letters, digits and dashes, which a record's key
/// couldn't hold whole.
pub fn token_of(said: &Value) -> Option<String> {
    said["start"]
        .as_str()
        .filter(|token| {
            !token.is_empty()
                && token.len() <= TOKEN_AT_MOST
                && token
                    .chars()
                    .all(|one| one.is_ascii_alphanumeric() || one == '-')
        })
        .map(str::to_owned)
}

/// The tokens the shell's `sprout-told` names.
pub fn told_of(said: &Value) -> Vec<String> {
    said["starts"]
        .as_array()
        .map(|tokens| {
            tokens
                .iter()
                .filter_map(|token| token_of(&json!({"start": token})))
                .collect()
        })
        .unwrap_or_default()
}

impl Starts {
    /// What `path` keeps at `now_ms`, as a daemon starting finds it: a
    /// start with no end was cut off by whatever stopped the daemon that
    /// asked, so it ends so, and one older than a day goes. None are kept
    /// when the file isn't there or can't be read whole, the latter said,
    /// as each start it held then goes untold.
    pub fn read(path: &Path, now_ms: u128) -> Self {
        let said = match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|why| why.to_string()),
            Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(Value::Null),
            Err(why) => Err(why.to_string()),
        }
        .unwrap_or_else(|why| {
            eprintln!(
                "agents: can't read {}, so no start kept there is told: {why}",
                path.display()
            );
            Value::Null
        });
        if !said.is_null() && !said["starts"].is_array() {
            eprintln!(
                "agents: {} holds no list of starts, so none kept there is told",
                path.display()
            );
        }
        let kept = said["starts"].as_array().map(|starts| {
            starts
                .iter()
                .filter_map(|one| {
                    let token = token_of(one)?;
                    let at = u128::from(one["at"].as_u64()?);
                    // Only what the live path keeps, so a file written by hand
                    // or by an older daemon serves no more than one asked now.
                    let ended = match &one["ended"] {
                        Value::Null => cut_off(),
                        ended => told_of_an_end(ended),
                    };
                    Some(Start {
                        token,
                        at,
                        ended: Some(ended),
                    })
                })
                .filter(|one| one.at + KEPT_FOR_MS > now_ms)
                .collect()
        });
        Self(kept.unwrap_or_default())
    }

    /// Writes every start to `path` whole, renamed into place, so a
    /// daemon stopped mid-write finds the last whole one.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let writing = path.with_extension("json.new");
        let starts: Vec<Value> = self
            .0
            .iter()
            .map(|one| {
                json!({"start": one.token, "at": u64::try_from(one.at).unwrap_or(0),
                    "ended": one.ended})
            })
            .collect();
        fs::write(&writing, json!({"starts": starts}).to_string())?;
        fs::rename(&writing, path)
    }

    /// The start `token` asked at `at_ms`, kept as the node is asked. One
    /// older than a day goes then too, so a daemon running for days
    /// doesn't serve it for good.
    pub fn begun(&mut self, token: &str, at_ms: u128) {
        self.0
            .retain(|one| one.token != token && one.at + KEPT_FOR_MS > at_ms);
        self.0.push(Start {
            token: token.to_owned(),
            at: at_ms,
            ended: None,
        });
        let over = self.0.len().saturating_sub(KEPT_AT_MOST);
        self.0.drain(..over);
    }

    /// The start `token` ended with `answer`, the daemon's answer to the
    /// shell, of which only what the shell tells is kept (Sprouting.js's
    /// `ended`): the node's whole answer holds the folder and the words
    /// that attach the session, which a record any watch hears needn't.
    /// One not kept, let go since, is kept no more.
    pub fn ended(&mut self, token: &str, answer: &Value) {
        if let Some(one) = self.0.iter_mut().find(|one| one.token == token) {
            one.ended = Some(told_of_an_end(answer));
        }
    }

    /// The shell told Joel how each of `tokens` ended, so they go.
    pub fn told(&mut self, tokens: &[String]) {
        self.0.retain(|one| !tokens.contains(&one.token));
    }

    /// Each start that ended and isn't told, a whole record the shell
    /// hears on its watch, by the token it named the start by.
    pub fn records(&self) -> Vec<Value> {
        self.0
            .iter()
            .filter_map(|one| {
                let ended = one.ended.as_ref()?;
                // In Agents' place, which a watch reads from a key's second part.
                let key = format!("{KIND}:{}:{}", served::PLACE, one.token);
                // When it was asked, so a shell that never asked it waits out
                // the patience of the one that did (Sprouting.untold).
                let at = u64::try_from(one.at).unwrap_or(0);
                Some(json!({"key": key, "kind": KIND,
                    "start": {"token": one.token, "at": at, "ended": ended}}))
            })
            .collect()
    }
}

/// Of `answer`, the daemon's to a sprout, only what the shell tells: the
/// machine and holder's id of one started, or the code and words of one
/// that wasn't.
fn told_of_an_end(answer: &Value) -> Value {
    let mut kept = json!({});
    if let Some(begun) = answer.get("sprouted") {
        kept["sprouted"] =
            json!({"machine": words(&begun["machine"]), "held": words(&begun["held"])});
    }
    if let Some(error) = answer.get("error") {
        kept["error"] = json!({"code": words(&error["code"]), "message": words(&error["message"])});
    }
    kept
}

/// The most of any one string of an end kept, in characters: a node's
/// words for a refusal run to a line or two.
const WORDS_AT_MOST: usize = 500;

/// `said` as a string of at most WORDS_AT_MOST characters, or none where
/// it isn't one, so no end grows the file or a record past a few lines.
fn words(said: &Value) -> Value {
    said.as_str().map_or(Value::Null, |text| {
        Value::String(text.chars().take(WORDS_AT_MOST).collect())
    })
}

/// A cut-off start's end, in the shape of the daemon's answer to a sprout.
fn cut_off() -> Value {
    json!({"error": {"code": "unanswered", "message": CUT_OFF}})
}

#[cfg(test)]
mod tests {
    use daemon_serve::asks::place_of;

    use super::*;

    const NOW: u128 = 1_800_000_000_000;

    /// A file of the test's own, `case` keeping each test's apart, since
    /// they run at once in one process.
    fn kept_at(case: &str) -> std::path::PathBuf {
        let folder =
            std::env::temp_dir().join(format!("agents-starts-{}-{case}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder.join(FILE)
    }

    #[test]
    fn a_start_kept_ends_as_cut_off_for_the_daemon_that_finds_it() {
        let path = kept_at("cut-off");
        let mut starts = Starts::default();
        starts.begun("a-1", NOW);
        starts.write(&path).unwrap();
        let found = Starts::read(&path, NOW + 1000);
        let records = found.records();
        assert_eq!(records.len(), 1, "{records:?}");
        // A watch narrowed to Agents' place reads a key's second part.
        let key = records[0]["key"].as_str().unwrap_or_default();
        assert_eq!(
            place_of(key),
            Some(served::PLACE),
            "{key} is served in no place Agents watches"
        );
        assert_eq!(records[0]["start"]["ended"]["error"]["code"], "unanswered");
        assert_eq!(records[0]["start"]["ended"]["error"]["message"], CUT_OFF);
        assert_eq!(
            records[0]["start"]["at"],
            json!(u64::try_from(NOW).unwrap()),
            "a kept start doesn't say when it was asked"
        );
    }

    #[test]
    fn a_file_serves_no_more_of_an_end_than_one_asked_now() {
        let path = kept_at("by-hand");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let long = "a".repeat(WORDS_AT_MOST + 1);
        let at = u64::try_from(NOW).unwrap();
        let file = json!({"starts": [{"start": "a-1", "at": at, "ended": {
            "sprouted": {"machine": long, "held": 7, "folder": "/home/j/src/r"},
            "extra": "x"}}]});
        fs::write(&path, file.to_string()).unwrap();
        let records = Starts::read(&path, NOW).records();
        let ended = &records[0]["start"]["ended"];
        assert_eq!(
            ended,
            &json!({"sprouted": {"machine": "a".repeat(WORDS_AT_MOST), "held": null}}),
            "a file served more than the live path keeps"
        );
    }

    #[test]
    fn a_start_that_ended_is_served_as_it_ended_until_it_is_told() {
        let path = kept_at("ended");
        let mut starts = Starts::default();
        starts.begun("a-1", NOW);
        assert!(
            starts.records().is_empty(),
            "a start still asked was served"
        );
        let answer = json!({"sprouted": {"machine": "heron", "held": "h1"}});
        // The folder and attach words aren't the shell's to tell, so go.
        let whole = json!({"sprouted": {"machine": "heron", "held": "h1",
            "folder": "/home/j/src/r", "attach": ["mycelium", "join", "h1"]}});
        starts.ended("a-1", &whole);
        starts.write(&path).unwrap();
        let mut found = Starts::read(&path, NOW);
        assert_eq!(found.records()[0]["start"]["ended"], answer);
        found.told(&["a-1".to_owned()]);
        found.write(&path).unwrap();
        assert_eq!(Starts::read(&path, NOW).records(), Vec::<Value>::new());
    }

    #[test]
    fn a_start_older_than_a_day_or_past_the_most_kept_goes() {
        let mut starts = Starts::default();
        for one in 0..=KEPT_AT_MOST {
            starts.begun(&format!("a-{one}"), NOW);
        }
        assert_eq!(starts.0.len(), KEPT_AT_MOST);
        assert_eq!(starts.0[0].token, "a-1", "the oldest wasn't let go first");
        let path = kept_at("old");
        starts.write(&path).unwrap();
        let day_less_a_moment = Starts::read(&path, NOW + KEPT_FOR_MS - 1).records();
        assert_eq!(day_less_a_moment.len(), KEPT_AT_MOST, "let go before a day");
        assert_eq!(
            Starts::read(&path, NOW + KEPT_FOR_MS).records(),
            Vec::<Value>::new()
        );
        // A daemon running all day lets go of them as the next is asked.
        starts.begun("b-1", NOW + KEPT_FOR_MS);
        assert_eq!(starts.0.len(), 1, "a running daemon kept day-old starts");
    }

    #[test]
    fn a_refusal_keeps_its_code_and_words() {
        let mut starts = Starts::default();
        starts.begun("a-1", NOW);
        let refused = json!({"error": {"code": "refused", "message": "no folder there"}});
        starts.ended("a-1", &refused);
        assert_eq!(starts.records()[0]["start"]["ended"], refused);
    }

    #[test]
    fn a_token_is_letters_digits_and_dashes_and_short() {
        assert_eq!(
            token_of(&json!({"start": "1763-ab"})),
            Some("1763-ab".to_owned())
        );
        assert_eq!(token_of(&json!({"start": "a:b"})), None);
        assert_eq!(token_of(&json!({"start": ""})), None);
        assert_eq!(token_of(&json!({"start": "a".repeat(65)})), None);
        assert_eq!(token_of(&json!({})), None);
        assert_eq!(
            told_of(&json!({"starts": ["a-1", "x y", 3]})),
            vec!["a-1".to_owned()]
        );
    }

    #[test]
    fn a_file_that_cant_be_read_keeps_none() {
        let path = kept_at("unread");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{\"starts\": [").unwrap();
        assert_eq!(Starts::read(&path, NOW), Starts::default());
    }
}

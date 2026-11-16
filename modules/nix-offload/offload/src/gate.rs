//! A gate asked from a machine that can't reach the builders, run by one
//! that can: `nix-offload ask` on a Mac, `nix-offload answer` where
//! mycelium starts it.
//!
//! A session on a Mac has no way to the lent builders, whose key only
//! the machine that lends them holds, and no build keeper, so it could
//! neither run `there --changed` nor keep the pass a merge reads. The
//! machines of the mycelium are one cluster, so the Mac asks through it:
//! `ask` leaves its tree's commits since origin's main as a bundle, and
//! what it asks, in its quest's scratch `handover/gates/<commit>/`, which
//! crosses to every machine sharing the quest, and has mycelium start
//! `answer` on the machine that gates. `answer` takes that commit only
//! from a clone of its own, checked against the forge's main
//! (`answer.rs`), runs `there --changed` for it as the agent fence's
//! exemption runs an agent's own (`outside.rs`), so the pass is kept
//! where a merge reads it, and leaves what it found as `result.json`
//! beside the bundle for `ask` to read back. That file is a convenience
//! and never proof: whoever writes the quest's scratch can write it, and a
//! merge reads the kept pass alone.
//!
//! What the request may say is a fixed grammar ([`Request`]): the repo,
//! the commit, the quest, the asking session, and nothing that reaches a
//! command line but the commit.

pub mod answer;
pub mod ask;
pub mod fence;

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// Where in a quest's scratch the gates are: under its handover, which
/// every machine sharing the quest gets, named or not.
pub const GATES: &str = "handover/gates";
/// The request, in a gate's folder.
pub const REQUEST: &str = "request.json";
/// The commits since origin's main, in a gate's folder.
pub const BUNDLE: &str = "gate.bundle";
/// What `answer` found, in a gate's folder.
pub const RESULT: &str = "result.json";
/// What the gate's run said, in a gate's folder.
pub const LOG: &str = "log";

/// The largest bundle a gate takes: the largest file a shared scratch
/// folder carries.
pub const LARGEST_BUNDLE: u64 = 64 << 20;
/// The largest request a gate reads.
const LARGEST_REQUEST: u64 = 16 << 10;

/// What a gate exits with when what it was asked is wrong: sysexits.h's
/// data error.
pub const REFUSED: u8 = 65;
/// What it exits with while it can't take the run now, or the answer
/// didn't come in time: sysexits.h's temporary failure.
pub const NOT_NOW: u8 = 75;

/// A repository on the forge, as `github:OWNER/NAME`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// Who holds it.
    pub owner: String,
    /// Its name.
    pub name: String,
}

impl Repo {
    /// The repo `github:OWNER/NAME` names, or nothing for anything else.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        let (owner, name) = key.strip_prefix("github:")?.split_once('/')?;
        let cloned = Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("git"));
        (plain_name(owner) && plain_name(name) && !cloned).then(|| Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    /// The repo a remote's URL names on GitHub, by https or ssh.
    #[must_use]
    pub fn of_remote(url: &str) -> Option<Self> {
        let path = url
            .strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("git@github.com:"))
            .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
        let path = path.strip_suffix('/').unwrap_or(path);
        let path = path.strip_suffix(".git").unwrap_or(path);
        Self::parse(&format!("github:{path}"))
    }

    /// Its key, `github:OWNER/NAME`.
    #[must_use]
    pub fn key(&self) -> String {
        format!("github:{}/{}", self.owner, self.name)
    }

    /// Where it's fetched from on `forge`, with no credential.
    #[must_use]
    pub fn url(&self, forge: &str) -> String {
        format!("{forge}/{}/{}.git", self.owner, self.name)
    }
}

/// Whether a name is one GitHub could give an owner or a repo: letters,
/// digits, `.`, `_` and `-`, never only dots.
fn plain_name(name: &str) -> bool {
    (1..=100).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        && !name.bytes().all(|byte| byte == b'.')
}

/// Whether `commit` is a full commit hash, as git prints one.
#[must_use]
pub fn plain_commit(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether `quest` is a quest's id, `QEST-YYYYMMDD-XXXXXX`.
#[must_use]
pub fn plain_quest(quest: &str) -> bool {
    let parts: Vec<&str> = quest.split('-').collect();
    matches!(
        parts.as_slice(),
        ["QEST", date, id]
            if date.len() == 8
                && date.bytes().all(|byte| byte.is_ascii_digit())
                && id.len() == 6
                && id.bytes().all(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase())
    )
}

/// Whether `session` is a mesh address, six capitals and digits.
#[must_use]
pub fn plain_session(session: &str) -> bool {
    session.len() == 6
        && session
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase())
}

/// What a Mac asks a gate to run: `there --changed` for a commit of a
/// repo, for a quest and the session in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The repo the commit is of.
    pub repo: Repo,
    /// The commit, in full.
    pub commit: String,
    /// The quest whose scratch carries it.
    pub quest: String,
    /// The session that asked, when it has an address.
    pub session: Option<String>,
}

/// The only run a gate takes today.
const CHANGED: &str = "changed";

impl Request {
    /// The request as `ask` writes it.
    #[must_use]
    pub fn json(&self) -> String {
        let mut fields = Map::new();
        fields.insert("repo".to_owned(), Value::from(self.repo.key()));
        fields.insert("commit".to_owned(), Value::from(self.commit.clone()));
        fields.insert("checks".to_owned(), Value::from(CHANGED));
        fields.insert("quest".to_owned(), Value::from(self.quest.clone()));
        if let Some(session) = &self.session {
            fields.insert("session".to_owned(), Value::from(session.clone()));
        }
        Value::Object(fields).to_string()
    }

    /// The request `text` holds, or why it isn't one: every field of the
    /// grammar, plain, and no other.
    ///
    /// # Errors
    ///
    /// When it isn't a JSON object of exactly these fields, or a field
    /// isn't plain.
    pub fn parse(text: &str) -> Result<Self, String> {
        let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(text) else {
            return Err("the request isn't a JSON object".to_owned());
        };
        let known = ["repo", "commit", "checks", "quest", "session"];
        if let Some(other) = fields.keys().find(|key| !known.contains(&key.as_str())) {
            return Err(format!(
                "the request says {other}, which a gate doesn't take"
            ));
        }
        let text = |key: &str| fields.get(key).and_then(Value::as_str);
        let repo = text("repo")
            .and_then(Repo::parse)
            .ok_or("the request's repo isn't github:OWNER/NAME")?;
        let commit = text("commit")
            .filter(|commit| plain_commit(commit))
            .ok_or("the request's commit isn't a full commit hash")?;
        if text("checks") != Some(CHANGED) {
            return Err(
                "the request's checks aren't \"changed\", the only run a gate takes".to_owned(),
            );
        }
        let quest = text("quest")
            .filter(|quest| plain_quest(quest))
            .ok_or("the request's quest isn't a quest's id")?;
        let session = match fields.get("session") {
            None => None,
            Some(Value::String(session)) if plain_session(session) => Some(session.clone()),
            Some(_) => return Err("the request's session isn't a mesh address".to_owned()),
        };
        Ok(Self {
            repo,
            commit: commit.to_owned(),
            quest: quest.to_owned(),
            session,
        })
    }
}

/// What a bundle's header says it holds: the commits it needs, and its
/// refs, each by name, with what each names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The commits the bundle's own depend on, which it doesn't hold.
    pub prerequisites: Vec<String>,
    /// Its refs, as (commit, name).
    pub refs: Vec<(String, String)>,
}

/// The largest header a gate reads before its pack.
const LARGEST_HEADER: usize = 64 << 10;

/// What a v2 or v3 bundle's header says, read from its bytes alone, so
/// it's judged before git is given it.
///
/// # Errors
///
/// When it isn't a bundle git writes, or names an object format but
/// SHA-1 or any other capability, such as a filter.
pub fn header(bytes: &[u8]) -> Result<Header, String> {
    let head = &bytes[..bytes.len().min(LARGEST_HEADER)];
    let end = head
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .ok_or("the bundle has no header git writes")?;
    let text = std::str::from_utf8(&head[..end])
        .map_err(|_| "the bundle's header isn't text".to_owned())?;
    let mut lines = text.lines();
    let version = lines.next().unwrap_or_default();
    if version != "# v2 git bundle" && version != "# v3 git bundle" {
        return Err("it isn't a v2 or v3 git bundle".to_owned());
    }
    let (mut prerequisites, mut refs) = (Vec::new(), Vec::new());
    for line in lines {
        if let Some(capability) = line.strip_prefix('@') {
            if capability != "object-format=sha1" {
                return Err(format!(
                    "the bundle asks for {capability}, which a gate doesn't take"
                ));
            }
        } else if let Some(needed) = line.strip_prefix('-') {
            let commit = needed.split(' ').next().unwrap_or_default();
            if !plain_commit(commit) {
                return Err("a prerequisite of the bundle isn't a full commit hash".to_owned());
            }
            prerequisites.push(commit.to_owned());
        } else {
            let (commit, name) = line
                .split_once(' ')
                .ok_or("a ref of the bundle has no name")?;
            if !plain_commit(commit) {
                return Err("a ref of the bundle isn't a full commit hash".to_owned());
            }
            refs.push((commit.to_owned(), name.to_owned()));
        }
    }
    Ok(Header {
        prerequisites,
        refs,
    })
}

/// Why a bundle can't be the commit asked for, or nothing when it can:
/// one ref, `HEAD`, at the commit, needing commits it doesn't hold, as
/// `ask` writes it from origin's main.
#[must_use]
pub fn unlike_asked(header: &Header, commit: &str) -> Option<String> {
    match header.refs.as_slice() {
        [(head, name)] if name == "HEAD" && head == commit => {
            header.prerequisites.is_empty().then(|| {
                "the bundle holds the whole history, not the commits since origin's main".to_owned()
            })
        }
        [(head, name)] if name == "HEAD" => Some(format!(
            "the bundle's HEAD is {head}, not the commit asked for"
        )),
        _ => Some("the bundle holds other refs than HEAD".to_owned()),
    }
}

/// Where repos are fetched from: GitHub, or the folder
/// `NIX_OFFLOAD_GATE_FORGE` names, as a test asks.
#[must_use]
pub fn forge() -> String {
    env::var("NIX_OFFLOAD_GATE_FORGE").unwrap_or_else(|_| "https://github.com".to_owned())
}

/// The folder each quest's scratch is in on this machine: the one
/// `NIX_OFFLOAD_QUEST_SCRATCH` names, as a test asks, or the harness's,
/// under XDG's cache folder.
#[must_use]
pub fn scratches() -> PathBuf {
    if let Some(folder) = env::var_os("NIX_OFFLOAD_QUEST_SCRATCH") {
        return PathBuf::from(folder);
    }
    let cache = env::var_os("XDG_CACHE_HOME").map_or_else(
        || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".cache"),
        PathBuf::from,
    );
    cache.join("pi/agentic-harness.pi/quest-workspace")
}

/// A gate's folder in a quest's scratch.
#[must_use]
pub fn folder(quest: &str, commit: &str) -> PathBuf {
    scratches().join(quest).join(GATES).join(commit)
}

/// Reads at most `largest` bytes of `path`, once, refusing a larger file
/// rather than reading part of it.
///
/// # Errors
///
/// When it can't be read, or is larger.
pub fn read_once(path: &Path, largest: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(largest + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > largest {
        return Err(format!("{} is larger than {largest} bytes", path.display()));
    }
    Ok(bytes)
}

/// Reads a request, once.
///
/// # Errors
///
/// When it can't be read, or isn't a request.
pub fn read_request(path: &Path) -> Result<Request, String> {
    let bytes = read_once(path, LARGEST_REQUEST)?;
    Request::parse(&String::from_utf8_lossy(&bytes))
}

/// Writes `bytes` as `name` in `folder` whole: hidden while it's written,
/// since mycelium carries no hidden file, then named.
///
/// # Errors
///
/// When it can't be written or named.
pub fn write_whole(folder: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let writing = folder.join(format!(".{name}.writing"));
    let said = |error: std::io::Error| format!("{}: {error}", folder.join(name).display());
    let mut file = fs::File::create(&writing).map_err(said)?;
    file.write_all(bytes).map_err(said)?;
    file.sync_all().map_err(said)?;
    fs::rename(&writing, folder.join(name)).map_err(said)
}

/// What a gate found, as `result.json` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The commit it was for.
    pub commit: String,
    /// What the run exited with, or what the gate did when it ran none.
    pub exit: u8,
    /// What it found, in a sentence.
    pub said: String,
}

impl Found {
    /// As `result.json` holds it.
    #[must_use]
    pub fn json(&self) -> String {
        serde_json::json!({ "commit": self.commit, "exit": self.exit, "said": self.said })
            .to_string()
    }

    /// What `text` says was found, or nothing when it isn't a result.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(text).ok()?;
        Some(Self {
            commit: value.get("commit")?.as_str()?.to_owned(),
            exit: u8::try_from(value.get("exit")?.as_u64()?).ok()?,
            said: value.get("said")?.as_str()?.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "0123456789abcdef0123456789abcdef01234567";
    const B: &str = "89abcdef0123456789abcdef0123456789abcdef";

    fn asked() -> Request {
        Request {
            repo: Repo::parse("github:kestrel/osprey").expect("a plain repo"),
            commit: A.to_owned(),
            quest: "QEST-20261115-ABC123".to_owned(),
            session: Some("SK1FF0".to_owned()),
        }
    }

    #[test]
    fn a_request_reads_back_as_it_was_written() {
        let request = asked();
        assert_eq!(
            Request::parse(&request.json()),
            Ok(request),
            "a request should read back whole"
        );
    }

    #[test]
    fn a_request_saying_more_than_the_grammar_is_refused() {
        let refused = [
            r#"{"repo":"github:kestrel/osprey","commit":"main","checks":"changed","quest":"QEST-20261115-ABC123"}"#.to_owned(),
            format!(r#"{{"repo":"github:kestrel/osprey","commit":"{A}","checks":"changed","quest":"QEST-20261115-ABC123","env":"X=1"}}"#),
            format!(r#"{{"repo":"github:kestrel/osprey","commit":"{A}","checks":"vm-everything","quest":"QEST-20261115-ABC123"}}"#),
            format!(r#"{{"repo":"github:kestrel/../osprey","commit":"{A}","checks":"changed","quest":"QEST-20261115-ABC123"}}"#),
            format!(r#"{{"repo":"github:kestrel/osprey","commit":"{A}","checks":"changed","quest":"../../etc"}}"#),
            format!(r#"{{"repo":"github:kestrel/osprey","commit":"{A}","checks":"changed","quest":"QEST-20261115-ABC123","session":"$(x)"}}"#),
            format!(r#"["github:kestrel/osprey","{A}"]"#),
        ];
        for text in refused {
            assert!(Request::parse(&text).is_err(), "{text} should be refused");
        }
    }

    #[test]
    fn only_a_plain_github_repo_is_named() {
        assert_eq!(
            Repo::of_remote("git@github.com:kestrel/osprey.git").map(|repo| repo.key()),
            Some("github:kestrel/osprey".to_owned()),
            "an ssh remote should name its repo"
        );
        assert_eq!(
            Repo::of_remote("https://github.com/kestrel/osprey").map(|repo| repo.key()),
            Some("github:kestrel/osprey".to_owned()),
            "an https remote should name its repo"
        );
        for remote in [
            "https://example.com/kestrel/osprey",
            "git@github.com:kestrel/a/b",
            "https://github.com/../x",
        ] {
            assert_eq!(
                Repo::of_remote(remote),
                None,
                "{remote} should name no repo"
            );
        }
    }

    #[test]
    fn a_bundle_header_says_what_it_needs_and_holds() {
        let bytes = format!("# v2 git bundle\n-{B} base\n{A} HEAD\n\nPACK...").into_bytes();
        let read = header(&bytes).expect("a v2 header should be read");
        assert_eq!(
            read,
            Header {
                prerequisites: vec![B.to_owned()],
                refs: vec![(A.to_owned(), "HEAD".to_owned())],
            },
            "the header should name the base it needs and its HEAD"
        );
        assert_eq!(
            unlike_asked(&read, A),
            None,
            "the commit asked for should be taken"
        );
        assert!(
            unlike_asked(&read, B).is_some(),
            "another commit should be refused"
        );
    }

    #[test]
    fn a_bundle_unlike_one_ask_writes_is_refused() {
        let filtered = format!("# v3 git bundle\n@filter=blob:none\n{A} HEAD\n\n").into_bytes();
        assert!(
            header(&filtered).is_err(),
            "a filtered bundle should be refused"
        );
        let whole = header(format!("# v2 git bundle\n{A} HEAD\n\n").as_bytes())
            .expect("a whole bundle reads");
        assert!(
            unlike_asked(&whole, A).is_some(),
            "a bundle of the whole history should be refused"
        );
        let more = header(
            format!("# v2 git bundle\n-{B} x\n{A} HEAD\n{A} refs/heads/main\n\n").as_bytes(),
        )
        .expect("two refs read");
        assert!(
            unlike_asked(&more, A).is_some(),
            "a bundle with more refs should be refused"
        );
        assert!(
            header(b"PACK").is_err(),
            "no header at all should be refused"
        );
    }

    #[test]
    fn a_result_reads_back_as_it_was_written() {
        let found = Found {
            commit: A.to_owned(),
            exit: 0,
            said: "passed".to_owned(),
        };
        assert_eq!(
            Found::parse(&found.json()),
            Some(found),
            "a result should read back whole"
        );
    }
}

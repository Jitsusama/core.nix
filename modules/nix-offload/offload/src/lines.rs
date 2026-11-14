//! `nix-offload lines --config PATH`: copies the lender's builder lines
//! into the files root's Nix reads, passing only lines naming a builder
//! root may trust.
//!
//! A machines line names a host, the key root reaches it with and the host
//! key root believes, so whoever writes the file root reads chooses whose
//! outputs root's build hook takes back, unchecked. The lender runs in a
//! person's account, where every program they run could write it, so this
//! runs as an account of its own and is the only writer of what root
//! reads. The lender goes on choosing which builders to list, its proofs,
//! stand-ins and floors included; this only refuses a line whose URI, key
//! or host key it can't vouch for.
//!
//! A line passes when its URI, compared whole since an `ssh-ng` query
//! parameter overrides the line's own fields, its key and its host key
//! match a triple trusted here: a standing builder's, from the machine's
//! own settings, or a lent builder's, whose host key this reads from the
//! cloud itself. The lender names the instance behind each lent builder in
//! a file of its own; this checks the name and zone against its settings,
//! asks the cloud whether the instance is still the one its maker made
//! (`src/vet.rs`), every [`ASK_AGAIN`], and asks the instance's guest
//! attributes for its host key, which only the instance can write. Its
//! other fields pass as written, since none of them changes who root
//! trusts, but only in plain characters: Nix reads `;` as the start of
//! another machine and `#` as the end of the line.
//!
//! It also writes [`LENT`], every lent builder root vouches for now, each
//! lender's whether or not root's own files name it, which the fence's
//! ssh agent reads to sign a login to that builder alone.
//!
//! Long because its tests, nearly half of it, stay beside the private
//! pieces they prove: the clock each instance is asked again by, and what
//! is kept of what the cloud last said.

use crate::vet::{self, Pinned};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How often the lender's files are read again. A builder named a few
/// seconds late only builds a few seconds less; this is the same pace the
/// keeper writes at.
const PACE: Duration = Duration::from_secs(2);

/// How often the cloud is asked again whether an instance is still the
/// one root trusted, and for a host key it didn't give or gave one a line
/// doesn't carry: a builder's guest agent publishes its key a few seconds
/// after it boots, and a lender names it only once that's done. So a
/// builder reset onto another startup script may be sent builds for up
/// to this long before its line goes; asking more often costs a gcloud
/// process per lent builder each time.
const ASK_AGAIN: Duration = Duration::from_secs(30);

/// How long what the cloud last vouched for stands while it can't be
/// heard, so a few calls lost to the network don't drop every lent
/// builder. A cloud refusing to answer, or a token that isn't one, stands
/// for nothing: the lender's account writes the token, so either could
/// otherwise buy this long for a change made unseen.
const STANDS: Duration = Duration::from_secs(120);

/// How often the cloud is asked again of a builder only [`LENT`] names,
/// while it vouches for it: one asked every [`ASK_AGAIN`] costs a gcloud
/// or two each time, and on 12 November 8 such builders took half a core.
/// What it last vouched for stands this much longer, and a builder it
/// didn't answer for is asked again at [`ASK_AGAIN`], so a call lost
/// drops nothing; a lender naming a builder anew is asked of at once, and
/// its helper gives back one changed once made within a minute.
const ASK_LENT: Duration = Duration::from_secs(300);

/// The file of every lent builder root vouches for now, one a line as
/// `<lender> <instance> ssh-ed25519 <base64-key>`, the lender named as its
/// ssh host is; empty while none is.
pub const LENT: &str = "lent";

/// How long after first trust the audit log is read once more, for what
/// it hadn't shown yet: a change logged late, or an insert made with a
/// script of its own and set back, whose entries don't say what metadata
/// it was made with. Its entries took seconds to show on 10 November.
const REREAD: Duration = Duration::from_secs(600);

/// How long gcloud has to answer, about thrice its slowest seen, so one
/// hung call never holds every line.
const PATIENCE: Duration = Duration::from_secs(60);

/// A builder root may trust, by the three fields of its line that say
/// who root reaches and believes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trusted {
    pub uri: String,
    pub key: String,
    pub host_key: String,
}

/// A lent builder: its URI, and the files its lender names its instance
/// and its cloud token in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lent {
    pub uri: String,
    pub instance: PathBuf,
    pub token: PathBuf,
}

/// What a machine says this may pass, from its settings file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Where the lender writes its lines.
    pub from: PathBuf,
    /// Where root's Nix reads them.
    pub into: PathBuf,
    /// The files, by name, in both.
    pub files: Vec<String>,
    /// The key root reaches every lent builder with.
    pub key: String,
    pub standing: Vec<Trusted>,
    pub lent: Vec<Lent>,
    /// The cloud project lent builders are made in.
    pub project: String,
    /// What a lent builder's name starts with.
    pub prefix: String,
    /// The zones a lent builder may be in.
    pub zones: Vec<String>,
    pub gcloud: String,
    /// The startup script every lent builder runs, whole; with none, a
    /// lent builder is trusted on its host key alone.
    pub startup_script: Option<PathBuf>,
    /// Root's public key, the one a lent builder's hook takes.
    pub hook_key: PathBuf,
    /// What a lent builder's boot image may start with.
    pub images: Vec<String>,
}

/// What `settings` hold a lent builder to, read from the files they name,
/// when they pin a startup script.
fn pinned(settings: &Settings) -> Result<Option<Pinned>, String> {
    let read = |path: &Path| {
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
    };
    let Some(script) = &settings.startup_script else {
        return Ok(None);
    };
    Ok(Some(Pinned {
        script: read(script)?,
        hook_key: read(&settings.hook_key)?,
        images: settings.images.clone(),
    }))
}

/// The settings in `text`, a JSON object, or what's wrong with them.
pub fn settings(text: &str) -> Result<Settings, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let text = |name: &str| {
        value[name]
            .as_str()
            .map(str::to_owned)
            .ok_or(format!("{name} is missing"))
    };
    let texts = |name: &str| -> Result<Vec<String>, String> {
        value[name]
            .as_array()
            .ok_or(format!("{name} is missing"))?
            .iter()
            .map(|item| item.as_str().map(str::to_owned))
            .collect::<Option<_>>()
            .ok_or(format!("{name} holds more than text"))
    };
    let field = |item: &Value, name: &str| {
        item[name]
            .as_str()
            .map(str::to_owned)
            .ok_or(format!("an entry lacks {name}"))
    };
    let standing = value["standing"]
        .as_array()
        .ok_or("standing is missing")?
        .iter()
        .map(|item| {
            Ok(Trusted {
                uri: field(item, "uri")?,
                key: field(item, "key")?,
                host_key: field(item, "hostKey")?,
            })
        })
        .collect::<Result<_, String>>()?;
    let lent = value["lent"]
        .as_array()
        .ok_or("lent is missing")?
        .iter()
        .map(|item| {
            Ok(Lent {
                uri: field(item, "uri")?,
                instance: PathBuf::from(field(item, "instance")?),
                token: PathBuf::from(field(item, "token")?),
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Settings {
        from: PathBuf::from(text("from")?),
        into: PathBuf::from(text("into")?),
        files: texts("files")?,
        key: text("key")?,
        standing,
        lent,
        project: text("project")?,
        prefix: text("prefix")?,
        zones: texts("zones")?,
        gcloud: text("gcloud")?,
        startup_script: value["startupScript"].as_str().map(PathBuf::from),
        hook_key: PathBuf::from(text("hookKey")?),
        images: texts("images")?,
    })
}

/// Whether a field passed as written holds only what Nix reads as one
/// field: letters, digits and `-_.,=:/+`, so no `;`, `#` or `@` can start
/// another machine, end the line or name a file of them.
fn plain(field: &str) -> bool {
    !field.is_empty()
        && field
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.,=:/+".contains(c))
}

/// Whether `line` names a builder `trusted` holds, in Nix's eight fields,
/// each of the rest plain.
pub fn passes(line: &str, trusted: &[Trusted]) -> bool {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let [
        uri,
        systems,
        key,
        jobs,
        speed,
        features,
        mandatory,
        host_key,
    ] = fields[..]
    else {
        return false;
    };
    [systems, jobs, speed, features, mandatory]
        .iter()
        .all(|field| plain(field))
        && trusted
            .iter()
            .any(|one| one.uri == uri && one.key == key && one.host_key == host_key)
}

/// The lines of `text` that pass, one a line, each ending in a newline.
pub fn passing(text: &str, trusted: &[Trusted]) -> String {
    text.lines()
        .filter(|line| passes(line, trusted))
        .fold(String::new(), |mut kept, line| {
            kept.push_str(line);
            kept.push('\n');
            kept
        })
}

/// A host key as a guest agent publishes it, with its type in front or
/// without, as a machines line carries it: base64 of `ssh-ed25519 KEY`.
pub fn carried(published: &str) -> Option<String> {
    let key = published.trim();
    let key = key.strip_prefix("ssh-ed25519 ").unwrap_or(key).trim();
    let plain = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c));
    plain.then(|| base64(format!("ssh-ed25519 {key}").as_bytes()))
}

/// `bytes` in standard base64, padded, as `base64 -w0` writes it.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let joined = chunk.iter().enumerate().fold(0u32, |all, (at, byte)| {
            all | u32::from(*byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            if at <= chunk.len() {
                let index = (joined >> (18 - 6 * at)) & 0x3f;
                out.push(char::from(ALPHABET[index as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The instance a lender names in `text`, `<name> <zone>`, when its name
/// starts with `prefix` and its zone is one of `zones`; why not otherwise.
pub fn instance(text: &str, prefix: &str, zones: &[String]) -> Result<(String, String), String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let [name, zone] = words[..] else {
        return Err("names no `<name> <zone>`".to_owned());
    };
    let named = name.len() > prefix.len()
        && name.starts_with(prefix)
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !named {
        return Err(format!("names {name}, which isn't one of {prefix}'s"));
    }
    if !zones.iter().any(|one| one == zone) {
        return Err(format!("names the zone {zone}, where no builder is made"));
    }
    Ok((name.to_owned(), zone.to_owned()))
}

/// What the cloud last said of an instance, and when it was asked.
#[derive(Default)]
struct Asked {
    /// Its host key, while root trusts it.
    host_key: Option<String>,
    at: Option<Instant>,
    /// When it had last started when root first trusted it, kept once
    /// it's refused too, so a reset is never trusted.
    started: Option<String>,
    /// Its fingerprints when root first trusted it, which any change to it
    /// moves, even one undone before the next look.
    marks: Option<String>,
    /// The lender's instance file and the host keys its lines carried when
    /// this was asked, so an answer for others is asked again at once.
    named: String,
    wanted: Vec<String>,
    /// When the cloud last vouched for it.
    vouched: Option<Instant>,
    /// When root first trusted it, and whether its audit log has been read
    /// again since, [`REREAD`] on.
    trusted_at: Option<Instant>,
    reread: bool,
    /// Whether it was refused once root had trusted it, which is for good:
    /// what it ran while changed may still be there once it's set back.
    spoiled: bool,
    /// Whether only [`LENT`] names it, no file root's Nix reads, so it's
    /// asked of every [`ASK_LENT`].
    lent_only: bool,
}

impl Asked {
    /// How long since this was asked before it's asked again, `lent_only`
    /// saying whether only [`LENT`] names it now: such a builder, last
    /// vouched for when last asked, waits [`ASK_LENT`]; any other
    /// [`ASK_AGAIN`].
    fn every(&self, lent_only: bool) -> Duration {
        if lent_only && self.vouched.is_some() && self.vouched == self.at {
            ASK_LENT
        } else {
            ASK_AGAIN
        }
    }
}

/// Why the cloud didn't vouch for an instance.
enum Unvouched {
    /// It wasn't heard in time, or the network to it failed: what it said
    /// last stands a while, since nothing running as Joel can bring that
    /// about.
    Silent,
    /// It wasn't asked, its token being no token, or it refused to answer:
    /// nothing stands, though the instance isn't known to be changed, and
    /// its pinned fingerprints show anything done to it meanwhile once the
    /// cloud answers again.
    Unanswered(String),
    /// It answered, and the instance isn't as made.
    Refused(String),
}

impl From<String> for Unvouched {
    fn from(why: String) -> Self {
        Self::Refused(why)
    }
}

/// The most a cloud token may hold.
const TOKEN_BYTES: u64 = 4096;

/// What gcloud says when the network to the cloud failed, as its HTTP
/// library words it. Each holds a space or a capital, which no instance's
/// name, the one thing of Joel's a refusal can quote, may hold.
const UNREACHED: [&str; 10] = [
    "(ConnectionError)",
    "(SSLError)",
    "(Timeout)",
    "Max retries exceeded",
    "Connection reset by peer",
    "Connection refused",
    "Read timed out",
    "Temporary failure in name resolution",
    "Network is unreachable",
    "Failed to establish a new connection",
];

/// What a failed gcloud's `complaint` on standard error makes of it: a
/// failed network is silence, and anything else, a refused token, a
/// missing permission or instance, is the cloud not answering.
fn failed(complaint: &str) -> Unvouched {
    if UNREACHED.iter().any(|said| complaint.contains(said)) {
        return Unvouched::Silent;
    }
    let said = complaint
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("nothing");
    Unvouched::Unanswered(format!("gcloud failed, saying {said}"))
}

/// The host keys asked of the cloud, by instance, and what was last said
/// of each thing that can be refused.
pub struct HostKeys {
    asked: HashMap<(String, String), Asked>,
    config: PathBuf,
    said: HashMap<String, String>,
    /// Whether the project's metadata holds, and when that was asked.
    project: Option<(Instant, Result<(), String>)>,
}

impl HostKeys {
    /// None asked yet, with gcloud keeping its state in `config`.
    pub fn new(config: PathBuf) -> Self {
        Self {
            asked: HashMap::new(),
            config,
            said: HashMap::new(),
            project: None,
        }
    }

    /// Says `what` of `about` when it isn't what was last said of it, so a
    /// refusal read every two seconds is journalled once; none says nothing
    /// and clears it.
    fn say(&mut self, about: &str, what: Option<String>) {
        match what {
            Some(what) if self.said.get(about) != Some(&what) => {
                eprintln!("nix-offload: {what}");
                self.said.insert(about.to_owned(), what);
            }
            Some(_) => {}
            None => {
                self.said.remove(about);
            }
        }
    }

    /// The host key of `lent`'s instance as its line carries it, while the
    /// cloud vouches for the instance, asking again every [`ASK_AGAIN`], or
    /// at once when the lender names it or its host key anew: after each
    /// switch on 10 November, an answer kept for the key before dropped
    /// the line for up to 30 s.
    fn of(
        &mut self,
        lent: &Lent,
        settings: &Settings,
        wanted: &[String],
        lent_only: bool,
    ) -> Option<(String, String)> {
        let about = lent.instance.display().to_string();
        // None named is a lender with nothing lent, which says nothing.
        let named = fs::read_to_string(&lent.instance).unwrap_or_default();
        if named.trim().is_empty() {
            self.say(&about, None);
            return None;
        }
        let (name, zone) = match instance(&named, &settings.prefix, &settings.zones) {
            Ok(instance) => instance,
            Err(why) => {
                self.say(&about, Some(format!("{about} {why}")));
                return None;
            }
        };
        self.say(&about, None);
        let key = (name.clone(), zone.clone());
        let due = self.asked.get(&key).is_none_or(|asked| {
            let anew = asked.named != named || asked.wanted != wanted;
            let every = asked.every(lent_only);
            asked.at.is_none_or(|at| anew || at.elapsed() >= every)
        });
        if due {
            let mut before = self.asked.remove(&key).unwrap_or_default();
            before.lent_only = lent_only;
            let mut asked = self.vet(settings, &name, &zone, &lent.token, wanted, before);
            (asked.named, asked.wanted, asked.lent_only) = (named, wanted.to_vec(), lent_only);
            self.asked.insert(key.clone(), asked);
        }
        self.asked
            .get(&key)
            .and_then(|asked| asked.host_key.clone())
            .map(|host_key| (name, host_key))
    }

    /// What the cloud says of instance `name` in `zone` now, `before` being
    /// what it said last: its host key, kept while one `wanted` holds, once
    /// the instance is vetted, and none once it isn't. A machine that pins
    /// no startup script has its instances vetted by host key alone.
    fn vet(
        &mut self,
        settings: &Settings,
        name: &str,
        zone: &str,
        token: &Path,
        wanted: &[String],
        before: Asked,
    ) -> Asked {
        let now = Instant::now();
        if before.spoiled {
            return Asked {
                at: Some(now),
                ..before
            };
        }
        let (trusted_at, reread) = (before.trusted_at, before.reread);
        let again = !reread && trusted_at.is_some_and(|at| at.elapsed() >= REREAD);
        let verdict = self
            .copied(token)
            .map_err(Unvouched::Unanswered)
            .and_then(|token| {
                let verdict = match pinned(settings) {
                    Ok(None) => Ok(None),
                    Ok(Some(pinned)) => {
                        let first = (before.started.as_deref().zip(before.marks.as_deref()))
                            .map(|(started, marks)| (started, marks, again));
                        self.vouched(settings, name, zone, &token, &pinned, first)
                            .map(Some)
                    }
                    Err(why) => Err(Unvouched::Refused(why)),
                };
                verdict.map(|vouched| (token, vouched))
            });
        let (token, (started, marks)) = match verdict {
            Ok((token, vouched)) => (token, vouched.unzip()),
            Err(Unvouched::Silent) => {
                self.say(
                    name,
                    Some(format!("the cloud didn't describe {name} in {zone}")),
                );
                let window = if before.lent_only {
                    STANDS + ASK_LENT
                } else {
                    STANDS
                };
                let stands = before.vouched.is_some_and(|at| at.elapsed() < window);
                return Asked {
                    host_key: before.host_key.filter(|_| stands),
                    at: Some(now),
                    ..before
                };
            }
            Err(Unvouched::Unanswered(why)) => {
                self.say(
                    name,
                    Some(format!(
                        "{name} in {zone} lends nothing until the cloud answers for it: {why}"
                    )),
                );
                return Asked {
                    host_key: None,
                    at: Some(now),
                    ..before
                };
            }
            Err(Unvouched::Refused(why)) => {
                self.say(
                    name,
                    Some(format!("{name} in {zone} {why}, so root trusts it no more")),
                );
                return Asked {
                    host_key: None,
                    at: Some(now),
                    spoiled: before.started.is_some(),
                    ..before
                };
            }
        };
        let host_key = match before.host_key {
            Some(host_key) if wanted.contains(&host_key) => Some(host_key),
            _ => self.host_key(settings, name, zone, &token),
        };
        let none = format!("the cloud gave no host key for {name} in {zone}");
        self.say(name, host_key.is_none().then_some(none));
        Asked {
            host_key,
            at: Some(now),
            vouched: started.is_some().then_some(now),
            trusted_at: started.is_some().then(|| trusted_at.unwrap_or(now)),
            reread: reread || again,
            started,
            marks,
            spoiled: false,
            ..Asked::default()
        }
    }

    /// When instance `name` in `zone` last started, and its fingerprints,
    /// while the cloud says it's as `pinned` holds it, in a project whose
    /// metadata holds, and that neither moved since `first`, when root
    /// first trusted it; why not otherwise. At first trust, its boot disk's
    /// image is asked, it must have started once, soon after its making,
    /// and the audit log must hold its making alone, as it must still when
    /// `first` says it's read again.
    fn vouched(
        &mut self,
        settings: &Settings,
        name: &str,
        zone: &str,
        token: &Path,
        pinned: &Pinned,
        first: Option<(&str, &str, bool)>,
    ) -> Result<(String, String), Unvouched> {
        let described = self.describe(settings, "instances", name, zone, token)?;
        let started = vet::vetted(&described, pinned)?;
        let marks = vet::marks(&described).ok_or("says nothing of its fingerprints".to_owned())?;
        self.project_holds(settings, token)?;
        match first {
            Some((first, _, _)) if first != started => Err(Unvouched::Refused(format!(
                "started again at {started}, after root first trusted it at {first}"
            ))),
            Some((_, was, _)) if was != marks => Err(Unvouched::Refused(format!(
                "was changed since root first trusted it, its fingerprints moving from {was} \
                 to {marks}"
            ))),
            Some((_, _, again)) => {
                if again {
                    let disk = vet::boot_disk(&described).ok_or("names no boot disk".to_owned())?;
                    let history = self.history(settings, name, zone, &disk, token)?;
                    vet::made_alone(&history)
                        .map_err(|why| format!("{why}, as its audit log said once read again"))?;
                }
                Ok((started, marks))
            }
            None => {
                vet::started_once(&described)?;
                let disk = vet::boot_disk(&described).ok_or("names no boot disk".to_owned())?;
                let described = self.describe(settings, "disks", &disk, zone, token)?;
                vet::imaged(&described, pinned)?;
                vet::made_alone(&self.history(settings, name, zone, &disk, token)?)?;
                Ok((started, marks))
            }
        }
    }

    /// Whether the project's metadata holds, as [`vet::project_holds`]
    /// says, asked again every [`ASK_AGAIN`] while the cloud answers.
    fn project_holds(&mut self, settings: &Settings, token: &Path) -> Result<(), Unvouched> {
        if let Some((at, holds)) = &self.project
            && at.elapsed() < ASK_AGAIN
        {
            return holds.clone().map_err(Unvouched::Refused);
        }
        let words = ["compute", "project-info", "describe", "--format=json"];
        let said = self.gcloud(settings, &words, token)?;
        let described: Value = serde_json::from_str(&said)
            .map_err(|_| Unvouched::Unanswered("its project's answer wasn't JSON".to_owned()))?;
        let holds = vet::project_holds(&described);
        self.project = Some((Instant::now(), holds.clone()));
        holds.map_err(Unvouched::Refused)
    }

    /// A copy of the token in `token`, which the lender's account writes,
    /// kept where only this service reads, once it's a file holding one
    /// token. A pipe there would make gcloud wait and garbage would make it
    /// fail, and either could keep a line standing while the cloud said
    /// nothing; a copy can't change while gcloud reads it.
    fn copied(&self, token: &Path) -> Result<PathBuf, String> {
        use std::io::{Read, Write};
        use std::os::unix::fs::OpenOptionsExt;
        let whose = token.display();
        let unread = |_: std::io::Error| format!("its token at {whose} can't be read");
        // Opened without waiting, so a pipe is seen for what it is.
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
            .open(token)
            .map_err(unread)?;
        let held = file.metadata().map_err(unread)?;
        if !held.is_file() || held.len() > TOKEN_BYTES {
            return Err(format!("its token at {whose} isn't a file of one token"));
        }
        let mut said = String::new();
        file.take(TOKEN_BYTES)
            .read_to_string(&mut said)
            .map_err(unread)?;
        let said = said.trim();
        let fits = |byte: u8| byte.is_ascii_alphanumeric() || b"._~+/=-".contains(&byte);
        if said.is_empty() || !said.bytes().all(fits) {
            return Err(format!("its token at {whose} isn't one"));
        }
        let unwritten = |_: std::io::Error| "its token couldn't be copied".to_owned();
        let copies = self.config.join("tokens");
        fs::create_dir_all(&copies).map_err(unwritten)?;
        let copy = copies.join(whose.to_string().replace('/', "_"));
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&copy)
            .and_then(|mut written| written.write_all(said.as_bytes()))
            .map_err(unwritten)?;
        Ok(copy)
    }

    /// What the audit log says was done to instance `name` in `zone` and
    /// its boot disk `disk`, as JSON, newest first: the last day's, since
    /// no builder lives longer.
    fn history(
        &self,
        settings: &Settings,
        name: &str,
        zone: &str,
        disk: &str,
        token: &Path,
    ) -> Result<Value, Unvouched> {
        let project = &settings.project;
        let resource = |kind: &str, name: &str| {
            format!("protoPayload.resourceName=\"projects/{project}/zones/{zone}/{kind}/{name}\"")
        };
        let filter = format!(
            "logName=\"projects/{project}/logs/cloudaudit.googleapis.com%2Factivity\" \
             AND ({} OR {})",
            resource("instances", name),
            resource("disks", disk)
        );
        let words = [
            "logging",
            "read",
            &filter,
            "--freshness=1d",
            "--format=json",
        ];
        serde_json::from_str(&self.gcloud(settings, &words, token)?)
            .map_err(|_| Unvouched::Unanswered("its audit log's answer wasn't JSON".to_owned()))
    }

    /// What the cloud says of the instance or disk (`kind`) `name` in
    /// `zone`, as JSON.
    fn describe(
        &self,
        settings: &Settings,
        kind: &str,
        name: &str,
        zone: &str,
        token: &Path,
    ) -> Result<Value, Unvouched> {
        let said = self.gcloud(
            settings,
            &[
                "compute",
                kind,
                "describe",
                name,
                &format!("--zone={zone}"),
                "--format=json",
            ],
            token,
        )?;
        serde_json::from_str(&said)
            .map_err(|_| Unvouched::Unanswered(format!("its {kind} answer wasn't JSON")))
    }

    /// Asks the cloud for `name`'s host key, with the token in `token`.
    fn host_key(
        &self,
        settings: &Settings,
        name: &str,
        zone: &str,
        token: &Path,
    ) -> Option<String> {
        let said = self
            .gcloud(
                settings,
                &[
                    "compute",
                    "instances",
                    "get-guest-attributes",
                    name,
                    &format!("--zone={zone}"),
                    "--query-path=hostkeys/ssh-ed25519",
                    "--format=value(value)",
                ],
                token,
            )
            .ok()?;
        carried(&said)
    }

    /// What gcloud says to `words` in the settings' project, with the token
    /// in `token`, when it succeeds in time; otherwise whether it was
    /// silent or refused, as [`failed`] tells.
    fn gcloud(
        &self,
        settings: &Settings,
        words: &[&str],
        token: &Path,
    ) -> Result<String, Unvouched> {
        let unrun = || Unvouched::Unanswered("gcloud couldn't be run".to_owned());
        let mut asking = Command::new(&settings.gcloud)
            .args(words)
            .arg(format!("--project={}", settings.project))
            .arg(format!("--access-token-file={}", token.display()))
            .args(["--quiet", "--verbosity=error"])
            .env("CLOUDSDK_CONFIG", &self.config)
            .env("CLOUDSDK_CORE_DISABLE_PROMPTS", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| unrun())?;
        // Read as it comes, since an instance described with its startup
        // script runs to tens of kilobytes, near what a pipe holds unread.
        let read = |mut from: Box<dyn std::io::Read + Send>| {
            std::thread::spawn(move || {
                let mut said = String::new();
                std::io::Read::read_to_string(&mut from, &mut said).map(|_| said)
            })
        };
        let out = read(Box::new(asking.stdout.take().ok_or_else(unrun)?));
        let complaint = read(Box::new(asking.stderr.take().ok_or_else(unrun)?));
        let status = crate::there::finished_within(&mut asking, PATIENCE);
        let said = out.join().ok().and_then(Result::ok);
        let complaint = complaint
            .join()
            .ok()
            .and_then(Result::ok)
            .unwrap_or_default();
        if !complaint.is_empty() {
            eprint!("{complaint}");
        }
        match (status, said) {
            (None, _) => Err(Unvouched::Silent),
            (Some(status), Some(said)) if status.success() => Ok(said),
            _ => Err(failed(&complaint)),
        }
    }
}

/// Every triple trusted now: the standing builders', and each lent one's
/// whose instance the cloud vouches for; and what [`LENT`] holds. `wanted`
/// is what root's files carry, and `own` what each lender's own do.
fn trusted(
    settings: &Settings,
    keys: &mut HostKeys,
    wanted: &HashMap<String, Vec<String>>,
    own: &HashMap<String, Vec<String>>,
) -> (Vec<Trusted>, String) {
    let mut trusted = settings.standing.clone();
    let mut lent_text = String::new();
    for lent in &settings.lent {
        // A builder no line names could pass nothing, so it's never asked
        // of; one only its lender's own files name is asked of less often
        // ([`ASK_LENT`]). A line naming it later changes what's wanted, so
        // it's asked of at once then.
        let root_wants = wanted.get(&lent.uri);
        let mut keys_wanted = root_wants.cloned().unwrap_or_default();
        keys_wanted.extend(own.get(&lent.uri).into_iter().flatten().cloned());
        if keys_wanted.is_empty() {
            continue;
        }
        let Some((instance, host_key)) =
            keys.of(lent, settings, &keys_wanted, root_wants.is_none())
        else {
            continue;
        };
        if let Some(line) = lent_line(&lent.uri, &instance, &host_key) {
            lent_text.push_str(&line);
        }
        trusted.push(Trusted {
            uri: lent.uri.clone(),
            key: settings.key.clone(),
            host_key,
        });
    }
    (trusted, lent_text)
}

/// The line [`LENT`] holds for `instance`, lent through `uri` with
/// `host_key` as a machines line carries it, when that key is an ed25519
/// key, the only kind the fence's agent signs a login for. The lender is
/// the URI's host, as `there` reaches it by name.
fn lent_line(uri: &str, instance: &str, host_key: &str) -> Option<String> {
    let after = uri.split_once("://").map_or(uri, |(_, rest)| rest);
    let host = after.split(['?', '/']).next().unwrap_or(after);
    let lender = host.rsplit('@').next().unwrap_or(host);
    let key = crate::there::decoded(host_key)?;
    let blob = key.trim().strip_prefix("ssh-ed25519 ")?;
    // Each field is one word of its own characters, so no line can hold
    // another's fields.
    let made_of = |text: &str, also: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || also.contains(c))
    };
    (made_of(lender, "-.") && made_of(instance, "-") && made_of(blob, "+/="))
        .then(|| format!("{lender} {instance} ssh-ed25519 {blob}\n"))
}

/// What each lender's own files carry, read from the folder it names its
/// instance in, by the URI it lends through.
fn own_wanted(settings: &Settings) -> HashMap<String, Vec<String>> {
    let mut own = HashMap::new();
    for lent in &settings.lent {
        let Some(folder) = lent.instance.parent() else {
            continue;
        };
        let texts: Vec<String> = settings
            .files
            .iter()
            .map(|file| fs::read_to_string(folder.join(file)).unwrap_or_default())
            .collect();
        if let Some(keys) = wanted(&texts).remove(&lent.uri) {
            own.insert(lent.uri.clone(), keys);
        }
    }
    own
}

/// The host keys the lender's lines carry for each URI, so a key the
/// cloud gave earlier and a line no longer carries is asked again.
fn wanted(texts: &[String]) -> HashMap<String, Vec<String>> {
    let mut wanted: HashMap<String, Vec<String>> = HashMap::new();
    for line in texts.iter().flat_map(|text| text.lines()) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if let [uri, .., host_key] = fields[..] {
            wanted
                .entry(uri.to_owned())
                .or_default()
                .push(host_key.to_owned());
        }
    }
    wanted
}

/// Writes `text` to `path` whole, through a file beside it, so Nix never
/// reads half a list; only when it differs from what's there.
fn put(path: &Path, text: &str) -> std::io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|there| there == text) {
        return Ok(());
    }
    let draft = path.with_extension("draft");
    fs::write(&draft, text)?;
    fs::rename(draft, path)
}

/// Copies what passes once: each of the lender's files into root's.
pub fn copy_once(settings: &Settings, keys: &mut HostKeys) {
    let texts: Vec<String> = settings
        .files
        .iter()
        .map(|file| fs::read_to_string(settings.from.join(file)).unwrap_or_default())
        .collect();
    let (trusted, lent_text) = trusted(settings, keys, &wanted(&texts), &own_wanted(settings));
    if let Err(error) = put(&settings.into.join(LENT), &lent_text) {
        eprintln!("nix-offload: couldn't write {LENT}: {error}");
    }
    for (file, text) in settings.files.iter().zip(&texts) {
        let passed = passing(text, &trusted);
        let dropped =
            text.lines().filter(|line| !line.trim().is_empty()).count() - passed.lines().count();
        let left_out = format!(
            "{dropped} of {file}'s lines name a builder root can't trust, so they're left out"
        );
        keys.say(file, (dropped > 0).then_some(left_out));
        if let Err(error) = put(&settings.into.join(file), &passed) {
            eprintln!("nix-offload: couldn't write {file}: {error}");
        }
    }
}

/// Copies what passes for as long as it runs, at [`PACE`].
pub fn serve(settings: &Settings, config: PathBuf) -> ! {
    let mut keys = HostKeys::new(config);
    loop {
        copy_once(settings, &mut keys);
        std::thread::sleep(PACE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::executable;
    use serde_json::json;

    /// What `base64 -w0` makes of `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHost`.
    const HOST: &str = "c3NoLWVkMjU1MTkgQUFBQUMzTnphQzFsWkRJMU5URTVBQUFBSUhvc3Q=";
    /// And of `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOther`.
    const OTHER: &str = "c3NoLWVkMjU1MTkgQUFBQUMzTnphQzFsWkRJMU5URTVBQUFBSU90aGVy";
    const URI: &str = "ssh-ng://nix-offload?compress=true";
    const KEY: &str = "/etc/nix/offload_ed25519";

    fn line(uri: &str, key: &str, host_key: &str) -> String {
        format!("{uri} x86_64-linux {key} 8 1 big-parallel,benchmark big-parallel {host_key}")
    }

    fn lent() -> Vec<Trusted> {
        vec![Trusted {
            uri: URI.to_owned(),
            key: KEY.to_owned(),
            host_key: HOST.to_owned(),
        }]
    }

    #[test]
    fn a_line_naming_a_trusted_builder_passes() {
        assert!(
            passes(&line(URI, KEY, HOST), &lent()),
            "the trusted line was refused"
        );
    }

    #[test]
    fn a_line_naming_another_host_key_is_refused() {
        assert!(
            !passes(&line(URI, KEY, OTHER), &lent()),
            "a host key the cloud didn't give passed"
        );
    }

    #[test]
    fn a_line_naming_another_host_or_key_is_refused() {
        let elsewhere = line("ssh-ng://someone@localhost?compress=true", KEY, HOST);
        assert!(
            !passes(&elsewhere, &lent()),
            "a builder of nobody's choosing passed"
        );
        let theirs = line(URI, "/home/someone/.ssh/id_ed25519", HOST);
        assert!(!passes(&theirs, &lent()), "a key other than root's passed");
    }

    #[test]
    fn a_query_overriding_the_host_key_is_refused() {
        let overriding = format!("{URI}&base64-ssh-public-host-key={OTHER}");
        assert!(
            !passes(&line(&overriding, KEY, HOST), &lent()),
            "a URI whose query overrides the host key passed"
        );
    }

    #[test]
    fn a_field_starting_another_machine_or_ending_the_line_is_refused() {
        for smuggled in [
            "big-parallel;ssh-ng://evil",
            "big-parallel#",
            "@/home/someone/machines",
        ] {
            let text = format!("{URI} x86_64-linux {KEY} 8 1 {smuggled} - {HOST}");
            assert!(!passes(&text, &lent()), "{smuggled} passed as a feature");
        }
    }

    #[test]
    fn a_line_of_other_than_eight_fields_is_refused() {
        let short = format!("{URI} x86_64-linux {KEY} 8 1 - {HOST}");
        assert!(!passes(&short, &lent()), "a seven-field line passed");
        let long = format!("{} extra", line(URI, KEY, HOST));
        assert!(!passes(&long, &lent()), "a nine-field line passed");
    }

    #[test]
    fn only_the_lines_that_pass_are_kept() {
        let text = format!("{}\n\n{}\n", line(URI, KEY, HOST), line(URI, KEY, OTHER));
        assert_eq!(
            passing(&text, &lent()),
            format!("{}\n", line(URI, KEY, HOST)),
            "the wrong lines were kept"
        );
    }

    #[test]
    fn a_host_key_is_carried_as_base64_writes_it() {
        let bare = "AAAAC3NzaC1lZDI1NTE5AAAAIHost";
        assert_eq!(
            carried(bare).as_deref(),
            Some(HOST),
            "a bare key was carried wrongly"
        );
        let typed = format!("ssh-ed25519 {bare}\n");
        assert_eq!(
            carried(&typed).as_deref(),
            Some(HOST),
            "a typed key was carried wrongly"
        );
        assert_eq!(base64(b"ab"), "YWI=", "two bytes weren't padded once");
        assert_eq!(carried(""), None, "nothing was carried as a key");
        assert_eq!(carried("AAAA;evil"), None, "a key holding `;` was carried");
    }

    #[test]
    fn an_instance_is_taken_only_by_its_prefix_and_zone() {
        let zones = vec!["us-east1-b".to_owned()];
        assert_eq!(
            instance("offload-1a2b us-east1-b\n", "offload-", &zones),
            Ok(("offload-1a2b".to_owned(), "us-east1-b".to_owned())),
            "a builder of the lender's was refused"
        );
        for wrong in [
            "theirs-1 us-east1-b",
            "offload-1 elsewhere-a",
            "offload- us-east1-b",
            "offload-1",
            "offload-X us-east1-b",
        ] {
            assert!(
                instance(wrong, "offload-", &zones).is_err(),
                "{wrong} was taken"
            );
        }
    }

    /// Settings for a lender in `dir`, whose gcloud is `gcloud`.
    fn settings_in(dir: &Path, gcloud: &Path) -> Settings {
        Settings {
            from: dir.join("from"),
            into: dir.join("into"),
            files: vec!["machines".to_owned(), "machines-with-standing".to_owned()],
            key: KEY.to_owned(),
            standing: vec![Trusted {
                uri: "ssh-ng://builder@home".to_owned(),
                key: "/etc/nix/home_ed25519".to_owned(),
                host_key: OTHER.to_owned(),
            }],
            lent: vec![Lent {
                uri: URI.to_owned(),
                instance: dir.join("from/instance"),
                token: dir.join("from/token"),
            }],
            project: "a-lab".to_owned(),
            prefix: "offload-".to_owned(),
            zones: vec!["us-east1-b".to_owned()],
            gcloud: gcloud.display().to_string(),
            startup_script: Some(dir.join("startup-script")),
            hook_key: dir.join("offload_ed25519.pub"),
            images: vec![format!("{IMAGES}debian-12-")],
        }
    }

    const IMAGES: &str = "https://compute.example/projects/debian-cloud/global/images/";
    const MADE: &str = "2026-11-10T05:14:21.390-08:00";
    const STARTED: &str = "2026-11-10T05:14:27.724-08:00";
    const ROOT_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIRoot";

    /// Writes what the cloud says of instance offload-1a2b into `dir`: that
    /// it started at `started`, carrying `extra` metadata beside its maker's.
    fn instance_in(dir: &Path, started: &str, extra: &[(&str, &str)]) {
        marked_in(dir, started, extra, "Kk2h");
    }

    /// As [`instance_in`], with `mark` for its own fingerprint.
    fn marked_in(dir: &Path, started: &str, extra: &[(&str, &str)], mark: &str) {
        let mut items = vec![
            json!({"key": "block-project-ssh-keys", "value": "TRUE"}),
            json!({"key": "offload-hook-key", "value": ROOT_KEY}),
            json!({"key": "startup-script", "value": "#!/bin/bash\necho the builder\n"}),
        ];
        items.extend(
            extra
                .iter()
                .map(|(key, value)| json!({"key": key, "value": value})),
        );
        let described = json!({
            "status": "RUNNING",
            "creationTimestamp": MADE,
            "lastStartTimestamp": started,
            "fingerprint": mark,
            "metadata": {"items": items, "fingerprint": "MQpH"},
            "disks": [{"boot": true, "source": "https://compute.example/disks/offload-1a2b"}],
        });
        fs::write(dir.join("instance.json"), described.to_string()).expect("the instance");
    }

    /// A cloud in `dir` that describes the instance, its disk and its
    /// project from `instance.json`, `disk.json` and `project.json`, reads
    /// its audit log from `history.json`, gives the host key `HOST`
    /// carries, and logs what it's asked to `asked`; that can't be reached
    /// while `unreachable` is there, and refuses the token while `refusing`
    /// is; and the files root pins a lent builder to, with its boot disk
    /// made from `image`.
    fn cloud_in(dir: &Path, image: &str) -> PathBuf {
        let gcloud = dir.join("gcloud");
        let dir = dir.display();
        executable(
            &gcloud,
            &format!(
                "#!/bin/sh\necho \"$*\" >> '{dir}/asked'\n\
                 if [ -e '{dir}/unreachable' ]; then\n\
                 echo 'ERROR: gcloud crashed (ConnectionError): Max retries exceeded' >&2\n\
                 exit 1\nfi\n\
                 if [ -e '{dir}/refusing' ]; then\n\
                 echo 'ERROR: Request had invalid authentication credentials.' >&2\n\
                 exit 1\nfi\n\
                 case \"$1 $2 $3\" in\n\
                 'compute instances describe') exec cat '{dir}/instance.json' ;;\n\
                 'compute disks describe') exec cat '{dir}/disk.json' ;;\n\
                 'compute project-info describe') exec cat '{dir}/project.json' ;;\n\
                 logging\\ read*) exec cat '{dir}/history.json' ;;\n\
                 *) echo AAAAC3NzaC1lZDI1NTE5AAAAIHost ;;\nesac\n"
            ),
        );
        let dir = gcloud.parent().expect("the cloud's folder");
        fs::write(
            dir.join("startup-script"),
            "#!/bin/bash\necho the builder\n",
        )
        .expect("the startup script");
        fs::write(
            dir.join("offload_ed25519.pub"),
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIRoot root@machine\n",
        )
        .expect("root's key");
        fs::write(
            dir.join("disk.json"),
            json!({"sourceImage": image}).to_string(),
        )
        .expect("the disk");
        let project = json!({"commonInstanceMetadata": {"items": [
            {"key": "enable-oslogin", "value": "TRUE"},
            {"key": "ssh-keys", "value": "me:ssh-ed25519 AAAA me"},
        ]}});
        fs::write(dir.join("project.json"), project.to_string()).expect("the project");
        history_in(dir, &["v1.compute.instances.insert"; 2]);
        instance_in(dir, STARTED, &[]);
        gcloud
    }

    /// Writes the audit log of offload-1a2b into `dir`: an entry for each of
    /// `methods`, newest first.
    fn history_in(dir: &Path, methods: &[&str]) {
        let entries: Vec<Value> = methods
            .iter()
            .map(|method| {
                json!({
                    "timestamp": "2026-11-10T13:14:20Z",
                    "protoPayload": {
                        "methodName": method,
                        "resourceName": "projects/a-lab/zones/us-east1-b/instances/offload-1a2b",
                    },
                })
            })
            .collect();
        fs::write(dir.join("history.json"), Value::from(entries).to_string()).expect("the log");
    }

    /// A lender in a new folder lending instance offload-1a2b, whose line
    /// carries `HOST`, from a cloud whose boot disk was made from `image`.
    fn lender(image: &str) -> (tempfile::TempDir, Settings, HostKeys) {
        let dir = tempfile::tempdir().expect("a folder");
        for folder in ["from", "into"] {
            fs::create_dir(dir.path().join(folder)).expect("a folder for the files");
        }
        let settings = settings_in(dir.path(), &cloud_in(dir.path(), image));
        fs::write(dir.path().join("from/token"), "ya29.a0-x_y\n").expect("the token");
        fs::write(
            dir.path().join("from/machines"),
            format!("{}\n", line(URI, KEY, HOST)),
        )
        .expect("the lender's machines");
        fs::write(
            dir.path().join("from/instance"),
            "offload-1a2b us-east1-b\n",
        )
        .expect("the instance");
        let keys = HostKeys::new(dir.path().join("config"));
        (dir, settings, keys)
    }

    /// What root's machines file holds in `dir`.
    fn lent_to_root(dir: &tempfile::TempDir) -> String {
        fs::read_to_string(dir.path().join("into/machines")).expect("root's machines")
    }

    /// Makes every instance due to be asked of again, as [`ASK_AGAIN`] on.
    fn due(keys: &mut HostKeys) {
        for asked in keys.asked.values_mut() {
            asked.at = None;
        }
    }

    #[test]
    fn lines_pass_once_the_cloud_vouches_for_the_instance() {
        let dir = tempfile::tempdir().expect("a folder");
        for folder in ["from", "into"] {
            fs::create_dir(dir.path().join(folder)).expect("a folder for the files");
        }
        let asked = dir.path().join("asked");
        let gcloud = cloud_in(dir.path(), &format!("{IMAGES}debian-12-bookworm-v1"));
        let settings = settings_in(dir.path(), &gcloud);
        fs::write(dir.path().join("from/token"), "ya29.a0-x_y\n").expect("the token");
        let standing =
            format!("ssh-ng://builder@home x86_64-linux /etc/nix/home_ed25519 4 1 - - {OTHER}");
        fs::write(
            dir.path().join("from/machines"),
            format!("{}\n{}\n", line(URI, KEY, HOST), line(URI, KEY, OTHER)),
        )
        .expect("the lender's machines");
        fs::write(
            dir.path().join("from/machines-with-standing"),
            format!("{standing}\n"),
        )
        .expect("the lender's standing");
        fs::write(
            dir.path().join("from/instance"),
            "offload-1a2b us-east1-b\n",
        )
        .expect("the instance");
        let mut keys = HostKeys::new(dir.path().join("config"));
        copy_once(&settings, &mut keys);
        let read = |name: &str| {
            fs::read_to_string(dir.path().join("into").join(name)).expect("a written file")
        };
        assert_eq!(
            read("machines"),
            format!("{}\n", line(URI, KEY, HOST)),
            "root's machines hold the wrong lines"
        );
        assert_eq!(
            read("machines-with-standing"),
            format!("{standing}\n"),
            "the standing builder was lost"
        );
        let said = fs::read_to_string(&asked).expect("gcloud was asked");
        let from = dir.path().join("from/token").display().to_string();
        let copy = dir
            .path()
            .join("config/tokens")
            .join(from.replace('/', "_"));
        assert!(
            said.contains("instances describe offload-1a2b --zone=us-east1-b --format=json")
                && said.contains("disks describe offload-1a2b --zone=us-east1-b")
                && said.contains("compute project-info describe --format=json")
                && said.contains(
                    "logging read logName=\"projects/a-lab/logs/\
                     cloudaudit.googleapis.com%2Factivity\" AND (protoPayload.resourceName=\"\
                     projects/a-lab/zones/us-east1-b/instances/offload-1a2b\" OR \
                     protoPayload.resourceName=\"projects/a-lab/zones/us-east1-b/disks/\
                     offload-1a2b\") --freshness=1d"
                )
                && said.contains("instances get-guest-attributes offload-1a2b --zone=us-east1-b")
                && said.contains("--project=a-lab")
                && said.contains(&format!("--access-token-file={}", copy.display())),
            "gcloud was asked otherwise: {said}"
        );
        assert_eq!(
            fs::read_to_string(&copy).expect("the token's copy"),
            "ya29.a0-x_y",
            "gcloud was given another token"
        );
        copy_once(&settings, &mut keys);
        assert_eq!(
            fs::read_to_string(&asked)
                .expect("gcloud's log")
                .lines()
                .count(),
            5,
            "the cloud was asked again before it was due"
        );
    }

    #[test]
    fn an_instance_changed_before_root_first_trusted_it_lends_nothing() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        let made = "v1.compute.instances.insert";
        history_in(dir.path(), &["v1.compute.instances.reset", made, made]);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "an instance reset once made was trusted"
        );
        fs::write(dir.path().join("history.json"), "not json").expect("the log");
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "an instance of unread history was trusted"
        );
    }

    #[test]
    fn an_instance_under_a_project_setting_more_lends_nothing() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        let project = json!({"commonInstanceMetadata": {"items": [
            {"key": "startup-script-url", "value": "gs://mine/script"},
        ]}});
        fs::write(dir.path().join("project.json"), project.to_string()).expect("the project");
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a project's startup script was let run"
        );
    }

    #[test]
    fn a_trusted_instance_whose_fingerprint_moved_is_refused_for_good() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        let trusted = format!("{}\n", line(URI, KEY, HOST));
        assert_eq!(lent_to_root(&dir), trusted, "never trusted");
        marked_in(dir.path(), STARTED, &[], "Zz9q");
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "an instance changed and set back stayed trusted"
        );
        instance_in(dir.path(), STARTED, &[]);
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "its first fingerprint trusted it again"
        );
    }

    #[test]
    fn a_line_naming_a_new_host_key_is_asked_of_at_once() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        let asked = || fs::read_to_string(dir.path().join("asked")).expect("gcloud's log");
        let before = asked().lines().count();
        fs::write(
            dir.path().join("from/machines"),
            format!("{}\n", line(URI, KEY, OTHER)),
        )
        .expect("the lender's machines");
        copy_once(&settings, &mut keys);
        assert!(
            asked().lines().count() > before,
            "a line naming a new key waited for its answer's 30 s"
        );
        let after = asked().lines().count();
        copy_once(&settings, &mut keys);
        assert_eq!(
            asked().lines().count(),
            after,
            "an unchanged line was asked of again"
        );
    }

    #[test]
    fn a_trusted_instance_changed_is_refused_for_good() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            format!("{}\n", line(URI, KEY, HOST)),
            "never trusted"
        );
        instance_in(
            dir.path(),
            STARTED,
            &[("ssh-keys", "me:ssh-ed25519 AAAA me")],
        );
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "an instance given ssh keys stayed trusted"
        );
        instance_in(dir.path(), STARTED, &[]);
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "an instance set back was trusted again"
        );
    }

    #[test]
    fn an_instance_started_again_before_root_first_trusted_it_lends_nothing() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        instance_in(dir.path(), "2026-11-10T06:00:00.000-08:00", &[]);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "an instance started again long after its making was trusted"
        );
    }

    #[test]
    fn a_trusted_instance_whose_audit_log_grows_is_refused_for_good() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        let trusted = format!("{}\n", line(URI, KEY, HOST));
        assert_eq!(lent_to_root(&dir), trusted, "never trusted");
        let made = "v1.compute.instances.insert";
        history_in(
            dir.path(),
            &["v1.compute.instances.setMetadata", made, made],
        );
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            trusted,
            "the audit log was read again before it was due"
        );
        for asked in keys.asked.values_mut() {
            asked.trusted_at = Instant::now().checked_sub(REREAD);
        }
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a change the audit log showed late was missed"
        );
        history_in(dir.path(), &[made, made]);
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a spoiled instance was trusted again"
        );
    }

    #[test]
    fn a_trusted_instance_started_again_is_refused() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            format!("{}\n", line(URI, KEY, HOST)),
            "never trusted"
        );
        instance_in(dir.path(), "2026-11-10T06:00:00.000-08:00", &[]);
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(lent_to_root(&dir), "", "an instance reset stayed trusted");
    }

    #[test]
    fn a_machine_pinning_no_script_trusts_the_host_key_alone() {
        let (dir, mut settings, mut keys) = lender("");
        settings.startup_script = None;
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            format!("{}\n", line(URI, KEY, HOST)),
            "never trusted"
        );
        let asked = fs::read_to_string(dir.path().join("asked")).expect("gcloud's log");
        assert!(
            !asked.contains("describe"),
            "an instance was described with nothing pinned: {asked}"
        );
    }

    #[test]
    fn an_instance_booted_from_another_image_lends_nothing() {
        let (dir, settings, mut keys) =
            lender("https://compute.example/projects/mine/global/images/debian-12-mine");
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a disk from another image was trusted"
        );
    }

    #[test]
    fn what_the_cloud_vouched_for_stands_a_while_when_it_stops_answering() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        fs::write(dir.path().join("unreachable"), "").expect("the network gone");
        due(&mut keys);
        copy_once(&settings, &mut keys);
        let trusted = format!("{}\n", line(URI, KEY, HOST));
        assert_eq!(
            lent_to_root(&dir),
            trusted,
            "one unanswered call dropped the line"
        );
        for asked in keys.asked.values_mut() {
            asked.vouched = Instant::now().checked_sub(STANDS);
        }
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a line stood long after the cloud last vouched"
        );
    }

    #[test]
    fn a_cloud_refusing_the_token_drops_the_line_at_once_until_it_answers() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        let trusted = format!("{}\n", line(URI, KEY, HOST));
        assert_eq!(lent_to_root(&dir), trusted, "the instance wasn't trusted");
        fs::write(dir.path().join("refusing"), "").expect("the token refused");
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a line stood while the cloud refused to answer"
        );
        fs::remove_file(dir.path().join("refusing")).expect("the token taken again");
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            trusted,
            "an instance its fingerprints show unchanged wasn't trusted again"
        );
    }

    #[test]
    fn a_token_that_isnt_one_drops_the_line_at_once() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        let token = dir.path().join("from/token");
        fs::write(&token, "ya29 --impersonate-service-account=x\n").expect("garbage");
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(lent_to_root(&dir), "", "a line stood on a garbage token");
        fs::remove_file(&token).expect("the garbage gone");
        let made = Command::new("mkfifo")
            .arg(&token)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "no pipe was made for the token");
        let began = Instant::now();
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(lent_to_root(&dir), "", "a line stood on a pipe for a token");
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "a pipe for a token kept lines waiting"
        );
    }

    #[test]
    fn only_a_failed_network_is_silence() {
        for (complaint, silent) in [
            (
                "ERROR: gcloud crashed (ConnectionError): Max retries exceeded",
                true,
            ),
            ("Read timed out. (read timeout=300)", true),
            (
                "ERROR: Request had invalid authentication credentials.",
                false,
            ),
            (
                "The resource 'projects/a/instances/offload-connectionerror' was not found",
                false,
            ),
            ("", false),
        ] {
            assert_eq!(
                matches!(failed(complaint), Unvouched::Silent),
                silent,
                "{complaint:?} was misjudged"
            );
        }
    }

    #[test]
    fn an_instance_outside_the_prefix_lends_nothing() {
        let dir = tempfile::tempdir().expect("a folder");
        for folder in ["from", "into"] {
            fs::create_dir(dir.path().join(folder)).expect("a folder for the files");
        }
        let gcloud = dir.path().join("gcloud");
        executable(&gcloud, "#!/bin/sh\necho AAAAC3NzaC1lZDI1NTE5AAAAIHost\n");
        let settings = settings_in(dir.path(), &gcloud);
        fs::write(
            dir.path().join("from/machines"),
            format!("{}\n", line(URI, KEY, HOST)),
        )
        .expect("the lender's machines");
        fs::write(dir.path().join("from/instance"), "theirs-1 us-east1-b\n").expect("the instance");
        copy_once(&settings, &mut HostKeys::new(dir.path().join("config")));
        let machines =
            fs::read_to_string(dir.path().join("into/machines")).expect("root's machines");
        assert_eq!(
            machines, "",
            "a builder outside the prefix was named to root"
        );
    }

    #[test]
    fn a_lender_with_nothing_lent_asks_the_cloud_nothing() {
        let dir = tempfile::tempdir().expect("a folder");
        for folder in ["from", "into"] {
            fs::create_dir(dir.path().join(folder)).expect("a folder for the files");
        }
        let asked = dir.path().join("asked");
        let gcloud = dir.path().join("gcloud");
        executable(
            &gcloud,
            &format!("#!/bin/sh\necho asked >> '{}'\n", asked.display()),
        );
        let settings = settings_in(dir.path(), &gcloud);
        fs::write(
            dir.path().join("from/machines"),
            format!("{}\n", line(URI, KEY, HOST)),
        )
        .expect("the lender's machines");
        fs::write(dir.path().join("from/instance"), "\n").expect("the instance");
        copy_once(&settings, &mut HostKeys::new(dir.path().join("config")));
        assert!(
            !asked.exists(),
            "the cloud was asked with no instance named"
        );
        let machines =
            fs::read_to_string(dir.path().join("into/machines")).expect("root's machines");
        assert_eq!(
            machines, "",
            "a builder no instance vouches for was named to root"
        );
    }

    #[test]
    fn a_builder_no_line_names_is_never_asked_of_until_one_does() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        let asked = dir.path().join("asked");
        let elsewhere = "ssh-ng://hook@elsewhere?compress=true";
        fs::write(
            dir.path().join("from/machines"),
            format!("{}\n", line(elsewhere, KEY, HOST)),
        )
        .expect("the lender's machines");
        for _ in 0..3 {
            copy_once(&settings, &mut keys);
            due(&mut keys);
        }
        assert!(
            !asked.exists(),
            "the cloud was asked of a builder no line root reads names"
        );
        fs::write(
            dir.path().join("from/machines"),
            format!("{}\n", line(URI, KEY, HOST)),
        )
        .expect("the lender's machines");
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            format!("{}\n", line(URI, KEY, HOST)),
            "a line naming the builder waited to be vouched for"
        );
    }

    /// The lent file in `dir`.
    fn lent_file(dir: &tempfile::TempDir) -> String {
        fs::read_to_string(dir.path().join("into").join(LENT)).expect("the lent file")
    }

    /// What [`LENT`] says of `instance`, lent by `lender` with `HOST`.
    fn named(lender: &str, instance: &str) -> String {
        format!("{lender} {instance} ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHost\n")
    }

    #[test]
    fn a_lent_builder_root_vouches_for_is_named_in_the_lent_file_until_it_changes() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_file(&dir),
            named("nix-offload", "offload-1a2b"),
            "the lent file doesn't name the builder root vouches for"
        );
        marked_in(dir.path(), STARTED, &[], "Zz9y");
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_file(&dir),
            "",
            "the lent file still names a changed builder"
        );
    }

    /// Settings beside [`lender`]'s for a second lender, `vm`, in a folder
    /// of its own, lending instance offload-3c4d through `ssh-ng://hook@
    /// nix-offload-vm`, whose line only its own files carry.
    fn beside(dir: &tempfile::TempDir, settings: &mut Settings) -> String {
        let uri = "ssh-ng://hook@nix-offload-vm?compress=true";
        let vm = dir.path().join("vm");
        fs::create_dir(&vm).expect("the second lender's folder");
        fs::write(vm.join("token"), "ya29.a0-x_y\n").expect("its token");
        fs::write(vm.join("instance"), "offload-3c4d us-east1-b\n").expect("its instance");
        fs::write(vm.join("machines"), format!("{}\n", line(uri, KEY, HOST)))
            .expect("its machines");
        settings.lent.push(Lent {
            uri: uri.to_owned(),
            instance: vm.join("instance"),
            token: vm.join("token"),
        });
        uri.to_owned()
    }

    #[test]
    fn a_builder_only_its_own_lender_names_is_in_the_lent_file_alone() {
        let (dir, mut settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        fs::write(dir.path().join("from/instance"), "\n").expect("nothing lent first");
        fs::write(dir.path().join("from/machines"), "").expect("no line first");
        beside(&dir, &mut settings);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_file(&dir),
            named("nix-offload-vm", "offload-3c4d"),
            "the lent file doesn't name the builder only its lender's files name"
        );
        assert_eq!(
            lent_to_root(&dir),
            "",
            "root was given a builder its files don't name"
        );
    }

    #[test]
    fn a_builder_only_the_lent_file_names_is_asked_of_less_often_while_vouched_for() {
        let now = Instant::now();
        let vouched = Asked {
            at: Some(now),
            vouched: Some(now),
            ..Asked::default()
        };
        assert_eq!(
            vouched.every(true),
            ASK_LENT,
            "a lent-only builder was asked of as often"
        );
        assert_eq!(
            vouched.every(false),
            ASK_AGAIN,
            "a builder root reads was asked of less"
        );
        let unheard = Asked {
            at: Some(now),
            vouched: now.checked_sub(Duration::from_secs(40)),
            ..Asked::default()
        };
        assert_eq!(
            unheard.every(true),
            ASK_AGAIN,
            "a lent-only builder the cloud didn't answer for waited to be asked again"
        );
    }

    #[test]
    fn what_the_cloud_vouched_for_stands_longer_for_a_builder_only_the_lent_file_names() {
        let (dir, mut settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        fs::write(dir.path().join("from/instance"), "\n").expect("nothing lent first");
        fs::write(dir.path().join("from/machines"), "").expect("no line first");
        beside(&dir, &mut settings);
        copy_once(&settings, &mut keys);
        for asked in keys.asked.values_mut() {
            asked.vouched = Instant::now().checked_sub(STANDS + Duration::from_secs(60));
            asked.at = None;
        }
        fs::write(dir.path().join("unreachable"), "").expect("the network down");
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_file(&dir),
            named("nix-offload-vm", "offload-3c4d"),
            "a lent-only builder was dropped for one call lost"
        );
    }

    #[test]
    fn a_lent_line_is_made_only_of_plain_words_and_an_ed25519_key() {
        let uri = "ssh-ng://hook@nix-offload-vm-3?compress=true";
        assert_eq!(
            lent_line(uri, "offload-3c4d", HOST),
            Some(named("nix-offload-vm-3", "offload-3c4d")),
            "a lent line was misread"
        );
        let rsa = "c3NoLXJzYSBBQUFBQjNOemFDMXljMkVBQUFBREFRQUI=";
        assert_eq!(
            lent_line(uri, "offload-3c4d", rsa),
            None,
            "an RSA key was named"
        );
        assert_eq!(
            lent_line(uri, "offload 3c4d", HOST),
            None,
            "an instance of two words was named"
        );
    }

    #[test]
    fn settings_are_read_from_json() {
        let text = r#"{"from":"/run/nix-offload","into":"/run/nix-offload-lines","files":["machines"],
            "key":"/etc/nix/offload_ed25519","standing":[{"uri":"u","key":"k","hostKey":"h"}],
            "lent":[{"uri":"l","instance":"/run/nix-offload/instance","token":"/run/nix-offload/token"}],
            "project":"p","prefix":"offload-","zones":["z"],"gcloud":"/bin/gcloud",
            "startupScript":"/etc/startup","hookKey":"/etc/nix/offload_ed25519.pub",
            "images":["i"]}"#;
        let read = settings(text).expect("the settings");
        assert_eq!(
            read.lent[0].token,
            PathBuf::from("/run/nix-offload/token"),
            "the token's path was misread"
        );
        assert_eq!(
            read.standing[0].host_key, "h",
            "the standing host key was misread"
        );
        assert_eq!(
            (read.startup_script, read.hook_key, read.images),
            (
                Some(PathBuf::from("/etc/startup")),
                PathBuf::from("/etc/nix/offload_ed25519.pub"),
                vec!["i".to_owned()]
            ),
            "what a lent builder is pinned to was misread"
        );
        assert!(settings("{}").is_err(), "empty settings were taken");
    }
}

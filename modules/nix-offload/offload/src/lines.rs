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
//! or host key it can't vouch for, or that asks a standing builder for a
//! system or more builds at once than the machine trusts it with.
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
use std::env;
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
/// who root reaches and believes, and what its line may ask of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trusted {
    pub uri: String,
    pub key: String,
    pub host_key: String,
    /// The systems its line may name, a standing builder's from the
    /// machine's settings, since its owner lent it for those alone; a lent
    /// builder is the lender's own, made for whatever it names.
    pub systems: Option<Vec<String>>,
    /// The most jobs its line may ask of it at once, where the machine
    /// says.
    pub max_jobs: Option<u64>,
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
            let systems = item["systems"]
                .as_array()
                .ok_or("a standing entry lacks systems")?
                .iter()
                .map(|system| system.as_str().map(str::to_owned))
                .collect::<Option<_>>()
                .ok_or("a standing entry's systems hold more than text")?;
            let max_jobs = match &item["maxJobs"] {
                Value::Null => None,
                jobs => Some(
                    jobs.as_u64()
                        .filter(|&jobs| jobs > 0)
                        .ok_or("a standing entry's maxJobs isn't a count")?,
                ),
            };
            Ok(Trusted {
                uri: field(item, "uri")?,
                key: field(item, "key")?,
                host_key: field(item, "hostKey")?,
                systems: Some(systems),
                max_jobs,
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

impl Trusted {
    /// Whether this names the builder `uri`, reached with `key`, whose host
    /// key is `host_key`.
    fn names(&self, uri: &str, key: &str, host_key: &str) -> bool {
        self.uri == uri && self.key == key && self.host_key == host_key
    }

    /// Whether a line may ask this builder for `systems`, comma-separated,
    /// and `jobs` at once, read as Nix reads them: `-` for the systems is
    /// this machine's own, and for the jobs is one.
    fn allows(&self, systems: &str, jobs: &str) -> bool {
        let own = own_system();
        let systems = if systems == "-" {
            own.as_str()
        } else {
            systems
        };
        let jobs = if jobs == "-" { "1" } else { jobs };
        let systems_held = self.systems.as_ref().is_none_or(|held| {
            systems
                .split(',')
                .all(|system| held.iter().any(|trusted| trusted == system))
        });
        let jobs_held = self
            .max_jobs
            .is_none_or(|most| jobs.parse::<u64>().is_ok_and(|asked| asked <= most));
        systems_held && jobs_held
    }
}

/// This machine's system as Nix names it, which a line's `-` stands for.
fn own_system() -> String {
    let os = match env::consts::OS {
        "macos" => "darwin",
        os => os,
    };
    format!("{}-{os}", env::consts::ARCH)
}

/// The eight fields Nix reads of a machines line, or none when it has
/// another number of them.
fn fields(line: &str) -> Option<[&str; 8]> {
    line.split_whitespace().collect::<Vec<_>>().try_into().ok()
}

/// Whether `line` names a builder `trusted` holds but asks it for a system
/// or more builds at once than it's trusted with, which says the machine's
/// settings and the lender's disagree rather than that the line is forged.
fn asks_more(line: &str, trusted: &[Trusted]) -> bool {
    fields(line).is_some_and(|[uri, systems, key, jobs, .., host_key]| {
        trusted
            .iter()
            .any(|one| one.names(uri, key, host_key) && !one.allows(systems, jobs))
    })
}

/// What's said of `file` when lines of it are left out: `untrusted` naming
/// a builder root can't trust and `asking` asking one for more than it's
/// trusted with, which a person mends in different places.
fn left_out(file: &str, untrusted: usize, asking: usize) -> Option<String> {
    let untrusted = (untrusted > 0)
        .then(|| format!("{untrusted} of {file}'s lines name a builder root can't trust"));
    let asking = (asking > 0).then(|| {
        format!(
            "{asking} of {file}'s lines ask a standing builder for a system or more builds \
             at once than the machine trusts it with"
        )
    });
    let said: Vec<String> = [untrusted, asking].into_iter().flatten().collect();
    (!said.is_empty()).then(|| format!("{}, so they're left out", said.join(", and ")))
}

/// Whether `line` names a builder `trusted` holds, in Nix's eight fields,
/// asking it only for what it's trusted with, each of the rest plain.
pub fn passes(line: &str, trusted: &[Trusted]) -> bool {
    let Some(
        [
            uri,
            systems,
            key,
            jobs,
            speed,
            features,
            mandatory,
            host_key,
        ],
    ) = fields(line)
    else {
        return false;
    };
    [systems, jobs, speed, features, mandatory]
        .iter()
        .all(|field| plain(field))
        && trusted
            .iter()
            .any(|one| one.names(uri, key, host_key) && one.allows(systems, jobs))
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
    /// Whether it's trusted no more, which is for good, outliving the
    /// service in [`SPOILED`]: refused once root had trusted it, or seen
    /// under a project setting that acts on it, whether trusted yet or not.
    /// What it ran while changed may still be there once it's set back.
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
    /// It answered, and the instance sits under a project setting that
    /// would run something on it or make accounts there. It's trusted no
    /// more, whether root trusted it yet or not, since what that ran stays
    /// on its disk once the setting goes, out of sight of its audit log.
    Project(vet::Sits),
    /// It answered, and the instance isn't as made.
    Refused(String),
}

impl From<String> for Unvouched {
    fn from(why: String) -> Self {
        Self::Refused(why)
    }
}

/// What's journalled of instance `name` in `zone`, trusted no more for
/// `why`, with how long that lasts and the one way back, [`SPOILED`] being
/// `file`.
fn spoiled_line(name: &str, zone: &str, why: &str, file: &Path) -> String {
    format!(
        "{name} in {zone} {why}, so root trusts it no more until the lender replaces it; \
         it's trusted again only once root, or the service's own account, takes its line \
         out of {} and nix-offload-lines restarts",
        file.display()
    )
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
    /// How many lines it has journalled, so a test can tell a note said
    /// once from one said again.
    #[cfg(test)]
    told: usize,
    /// The project as the cloud last described it, and when that was asked.
    project: Option<(Instant, Value)>,
    /// Why [`SPOILED`] couldn't be read at the start, when it couldn't:
    /// then root trusts no builder, and the file is never written, so the
    /// builders it names aren't forgotten.
    unread: Option<String>,
    /// Whether [`SPOILED`] lacks a builder spoiled since, its write having
    /// failed, so each pass writes it again until one holds.
    unkept: bool,
}

/// The file beside gcloud's state naming each instance root trusts no
/// more, by name and zone, one to a line, so a restart of the service,
/// after which first trust can't see a project setting gone since, doesn't
/// trust it again. A builder's name is its own for good, so the file grows
/// only by the builders spoiled, and only root, or the service's own
/// account, which owns the folder, can take one out.
const SPOILED: &str = "spoiled";

/// The builders, by name and zone, the [`SPOILED`] file's `text` names,
/// or the first line that names none, by its number.
fn spoiled_in(text: &str) -> Result<Vec<(String, String)>, usize> {
    let mut spoiled = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words[..] {
            [] => {}
            [name, zone] => spoiled.push((name.to_owned(), zone.to_owned())),
            _ => return Err(number + 1),
        }
    }
    Ok(spoiled)
}

impl HostKeys {
    /// None asked yet but those [`SPOILED`] beside `config` names, with
    /// gcloud keeping its state in `config`.
    pub fn new(config: PathBuf) -> Self {
        let mut keys = Self {
            asked: HashMap::new(),
            config,
            said: HashMap::new(),
            #[cfg(test)]
            told: 0,
            project: None,
            unread: None,
            unkept: false,
        };
        let file = keys.spoiled_file();
        // Only a file that isn't there names nothing: one unread, or a line
        // naming no builder, might name any, so root trusts none until it
        // reads.
        let kept = match fs::read_to_string(&file) {
            Ok(text) => spoiled_in(&text).map_err(|line| format!("line {line} names no builder")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error.to_string()),
        };
        match kept {
            Ok(spoiled) => {
                for key in spoiled {
                    let spoiled = Asked {
                        spoiled: true,
                        ..Asked::default()
                    };
                    keys.asked.insert(key, spoiled);
                }
            }
            Err(why) => keys.unread = Some(format!("{} can't be read: {why}", file.display())),
        }
        keys
    }

    /// Where [`SPOILED`] is kept.
    fn spoiled_file(&self) -> PathBuf {
        self.config.with_file_name(SPOILED)
    }

    /// Writes every instance root trusts no more, `also` among them, to
    /// [`SPOILED`], whole, flushed to disk and then named, so neither a
    /// reader nor a lost power finds half of it, saying when it can't and
    /// trying again each pass until it can. A file that couldn't be read
    /// at the start is never written over.
    fn keep_spoiled(&mut self, also: Option<(&str, &str)>) {
        if self.unread.is_some() {
            return;
        }
        let mut spoiled: Vec<String> = self
            .asked
            .iter()
            .filter(|(_, asked)| asked.spoiled)
            .map(|((name, zone), _)| format!("{name} {zone}"))
            .chain(also.map(|(name, zone)| format!("{name} {zone}")))
            .collect();
        spoiled.sort();
        spoiled.dedup();
        let file = self.spoiled_file();
        let writing = file.with_extension("writing");
        let mut text = spoiled.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        let kept = fs::File::create(&writing)
            .and_then(|mut draft| {
                std::io::Write::write_all(&mut draft, text.as_bytes())?;
                draft.sync_all()
            })
            .and_then(|()| fs::rename(&writing, &file))
            .and_then(|()| {
                file.parent()
                    .map_or(Ok(()), |folder| fs::File::open(folder)?.sync_all())
            });
        self.unkept = kept.is_err();
        if kept.is_err() {
            let _ = fs::remove_file(&writing);
        }
        let unkept = kept.err().map(|error| {
            format!(
                "couldn't keep the builders root trusts no more in {} ({error}), so a restart \
                 now would trust them again; it's tried again each pass",
                file.display()
            )
        });
        self.say(SPOILED, unkept);
    }

    /// Trusts no more every other instance root trusts, as the project's
    /// setting `why` reaches every builder alike, and journals each.
    fn spoil_trusted(&mut self, why: &str) {
        let file = self.spoiled_file();
        let mut spoiled = Vec::new();
        for ((name, zone), asked) in &mut self.asked {
            if asked.started.is_some() && !asked.spoiled {
                (asked.spoiled, asked.host_key) = (true, None);
                spoiled.push((name.clone(), zone.clone()));
            }
        }
        for (name, zone) in spoiled {
            let line = spoiled_line(&name, &zone, why, &file);
            self.say(&name, Some(line));
        }
    }

    /// What instance `name` in `zone` is once the project's setting `sits`
    /// reaches it, `before` being what was said of it last: spoiled,
    /// whether root trusted it yet or not, and every builder root trusts
    /// with it, since the project is every builder's.
    fn under_project(
        &mut self,
        (name, zone): (&str, &str),
        sits: &vet::Sits,
        before: Asked,
    ) -> Asked {
        self.spoil_trusted(&sits.why);
        let said = spoiled_line(name, zone, &sits.why, &self.spoiled_file());
        self.say(name, Some(said));
        self.keep_spoiled(Some((name, zone)));
        Asked {
            host_key: None,
            at: Some(Instant::now()),
            spoiled: true,
            ..before
        }
    }

    /// What instance `name` in `zone` is once the cloud says `why` it's
    /// refused, `before` being what was said of it last: spoiled when root
    /// trusted it already, and untrusted otherwise.
    fn refused(&mut self, (name, zone): (&str, &str), why: &str, before: Asked) -> Asked {
        let spoiled = before.started.is_some();
        let said = if spoiled {
            spoiled_line(name, zone, why, &self.spoiled_file())
        } else {
            format!("{name} in {zone} {why}, so root trusts it no more")
        };
        self.say(name, Some(said));
        if spoiled {
            self.keep_spoiled(Some((name, zone)));
        }
        Asked {
            host_key: None,
            at: Some(Instant::now()),
            spoiled,
            ..before
        }
    }

    /// Says `what` of `about` when it isn't what was last said of it, so a
    /// refusal read every two seconds is journalled once; none says nothing
    /// and clears it.
    fn say(&mut self, about: &str, what: Option<String>) {
        match what {
            Some(what) if self.said.get(about) != Some(&what) => {
                eprintln!("nix-offload: {what}");
                #[cfg(test)]
                {
                    self.told += 1;
                }
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
        if let Some(why) = self.unread.clone() {
            let said = format!(
                "root trusts no builder while {why}; once it reads, restart nix-offload-lines"
            );
            self.say(SPOILED, Some(said));
            return Asked {
                host_key: None,
                at: Some(now),
                ..before
            };
        }
        if before.spoiled {
            // Said once after a start, when only the file says why.
            if !self.said.contains_key(name) {
                let why = "was trusted no more before nix-offload-lines last started";
                let said = spoiled_line(name, zone, why, &self.spoiled_file());
                self.say(name, Some(said));
            }
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
                    // Root's own files unread say nothing of the instance,
                    // so nothing stands, and nothing is spoiled for good.
                    Err(why) => Err(Unvouched::Unanswered(format!("root can't read {why}"))),
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
            Err(Unvouched::Project(sits)) => {
                return self.under_project((name, zone), &sits, before);
            }
            Err(Unvouched::Refused(why)) => return self.refused((name, zone), &why, before),
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
    /// first trusted it; why not otherwise. A start or fingerprint moved is
    /// said before the project, so the journal gives that as the reason. At
    /// first trust, the project is asked afresh, so a read taken before the
    /// builder booted can't vouch for it, its boot disk's image is asked,
    /// it must have started once, soon after its making, and the audit log
    /// must hold its making alone, as it must still when `first` says it's
    /// read again.
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
        match first {
            Some((first, _, _)) if first != started => Err(Unvouched::Refused(format!(
                "started again at {started}, after root first trusted it at {first}"
            ))),
            Some((_, was, _)) if was != marks => Err(Unvouched::Refused(format!(
                "was changed since root first trusted it, its fingerprints moving from {was} \
                 to {marks}"
            ))),
            Some((_, _, again)) => {
                self.project_holds(settings, token, &described, false)?;
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
                self.project_holds(settings, token, &described, true)?;
                let disk = vet::boot_disk(&described).ok_or("names no boot disk".to_owned())?;
                let described = self.describe(settings, "disks", &disk, zone, token)?;
                vet::imaged(&described, pinned)?;
                vet::made_alone(&self.history(settings, name, zone, &disk, token)?)?;
                Ok((started, marks))
            }
        }
    }

    /// Whether the project's metadata holds for the instance `instance`,
    /// as [`vet::project_holds`] says, the project asked again every
    /// [`ASK_AGAIN`] while the cloud answers, or at once when `fresh`. A
    /// key it lets through that root doesn't know is journalled once,
    /// whatever the verdict, so it can be looked into, and the note goes
    /// once the key does.
    fn project_holds(
        &mut self,
        settings: &Settings,
        token: &Path,
        instance: &Value,
        fresh: bool,
    ) -> Result<(), Unvouched> {
        let described = match &self.project {
            Some((at, described)) if !fresh && at.elapsed() < ASK_AGAIN => described.clone(),
            _ => {
                let words = ["compute", "project-info", "describe", "--format=json"];
                let said = self.gcloud(settings, &words, token)?;
                let described: Value = serde_json::from_str(&said).map_err(|_| {
                    Unvouched::Unanswered("its project's answer wasn't JSON".to_owned())
                })?;
                // The cloud gives every project's metadata a fingerprint,
                // set or not, so an answer without one is no description of
                // it, and would otherwise read as a project setting nothing.
                let marked = described["commonInstanceMetadata"]["fingerprint"]
                    .as_str()
                    .is_some_and(|mark| !mark.is_empty());
                if !marked {
                    return Err(Unvouched::Unanswered(
                        "its project's answer gave no metadata fingerprint".to_owned(),
                    ));
                }
                self.project = Some((Instant::now(), described.clone()));
                described
            }
        };
        let unknown = vet::project_unknown(&described);
        let noted = (!unknown.is_empty()).then(|| {
            format!(
                "the project sets the metadata {}, which root doesn't know what it does; it \
                 names no boot script, ssh key, OS Login or OS Config, so it alone takes no \
                 builder away, and it's worth a look",
                unknown.join(", ")
            )
        });
        self.say("the project", noted);
        vet::project_holds(&described, instance).map_err(Unvouched::Project)
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
            systems: None,
            max_jobs: None,
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
    if keys.unkept {
        keys.keep_spoiled(None);
    }
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
        let asking = text
            .lines()
            .filter(|line| asks_more(line, &trusted))
            .count();
        keys.say(file, left_out(file, dropped - asking, asking));
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
            systems: None,
            max_jobs: None,
        }]
    }

    /// A standing builder the machine lets build `systems`, `max_jobs` at once.
    fn standing(systems: &[&str], max_jobs: Option<u64>) -> Vec<Trusted> {
        vec![Trusted {
            systems: Some(systems.iter().map(|&system| system.to_owned()).collect()),
            max_jobs,
            ..lent().remove(0)
        }]
    }

    #[test]
    fn a_standing_line_passes_for_the_systems_and_jobs_it_is_trusted_with() {
        let both = standing(&["x86_64-linux", "aarch64-linux"], Some(8));
        assert!(
            passes(&line(URI, KEY, HOST), &both),
            "a trusted line was refused"
        );
        let two = format!("{URI} aarch64-linux,x86_64-linux {KEY} 4 1 - - {HOST}");
        assert!(
            passes(&two, &both),
            "a line naming both its systems was refused"
        );
        let unpinned = standing(&["x86_64-linux"], None);
        let many = format!("{URI} x86_64-linux {KEY} 64 1 - - {HOST}");
        assert!(
            passes(&many, &unpinned),
            "jobs were held to a count no one set"
        );
    }

    #[test]
    fn a_standing_line_naming_a_system_it_isnt_trusted_with_is_refused() {
        let linux = standing(&["x86_64-linux"], None);
        for systems in [
            "aarch64-darwin",
            "x86_64-linux,aarch64-darwin",
            "x86_64-linux,",
        ] {
            let text = format!("{URI} {systems} {KEY} 4 1 - - {HOST}");
            assert!(
                !passes(&text, &linux),
                "{systems} passed for a Linux builder"
            );
        }
    }

    #[test]
    fn a_standing_line_asking_more_jobs_than_it_is_trusted_with_is_refused() {
        let four = standing(&["x86_64-linux"], Some(4));
        for jobs in ["5", "4x", "0x4"] {
            let text = format!("{URI} x86_64-linux {KEY} {jobs} 1 - - {HOST}");
            assert!(
                !passes(&text, &four),
                "{jobs} jobs passed where 4 are allowed"
            );
        }
    }

    #[test]
    fn a_dash_is_read_as_nix_reads_it_this_machines_system_and_one_job() {
        let own = standing(&[&own_system()], Some(1));
        let dashed = format!("{URI} - {KEY} - 1 - - {HOST}");
        assert!(
            passes(&dashed, &own),
            "a dash asking what's trusted was refused"
        );
        let other = standing(&["riscv64-none"], Some(1));
        assert!(
            !passes(&dashed, &other),
            "a dash passed for a builder not trusted with this machine's system"
        );
    }

    #[test]
    fn a_lent_builder_is_held_to_no_systems_or_jobs() {
        let text = format!("{URI} aarch64-darwin,x86_64-linux {KEY} 64 1 - - {HOST}");
        assert!(
            passes(&text, &lent()),
            "a lent builder's line was held to a pin"
        );
    }

    #[test]
    fn a_line_asking_more_is_told_apart_from_one_naming_an_untrusted_builder() {
        let four = standing(&["x86_64-linux"], Some(4));
        let more = format!("{URI} x86_64-linux {KEY} 8 1 - - {HOST}");
        let forged = line(URI, KEY, OTHER);
        assert!(asks_more(&more, &four), "a line asking 8 of 4 wasn't told");
        assert!(
            !asks_more(&forged, &four),
            "a line naming another host key was told as asking more"
        );
        assert_eq!(left_out("machines", 0, 0), None);
        assert_eq!(
            left_out("machines", 1, 1).as_deref(),
            Some(
                "1 of machines's lines name a builder root can't trust, and 1 of machines's \
                 lines ask a standing builder for a system or more builds at once than the \
                 machine trusts it with, so they're left out"
            )
        );
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
                systems: Some(vec!["x86_64-linux".to_owned()]),
                max_jobs: Some(4),
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
            json!({"key": "enable-oslogin", "value": "FALSE"}),
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
        let project = json!({"commonInstanceMetadata": {"fingerprint": "p1", "items": [
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

    /// Sets the project's metadata in `dir`'s cloud to `items`.
    fn project_in(dir: &tempfile::TempDir, items: &Value) {
        let project = json!({"commonInstanceMetadata": {"fingerprint": "p2", "items": items}});
        fs::write(dir.path().join("project.json"), project.to_string()).expect("the project");
    }

    /// Sets the project's startup script in `dir`'s cloud.
    fn scripted(dir: &tempfile::TempDir) {
        project_in(
            dir,
            &json!([{"key": "startup-script-url", "value": "gs://mine/script"}]),
        );
    }

    /// Clears the project's metadata in `dir`'s cloud, and root's read of it.
    fn cleared(dir: &tempfile::TempDir, keys: &mut HostKeys) {
        project_in(dir, &json!([]));
        keys.project = None;
    }

    // A builder first seen under a project script ran it at its one boot,
    // which nothing on the instance or its disk shows, so it's never lent,
    // even once the script goes, nor once root's lines service starts again,
    // when first trust would find nothing of it.
    #[test]
    fn an_instance_under_a_project_setting_more_lends_nothing() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        scripted(&dir);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a project's startup script was let run"
        );
        cleared(&dir, &mut keys);
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a builder first seen under a project script was trusted once it went"
        );
        let mut started = HostKeys::new(dir.path().join("config"));
        copy_once(&settings, &mut started);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a builder seen under a project script was trusted once the service started again"
        );
        let restarted = started.said.get("offload-1a2b");
        assert!(
            restarted.is_some_and(|said| said.contains("before nix-offload-lines last started")),
            "a builder kept out after a start wasn't said to be: {restarted:?}"
        );
        let said = keys.said.get("offload-1a2b");
        assert!(
            said.is_some_and(|said| said.contains("until the lender replaces it")
                && said.contains(&dir.path().join(SPOILED).display().to_string())),
            "the spoil didn't say how long it lasts or the way back: {said:?}"
        );
    }

    // What an OS Config job ran as root stays on a builder's disk once the
    // setting goes, logged under the project rather than the instance, so a
    // builder seen under it, trusted yet or not, is trusted no more, and is
    // kept for a restart.
    #[test]
    fn a_builder_under_os_config_stays_out_once_it_goes() {
        for trusted in [false, true] {
            let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
            if trusted {
                copy_once(&settings, &mut keys);
                assert_eq!(
                    lent_to_root(&dir),
                    format!("{}\n", line(URI, KEY, HOST)),
                    "never trusted"
                );
            }
            project_in(&dir, &json!([{"key": "ENABLE-OSCONFIG", "value": "TRUE"}]));
            keys.project = None;
            due(&mut keys);
            copy_once(&settings, &mut keys);
            assert_eq!(lent_to_root(&dir), "", "a builder under OS Config was lent");
            cleared(&dir, &mut keys);
            due(&mut keys);
            copy_once(&settings, &mut keys);
            assert_eq!(
                lent_to_root(&dir),
                "",
                "a builder seen under OS Config, trusted before: {trusted}, was lent once it went"
            );
            assert_eq!(
                fs::read_to_string(dir.path().join(SPOILED)).unwrap_or_default(),
                "offload-1a2b us-east1-b\n",
                "a builder seen under OS Config, trusted before: {trusted}, wasn't kept"
            );
        }
    }

    // Only a spoiled file that isn't there names nothing: one that can't be
    // read, or holds a line naming no builder, might name any, so root
    // trusts none and never writes over it, and a line spaced otherwise
    // still names its builder.
    #[test]
    fn a_spoiled_file_that_cant_be_read_trusts_nothing() {
        for (kept, said) in [
            (None, "can't be read"),
            (
                Some("offload-1a2b us-east1-b x\n"),
                "line 1 names no builder",
            ),
            (
                Some("offload-1a2b\tus-east1-b \n"),
                "before nix-offload-lines",
            ),
        ] {
            let (dir, settings, _) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
            let file = dir.path().join(SPOILED);
            match kept {
                Some(text) => fs::write(&file, text).expect("the spoiled file"),
                None => fs::create_dir(&file).expect("a spoiled file unread"),
            }
            let mut keys = HostKeys::new(dir.path().join("config"));
            copy_once(&settings, &mut keys);
            assert_eq!(
                lent_to_root(&dir),
                "",
                "a builder was lent beside a spoiled file reading {kept:?}"
            );
            assert!(
                keys.said.values().any(|line| line.contains(said)),
                "a spoiled file reading {kept:?} wasn't said: {:?}",
                keys.said
            );
            if let Some(text) = kept {
                assert_eq!(
                    fs::read_to_string(&file).expect("the spoiled file"),
                    text,
                    "the spoiled file was written over"
                );
            }
        }
    }

    // A spoil the file couldn't take is written at the next pass that can,
    // so a restart after it doesn't trust the builder again.
    #[test]
    fn a_spoil_that_couldnt_be_kept_is_kept_once_it_can_be() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        let blocking = dir.path().join(SPOILED).with_extension("writing");
        fs::create_dir(&blocking).expect("a draft that can't be written");
        scripted(&dir);
        copy_once(&settings, &mut keys);
        assert!(
            keys.unkept && !dir.path().join(SPOILED).exists(),
            "a spoil was kept though its draft couldn't be written"
        );
        fs::remove_dir(&blocking).expect("the draft unblocked");
        copy_once(&settings, &mut keys);
        assert_eq!(
            fs::read_to_string(dir.path().join(SPOILED)).unwrap_or_default(),
            "offload-1a2b us-east1-b\n",
            "a spoil once unkept wasn't kept once it could be"
        );
        assert!(!keys.unkept, "a spoil kept was still said to be unkept");
    }

    // The project is every builder's, so what spoils one spoils every
    // builder root trusts, whenever its own next look would fall, and the
    // one that looked as well, trusted or not, each kept for a restart.
    #[test]
    fn a_project_setting_spoils_every_trusted_builder() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        let other = ("offload-9z8y".to_owned(), "us-east1-b".to_owned());
        let trusted = Asked {
            host_key: Some(OTHER.to_owned()),
            started: Some(STARTED.to_owned()),
            ..Asked::default()
        };
        keys.asked.insert(other.clone(), trusted);
        project_in(&dir, &json!([{"key": "enable-osconfig", "value": "TRUE"}]));
        copy_once(&settings, &mut keys);
        let spoiled = &keys.asked[&other];
        assert!(
            spoiled.spoiled && spoiled.host_key.is_none(),
            "a trusted builder stayed trusted under a project setting another's look found"
        );
        let kept = fs::read_to_string(dir.path().join(SPOILED)).unwrap_or_default();
        assert_eq!(
            kept, "offload-1a2b us-east1-b\noffload-9z8y us-east1-b\n",
            "the builder that looked and the trusted one should both be kept as spoiled"
        );
    }

    // An answer naming no metadata fingerprint is no description of the
    // project, so it vouches for nothing.
    #[test]
    fn a_project_answer_without_its_fingerprint_lends_nothing() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        fs::write(dir.path().join("project.json"), "{}").expect("the project");
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "an empty answer vouched for the project"
        );
        let said = keys.said.get("offload-1a2b");
        assert!(
            said.is_some_and(|said| said.contains("no metadata fingerprint")),
            "the empty answer wasn't said: {said:?}"
        );
    }

    // First trust asks the project afresh, so a read another builder left
    // from before this one booted can't vouch for it.
    #[test]
    fn first_trust_reads_the_project_afresh() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        keys.project = Some((Instant::now(), json!({})));
        scripted(&dir);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a project read from before the builder booted vouched for it"
        );
    }

    #[test]
    fn a_project_key_root_doesnt_know_leaves_a_trusted_builder_lent() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        let trusted = format!("{}\n", line(URI, KEY, HOST));
        assert_eq!(lent_to_root(&dir), trusted, "never trusted");
        project_in(
            &dir,
            &json!([
                {"key": "ssh-keys", "value": "someone:ssh-ed25519 AAAA"},
                {"key": "gke-a-cluster-0e6a923f-secondary-ranges", "value": "pods:default"},
            ]),
        );
        keys.project = None;
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            trusted,
            "a cloud product's own project key took a trusted builder away"
        );
        let noted = keys.said.get("the project").cloned();
        assert!(
            noted
                .as_deref()
                .is_some_and(|said| said.contains("gke-a-cluster-0e6a923f-secondary-ranges")),
            "the key root doesn't know wasn't named: {noted:?}"
        );
        let told = keys.told;
        keys.project = None;
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(keys.told, told, "something was journalled anew");
        cleared(&dir, &mut keys);
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            keys.said.get("the project"),
            None,
            "a key gone from the project was still said of it"
        );
    }

    // A reboot from inside the guest runs a project's boot script without
    // the cloud counting a start, so a trusted builder seen under one is
    // trusted no more, and the unknown key that came with it is still said.
    #[test]
    fn a_project_boot_script_spoils_a_trusted_builder() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        let trusted = format!("{}\n", line(URI, KEY, HOST));
        assert_eq!(lent_to_root(&dir), trusted, "never trusted");
        project_in(
            &dir,
            &json!([
                {"key": "startup-script-url", "value": "gs://mine/script"},
                {"key": "came-with-it", "value": "x"},
            ]),
        );
        // Root's read of the project runs out as it would, by its age.
        keys.project = keys.project.take().map(|(at, described)| {
            let aged = at.checked_sub(ASK_AGAIN).expect("a time past");
            (aged, described)
        });
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a project's startup script was let run"
        );
        assert!(
            keys.said
                .get("the project")
                .is_some_and(|said| said.contains("came-with-it")),
            "a key that came with the script wasn't said"
        );
        cleared(&dir, &mut keys);
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            lent_to_root(&dir),
            "",
            "a builder seen under a project script was trusted again once it went"
        );
    }

    // A trusted builder's read of the project stands for ASK_AGAIN, so the
    // cloud is asked it once in a while, not at every look.
    #[test]
    fn a_trusted_builder_reads_the_project_again_only_once_its_read_runs_out() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        let trusted = format!("{}\n", line(URI, KEY, HOST));
        assert_eq!(lent_to_root(&dir), trusted, "never trusted");
        let read = || {
            fs::read_to_string(dir.path().join("asked"))
                .expect("gcloud's log")
                .matches("project-info describe")
                .count()
        };
        let before = read();
        scripted(&dir);
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(
            read(),
            before,
            "the project was asked again within its read's time"
        );
        assert_eq!(
            lent_to_root(&dir),
            trusted,
            "a read still standing wasn't used"
        );
    }

    // A builder started again while a project script is set is said to
    // have started again, which is what spoiled it.
    #[test]
    fn a_builder_started_again_under_a_project_script_is_said_to_have_started_again() {
        let (dir, settings, mut keys) = lender(&format!("{IMAGES}debian-12-bookworm-v1"));
        copy_once(&settings, &mut keys);
        scripted(&dir);
        instance_in(dir.path(), "2026-11-10T06:00:00.000-08:00", &[]);
        keys.project = None;
        due(&mut keys);
        copy_once(&settings, &mut keys);
        assert_eq!(lent_to_root(&dir), "", "a restarted builder was lent");
        let said = keys.said.get("offload-1a2b");
        assert!(
            said.is_some_and(|said| said.contains("started again")),
            "the journal gave another reason than the start: {said:?}"
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
        copy_once(&settings, &mut HostKeys::new(dir.path().join("config")));
        assert_eq!(
            lent_to_root(&dir),
            "",
            "an instance changed and set back was trusted once the service started again"
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
            "key":"/etc/nix/offload_ed25519","standing":[
                {"uri":"u","key":"k","hostKey":"h","systems":["x86_64-linux"],"maxJobs":4},
                {"uri":"v","key":"k","hostKey":"h","systems":["aarch64-linux"],"maxJobs":null}],
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
            (&read.standing[0].systems, read.standing[0].max_jobs),
            (&Some(vec!["x86_64-linux".to_owned()]), Some(4)),
            "what a standing builder may build was misread"
        );
        assert_eq!(
            read.standing[1].max_jobs, None,
            "a standing builder with no count was held to one"
        );
        let unpinned = text.replacen(r#""systems":["x86_64-linux"],"#, "", 1);
        assert!(
            settings(&unpinned).is_err(),
            "a standing builder with no systems was taken"
        );
        let negative = text.replacen(r#""maxJobs":4"#, r#""maxJobs":-1"#, 1);
        assert!(
            settings(&negative).is_err(),
            "a standing builder of -1 jobs was taken"
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

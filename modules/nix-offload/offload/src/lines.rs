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

/// How long what the cloud last vouched for stands while it doesn't
/// answer, so a few failed calls in a row don't drop every lent builder.
const STANDS: Duration = Duration::from_secs(120);

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
    /// When the cloud last vouched for it.
    vouched: Option<Instant>,
    /// Whether it was refused once root had trusted it, which is for good:
    /// what it ran while changed may still be there once it's set back.
    spoiled: bool,
}

/// The host keys asked of the cloud, by instance, and what was last said
/// of each thing that can be refused.
pub struct HostKeys {
    asked: HashMap<(String, String), Asked>,
    config: PathBuf,
    said: HashMap<String, String>,
}

impl HostKeys {
    /// None asked yet, with gcloud keeping its state in `config`.
    pub fn new(config: PathBuf) -> Self {
        Self {
            asked: HashMap::new(),
            config,
            said: HashMap::new(),
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
    /// cloud vouches for the instance, asking again every [`ASK_AGAIN`].
    fn of(&mut self, lent: &Lent, settings: &Settings, wanted: &[String]) -> Option<String> {
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
        let due = self
            .asked
            .get(&key)
            .is_none_or(|asked| asked.at.is_none_or(|at| at.elapsed() >= ASK_AGAIN));
        if due {
            let before = self.asked.remove(&key).unwrap_or_default();
            let asked = self.vet(settings, &name, &zone, &lent.token, wanted, before);
            self.asked.insert(key.clone(), asked);
        }
        self.asked
            .get(&key)
            .and_then(|asked| asked.host_key.clone())
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
        let verdict = match pinned(settings) {
            Ok(None) => Ok(None),
            Ok(Some(pinned)) => {
                let first = before.started.as_deref();
                self.vouched(settings, name, zone, token, &pinned, first)
                    .map(Some)
            }
            Err(why) => Err(Some(why)),
        };
        let started = match verdict {
            Ok(started) => started,
            Err(None) => {
                self.say(
                    name,
                    Some(format!("the cloud didn't describe {name} in {zone}")),
                );
                let stands = before.vouched.is_some_and(|at| at.elapsed() < STANDS);
                return Asked {
                    host_key: before.host_key.filter(|_| stands),
                    at: Some(now),
                    ..before
                };
            }
            Err(Some(why)) => {
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
            _ => self.host_key(settings, name, zone, token),
        };
        let none = format!("the cloud gave no host key for {name} in {zone}");
        self.say(name, host_key.is_none().then_some(none));
        Asked {
            host_key,
            at: Some(now),
            vouched: started.is_some().then_some(now),
            started,
            spoiled: false,
        }
    }

    /// When instance `name` in `zone` last started, while the cloud says
    /// it's as `pinned` holds it and hasn't started again since `first`,
    /// when root first trusted it; why not otherwise, or nothing when the
    /// cloud doesn't answer. Its boot disk's image is asked at first trust.
    fn vouched(
        &self,
        settings: &Settings,
        name: &str,
        zone: &str,
        token: &Path,
        pinned: &Pinned,
        first: Option<&str>,
    ) -> Result<String, Option<String>> {
        let described = self
            .describe(settings, "instances", name, zone, token)
            .ok_or(None)?;
        let started = vet::vetted(&described, pinned)?;
        match first {
            Some(first) if first != started => Err(Some(format!(
                "started again at {started}, after root first trusted it at {first}"
            ))),
            Some(_) => Ok(started),
            None => {
                let disk = vet::boot_disk(&described).ok_or("names no boot disk".to_owned())?;
                let disk = self
                    .describe(settings, "disks", &disk, zone, token)
                    .ok_or("its boot disk couldn't be described".to_owned())?;
                vet::imaged(&disk, pinned)?;
                Ok(started)
            }
        }
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
    ) -> Option<Value> {
        let said = self.gcloud(
            settings,
            &[
                kind,
                "describe",
                name,
                &format!("--zone={zone}"),
                "--format=json",
            ],
            token,
        )?;
        serde_json::from_str(&said).ok()
    }

    /// Asks the cloud for `name`'s host key, with the token in `token`.
    fn host_key(
        &self,
        settings: &Settings,
        name: &str,
        zone: &str,
        token: &Path,
    ) -> Option<String> {
        let said = self.gcloud(
            settings,
            &[
                "instances",
                "get-guest-attributes",
                name,
                &format!("--zone={zone}"),
                "--query-path=hostkeys/ssh-ed25519",
                "--format=value(value)",
            ],
            token,
        )?;
        carried(&said)
    }

    /// What gcloud's `compute` says to `words` in the settings' project,
    /// with the token in `token`, when it succeeds in time.
    fn gcloud(&self, settings: &Settings, words: &[&str], token: &Path) -> Option<String> {
        let mut asking = Command::new(&settings.gcloud)
            .arg("compute")
            .args(words)
            .arg(format!("--project={}", settings.project))
            .arg(format!("--access-token-file={}", token.display()))
            .args(["--quiet", "--verbosity=error"])
            .env("CLOUDSDK_CONFIG", &self.config)
            .env("CLOUDSDK_CORE_DISABLE_PROMPTS", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .ok()?;
        // Read as it comes, since an instance described with its startup
        // script runs to tens of kilobytes, near what a pipe holds unread.
        let mut out = asking.stdout.take()?;
        let reading = std::thread::spawn(move || {
            let mut said = String::new();
            std::io::Read::read_to_string(&mut out, &mut said).map(|_| said)
        });
        let status = crate::there::finished_within(&mut asking, PATIENCE);
        let said = reading.join().ok()?.ok()?;
        status
            .filter(std::process::ExitStatus::success)
            .map(|_| said)
    }
}

/// Every triple trusted now: the standing builders', and each lent one's
/// whose instance the cloud vouches for.
fn trusted(
    settings: &Settings,
    keys: &mut HostKeys,
    wanted: &HashMap<String, Vec<String>>,
) -> Vec<Trusted> {
    let mut trusted = settings.standing.clone();
    for lent in &settings.lent {
        let wanted = wanted.get(&lent.uri).map_or(&[][..], Vec::as_slice);
        if let Some(host_key) = keys.of(lent, settings, wanted) {
            trusted.push(Trusted {
                uri: lent.uri.clone(),
                key: settings.key.clone(),
                host_key,
            });
        }
    }
    trusted
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
    let trusted = trusted(settings, keys, &wanted(&texts));
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
    const STARTED: &str = "2026-11-10T05:14:27.724-08:00";
    const ROOT_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIRoot";

    /// Writes what the cloud says of instance offload-1a2b into `dir`: that
    /// it started at `started`, carrying `extra` metadata beside its maker's.
    fn instance_in(dir: &Path, started: &str, extra: &[(&str, &str)]) {
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
            "lastStartTimestamp": started,
            "metadata": {"items": items},
            "disks": [{"boot": true, "source": "https://compute.example/disks/offload-1a2b"}],
        });
        fs::write(dir.join("instance.json"), described.to_string()).expect("the instance");
    }

    /// A cloud in `dir` that describes the instance and its disk from
    /// `instance.json` and `disk.json`, gives the host key `HOST` carries,
    /// and logs what it's asked to `asked`; and the files root pins a lent
    /// builder to, with its boot disk made from `image`.
    fn cloud_in(dir: &Path, image: &str) -> PathBuf {
        let gcloud = dir.join("gcloud");
        let dir = dir.display();
        executable(
            &gcloud,
            &format!(
                "#!/bin/sh\necho \"$*\" >> '{dir}/asked'\ncase \"$2 $3\" in\n\
                 'instances describe') exec cat '{dir}/instance.json' ;;\n\
                 'disks describe') exec cat '{dir}/disk.json' ;;\n\
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
        instance_in(dir, STARTED, &[]);
        gcloud
    }

    /// A lender in a new folder lending instance offload-1a2b, whose line
    /// carries `HOST`, from a cloud whose boot disk was made from `image`.
    fn lender(image: &str) -> (tempfile::TempDir, Settings, HostKeys) {
        let dir = tempfile::tempdir().expect("a folder");
        for folder in ["from", "into"] {
            fs::create_dir(dir.path().join(folder)).expect("a folder for the files");
        }
        let settings = settings_in(dir.path(), &cloud_in(dir.path(), image));
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
        assert!(
            said.contains("instances describe offload-1a2b --zone=us-east1-b --format=json")
                && said.contains("disks describe offload-1a2b --zone=us-east1-b")
                && said.contains("instances get-guest-attributes offload-1a2b --zone=us-east1-b")
                && said.contains("--project=a-lab")
                && said.contains(&format!(
                    "--access-token-file={}",
                    dir.path().join("from/token").display()
                )),
            "gcloud was asked otherwise: {said}"
        );
        copy_once(&settings, &mut keys);
        assert_eq!(
            fs::read_to_string(&asked)
                .expect("gcloud's log")
                .lines()
                .count(),
            3,
            "the cloud was asked again before it was due"
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
        fs::remove_file(dir.path().join("instance.json")).expect("the instance gone");
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

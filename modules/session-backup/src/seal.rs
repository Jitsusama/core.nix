//! Packing and sealing a log: zstd, then age to every recipient; and
//! opening a sealed copy again with a key this machine holds.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use age::secrecy::SecretString;

/// zstd's level: about 4.4 times smaller on a pi log, quickly enough that
/// a year of logs packs in minutes.
const LEVEL: i32 = 9;

/// The recipients, ready to seal to.
pub struct Recipients {
    native: Vec<age::x25519::Recipient>,
    plugins: Vec<age::plugin::RecipientPluginV1<Quiet>>,
}

impl Recipients {
    /// Each recipient by its string: an `age1` key age seals to itself, or
    /// a plugin's, such as a TPM's `age1tag1` or a YubiKey's
    /// `age1fido2-hmac1`, sealed to through the plugin on PATH.
    pub fn parse(all: &[String]) -> Result<Recipients, String> {
        let mut native = Vec::new();
        let mut by_plugin: BTreeMap<String, Vec<age::plugin::Recipient>> = BTreeMap::new();
        for each in all {
            if let Ok(key) = each.parse::<age::x25519::Recipient>() {
                native.push(key);
                continue;
            }
            let plugin = each
                .parse::<age::plugin::Recipient>()
                .map_err(|why| format!("{each} isn't an age recipient: {why}"))?;
            by_plugin
                .entry(plugin.plugin().to_owned())
                .or_default()
                .push(plugin);
        }
        let plugins = by_plugin
            .iter()
            .map(|(name, recipients)| {
                age::plugin::RecipientPluginV1::new(name, recipients, &[], Quiet)
                    .map_err(|why| format!("can't seal through age-plugin-{name}: {why}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Recipients { native, plugins })
    }

    fn all(&self) -> impl Iterator<Item = &dyn age::Recipient> {
        self.native
            .iter()
            .map(|key| key as &dyn age::Recipient)
            .chain(
                self.plugins
                    .iter()
                    .map(|plugin| plugin as &dyn age::Recipient),
            )
    }
}

/// Pack the log at `from` and seal it into `to`; answers how long the
/// sealed copy is.
pub fn seal(from: &Path, to: &Path, recipients: &Recipients) -> Result<u64, String> {
    let sealed = seal_into(from, to, recipients);
    reap_plugins();
    if sealed.is_err() {
        // A half-sealed copy is scratch; the next run seals it again.
        let _ = fs::remove_file(to);
    }
    sealed
}

fn seal_into(from: &Path, to: &Path, recipients: &Recipients) -> Result<u64, String> {
    let mut log = BufReader::new(
        File::open(from).map_err(|why| format!("can't read {}: {why}", from.display()))?,
    );
    let out = File::create(to).map_err(|why| format!("can't write {}: {why}", to.display()))?;
    let encryptor = age::Encryptor::with_recipients(recipients.all())
        .map_err(|why| format!("can't seal {}: {why}", from.display()))?;
    let failed = |why: io::Error| format!("can't seal {}: {why}", from.display());
    let sealing = encryptor.wrap_output(BufWriter::new(out)).map_err(failed)?;
    let mut packing = zstd::Encoder::new(sealing, LEVEL).map_err(failed)?;
    io::copy(&mut log, &mut packing).map_err(failed)?;
    let sealing = packing.finish().map_err(failed)?;
    let mut out = sealing.finish().map_err(failed)?;
    out.flush().map_err(failed)?;
    let out = out.into_inner().map_err(|why| failed(why.into_error()))?;
    out.sync_all().map_err(failed)?;
    Ok(out.metadata().map_err(failed)?.len())
}

/// Open the sealed copy at `from` into `to`, unpacked, with the first of
/// the identity files at `identities` that this machine holds and that
/// opens it; answers how long the log is. A plugin's identity, such as a
/// TPM's, is opened through the plugin on PATH. Whatever stops it leaves
/// nothing at `to`.
pub fn open(from: &Path, to: &Path, identities: &[PathBuf]) -> Result<u64, String> {
    let opened = open_into(from, to, identities);
    reap_plugins();
    if opened.is_err() {
        // A half-opened log is no copy of anything.
        let _ = fs::remove_file(to);
    }
    opened
}

fn open_into(from: &Path, to: &Path, identities: &[PathBuf]) -> Result<u64, String> {
    let present: Vec<&PathBuf> = identities.iter().filter(|path| path.exists()).collect();
    if present.is_empty() {
        return Err(format!(
            "none of the keys the settings name is on this machine ({})",
            identities
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let mut keys = Vec::new();
    for path in present {
        let file = age::IdentityFile::from_file(path.display().to_string())
            .map_err(|why| format!("can't read the key {}: {why}", path.display()))?
            .with_callbacks(Quiet);
        keys.extend(
            file.into_identities()
                .map_err(|why| format!("can't use the key {}: {why}", path.display()))?,
        );
    }
    let sealed = BufReader::new(
        File::open(from).map_err(|why| format!("can't read {}: {why}", from.display()))?,
    );
    let refused = |why: age::DecryptError| match why {
        age::DecryptError::NoMatchingKeys => {
            format!("no key here opens {}", from.display())
        }
        why => format!("can't open {}: {why}", from.display()),
    };
    let opening = age::Decryptor::new_buffered(sealed)
        .map_err(refused)?
        .decrypt(keys.iter().map(|key| key.as_ref() as &dyn age::Identity))
        .map_err(refused)?;
    let failed = |why: io::Error| format!("can't open {}: {why}", from.display());
    let mut unpacking = zstd::Decoder::new(opening).map_err(failed)?;
    let mut out = BufWriter::new(
        File::create(to).map_err(|why| format!("can't write {}: {why}", to.display()))?,
    );
    let bytes = io::copy(&mut unpacking, &mut out).map_err(failed)?;
    let out = out.into_inner().map_err(|why| failed(why.into_error()))?;
    out.sync_all().map_err(failed)?;
    Ok(bytes)
}

/// Wait on the plugins age started and left unwaited, so a run sealing a
/// thousand logs leaves no thousand ended processes behind. This program
/// starts no other children.
fn reap_plugins() {
    loop {
        // SAFETY: waitpid writes nothing through a null status pointer.
        let reaped = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
        if reaped <= 0 {
            break;
        }
    }
}

/// Answers nothing a plugin asks: sealing asks for nothing, nor does
/// opening with a key made to need no PIN, and a plugin that asks stops
/// rather than waiting on a prompt nobody sees.
#[derive(Clone, Copy)]
pub struct Quiet;

impl age::Callbacks for Quiet {
    fn display_message(&self, _: &str) {}
    fn confirm(&self, _: &str, _: &str, _: Option<&str>) -> Option<bool> {
        None
    }
    fn request_public_string(&self, _: &str) -> Option<String> {
        None
    }
    fn request_passphrase(&self, _: &str) -> Option<SecretString> {
        None
    }
}

//! Network passwords.
//!
//! A network's Secret names the file its password is in. The launcher keeps
//! each one it's given sealed under wifid's vault, in a file ending in
//! .sealed, which opens only on this machine (vault.rs says how). A network
//! Nix declares can name a file systemd's credential service sealed, ending
//! in .cred, or any file its secrets manager writes, which is read as the
//! password itself.
//!
//! Opening a .cred takes the TPM about two seconds, so wifid opens each
//! once and keeps it sealed under the vault in memory after, until its file
//! changes. Passwords the launcher kept that way before the vault are moved
//! into it as wifid starts.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::OnceCell;
use zeroize::{Zeroize, Zeroizing};

use crate::network::Network;
use crate::store::{Failure, Store};
use crate::vault::Vault;

/// systemd's credential service, which seals and opens secrets.
pub struct Credentials {
    socket: PathBuf,
}

impl Credentials {
    /// The service every systemd machine runs.
    pub fn system() -> Credentials {
        Credentials::at(Path::new("/run/systemd/io.systemd.Credentials"))
    }

    pub fn at(socket: &Path) -> Credentials {
        Credentials {
            socket: socket.to_path_buf(),
        }
    }

    /// Seals a secret for this machine under a name, and hands back the
    /// blob to keep in a .cred file of that name. systemd uses the TPM
    /// where there is one, and the install's own key.
    pub async fn seal(&self, name: &str, secret: &str) -> io::Result<String> {
        let parameters = json!({"name": name, "text": secret, "scope": "system"});
        let reply = self
            .call("io.systemd.Credentials.Encrypt", parameters)
            .await?;
        let blob = reply["blob"]
            .as_str()
            .ok_or_else(|| invalid("no blob came back".into()))?;
        Ok(blob.to_string())
    }

    /// Opens a sealed secret, which systemd checks was sealed under this
    /// name, so one network's file can't stand in for another's.
    pub async fn unseal(&self, name: &str, blob: &str) -> io::Result<Zeroizing<String>> {
        let parameters = json!({"name": name, "blob": blob});
        let mut reply = self
            .call("io.systemd.Credentials.Decrypt", parameters)
            .await?;
        let data = match reply["data"].take() {
            Value::String(data) => Zeroizing::new(data),
            _ => Zeroizing::default(),
        };
        let plain = STANDARD.decode(data.as_bytes());
        let plain = Zeroizing::new(plain.map_err(|e| invalid(e.to_string()))?);
        let plain = std::str::from_utf8(&plain).map_err(|e| invalid(e.to_string()))?;
        Ok(Zeroizing::new(plain.to_string()))
    }

    /// Asks the service one thing over varlink, whose messages are JSON
    /// objects each ended by a zero byte. What went each way is zeroed once
    /// it's read, since it may carry a secret, and is given room enough at
    /// the start that it's never copied as it grows.
    async fn call(&self, method: &str, parameters: Value) -> io::Result<Value> {
        let mut stream = BufReader::new(UnixStream::connect(&self.socket).await?);
        let mut call = json!({"method": method, "parameters": parameters});
        let mut sent = Zeroizing::new(Vec::with_capacity(ROOM));
        serde_json::to_writer(&mut *sent, &call)?;
        scrub(&mut call);
        sent.push(0);
        stream.get_mut().write_all(&sent).await?;
        let mut reply = Zeroizing::new(Vec::with_capacity(ROOM));
        stream.read_until(0, &mut reply).await?;
        reply.pop();
        let mut reply: Value = serde_json::from_slice(&reply)?;
        match reply["error"].as_str() {
            Some(error) => {
                let reason = format!("systemd's credential service refused it: {error}");
                Err(io::Error::new(io::ErrorKind::PermissionDenied, reason))
            }
            None => Ok(reply["parameters"].take()),
        }
    }
}

/// Enough for any message carrying a password, which is at most 63
/// characters, or a blob sealing one.
const ROOM: usize = 4096;

/// Where every password wifid hands wpa_supplicant is opened.
pub struct Secrets {
    credentials: Credentials,
    /// Where the vault's key is kept, sealed.
    vault_file: PathBuf,
    /// The vault, or why it won't open, once it's been opened.
    vault: OnceCell<Result<Vault, String>>,
    /// Passwords systemd opened, sealed again under the vault's key.
    opened: Mutex<HashMap<Opened, String>>,
}

/// A file systemd sealed, as it was when it was opened, so one that's
/// changed since is opened again.
#[derive(Hash, PartialEq, Eq)]
struct Opened {
    file: PathBuf,
    modified: Option<SystemTime>,
    length: u64,
}

/// What moving the launcher's older passwords into the vault came to.
#[derive(Debug, Default, PartialEq)]
pub struct Moved {
    pub moved: usize,
    /// The networks whose passwords stayed where they were, and why.
    pub failed: Vec<(String, String)>,
}

impl Moved {
    /// Says what moved in the journal, and what didn't.
    fn say(&self) {
        match self.moved {
            0 => {}
            1 => eprintln!("moved 1 saved password into wifid's vault"),
            n => eprintln!("moved {n} saved passwords into wifid's vault"),
        }
        for (name, why) in &self.failed {
            eprintln!("<4>{name}: can't move its password into wifid's vault: {why}");
        }
    }
}

impl Secrets {
    pub fn new(credentials: Credentials, vault_file: &Path) -> Secrets {
        Secrets {
            credentials,
            vault_file: vault_file.to_path_buf(),
            vault: OnceCell::new(),
            opened: Mutex::default(),
        }
    }

    /// Opens the vault, and moves the launcher's older passwords into it,
    /// the first time it's asked. Later asks wait for that, or find it
    /// done.
    pub async fn ready(&self, store: &Store) {
        let opening = || async {
            let vault = Vault::kept_in(&self.vault_file, &self.credentials).await;
            match &vault {
                Ok(vault) => match self.move_into(vault, store).await {
                    Ok(moved) => moved.say(),
                    Err(error) => {
                        eprintln!("<4>can't move saved passwords into wifid's vault: {error}");
                    }
                },
                Err(error) => {
                    let error = format!("can't open wifid's key: {error}");
                    eprintln!("<3>{error}, so no password the launcher keeps opens");
                }
            }
            vault.map_err(|error| error.to_string())
        };
        self.vault.get_or_init(opening).await;
    }

    /// The network's password, or none when it names no Secret. A sealed
    /// one was sealed under its file's name; a plain file's last newline is
    /// the file's, not the password's.
    pub async fn password(&self, network: &Network) -> io::Result<Option<Zeroizing<String>>> {
        let Some(file) = &network.secret else {
            return Ok(None);
        };
        // Read before the file, so a change between the two is opened again.
        let metadata = fs::metadata(file)?;
        let mut text = Zeroizing::new(fs::read_to_string(file)?);
        if text.ends_with('\n') {
            text.pop();
        }
        let name = file.file_stem().unwrap_or_default().to_string_lossy();
        match file.extension().and_then(|extension| extension.to_str()) {
            Some("sealed") => self.vault()?.open(&name, &text).map(Some),
            Some("cred") => {
                let opened = Opened {
                    file: file.clone(),
                    modified: metadata.modified().ok(),
                    length: metadata.len(),
                };
                self.opened_by_systemd(opened, &name, &text).await.map(Some)
            }
            _ => Ok(Some(text)),
        }
    }

    /// The password sealed under the vault's key and this name, as text
    /// for a .sealed file of that name.
    pub fn seal(&self, name: &str, password: &str) -> io::Result<String> {
        self.vault()?.seal(name, password)
    }

    /// Moves every launcher password systemd sealed into the vault. Each
    /// old file is taken away only once its network names the new one, and
    /// both are on the disk, so a power cut part-way loses nothing.
    async fn move_into(&self, vault: &Vault, store: &Store) -> io::Result<Moved> {
        let mut moved = Moved::default();
        for saved in store.networks()?.saved {
            let Some(old) = saved.network.secret.clone() else {
                continue;
            };
            let launchers = !saved.declared && old.parent() == Some(&store.saved);
            if !launchers || old.extension().is_none_or(|extension| extension != "cred") {
                continue;
            }
            let name = saved.network.name.clone();
            match self.move_one(vault, store, saved.network, &old).await {
                Ok(()) => moved.moved += 1,
                Err(why) => moved.failed.push((name, why.to_string())),
            }
        }
        Ok(moved)
    }

    async fn move_one(
        &self,
        vault: &Vault,
        store: &Store,
        mut network: Network,
        old: &Path,
    ) -> io::Result<()> {
        let name = old.file_stem().unwrap_or_default().to_string_lossy();
        let blob = Zeroizing::new(fs::read_to_string(old)?);
        let password = self.credentials.unseal(&name, blob.trim_end()).await?;
        let new = old.with_extension("sealed");
        store.keep_password(&new, &vault.seal(&name, &password)?)?;
        network.secret = Some(new);
        store.save(&network).map_err(|failure| match failure {
            Failure::Io(error) => error,
            other => io::Error::other(format!("{other:?}")),
        })?;
        store.let_go_of_password(old)
    }

    /// A password systemd sealed, opened through it the first time and
    /// from the vault after.
    async fn opened_by_systemd(
        &self,
        opened: Opened,
        name: &str,
        blob: &str,
    ) -> io::Result<Zeroizing<String>> {
        let Some(Ok(vault)) = self.vault.get() else {
            return self.credentials.unseal(name, blob).await;
        };
        let held_as = opened.file.to_string_lossy().into_owned();
        let held = self.held().get(&opened).cloned();
        if let Some(held) = held {
            return vault.open(&held_as, &held);
        }
        let password = self.credentials.unseal(name, blob).await?;
        let held = vault.seal(&held_as, &password)?;
        let mut all = self.held();
        all.retain(|before, _| before.file != opened.file);
        all.insert(opened, held);
        Ok(password)
    }

    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<Opened, String>> {
        // What's held is whole even if a holder panicked: each is put in
        // with one insert.
        self.opened.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn vault(&self) -> io::Result<&Vault> {
        match self.vault.get() {
            Some(Ok(vault)) => Ok(vault),
            Some(Err(why)) => Err(io::Error::other(format!("wifid's key won't open: {why}"))),
            None => Err(io::Error::other("wifid's key isn't open yet")),
        }
    }

    /// Secrets with a vault already opened, or that wouldn't open.
    #[cfg(test)]
    fn with_vault(credentials: Credentials, vault: io::Result<Vault>) -> Secrets {
        let vault = vault.map_err(|error| error.to_string());
        Secrets {
            vault: OnceCell::new_with(Some(vault)),
            ..Secrets::new(credentials, Path::new("nowhere"))
        }
    }
}

/// Zeroes every string in a message.
fn scrub(value: &mut Value) {
    match value {
        Value::String(text) => text.zeroize(),
        Value::Array(values) => values.iter_mut().for_each(scrub),
        Value::Object(fields) => fields.values_mut().for_each(scrub),
        _ => {}
    }
}

fn invalid(reason: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::net::UnixListener;

    use super::*;
    use crate::network::{Address, Security};

    fn lantern(secret: &Path) -> Network {
        Network {
            name: "Lantern".to_string(),
            security: Security::Personal,
            secret: Some(secret.to_path_buf()),
            priority: 0,
            hidden: false,
            address: Address::Network,
            trusted: false,
            account: None,
        }
    }

    fn secrets(socket: &Path) -> Secrets {
        Secrets::with_vault(Credentials::at(socket), Vault::with_key([7; 32]))
    }

    /// A store in this folder, with nothing saved yet.
    fn store(root: &Path) -> Store {
        let store = Store {
            declared: root.join("etc"),
            saved: root.join("var"),
            runtime: root.join("run"),
            defaults: root.join("etc/90-wifi.network"),
        };
        for directory in [&store.declared, &store.saved, &store.runtime] {
            fs::create_dir(directory).unwrap();
        }
        fs::write(&store.defaults, "[Network]\nDHCP=yes\n").unwrap();
        store
    }

    /// A stand-in for systemd's service that answers one call, and hands
    /// back the call it was asked.
    fn service(socket: &Path, reply: Value) -> tokio::task::JoinHandle<Value> {
        let asked = services(socket, vec![reply]);
        tokio::spawn(async move { asked.await.unwrap().remove(0) })
    }

    /// A stand-in that answers a call with each of these replies in turn,
    /// and hands back the calls it was asked.
    fn services(socket: &Path, replies: Vec<Value>) -> tokio::task::JoinHandle<Vec<Value>> {
        let listener = UnixListener::bind(socket).unwrap();
        tokio::spawn(async move {
            let mut calls = Vec::new();
            for reply in replies {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = BufReader::new(stream);
                let mut call = Vec::new();
                stream.read_until(0, &mut call).await.unwrap();
                call.pop();
                let mut answer = serde_json::to_vec(&reply).unwrap();
                answer.push(0);
                stream.get_mut().write_all(&answer).await.unwrap();
                calls.push(serde_json::from_slice(&call).unwrap());
            }
            calls
        })
    }

    /// The call the stand-in was asked, or none when nothing asked it
    /// within a few seconds.
    async fn call_to(service: tokio::task::JoinHandle<Value>) -> Value {
        let waited = tokio::time::timeout(Duration::from_secs(3), service).await;
        waited.map(Result::unwrap).unwrap_or(Value::Null)
    }

    fn opened(password: io::Result<Option<Zeroizing<String>>>) -> Option<String> {
        password.unwrap().map(|password| password.to_string())
    }

    #[tokio::test]
    async fn a_password_the_launcher_kept_opens_from_the_vault_alone() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = secrets(&dir.path().join("nowhere"));
        let sealed = dir.path().join("lantern.sealed");
        let kept = secrets.seal("lantern", "glowing in the dark").unwrap();
        fs::write(&sealed, kept + "\n").unwrap();

        let password = secrets.password(&lantern(&sealed)).await;

        assert_eq!(opened(password).as_deref(), Some("glowing in the dark"));
    }

    #[tokio::test]
    async fn a_password_systemd_sealed_is_opened_by_it_under_its_file_s_name() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let sealed = dir.path().join("lantern.cred");
        fs::write(&sealed, "c2VhbGVk\n").unwrap();
        let asked = service(
            &socket,
            json!({"parameters": {"data": "Z2xvd2luZyBpbiB0aGUgZGFyaw=="}}),
        );

        let password = secrets(&socket).password(&lantern(&sealed)).await;

        assert_eq!(opened(password).as_deref(), Some("glowing in the dark"));
        assert_eq!(
            call_to(asked).await,
            json!({
                "method": "io.systemd.Credentials.Decrypt",
                "parameters": {"name": "lantern", "blob": "c2VhbGVk"},
            })
        );
    }

    #[tokio::test]
    async fn a_password_systemd_sealed_is_opened_through_it_only_once() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let sealed = dir.path().join("lantern.cred");
        fs::write(&sealed, "c2VhbGVk").unwrap();
        let asked = service(
            &socket,
            json!({"parameters": {"data": "Z2xvd2luZyBpbiB0aGUgZGFyaw=="}}),
        );
        let secrets = secrets(&socket);
        let network = lantern(&sealed);
        let first = secrets.password(&network).await;
        call_to(asked).await;

        let again = secrets.password(&network).await;

        assert_eq!(opened(first).as_deref(), Some("glowing in the dark"));
        assert_eq!(opened(again).as_deref(), Some("glowing in the dark"));
    }

    #[tokio::test]
    async fn a_password_systemd_sealed_is_opened_again_once_its_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let sealed = dir.path().join("lantern.cred");
        fs::write(&sealed, "c2VhbGVk").unwrap();
        let secrets = secrets(&socket);
        let network = lantern(&sealed);
        let asked = service(&socket, json!({"parameters": {"data": "YmVmb3Jl"}}));
        let before = secrets.password(&network).await;
        call_to(asked).await;
        fs::remove_file(&socket).unwrap();
        fs::write(&sealed, "c2VhbGVkIGFnYWlu").unwrap();
        let asked = service(&socket, json!({"parameters": {"data": "YWZ0ZXI="}}));

        let after = secrets.password(&network).await;

        call_to(asked).await;
        assert_eq!(opened(before).as_deref(), Some("before"));
        assert_eq!(opened(after).as_deref(), Some("after"));
    }

    #[tokio::test]
    async fn a_password_in_any_other_file_is_read_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("lantern");
        fs::write(&plain, "glowing in the dark \n").unwrap();

        let password = secrets(&dir.path().join("nowhere"))
            .password(&lantern(&plain))
            .await;

        assert_eq!(opened(password).as_deref(), Some("glowing in the dark "));
    }

    #[tokio::test]
    async fn a_network_without_a_secret_has_no_password() {
        let dir = tempfile::tempdir().unwrap();
        let network = Network {
            secret: None,
            ..lantern(dir.path())
        };

        let password = secrets(&dir.path().join("nowhere"))
            .password(&network)
            .await;

        assert_eq!(opened(password), None);
    }

    #[tokio::test]
    async fn a_secret_is_sealed_for_the_system_under_a_name() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let asked = service(&socket, json!({"parameters": {"blob": "c2VhbGVk"}}));

        let blob = Credentials::at(&socket)
            .seal("lantern", "glowing in the dark")
            .await;

        assert_eq!(blob.unwrap(), "c2VhbGVk");
        assert_eq!(
            call_to(asked).await,
            json!({
                "method": "io.systemd.Credentials.Encrypt",
                "parameters": {"name": "lantern", "text": "glowing in the dark", "scope": "system"},
            })
        );
    }

    #[tokio::test]
    async fn a_launcher_password_moves_into_the_vault() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let store = store(dir.path());
        let old = store.saved.join("lantern.cred");
        fs::write(&old, "c2VhbGVk").unwrap();
        store.save(&lantern(&old)).unwrap();
        let secrets = secrets(&socket);
        let asked = service(
            &socket,
            json!({"parameters": {"data": "Z2xvd2luZyBpbiB0aGUgZGFyaw=="}}),
        );

        let moved = secrets.move_into(secrets.vault().unwrap(), &store).await;
        let moved = moved.unwrap();

        call_to(asked).await;
        assert_eq!(
            moved,
            Moved {
                moved: 1,
                failed: vec![]
            }
        );
        assert!(!old.exists());
        let network = store.networks().unwrap().saved.remove(0).network;
        assert_eq!(network.secret, Some(store.saved.join("lantern.sealed")));
        let password = secrets.password(&network).await;
        assert_eq!(opened(password).as_deref(), Some("glowing in the dark"));
    }

    #[tokio::test]
    async fn a_password_nix_declares_stays_where_nix_put_it() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        let sealed = store.declared.join("lantern.cred");
        fs::write(&sealed, "c2VhbGVk").unwrap();
        let mut file = crate::syntax::Document::default();
        lantern(&sealed).write_into(&mut file);
        fs::write(store.declared.join("50-lantern.network"), file.render()).unwrap();

        let secrets = secrets(&dir.path().join("nowhere"));
        let moved = secrets.move_into(secrets.vault().unwrap(), &store).await;

        assert_eq!(moved.unwrap(), Moved::default());
        assert!(sealed.exists());
    }

    #[tokio::test]
    async fn a_password_systemd_wont_open_stays_where_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let store = store(dir.path());
        let old = store.saved.join("lantern.cred");
        fs::write(&old, "c2VhbGVk").unwrap();
        store.save(&lantern(&old)).unwrap();
        let asked = service(
            &socket,
            json!({"error": "io.systemd.Credentials.NameMismatch"}),
        );

        let secrets = secrets(&socket);
        let moved = secrets.move_into(secrets.vault().unwrap(), &store).await;
        let moved = moved.unwrap();

        call_to(asked).await;
        assert_eq!(moved.moved, 0);
        assert_eq!(moved.failed.len(), 1);
        assert_eq!(moved.failed[0].0, "Lantern");
        assert!(moved.failed[0].1.contains("NameMismatch"), "{moved:?}");
        assert_eq!(fs::read_to_string(&old).unwrap(), "c2VhbGVk");
        let network = store.networks().unwrap().saved.remove(0).network;
        assert_eq!(network.secret, Some(old));
    }

    #[tokio::test]
    async fn a_password_systemd_wont_open_is_refused_with_its_reason() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let sealed = dir.path().join("lantern.cred");
        fs::write(&sealed, "c2VhbGVk").unwrap();
        let asked = service(
            &socket,
            json!({"error": "io.systemd.Credentials.NameMismatch"}),
        );

        let refused = secrets(&socket).password(&lantern(&sealed)).await;

        call_to(asked).await;
        let error = refused.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("NameMismatch"), "{error}");
    }

    #[tokio::test]
    async fn the_vault_is_opened_once_however_often_it_is_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let store = store(dir.path());
        let secrets = Secrets::new(Credentials::at(&socket), &dir.path().join("vault.cred"));
        let asked = service(&socket, json!({"parameters": {"blob": "c2VhbGVk"}}));

        secrets.ready(&store).await;
        secrets.ready(&store).await;

        assert_eq!(
            call_to(asked).await["method"],
            "io.systemd.Credentials.Encrypt"
        );
        assert!(secrets.seal("lantern", "glowing in the dark").is_ok());
    }

    #[tokio::test]
    async fn the_vault_moves_the_launcher_s_older_passwords_in_as_it_opens() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let store = store(dir.path());
        let old = store.saved.join("lantern.cred");
        fs::write(&old, "c2VhbGVk").unwrap();
        store.save(&lantern(&old)).unwrap();
        let secrets = Secrets::new(Credentials::at(&socket), &dir.path().join("vault.cred"));
        let made = json!({"parameters": {"blob": "c2VhbGVk"}});
        let unsealed = json!({"parameters": {"data": "Z2xvd2luZyBpbiB0aGUgZGFyaw=="}});
        let asked = services(&socket, vec![made, unsealed]);

        secrets.ready(&store).await;

        let asked = tokio::time::timeout(Duration::from_secs(3), asked).await;
        let asked = asked.unwrap().unwrap();
        assert_eq!(asked[1]["method"], "io.systemd.Credentials.Decrypt");
        assert!(!old.exists());
        let network = store.networks().unwrap().saved.remove(0).network;
        let password = secrets.password(&network).await;
        assert_eq!(opened(password).as_deref(), Some("glowing in the dark"));
    }

    #[tokio::test]
    async fn nothing_opens_from_the_vault_before_it_is_ready() {
        let dir = tempfile::tempdir().unwrap();
        let sealed = dir.path().join("lantern.sealed");
        fs::write(&sealed, "c2VhbGVk").unwrap();
        let secrets = Secrets::new(Credentials::at(&dir.path().join("nowhere")), &sealed);

        let refused = secrets.password(&lantern(&sealed)).await;

        assert!(refused.is_err(), "{:?}", refused.map(|_| ()));
    }

    #[tokio::test]
    async fn nothing_is_sealed_while_the_vault_wont_open() {
        let dir = tempfile::tempdir().unwrap();
        let broken = io::Error::other("the TPM said no");
        let nowhere = Credentials::at(&dir.path().join("nowhere"));
        let secrets = Secrets::with_vault(nowhere, Err(broken));

        let refused = secrets.seal("lantern", "glowing in the dark");

        let error = refused.unwrap_err();
        assert!(error.to_string().contains("the TPM said no"), "{error}");
    }
}

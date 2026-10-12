//! Network passwords.
//!
//! A network's Secret names the file its password is in. The launcher keeps
//! each one it's given sealed, in a file ending in .cred, which systemd's
//! credential service opens only on this machine: with the TPM where there
//! is one, and with the install's own key. A network Nix declares can name
//! a sealed file too, or any file its secrets manager writes, which is read
//! as the password itself.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::network::Network;

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

    /// Seals a password for this machine under a name, and hands back the
    /// blob to keep in a .cred file of that name. systemd uses the TPM
    /// where there is one, and the install's own key.
    pub async fn seal(&self, name: &str, password: &str) -> io::Result<String> {
        let parameters = json!({"name": name, "text": password, "scope": "system"});
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
    pub async fn unseal(&self, name: &str, blob: &str) -> io::Result<String> {
        let parameters = json!({"name": name, "blob": blob});
        let reply = self
            .call("io.systemd.Credentials.Decrypt", parameters)
            .await?;
        let data = reply["data"].as_str().unwrap_or_default();
        let plain = STANDARD.decode(data).map_err(|e| invalid(e.to_string()))?;
        String::from_utf8(plain).map_err(|e| invalid(e.to_string()))
    }

    /// Asks the service one thing over varlink, whose messages are JSON
    /// objects each ended by a zero byte.
    async fn call(&self, method: &str, parameters: Value) -> io::Result<Value> {
        let mut stream = BufReader::new(UnixStream::connect(&self.socket).await?);
        let mut call = serde_json::to_vec(&json!({"method": method, "parameters": parameters}))?;
        call.push(0);
        stream.get_mut().write_all(&call).await?;
        let mut reply = Vec::new();
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

/// The network's password, or none when it names no Secret. A sealed one
/// was sealed under its file's name; a plain file's last newline is the
/// file's, not the password's.
pub async fn password(network: &Network, credentials: &Credentials) -> io::Result<Option<String>> {
    let Some(file) = &network.secret else {
        return Ok(None);
    };
    let mut text = fs::read_to_string(file)?;
    if text.ends_with('\n') {
        text.pop();
    }
    if file
        .extension()
        .is_some_and(|extension| extension == "cred")
    {
        let name = file.file_stem().unwrap_or_default().to_string_lossy();
        return credentials.unseal(&name, &text).await.map(Some);
    }
    Ok(Some(text))
}

fn invalid(reason: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

#[cfg(test)]
mod tests {
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
        }
    }

    /// A stand-in for systemd's service that answers one call, and hands
    /// back the call it was asked.
    fn service(socket: &Path, reply: Value) -> tokio::task::JoinHandle<Value> {
        let listener = UnixListener::bind(socket).unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut call = Vec::new();
            stream.read_until(0, &mut call).await.unwrap();
            call.pop();
            let mut answer = serde_json::to_vec(&reply).unwrap();
            answer.push(0);
            stream.get_mut().write_all(&answer).await.unwrap();
            serde_json::from_slice(&call).unwrap()
        })
    }

    #[tokio::test]
    async fn a_sealed_password_is_opened_by_systemd_under_its_file_s_name() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let sealed = dir.path().join("lantern.cred");
        fs::write(&sealed, "c2VhbGVk\n").unwrap();
        let asked = service(
            &socket,
            json!({"parameters": {"data": "Z2xvd2luZyBpbiB0aGUgZGFyaw=="}}),
        );

        let password = password(&lantern(&sealed), &Credentials::at(&socket))
            .await
            .unwrap();

        assert_eq!(password.as_deref(), Some("glowing in the dark"));
        assert_eq!(
            asked.await.unwrap(),
            json!({
                "method": "io.systemd.Credentials.Decrypt",
                "parameters": {"name": "lantern", "blob": "c2VhbGVk"},
            })
        );
    }

    #[tokio::test]
    async fn a_password_is_sealed_for_the_system_under_a_name() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("credentials");
        let asked = service(&socket, json!({"parameters": {"blob": "c2VhbGVk"}}));

        let blob = Credentials::at(&socket)
            .seal("lantern", "glowing in the dark")
            .await;

        assert_eq!(blob.unwrap(), "c2VhbGVk");
        assert_eq!(
            asked.await.unwrap(),
            json!({
                "method": "io.systemd.Credentials.Encrypt",
                "parameters": {"name": "lantern", "text": "glowing in the dark", "scope": "system"},
            })
        );
    }

    #[tokio::test]
    async fn a_password_in_any_other_file_is_read_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("lantern");
        fs::write(&plain, "glowing in the dark \n").unwrap();
        let nowhere = Credentials::at(&dir.path().join("credentials"));

        let password = password(&lantern(&plain), &nowhere).await.unwrap();

        assert_eq!(password.as_deref(), Some("glowing in the dark "));
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

        let refused = password(&lantern(&sealed), &Credentials::at(&socket)).await;

        asked.await.unwrap();
        let error = refused.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("NameMismatch"), "{error}");
    }
}

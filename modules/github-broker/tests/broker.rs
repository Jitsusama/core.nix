//! The broker end to end, through its socket, against a stand-in for
//! GitHub that records what it's asked and answers as it's told.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use github_broker::app::App;
use github_broker::mint::Minter;
use github_broker::wire;
use rsa::RsaPrivateKey;
use rsa::pkcs1::{EncodeRsaPrivateKey, LineEnding};
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::sha2::Sha256;
use rsa::signature::Verifier;

/// What the stand-in was asked: the path, the bearer token and the body.
type Asked = Arc<Mutex<Vec<(String, String, String)>>>;

/// A stand-in for GitHub's API answering every request with `status` and
/// `body`, and the address it listens at.
fn github(status: &'static str, body: &'static str) -> (String, Asked) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let api = format!("http://{}", listener.local_addr().unwrap());
    let asked: Asked = Arc::default();
    let record = asked.clone();
    std::thread::spawn(move || {
        for caller in listener.incoming().flatten() {
            let mut reader = BufReader::new(&caller);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let path = line.split(' ').nth(1).unwrap_or_default().to_owned();
            let (mut length, mut bearer) = (0, String::new());
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                let (name, value) = header.split_once(": ").unwrap_or_default();
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => length = value.parse().unwrap(),
                    "authorization" => bearer = value.trim_start_matches("Bearer ").to_owned(),
                    _ => {}
                }
            }
            let mut sent = vec![0; length];
            reader.read_exact(&mut sent).unwrap();
            record
                .lock()
                .unwrap()
                .push((path, bearer, String::from_utf8(sent).unwrap()));
            write!(
                &caller,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    (api, asked)
}

const MINTED: &str = r#"{"token":"ghs_standin","expires_at":"2026-11-08T17:00:00Z"}"#;

/// A broker for an App with a key of its own, listening in a folder of its
/// own; the key, so a test can check what was signed with it, and the
/// socket to ask at.
fn broker(
    minter: impl FnOnce(Box<dyn FnMut() -> Result<App, String> + Send>) -> Minter,
) -> (RsaPrivateKey, PathBuf, tempfile::TempDir) {
    let key = RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
    let pem = key.to_pkcs1_pem(LineEnding::LF).unwrap().to_string();
    let mut minter = minter(Box::new(move || App::from_parts("42", &pem, "7")));
    let folder = tempfile::tempdir().unwrap();
    let socket = folder.path().join("github-broker.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    std::thread::spawn(move || wire::serve(&listener, &mut minter));
    (key, socket, folder)
}

#[test]
fn mints_a_narrow_token_signed_with_the_apps_key() {
    let (api, asked) = github("201 Created", MINTED);
    let (key, socket, _folder) = broker(|open| Minter::new(&api, open));
    let token = wire::ask(&socket).unwrap();
    assert_eq!(token.value, "ghs_standin");
    assert_eq!(token.expires_at, "2026-11-08T17:00:00Z");

    let asked = asked.lock().unwrap();
    let (path, jwt, body) = &asked[0];
    assert_eq!(path, "/app/installations/7/access_tokens");
    let body: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        body["permissions"],
        serde_json::json!({
            "metadata": "read", "contents": "read",
            "pull_requests": "write", "issues": "write"
        }),
        "a token may never write code, so it can never push or merge"
    );
    let (signed, signature) = jwt.rsplit_once('.').unwrap();
    let signature =
        Signature::try_from(URL_SAFE_NO_PAD.decode(signature).unwrap().as_slice()).unwrap();
    VerifyingKey::<Sha256>::new(key.to_public_key())
        .verify(signed.as_bytes(), &signature)
        .expect("GitHub is asked with a token the App's key signed");
}

#[test]
fn asks_github_once_while_a_token_lasts() {
    let (api, asked) = github("201 Created", MINTED);
    let (_key, socket, _folder) = broker(|open| Minter::new(&api, open));
    wire::ask(&socket).unwrap();
    wire::ask(&socket).unwrap();
    assert_eq!(asked.lock().unwrap().len(), 1);
}

#[test]
fn asks_again_once_a_token_is_nearly_spent() {
    let (api, asked) = github("201 Created", MINTED);
    let (_key, socket, _folder) = broker(|open| Minter::new(&api, open).keeping(Duration::ZERO));
    wire::ask(&socket).unwrap();
    wire::ask(&socket).unwrap();
    assert_eq!(asked.lock().unwrap().len(), 2);
}

#[test]
fn says_why_github_refused() {
    let (api, _asked) = github("401 Unauthorized", r#"{"message":"Bad credentials"}"#);
    let (_key, socket, _folder) = broker(|open| Minter::new(&api, open));
    let refused = wire::ask(&socket).unwrap_err();
    assert!(refused.contains("401"), "{refused}");
    assert!(refused.contains("Bad credentials"), "{refused}");
}

#[test]
fn answers_before_the_app_is_made_and_mints_once_it_is() {
    let (api, _asked) = github("201 Created", MINTED);
    let opened = Arc::new(AtomicUsize::new(0));
    let tries = opened.clone();
    let (_key, socket, _folder) = broker(move |mut open| {
        Minter::new(
            &api,
            Box::new(move || {
                if tries.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err("the App isn't made yet: app.json.age is missing".to_owned())
                } else {
                    open()
                }
            }),
        )
    });
    assert_eq!(
        wire::ask(&socket).unwrap_err(),
        "the App isn't made yet: app.json.age is missing"
    );
    assert_eq!(wire::ask(&socket).unwrap().value, "ghs_standin");
    assert_eq!(opened.load(Ordering::SeqCst), 2);
}

#[test]
fn refuses_anything_but_a_token() {
    let (api, _asked) = github("201 Created", MINTED);
    let (_key, socket, _folder) = broker(|open| Minter::new(&api, open));
    let caller = std::os::unix::net::UnixStream::connect(&socket).unwrap();
    (&caller).write_all(b"key\n").unwrap();
    let mut said = String::new();
    BufReader::new(&caller).read_line(&mut said).unwrap();
    assert_eq!(said, "no \"key\" isn't something to ask; ask `token`\n");
}

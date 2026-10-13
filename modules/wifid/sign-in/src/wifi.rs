//! What the window asks wifid, over varlink on wifid's socket: what wants a
//! sign-in, where a name is on the card's network, and to check again.
//! Each message is a JSON object ended by a zero byte. The window asks off
//! GTK's thread, so a plain blocking connection serves.

use std::io::{self, BufRead, BufReader, Write};
use std::net::IpAddr;
use std::os::unix::net::UnixStream;
use std::path::Path;

use serde_json::{Value, json};

/// Where wifid answers.
pub const SOCKET: &str = "/run/wifid/io.jitsusama.Wifi";

/// Asks wifid something once, and gives its answer's parameters. A method
/// taking none is asked with none, since wifid closes the connection on
/// empty ones.
pub fn call(socket: &Path, method: &str, parameters: Value) -> io::Result<Value> {
    let mut answers = ask(socket, method, parameters, false)?;
    let answer = answers
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "wifid didn't answer"))??;
    Ok(answer)
}

/// Every description wifid sends, the first at once and another each time
/// anything changes, until wifid stops.
pub fn follow(socket: &Path) -> io::Result<impl Iterator<Item = Value>> {
    let answers = ask(socket, "Describe", json!({}), true)?;
    Ok(answers.map_while(Result::ok))
}

/// The addresses the card's network says a name has.
pub fn lookup(socket: &Path, card: &str, name: &str) -> io::Result<Vec<IpAddr>> {
    let found = call(socket, "Lookup", json!({"card": card, "name": name}))?;
    let addresses = found["addresses"].as_array().into_iter().flatten();
    Ok(addresses
        .filter_map(|address| address.as_str()?.parse().ok())
        .collect())
}

/// Sends one call and gives each answer's parameters as it comes, or why
/// wifid refused.
fn ask(
    socket: &Path,
    method: &str,
    parameters: Value,
    more: bool,
) -> io::Result<impl Iterator<Item = io::Result<Value>>> {
    let mut stream = UnixStream::connect(socket)?;
    let mut call = json!({"method": format!("io.jitsusama.Wifi.{method}")});
    if parameters
        .as_object()
        .is_some_and(|given| !given.is_empty())
    {
        call["parameters"] = parameters;
    }
    if more {
        call["more"] = json!(true);
    }
    let mut sent = serde_json::to_vec(&call)?;
    sent.push(0);
    stream.write_all(&sent)?;
    let mut reader = BufReader::new(stream);
    Ok(std::iter::from_fn(move || {
        let mut message = Vec::new();
        match reader.read_until(0, &mut message) {
            Ok(0) => return None,
            Ok(_) => {}
            Err(error) => return Some(Err(error)),
        }
        message.pop();
        let mut answer: Value = match serde_json::from_slice(&message) {
            Ok(answer) => answer,
            Err(error) => return Some(Err(error.into())),
        };
        if let Some(error) = answer["error"].as_str() {
            let refused = match answer["parameters"]["reason"].as_str() {
                Some(reason) => format!("wifid refused: {error}: {reason}"),
                None => format!("wifid refused: {error}"),
            };
            return Some(Err(io::Error::other(refused)));
        }
        Some(Ok(answer["parameters"].take()))
    }))
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    /// A stand-in wifid that answers one connection with these replies,
    /// and gives back what it was asked.
    fn wifid(
        replies: Vec<Value>,
    ) -> (
        tempfile::TempDir,
        std::path::PathBuf,
        thread::JoinHandle<Value>,
    ) {
        let dir = tempfile::TempDir::new().unwrap();
        let socket = dir.path().join("wifi");
        let listener = UnixListener::bind(&socket).unwrap();
        // Nobody asking within a moment is a failure, not a wait forever.
        listener.set_nonblocking(true).unwrap();
        let answering = thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(_) if std::time::Instant::now() < deadline => {
                        thread::sleep(std::time::Duration::from_millis(10))
                    }
                    Err(_) => panic!("wifid wasn't asked anything"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            let mut asked = Vec::new();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            reader.read_until(0, &mut asked).unwrap();
            asked.pop();
            let mut stream = stream;
            for reply in replies {
                let mut sent = serde_json::to_vec(&reply).unwrap();
                sent.push(0);
                stream.write_all(&sent).unwrap();
            }
            serde_json::from_slice(&asked).unwrap()
        });
        (dir, socket, answering)
    }

    #[test]
    fn a_call_gives_back_what_wifid_answered() {
        let (_dir, socket, wifid) =
            wifid(vec![json!({"parameters": {"addresses": ["10.43.0.1"]}})]);

        let found = lookup(&socket, "wlan2", "front-desk.inn").unwrap();

        assert_eq!(found, ["10.43.0.1".parse::<IpAddr>().unwrap()]);
        assert_eq!(
            wifid.join().unwrap(),
            json!({"method": "io.jitsusama.Wifi.Lookup", "parameters": {"card": "wlan2", "name": "front-desk.inn"}})
        );
    }

    #[test]
    fn a_method_taking_nothing_is_asked_with_nothing() {
        let (_dir, socket, wifid) = wifid(vec![json!({})]);

        call(&socket, "Check", json!({})).unwrap();

        assert_eq!(
            wifid.join().unwrap(),
            json!({"method": "io.jitsusama.Wifi.Check"})
        );
    }

    #[test]
    fn following_gives_each_description_until_wifid_stops() {
        let first = json!({"parameters": {"cards": []}, "continues": true});
        let second = json!({"parameters": {"cards": [{"name": "wlan2"}]}, "continues": true});
        let (_dir, socket, wifid) = wifid(vec![first, second]);

        let described: Vec<Value> = follow(&socket).unwrap().collect();

        assert_eq!(
            described,
            [json!({"cards": []}), json!({"cards": [{"name": "wlan2"}]})]
        );
        assert_eq!(
            wifid.join().unwrap(),
            json!({"method": "io.jitsusama.Wifi.Describe", "more": true})
        );
    }

    #[test]
    fn a_failure_says_wifid_s_reason() {
        let failed = json!({"error": "io.jitsusama.Wifi.Failed", "parameters": {"reason": "10.42.0.1 didn't answer"}});
        let (_dir, socket, _wifid) = wifid(vec![failed]);

        let refused = lookup(&socket, "wlan2", "front-desk.inn").unwrap_err();

        assert!(
            refused.to_string().contains("10.42.0.1 didn't answer"),
            "{refused}"
        );
    }

    #[test]
    fn a_refusal_says_why() {
        let (_dir, socket, _wifid) = wifid(vec![json!({"error": "io.jitsusama.Wifi.NotJoined"})]);

        let refused = lookup(&socket, "wlan9", "front-desk.inn").unwrap_err();

        assert!(refused.to_string().contains("NotJoined"), "{refused}");
    }
}

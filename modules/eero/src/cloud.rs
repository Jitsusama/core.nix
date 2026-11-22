//! Talking to eero's cloud, which works the same at home and away: the eero
//! itself answers nothing on the LAN but DNS and the ports games ask it to
//! open.

use std::fmt;
use std::time::Duration;

use serde_json::{Value, json};
use ureq::Agent;

use crate::api;
use crate::session;

/// A conversation with eero's cloud under one session.
pub struct Cloud {
    agent: Agent,
    token: Option<String>,
}

impl Cloud {
    /// A conversation not yet signed in.
    pub fn new() -> Cloud {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .http_status_as_error(false)
            .user_agent("eero (core.nix)")
            .build()
            .into();
        Cloud { agent, token: None }
    }

    /// A conversation under the sealed session.
    pub fn signed_in() -> Result<Cloud, Failure> {
        let mut cloud = Cloud::new();
        cloud.token = Some(session::open().map_err(|e| Failure::SignedOut(e.to_string()))?);
        Ok(cloud)
    }

    /// Asks eero to send a code to an email address or phone number, and
    /// holds the token the code will make good.
    pub fn sign_in(&mut self, login: &str) -> Result<(), String> {
        let data = self.send(Method::Post, "/2.2/login", Some(&json!({"login": login})))?;
        let token = data.get("user_token").and_then(Value::as_str);
        self.token = Some(token.ok_or("eero gave no token for that login")?.to_owned());
        Ok(())
    }

    /// Makes the held token good with the code eero sent, and seals it.
    pub fn verify(&mut self, code: &str) -> Result<(), String> {
        self.send(
            Method::Post,
            "/2.2/login/verify",
            Some(&json!({"code": code})),
        )?;
        self.keep()
    }

    /// The data of an answer to a GET.
    pub fn get(&mut self, path: &str) -> Result<Value, Failure> {
        self.call(Method::Get, path, None)
    }

    /// The data of an answer to a POST.
    pub fn post(&mut self, path: &str, body: Option<&Value>) -> Result<Value, Failure> {
        self.call(Method::Post, path, body)
    }

    /// The data of an answer to a PUT, which is how a setting changes.
    pub fn put(&mut self, path: &str, body: &Value) -> Result<Value, Failure> {
        self.call(Method::Put, path, Some(body))
    }

    /// An answer to a DELETE, which is how a reservation goes. eero may
    /// answer one with no data, so its data is null then rather than a
    /// refusal (`judge`).
    pub fn delete(&mut self, path: &str) -> Result<Value, Failure> {
        self.call(Method::Delete, path, None)
    }

    /// Asks, and once more after refreshing the session if eero says it
    /// aged, as it does every so often.
    fn call(&mut self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, Failure> {
        match self.send(method, path, body) {
            Err(Refused::Aged) => {
                self.refresh()?;
                Ok(self.send(method, path, body)?)
            }
            answer => Ok(answer?),
        }
    }

    /// Trades the aged token for a fresh one and seals it. Only eero being
    /// out of reach leaves the session worth trying again; any other
    /// refusal means signing in again.
    fn refresh(&mut self) -> Result<(), Failure> {
        let data = match self.send(Method::Post, "/2.2/login/refresh", None) {
            Err(Refused::Failed(Failure::Unreachable(reason))) => {
                return Err(Failure::Unreachable(reason));
            }
            answer => answer.map_err(|_| Failure::SignedOut(AGED.into()))?,
        };
        let token = data.get("user_token").and_then(Value::as_str);
        self.token = Some(token.ok_or_else(|| Failure::SignedOut(AGED.into()))?.into());
        // The fresh token works for this conversation whether or not it's
        // sealed; one left unsealed only means the next start refreshes the
        // aged one again.
        if let Err(reason) = self.keep() {
            eprintln!("eero: couldn't seal the refreshed session: {reason}");
        }
        Ok(())
    }

    fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, Refused> {
        let url = format!("{}{path}", api::ORIGIN);
        let cookie = self.token.as_ref().map(|token| format!("s={token}"));
        let answer = match method {
            Method::Get => {
                let request = self.agent.get(&url);
                match &cookie {
                    Some(cookie) => request.header("Cookie", cookie).call(),
                    None => request.call(),
                }
            }
            Method::Delete => {
                let request = self.agent.delete(&url);
                match &cookie {
                    Some(cookie) => request.header("Cookie", cookie).call(),
                    None => request.call(),
                }
            }
            Method::Post | Method::Put => {
                let request = if matches!(method, Method::Put) {
                    self.agent.put(&url)
                } else {
                    self.agent.post(&url)
                };
                let request = match &cookie {
                    Some(cookie) => request.header("Cookie", cookie),
                    None => request,
                };
                match body {
                    Some(body) => request.send_json(body),
                    None => request.send_empty(),
                }
            }
        };
        let mut answer = answer.map_err(|e| {
            Refused::Failed(Failure::Unreachable(format!("couldn't reach eero: {e}")))
        })?;
        let status = answer.status().as_u16();
        let read: Value = answer.body_mut().read_json().unwrap_or(Value::Null);
        judge(method, path, status, &read)
    }

    fn keep(&self) -> Result<(), String> {
        let token = self.token.as_deref().ok_or("no session to keep")?;
        session::keep(token).map_err(|e| e.to_string())
    }
}

/// The data of an answer to `path`, or why there's none: an aged session
/// apart, since a refresh mends it, and eero failing on its side apart
/// from eero refusing, since only the first may answer if asked again.
/// A DELETE eero took may come back with no data, which is null then
/// rather than a refusal; only a DELETE, since a read with no data is
/// one eero couldn't answer, never an empty one.
fn judge(method: Method, path: &str, status: u16, read: &Value) -> Result<Value, Refused> {
    if matches!(method, Method::Delete) && (200..300).contains(&status) {
        return Ok(api::data(read).cloned().unwrap_or(Value::Null));
    }
    if api::session_expired(status, read) {
        return Err(Refused::Aged);
    }
    let failed = |failure| Err(Refused::Failed(failure));
    if status >= 500 {
        return failed(Failure::Unreachable(format!(
            "eero answered {status} to {path}"
        )));
    }
    if !(200..300).contains(&status) {
        return failed(Failure::Refused(format!(
            "eero answered {status} to {path}: {read}"
        )));
    }
    api::data(read)
        .cloned()
        .map_err(|reason| Refused::Failed(Failure::Refused(reason)))
}

/// What the acts ask of eero's cloud, so a test can answer in its place
/// (reservations.rs).
pub trait Asks {
    fn get(&mut self, path: &str) -> Result<Value, Failure>;
    fn post(&mut self, path: &str, body: Option<&Value>) -> Result<Value, Failure>;
    fn put(&mut self, path: &str, body: &Value) -> Result<Value, Failure>;
    fn delete(&mut self, path: &str) -> Result<Value, Failure>;
}

impl Asks for Cloud {
    fn get(&mut self, path: &str) -> Result<Value, Failure> {
        Cloud::get(self, path)
    }

    fn post(&mut self, path: &str, body: Option<&Value>) -> Result<Value, Failure> {
        Cloud::post(self, path, body)
    }

    fn put(&mut self, path: &str, body: &Value) -> Result<Value, Failure> {
        Cloud::put(self, path, body)
    }

    fn delete(&mut self, path: &str) -> Result<Value, Failure> {
        Cloud::delete(self, path)
    }
}

/// How a request asks: eero's reads are GETs, a setting changes by PUT,
/// a reservation goes by DELETE, and everything else POSTs.
#[derive(Clone, Copy, Debug)]
enum Method {
    Get,
    Post,
    Put,
    Delete,
}

/// What's said when a session has aged past refreshing.
const AGED: &str =
    "eero's session aged and couldn't be refreshed: run `eero sign-in <email or phone>`";

/// Why eero gave no data, told apart by what mends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No session eero takes: none is sealed, or it aged past refreshing.
    /// Only signing in again mends it, and the reason says how.
    SignedOut(String),
    /// eero's cloud couldn't be reached, or failed on its side; asking
    /// again later may work.
    Unreachable(String),
    /// eero answered, but refused this request or gave it no data.
    Refused(String),
    /// A change was sent, but neither eero's answer nor a read after it
    /// says whether eero took it, so whoever asked reads it again rather
    /// than trusting either way.
    Unknown(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::SignedOut(reason)
            | Failure::Unreachable(reason)
            | Failure::Refused(reason)
            | Failure::Unknown(reason) => f.write_str(reason),
        }
    }
}

impl From<Failure> for String {
    fn from(failure: Failure) -> String {
        failure.to_string()
    }
}

/// Why one request got no data.
enum Refused {
    /// The session aged, and a refresh may make it good again.
    Aged,
    Failed(Failure),
}

impl From<Refused> for Failure {
    fn from(refused: Refused) -> Failure {
        match refused {
            Refused::Aged => Failure::SignedOut(AGED.into()),
            Refused::Failed(failure) => failure,
        }
    }
}

impl From<Refused> for String {
    fn from(refused: Refused) -> String {
        Failure::from(refused).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn failure(status: u16, read: &Value) -> Option<Failure> {
        match judge(Method::Get, "/2.2/networks/1001", status, read) {
            Ok(_) => None,
            Err(refused) => Some(refused.into()),
        }
    }

    #[test]
    fn an_answer_with_data_gives_its_data() {
        let read = json!({"meta": {"code": 200}, "data": {"name": "home"}});

        assert_eq!(
            judge(Method::Get, "/2.2/networks/1001", 200, &read).ok(),
            Some(json!({"name": "home"}))
        );
    }

    #[test]
    fn eero_failing_on_its_side_is_unreachable() {
        assert_eq!(
            failure(503, &Value::Null),
            Some(Failure::Unreachable(
                "eero answered 503 to /2.2/networks/1001".into()
            ))
        );
    }

    #[test]
    fn eero_refusing_a_read_or_giving_it_no_data_is_refused() {
        let missing = json!({"meta": {"code": 404, "error": "error.network.missing"}});

        assert!(matches!(failure(404, &missing), Some(Failure::Refused(_))));
        assert!(matches!(
            failure(200, &json!({"data": null})),
            Some(Failure::Refused(_))
        ));
    }

    #[test]
    fn an_aged_session_left_unrefreshed_is_signed_out_and_says_to_sign_in() {
        let aged = json!({"meta": {"code": 401, "error": "error.session.refresh"}});

        let Some(Failure::SignedOut(reason)) = failure(401, &aged) else {
            panic!("an aged session reads as signed out");
        };

        assert!(reason.contains("run `eero sign-in"), "{reason}");
    }

    #[test]
    fn only_a_delete_eero_took_may_come_back_without_data() {
        let path = "/2.2/networks/1001/reservations/2c8db13091a2";
        let judged = |method, status, read: Value| match judge(method, path, status, &read) {
            Ok(data) => Ok(data),
            Err(refused) => Err(Failure::from(refused)),
        };
        let missing = json!({"meta": {"code": 404, "error": "error.reservation.missing"}});
        assert_eq!(
            [
                judged(Method::Delete, 204, Value::Null),
                judged(Method::Delete, 200, json!({"meta": {"code": 200}})),
                judged(
                    Method::Delete,
                    200,
                    json!({"meta": {"code": 200}, "data": {}})
                ),
            ],
            [Ok(Value::Null), Ok(Value::Null), Ok(json!({}))],
            "a DELETE eero took read as refused"
        );
        assert!(
            matches!(
                judged(Method::Delete, 404, missing),
                Err(Failure::Refused(_))
            ),
            "a DELETE eero refused read as taken"
        );
        for method in [Method::Get, Method::Post, Method::Put] {
            assert!(
                matches!(
                    judged(method, 200, json!({"meta": {"code": 200}, "data": null})),
                    Err(Failure::Refused(_))
                ),
                "a {method:?} answered with no data read as taken"
            );
        }
    }
}

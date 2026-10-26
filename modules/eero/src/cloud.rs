//! Talking to eero's cloud, which works the same at home and away: the eero
//! itself answers nothing on the LAN but DNS and the ports games ask it to
//! open.

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
    pub fn signed_in() -> Result<Cloud, String> {
        let mut cloud = Cloud::new();
        cloud.token = Some(session::open().map_err(|e| e.to_string())?);
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
    pub fn get(&mut self, path: &str) -> Result<Value, String> {
        self.call(Method::Get, path, None)
    }

    /// The data of an answer to a POST.
    pub fn post(&mut self, path: &str, body: Option<&Value>) -> Result<Value, String> {
        self.call(Method::Post, path, body)
    }

    /// The data of an answer to a PUT, which is how a setting changes.
    pub fn put(&mut self, path: &str, body: &Value) -> Result<Value, String> {
        self.call(Method::Put, path, Some(body))
    }

    /// Asks, and once more after refreshing the session if eero says it
    /// aged, as it does every so often.
    fn call(&mut self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, String> {
        match self.send(method, path, body) {
            Err(Refused::Aged) => {
                let data = self.send(Method::Post, "/2.2/login/refresh", None)?;
                let token = data.get("user_token").and_then(Value::as_str);
                self.token = Some(
                    token
                        .ok_or("eero refreshed the session without a token")?
                        .into(),
                );
                self.keep()?;
                Ok(self.send(method, path, body)?)
            }
            answer => Ok(answer?),
        }
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
        let mut answer = answer.map_err(|e| Refused::Other(format!("couldn't reach eero: {e}")))?;
        let status = answer.status().as_u16();
        let read: Value = answer.body_mut().read_json().unwrap_or(Value::Null);
        if api::session_expired(status, &read) {
            return Err(Refused::Aged);
        }
        if !(200..300).contains(&status) {
            return Err(Refused::Other(format!(
                "eero answered {status} to {path}: {read}"
            )));
        }
        api::data(&read).cloned().map_err(Refused::Other)
    }

    fn keep(&self) -> Result<(), String> {
        let token = self.token.as_deref().ok_or("no session to keep")?;
        session::keep(token).map_err(|e| e.to_string())
    }
}

/// How a request asks: eero's reads are GETs, a setting changes by PUT,
/// and everything else POSTs.
#[derive(Clone, Copy)]
enum Method {
    Get,
    Post,
    Put,
}

/// Why eero didn't answer with data.
enum Refused {
    /// The session aged, and a refresh makes it good again.
    Aged,
    Other(String),
}

impl From<Refused> for String {
    fn from(refused: Refused) -> String {
        match refused {
            Refused::Aged => "eero's session aged and couldn't be refreshed".into(),
            Refused::Other(reason) => reason,
        }
    }
}

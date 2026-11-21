//! A tool gateway's tools, called over the Model Context Protocol's HTTP
//! transport: one JSON-RPC request a POST, its answer the body.
//!
//! A gateway is opened once, with `initialize`, and keeps a session for
//! what follows; Joel's sign-in to it comes from the program a reader is
//! given, run only when there is none or the gateway refused the last
//! (`src/sign_in.rs`). Three things a gateway does on its own are taken in
//! stride, each once a call: a session it forgot is opened again, a
//! sign-in it refused is asked for again, and a tool whose backend it
//! turned off after a quiet spell has that backend turned back on. A
//! tool's answer is the text it said, as the gateway words it; reading it
//! is src/gateway.rs's.

use std::ffi::{OsStr, OsString};

use serde_json::{Value, json};
use ureq::Agent;

use crate::sign_in::{PATIENCE, SignIn, token};

/// The protocol's version this speaks, the one gateways answer with.
const VERSION: &str = "2024-11-05";

/// The header a gateway names its session in, and expects it back in.
const SESSION: &str = "mcp-session-id";

/// What a gateway says of a tool whose backend it has turned off.
const NOT_AVAILABLE: &str = "is not available";

/// The tool that turns a backend back on.
const ENABLE: &str = "tool_gateway_enable_backend";

/// HTTP's word for a sign-in refused, and for a session not found.
const REFUSED: u16 = 401;
const NOT_FOUND: u16 = 404;

/// Why a call came to nothing.
#[derive(Debug, PartialEq, Eq)]
pub enum Called {
    /// No sign-in to give it, as its program said: missing, locked or
    /// run over, for the reader to say as it says Slack's.
    NoSignIn(SignIn),
    /// The gateway refused the sign-in given, twice.
    Refused,
    /// The gateway couldn't be reached, or answered what isn't JSON-RPC.
    Unreachable(String),
    /// The gateway or its tool said it failed, in these words.
    Failed(String),
}

/// A tool gateway at `url`, signed in to by `program`.
pub struct Gateway {
    url: String,
    program: OsString,
    agent: Agent,
    token: Option<String>,
    session: Option<String>,
    asked: u64,
}

/// What one POST got: its status, its session header and its body.
struct Answer {
    status: u16,
    session: Option<String>,
    body: Option<Value>,
}

impl Gateway {
    /// A gateway nothing has been asked of yet.
    #[must_use]
    pub fn new(url: &str, program: &OsStr) -> Self {
        Self {
            url: url.to_owned(),
            program: program.to_owned(),
            agent: Agent::config_builder()
                .timeout_global(Some(PATIENCE))
                .http_status_as_error(false)
                .build()
                .into(),
            token: None,
            session: None,
            asked: 0,
        }
    }

    /// What `tool` says, asked with `arguments`.
    ///
    /// # Errors
    ///
    /// Why it said nothing: see [`Called`].
    pub fn call(&mut self, tool: &str, arguments: &Value) -> Result<String, Called> {
        match self.once(tool, arguments) {
            Err(Called::Failed(why)) if why.contains(NOT_AVAILABLE) => {
                let backend = tool.split('_').next().unwrap_or_default();
                self.once(ENABLE, &json!({ "backend": backend }))?;
                self.once(tool, arguments)
            }
            other => other,
        }
    }

    /// One call, signing in and opening a session again once each if the
    /// gateway forgot either.
    fn once(&mut self, tool: &str, arguments: &Value) -> Result<String, Called> {
        let params = json!({ "name": tool, "arguments": arguments });
        let mut refused = false;
        let mut forgotten = false;
        loop {
            self.open()?;
            let answer = self.post("tools/call", Some(&params))?;
            match answer.status {
                REFUSED if !refused => {
                    refused = true;
                    self.token = None;
                    self.session = None;
                }
                REFUSED => return Err(Called::Refused),
                NOT_FOUND if !forgotten => {
                    forgotten = true;
                    self.session = None;
                }
                _ => return said(answer),
            }
        }
    }

    /// Signs in and opens a session, where there is none.
    fn open(&mut self) -> Result<(), Called> {
        if self.token.is_none() {
            self.token = Some(match token(&self.program, PATIENCE) {
                SignIn::Token(token) => token,
                signed => return Err(Called::NoSignIn(signed)),
            });
        }
        if self.session.is_some() {
            return Ok(());
        }
        let opened = self.post(
            "initialize",
            Some(&json!({
                "protocolVersion": VERSION,
                "capabilities": {},
                "clientInfo": { "name": "pineapple", "version": env!("CARGO_PKG_VERSION") },
            })),
        )?;
        if opened.status == REFUSED {
            self.token = None;
            return Err(Called::Refused);
        }
        let session = opened.session.clone();
        said(opened)?;
        self.session = session;
        self.post("notifications/initialized", None)?;
        Ok(())
    }

    /// POSTs `method` with `params`, as a request when it has params or is
    /// `initialize`, and as a notification otherwise.
    fn post(&mut self, method: &str, params: Option<&Value>) -> Result<Answer, Called> {
        let mut request = json!({ "jsonrpc": "2.0", "method": method });
        if let Some(params) = params {
            self.asked += 1;
            request["id"] = json!(self.asked);
            request["params"] = params.clone();
        }
        let mut post = self
            .agent
            .post(&self.url)
            .header("Accept", "application/json, text/event-stream")
            .header(
                "Authorization",
                &format!("Bearer {}", self.token.as_deref().unwrap_or_default()),
            );
        if let Some(session) = &self.session {
            post = post.header(SESSION, session);
        }
        let mut answer = post
            .send_json(&request)
            .map_err(|error| Called::Unreachable(format!("couldn't reach the gateway: {error}")))?;
        let status = answer.status().as_u16();
        let session = answer
            .headers()
            .get(SESSION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let text = answer.body_mut().read_to_string().unwrap_or_default();
        let body = if params.is_some() && status != REFUSED && status != NOT_FOUND {
            Some(answered(&text).ok_or_else(|| {
                Called::Unreachable(format!("the gateway answered {status}, not JSON-RPC"))
            })?)
        } else {
            None
        };
        Ok(Answer {
            status,
            session,
            body,
        })
    }
}

/// The JSON-RPC answer in `text`: a body of JSON, or the last message of
/// an event stream, as the transport lets a server answer either way.
fn answered(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok().or_else(|| {
        text.lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok())
            .next_back()
    })
}

/// What an answer said: its text, or why it failed.
fn said(answer: Answer) -> Result<String, Called> {
    let Some(body) = answer.body else {
        return Err(Called::Unreachable(format!(
            "the gateway answered {} with nothing",
            answer.status
        )));
    };
    if let Some(error) = body.get("error") {
        let why = error["message"].as_str().unwrap_or("no reason");
        return Err(Called::Failed(why.to_owned()));
    }
    let result = &body["result"];
    let text = result["content"]
        .as_array()
        .map(|content| {
            content
                .iter()
                .filter_map(|part| part["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if result["isError"].as_bool() == Some(true) {
        Err(Called::Failed(text))
    } else {
        Ok(text)
    }
}

//! A tool gateway's tools, called over the Model Context Protocol's HTTP
//! transport: one JSON-RPC request a POST, its answer the body.
//!
//! A gateway is opened once, with `initialize`, and keeps a session for
//! what follows, if it names one; Joel's sign-in to it comes from the
//! program a reader is given, run only when there is none or the gateway
//! refused the last (`src/sign_in.rs`), and rested from for a spell once
//! it ran over or found its keyring locked, since a locked keyring asks
//! Joel to unlock it each time. Three things a gateway does on its own are
//! taken in stride, each once a call: a session it forgot is opened again,
//! a sign-in it refused, opening or calling, is asked for again (a
//! redirect, never followed, is one refused), and a
//! tool whose backend it turned off after a quiet spell has that backend
//! turned back on. Any other failure lets the session go, so the next call
//! opens one afresh. A tool's answer is the text it said, as the gateway
//! words it; reading it is src/gateway.rs's.

use std::ffi::{OsStr, OsString};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use ureq::Agent;

use crate::sign_in::{PATIENCE, SignIn, token, waited};

/// How long a sign-in that ran over, or found its keyring locked, is let
/// be before its program is run again.
const REST: Duration = slack_people::REST;

/// The protocol's version this speaks, the one gateways answer with.
const VERSION: &str = "2024-11-05";

/// The header a gateway names its session in, and expects it back in.
const SESSION: &str = "mcp-session-id";

/// What a gateway says of a tool whose backend it has turned off, after
/// the tool's quoted name; a tool's own failure saying something else is
/// "not available" is no such thing.
const NOT_AVAILABLE: &str = "\" is not available";

/// The tool that turns a backend back on.
const ENABLE: &str = "tool_gateway_enable_backend";

/// HTTP's words for a sign-in refused or not allowed, and for a session
/// not found.
const REFUSED: u16 = 401;
const FORBIDDEN: u16 = 403;
const NOT_FOUND: u16 = 404;

/// Where HTTP's failures start, and where a server's own do.
const FAILING: u16 = 400;
const SERVER_FAILING: u16 = 500;

/// HTTP's redirects, which are never followed, since one would carry the
/// sign-in wherever it pointed.
const REDIRECTS: std::ops::Range<u16> = 300..400;

/// Whether `status` refuses the sign-in given: a 401 or 403, or a
/// redirect, which is a gateway sending its caller off to sign in again.
fn refuses(status: u16) -> bool {
    status == REFUSED || status == FORBIDDEN || REDIRECTS.contains(&status)
}

/// Why a call came to nothing.
#[derive(Debug, PartialEq, Eq)]
pub enum Called {
    /// No sign-in to give it, as its program said: missing, locked or
    /// run over, for the reader to say as it says Slack's.
    NoSignIn(SignIn),
    /// The gateway refused the sign-in given, twice.
    Refused,
    /// The gateway couldn't be reached, or its answer couldn't be read
    /// to its end.
    Unreachable(String),
    /// The gateway answered what isn't JSON-RPC, a cause that lasts.
    Unreadable(String),
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
    opened: bool,
    resting: Option<(Instant, Rest)>,
    asked: u64,
}

/// Why a sign-in is rested from.
#[derive(Clone, Copy)]
enum Rest {
    Locked,
    RanOver,
}

impl Rest {
    fn signed(self) -> SignIn {
        match self {
            Self::Locked => SignIn::Locked,
            Self::RanOver => SignIn::RanOver,
        }
    }
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
            // A redirect would carry the sign-in wherever it pointed.
            agent: Agent::config_builder()
                .timeout_global(Some(PATIENCE))
                .http_status_as_error(false)
                .max_redirects(0)
                .build()
                .into(),
            token: None,
            session: None,
            opened: false,
            resting: None,
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
            Err(Called::Failed(why)) if why.contains(&format!("{tool}{NOT_AVAILABLE}")) => {
                let backend = tool.split('_').next().unwrap_or_default();
                eprintln!("<5>the gateway turned its {backend} tools off, so they're turned on");
                self.once(ENABLE, &json!({ "backend": backend }))?;
                self.once(tool, arguments)
            }
            other => other,
        }
    }

    /// Lets the session go, so the next call opens one afresh.
    fn forget(&mut self) {
        self.session = None;
        self.opened = false;
    }

    /// One call, signing in and opening a session again once each if the
    /// gateway forgot either.
    fn once(&mut self, tool: &str, arguments: &Value) -> Result<String, Called> {
        let params = json!({ "name": tool, "arguments": arguments });
        let mut refused = false;
        let mut forgotten = false;
        loop {
            let answer = if self.open()? {
                match self.post("tools/call", Some(&params)) {
                    Ok(answer) => answer,
                    Err(unreached) => {
                        self.forget();
                        return Err(unreached);
                    }
                }
            } else {
                Answer {
                    status: REFUSED,
                    session: None,
                    body: None,
                }
            };
            match answer.status {
                status if refuses(status) && !refused => {
                    refused = true;
                    self.token = None;
                    self.forget();
                }
                status if refuses(status) => return Err(Called::Refused),
                NOT_FOUND if !forgotten => {
                    forgotten = true;
                    self.forget();
                }
                status => {
                    if status >= FAILING {
                        self.forget();
                    }
                    return said(answer);
                }
            }
        }
    }

    /// Signs in and opens a session, where neither is; false when the
    /// gateway refused the sign-in as it opened.
    fn open(&mut self) -> Result<bool, Called> {
        if let Some((until, rest)) = self.resting {
            if Instant::now() < until {
                return Err(Called::NoSignIn(rest.signed()));
            }
            self.resting = None;
        }
        if self.token.is_none() {
            self.token = Some(match token(&self.program, PATIENCE) {
                SignIn::Token(token) => token,
                signed => {
                    let rest = match signed {
                        SignIn::Locked => Some(Rest::Locked),
                        SignIn::RanOver => Some(Rest::RanOver),
                        SignIn::Token(_) | SignIn::Missing => None,
                    };
                    if let Some(rest) = rest {
                        eprintln!(
                            "<4>{}",
                            waited(&signed, "the gateway", "slack-gateway", REST)
                        );
                        self.resting = Some((Instant::now() + REST, rest));
                    }
                    return Err(Called::NoSignIn(signed));
                }
            });
        }
        if self.opened {
            return Ok(true);
        }
        let opened = self.post(
            "initialize",
            Some(&json!({
                "protocolVersion": VERSION,
                "capabilities": {},
                "clientInfo": { "name": "pineapple", "version": env!("CARGO_PKG_VERSION") },
            })),
        )?;
        if refuses(opened.status) {
            return Ok(false);
        }
        let session = opened.session.clone();
        said(opened)?;
        self.session = session;
        if let Err(unreached) = self.post("notifications/initialized", None) {
            self.forget();
            return Err(unreached);
        }
        self.opened = true;
        Ok(true)
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
        let refused = refuses(status) || status == NOT_FOUND;
        let body = if params.is_some() && !refused {
            let text = answer.body_mut().read_to_string().map_err(|error| {
                Called::Unreachable(format!("couldn't read the gateway's answer: {error}"))
            })?;
            Some(answered(&text, self.asked).ok_or_else(|| {
                let why = format!("the gateway answered {status}, not JSON-RPC");
                // A gateway that's down says so in its own page, and passes.
                if status >= SERVER_FAILING {
                    Called::Unreachable(why)
                } else {
                    Called::Unreadable(why)
                }
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

/// The JSON-RPC answer in `text`: a body of JSON, or the event of an
/// event stream answering request `asked`, as the transport lets a server
/// answer either way. An event's data may run over several lines, and
/// the stream may carry the server's own notices beside the answer.
fn answered(text: &str, asked: u64) -> Option<Value> {
    serde_json::from_str(text).ok().or_else(|| {
        text.replace("\r\n", "\n")
            .split("\n\n")
            .filter_map(|event| {
                let data: Vec<&str> = event
                    .lines()
                    .filter_map(|line| line.strip_prefix("data:"))
                    .map(|data| data.strip_prefix(' ').unwrap_or(data))
                    .collect();
                serde_json::from_str::<Value>(&data.join("\n")).ok()
            })
            .find(|event| event["id"] == json!(asked))
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

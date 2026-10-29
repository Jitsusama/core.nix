//! An account's keys in the TPM added to GitHub, once the person signs in
//! on their phone: GitHub's device sign-in, run by the guide itself so its
//! own screen says what to type where, then each key the account's
//! modules mark for GitHub added as they say, as a login key or a signing
//! key. Run as the account, signed in, since the installed guide runs at
//! boot before there's a network (`guide github`).
//!
//! The sign-in is GitHub's command line's own (its OAuth app), so the token
//! it gives is handed to gh, which keeps it: gh's git credential helper
//! signs git in with it from then on.

use serde::Deserialize;

use crate::installed::facts;
use crate::installed::keys::TpmKey;
use crate::{Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool};

/// GitHub's command line's OAuth app, whose tokens gh takes.
const CLIENT_ID: &str = "178c6fc778ccc68e1d6a";
/// What gh's own sign-in asks for, with what adding keys needs.
const SCOPES: &str = "repo read:org gist admin:public_key admin:ssh_signing_key";
const HOST: &str = "github.com";
const DEVICE_CODE: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN: &str = "https://github.com/login/oauth/access_token";
/// curl, asking GitHub with the form it reads on its standard input, so
/// no device code is on a command line.
const ASK: &[&str] = &[
    "--silent",
    "--show-error",
    "--request",
    "POST",
    "--header",
    "Accept: application/json",
    "--data",
    "@-",
];

/// What GitHub takes a key as.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A key that signs in over SSH.
    Authentication,
    /// A key whose signatures on commits GitHub shows as verified.
    Signing,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Authentication => "authentication",
            Kind::Signing => "signing",
        }
    }

    /// Where GitHub lists the keys of this kind an account holds.
    fn listed(self) -> &'static str {
        match self {
            Kind::Authentication => "user/keys",
            Kind::Signing => "user/ssh_signing_keys",
        }
    }
}

/// How a key goes to GitHub, as core's guide module says.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OnGitHub {
    pub kind: Kind,
    /// What GitHub shows it as, such as `machine signing`.
    pub title: String,
}

/// Who the guide runs as, and what gh needs of their session to keep and
/// find its sign-in.
#[derive(Clone, Debug)]
pub struct Account {
    pub name: String,
    pub home: String,
    /// Their own runtime folder, where the sign-in's code waits.
    pub runtime: String,
    /// Their session's bus, for gh to keep its token in their keyring.
    pub bus: Option<String>,
}

impl Account {
    fn held(&self) -> String {
        format!("{}/guide-github-code.json", self.runtime)
    }

    fn given(&self) -> Given<'_> {
        let mut plain = vec![
            ("HOME", self.home.as_str()),
            ("XDG_RUNTIME_DIR", self.runtime.as_str()),
        ];
        if let Some(bus) = &self.bus {
            plain.push(("DBUS_SESSION_BUS_ADDRESS", bus.as_str()));
        }
        Given {
            plain,
            ..Given::default()
        }
    }

    fn signed_in(&self, machine: &mut dyn Machine) -> Result<bool, Fault> {
        let status = machine.run_given(
            Program::Tool(Tool::Gh),
            &["auth", "status", "--hostname", HOST],
            self.given(),
        )?;
        Ok(status.status == 0)
    }

    /// The account's keys its modules send to GitHub.
    fn keys(&self, machine: &mut dyn Machine) -> Result<Vec<(TpmKey, OnGitHub)>, Fault> {
        Ok(facts(machine)?
            .keys
            .into_iter()
            .filter(|key| key.account == self.name)
            .filter_map(|key| key.github.clone().map(|github| (key, github)))
            .collect())
    }
}

/// The sign-in's code, as GitHub gave it, kept until it's used or runs out.
#[derive(Debug, Deserialize, serde::Serialize)]
struct Pending {
    device_code: String,
    user_code: String,
    verification_uri: String,
    /// Seconds since the epoch.
    expires_at: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn held(machine: &mut dyn Machine, account: &Account) -> Result<Option<Pending>, Fault> {
    let path = account.held();
    let Some(bytes) = machine.read(&path)? else {
        return Ok(None);
    };
    let code: Pending =
        serde_json::from_slice(&bytes).map_err(|why| Fault::unreadable(&path, why))?;
    Ok((code.expires_at > now()).then_some(code))
}

/// A code from GitHub for the person to type on their phone.
pub struct AskForCode {
    pub account: Account,
}

impl Step for AskForCode {
    fn id(&self) -> &'static str {
        "github-code"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Ask GitHub for a sign-in code"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if self.account.signed_in(machine)? {
            return Ok(Standing::Done {
                evidence: "gh is signed in to GitHub".into(),
            });
        }
        Ok(match held(machine, &self.account)? {
            Some(code) => Standing::Done {
                evidence: format!("GitHub gave the code {}", code.user_code),
            },
            None => Standing::Needed { asks: vec![] },
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &crate::Answers) -> Result<(), Fault> {
        #[derive(Deserialize)]
        struct Issued {
            device_code: String,
            user_code: String,
            verification_uri: String,
            expires_in: u64,
        }
        let form = Secret::new(format!(
            "client_id={CLIENT_ID}&scope={}",
            SCOPES.replace(' ', "%20")
        ));
        let asked = machine
            .run_given(
                Program::Tool(Tool::Curl),
                &[ASK, &[DEVICE_CODE]].concat(),
                Given {
                    stdin: Some(&form),
                    ..Given::default()
                },
            )?
            .ok()?;
        let given: Issued = serde_json::from_str(&asked)
            .map_err(|why| Fault::unreadable("GitHub's sign-in code", why))?;
        let code = Pending {
            device_code: given.device_code,
            user_code: given.user_code,
            verification_uri: given.verification_uri,
            expires_at: now() + given.expires_in,
        };
        let kept =
            serde_json::to_vec(&code).map_err(|why| Fault::unreadable("the sign-in code", why))?;
        machine.write(&self.account.held(), &kept)
    }
}

/// The person signing in on their phone with the code, and gh given what
/// GitHub then hands out.
pub struct SignIn {
    pub account: Account,
}

impl Step for SignIn {
    fn id(&self) -> &'static str {
        "github-sign-in"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Sign in to GitHub on your phone"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if self.account.signed_in(machine)? {
            return Ok(Standing::Done {
                evidence: "gh is signed in to GitHub".into(),
            });
        }
        Ok(match held(machine, &self.account)? {
            Some(code) => Standing::Needed {
                asks: vec![Ask::Act {
                    id: "approved",
                    what: format!(
                        "On your phone, open {} and type {}. Approve it there, then press Return.",
                        code.verification_uri, code.user_code
                    ),
                }],
            },
            None => Standing::Blocked {
                why: "GitHub's code ran out before it was typed".into(),
                remedy: "Look again, and the guide asks GitHub for a new one.".into(),
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &crate::Answers) -> Result<(), Fault> {
        #[derive(Deserialize)]
        struct Answered {
            access_token: Option<String>,
            error: Option<String>,
        }
        let blocked = |why: &str| Fault::Blocked {
            step: "github-sign-in",
            why: why.into(),
        };
        let code = held(machine, &self.account)?
            .ok_or_else(|| blocked("GitHub's code ran out before it was typed"))?;
        let form = Secret::new(format!(
            "client_id={CLIENT_ID}&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code",
            code.device_code
        ));
        let answered = machine
            .run_given(
                Program::Tool(Tool::Curl),
                &[ASK, &[ACCESS_TOKEN]].concat(),
                Given {
                    stdin: Some(&form),
                    prints_secret: true,
                    ..Given::default()
                },
            )?
            .ok()?;
        let answered: Answered = serde_json::from_str(&answered)
            .map_err(|why| Fault::unreadable("GitHub's answer to the sign-in", why))?;
        let token = match (answered.access_token, answered.error.as_deref()) {
            (Some(token), _) => Secret::new(format!("{token}\n")),
            (None, Some("authorization_pending" | "slow_down")) => {
                return Err(blocked(
                    "GitHub hasn't been told yet that the code was approved",
                ));
            }
            (None, Some("access_denied")) => {
                machine.remove(&self.account.held())?;
                return Err(blocked("the sign-in was turned down on GitHub"));
            }
            (None, Some("expired_token")) => {
                machine.remove(&self.account.held())?;
                return Err(blocked("GitHub's code ran out before it was approved"));
            }
            (None, other) => {
                return Err(blocked(&format!(
                    "GitHub answered the sign-in with {}",
                    other.unwrap_or("nothing the guide knows")
                )));
            }
        };
        let mut given = self.account.given();
        given.stdin = Some(&token);
        machine
            .run_given(
                Program::Tool(Tool::Gh),
                &[
                    "auth",
                    "login",
                    "--hostname",
                    HOST,
                    "--git-protocol",
                    "https",
                    "--with-token",
                ],
                given,
            )?
            .ok()?;
        machine.remove(&self.account.held())
    }
}

/// Each key the account's modules send to GitHub, added as they say,
/// unless GitHub holds it already.
pub struct AddKeys {
    pub account: Account,
}

/// A public key as GitHub lists it: its type and its key, without the
/// comment.
fn bare(public: &str) -> String {
    public
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

impl AddKeys {
    /// The keys GitHub doesn't hold yet, with their public halves.
    fn missing(&self, machine: &mut dyn Machine) -> Result<Vec<(String, OnGitHub)>, Fault> {
        let mut missing = vec![];
        let mut listed: Vec<(Kind, Vec<String>)> = vec![];
        for (key, github) in self.account.keys(machine)? {
            let public_file = format!("{}.pub", key.file);
            let public = machine.read(&public_file)?.ok_or_else(|| Fault::Blocked {
                step: "github-keys",
                why: format!("{public_file} isn't here, so there's no key to add"),
            })?;
            let public = bare(&String::from_utf8_lossy(&public));
            if !listed.iter().any(|(kind, _)| *kind == github.kind) {
                let held = machine
                    .run_given(
                        Program::Tool(Tool::Gh),
                        &["api", github.kind.listed(), "--jq", ".[].key"],
                        self.account.given(),
                    )?
                    .ok()?;
                listed.push((github.kind, held.lines().map(bare).collect()));
            }
            let holds = listed
                .iter()
                .any(|(kind, keys)| *kind == github.kind && keys.contains(&public));
            if !holds {
                missing.push((public_file, github));
            }
        }
        Ok(missing)
    }
}

impl Step for AddKeys {
    fn id(&self) -> &'static str {
        "github-keys"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Add this machine's keys to GitHub"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let keys = self.account.keys(machine)?;
        if keys.is_empty() {
            return Ok(Standing::Done {
                evidence: "no key here goes to GitHub".into(),
            });
        }
        if !self.account.signed_in(machine)? {
            return Ok(Standing::Needed { asks: vec![] });
        }
        Ok(if self.missing(machine)?.is_empty() {
            let titles: Vec<&str> = keys
                .iter()
                .map(|(_, github)| github.title.as_str())
                .collect();
            Standing::Done {
                evidence: format!("GitHub holds {}", titles.join(", ")),
            }
        } else {
            Standing::Needed { asks: vec![] }
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &crate::Answers) -> Result<(), Fault> {
        for (public_file, github) in self.missing(machine)? {
            machine
                .run_given(
                    Program::Tool(Tool::Gh),
                    &[
                        "ssh-key",
                        "add",
                        &public_file,
                        "--title",
                        &github.title,
                        "--type",
                        github.kind.name(),
                    ],
                    self.account.given(),
                )?
                .ok()?;
        }
        Ok(())
    }
}

/// Every step of `guide github`, in order.
pub fn steps(account: &Account) -> Vec<Box<dyn Step>> {
    vec![
        Box::new(AskForCode {
            account: account.clone(),
        }),
        Box::new(SignIn {
            account: account.clone(),
        }),
        Box::new(AddKeys {
            account: account.clone(),
        }),
    ]
}

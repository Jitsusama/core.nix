//! Spotify's Web API, asked with spotify-player's sign-in, at addresses
//! given rather than fixed, so the tests ask a stand-in on this machine.

use crate::library::Library;
use crate::recent::{self, LIKED, Row};
use crate::token::{self, Read, SignIn};
use jiff::Timestamp;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use terminal_kit::Exit;
use ureq::Agent;

/// ncspot's Spotify app, which spotify-player signs in through: a PKCE
/// app, so a refresh names it and needs no secret, and the sign-in's file
/// is named after it.
pub const APP: &str = "d420a117a32841c2b3474932e49fb54b";

/// Why Spotify couldn't be asked, each said to the shell by its own exit
/// status, so the launcher can say which in its own words.
#[derive(Debug, PartialEq)]
pub enum Refused {
    /// There's no sign-in, or Spotify no longer takes it: spotify-player
    /// has to sign in again.
    SignedOut,
    /// Spotify couldn't be reached, or failed as it answered; asking
    /// again later may work.
    Unreachable(String),
    /// Spotify asked to be asked less often.
    Busy,
    /// Nothing is signed in to Spotify Connect to play on.
    NoDevice,
    /// Anything else, said as it is.
    Failed(String),
}

impl Refused {
    /// The exit it's said by, from the table every command of ours keeps
    /// (docs/commands.md): a sign-in to make again is configuration (78),
    /// no device to play on is the thing named not there (66), Spotify
    /// out of reach is unavailable (69) and busy is to try again (75).
    pub fn exit(&self) -> Exit {
        match self {
            Refused::SignedOut => Exit::Config,
            Refused::NoDevice => Exit::NotFound,
            Refused::Unreachable(_) => Exit::Unavailable,
            Refused::Busy => Exit::TryAgain,
            Refused::Failed(_) => Exit::Failed,
        }
    }

    /// What it says on standard error, for the journal and a person
    /// running it, never holding the sign-in.
    pub fn said(&self) -> String {
        match self {
            Refused::SignedOut => {
                "Spotify isn't signed in; run `spotify_player authenticate`.".to_owned()
            }
            Refused::NoDevice => "Nothing is open to play Spotify on.".to_owned(),
            Refused::Busy => "Spotify is busy.".to_owned(),
            Refused::Unreachable(why) | Refused::Failed(why) => why.clone(),
        }
    }
}

/// Spotify's API and accounts server, and the sign-in they're asked with.
pub struct Spotify {
    agent: Agent,
    api: String,
    accounts: String,
    path: PathBuf,
    token: Option<SignIn>,
}

impl Spotify {
    /// Spotify as the environment says: `SPOTIFY_RECENT_API` and
    /// `SPOTIFY_RECENT_ACCOUNTS` its addresses, Spotify's own unless
    /// given, and `SPOTIFY_RECENT_TOKEN` the sign-in, spotify-player's
    /// own unless given.
    pub fn from_environment() -> Spotify {
        let given = |name: &str, otherwise: String| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or(otherwise)
        };
        let home = std::env::var("HOME").unwrap_or_default();
        let saved = format!("{home}/.cache/spotify-player/{APP}_token.json");
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .http_status_as_error(false)
            .build()
            .into();
        Spotify {
            agent,
            api: given(
                "SPOTIFY_RECENT_API",
                "https://api.spotify.com/v1".to_owned(),
            ),
            accounts: given(
                "SPOTIFY_RECENT_ACCOUNTS",
                "https://accounts.spotify.com".to_owned(),
            ),
            path: PathBuf::from(given("SPOTIFY_RECENT_TOKEN", saved)),
            token: None,
        }
    }

    /// What Spotify played lately, a row a context, what it plays now
    /// first (recent.rs says how). Spotify leaves an episode playing out
    /// of what it has now unless asked for episodes too.
    pub fn recent(&mut self) -> Result<Vec<Row>, Refused> {
        let (status, player) = self.get("/me/player?additional_types=episode")?;
        // 204 is Spotify having nothing playing anywhere.
        let player = if status == 200 { player } else { Value::Null };
        let recently = self.must("/me/player/recently-played?limit=50")?;
        let plays = recent::plays(&player, &recently);
        recent::rows(&plays, |kind, uri| self.named(kind, uri))
    }

    /// A playlist's, an album's or a show's row, its name, who made it and
    /// how many it holds, or none where Spotify won't say. A playlist is
    /// asked for those fields alone, since whole it holds its first
    /// hundred songs.
    fn named(&mut self, kind: &str, uri: &str) -> Result<Option<Row>, Refused> {
        let id = uri.rsplit(':').next().unwrap_or_default();
        let (asked, read): (String, fn(&Value) -> Option<Row>) = match kind {
            "playlist" => (
                format!("/playlists/{id}?fields=name,owner(display_name),items(total)"),
                recent::playlist,
            ),
            "album" => (format!("/albums/{id}"), recent::album),
            "show" => (format!("/shows/{id}"), recent::show),
            _ => return Ok(None),
        };
        let (status, mut said) = self.get(&asked)?;
        if status != 200 {
            return Ok(None);
        }
        // It's what was asked for, which a playlist asked by its fields
        // doesn't say again.
        if let Some(fields) = said.as_object_mut() {
            fields.insert("uri".to_owned(), Value::from(uri));
        }
        Ok(read(&said))
    }

    /// Every row of one of Joel's lists, page after page.
    pub fn library(&mut self, library: Library) -> Result<Vec<Row>, Refused> {
        let mut rows = Vec::new();
        let mut next = Some(library.first().to_owned());
        while let Some(page) = next {
            let (more, after) = library.page(&self.must(&page)?);
            rows.extend(more);
            next = after;
        }
        Ok(rows)
    }

    /// How many songs Joel likes, which Spotify counts on any page of
    /// them, so it asks for one.
    pub fn liked(&mut self) -> Result<u64, Refused> {
        self.must("/me/tracks?limit=1")?
            .get("total")
            .and_then(Value::as_u64)
            .ok_or_else(|| Refused::Failed("Spotify didn't count the liked songs.".to_owned()))
    }

    /// Plays `uri` where Spotify plays now. With nowhere playing, it plays
    /// on the first device signed in that can be told to, as the client
    /// open but idle is, and with none, says so, for the launcher to open
    /// the client on it instead.
    pub fn play(&mut self, uri: &str) -> Result<(), Refused> {
        let account = if uri == LIKED {
            let me = self.must("/me")?;
            me.get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        } else {
            String::new()
        };
        let Some(body) = recent::playing(uri, &account) else {
            return Err(Refused::Failed(format!(
                "{uri} isn't something Spotify plays."
            )));
        };
        let (status, said) = self.put("/me/player/play", &body)?;
        if status != 404 {
            return played(status, &said);
        }
        let devices = self.must("/me/player/devices")?;
        let Some(device) = first_device(&devices) else {
            return Err(Refused::NoDevice);
        };
        let (status, said) = self.put(&format!("/me/player/play?device_id={device}"), &body)?;
        if status == 404 {
            return Err(Refused::NoDevice);
        }
        played(status, &said)
    }

    /// The access token, refreshed first if it's run out.
    fn access(&mut self) -> Result<String, Refused> {
        if let Some(token) = &self.token {
            return Ok(token.access_token.clone());
        }
        let Read::Found(token) = token::read(&self.path) else {
            return Err(Refused::SignedOut);
        };
        let token = if token.expired(Timestamp::now()) {
            self.refreshed()?
        } else {
            token
        };
        let access = token.access_token.clone();
        self.token = Some(token);
        Ok(access)
    }

    /// The sign-in refreshed and written back, under the lock, read again
    /// once the lock is held, since another may have refreshed it while
    /// this waited, and a refresh token Spotify replaced is spent.
    fn refreshed(&self) -> Result<SignIn, Refused> {
        let unwritable = |error: std::io::Error| {
            Refused::Failed(format!("The sign-in couldn't be refreshed: {error}."))
        };
        let _held = token::lock(&self.path).map_err(unwritable)?;
        let Read::Found(token) = token::read(&self.path) else {
            return Err(Refused::SignedOut);
        };
        let now = Timestamp::now();
        if !token.expired(now) {
            return Ok(token);
        }
        let Some(refresh) = token.refresh_token.clone() else {
            return Err(Refused::SignedOut);
        };
        let mut answer = self
            .agent
            .post(format!("{}/api/token", self.accounts))
            .send_form([
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh.as_str()),
                ("client_id", APP),
            ])
            .map_err(|error| Refused::Unreachable(format!("Spotify can't be reached: {error}.")))?;
        let status = answer.status().as_u16();
        // A refresh token Spotify no longer takes is refused as a bad
        // request, invalid_grant, which signing in again mends.
        if matches!(status, 400 | 401) {
            return Err(Refused::SignedOut);
        }
        if status != 200 {
            return Err(Refused::Unreachable(format!(
                "Spotify's accounts answered {status}."
            )));
        }
        let said: Value = answer
            .body_mut()
            .read_json()
            .map_err(|error| Refused::Failed(format!("Spotify's refresh: {error}.")))?;
        let fresh = token
            .refreshed(&said, now)
            .ok_or_else(|| Refused::Failed("Spotify's refresh held no token.".to_owned()))?;
        token::write(&self.path, &fresh).map_err(unwritable)?;
        Ok(fresh)
    }

    /// Where `asked` is: a path under the API, or a whole address, as a
    /// page's next is given.
    fn address(&self, asked: &str) -> String {
        if asked.starts_with("http") {
            asked.to_owned()
        } else {
            format!("{}{asked}", self.api)
        }
    }

    /// What the API answers to a read that has to answer.
    fn must(&mut self, asked: &str) -> Result<Value, Refused> {
        match self.get(asked)? {
            (200, said) => Ok(said),
            (status, _) => Err(Refused::Failed(format!("Spotify answered {status}."))),
        }
    }

    fn get(&mut self, asked: &str) -> Result<(u16, Value), Refused> {
        let bearer = format!("Bearer {}", self.access()?);
        let answer = self
            .agent
            .get(self.address(asked))
            .header("Authorization", &bearer)
            .call();
        answered(answer)
    }

    fn put(&mut self, asked: &str, body: &Value) -> Result<(u16, Value), Refused> {
        let bearer = format!("Bearer {}", self.access()?);
        let answer = self
            .agent
            .put(self.address(asked))
            .header("Authorization", &bearer)
            .send_json(body);
        answered(answer)
    }
}

/// An answer's status and its JSON, or null where it has none, refusals
/// that hold for every request said as such: the sign-in no longer
/// taken, and Spotify busy or failing.
fn answered(
    answer: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<(u16, Value), Refused> {
    let mut answer = answer
        .map_err(|error| Refused::Unreachable(format!("Spotify can't be reached: {error}.")))?;
    let status = answer.status().as_u16();
    match status {
        401 => return Err(Refused::SignedOut),
        429 => return Err(Refused::Busy),
        500.. => return Err(Refused::Unreachable(format!("Spotify answered {status}."))),
        _ => {}
    }
    let text = answer.body_mut().read_to_string().unwrap_or_default();
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// Whether a play was taken, and Spotify's reason where it wasn't, as one
/// needing Premium is refused.
fn played(status: u16, said: &Value) -> Result<(), Refused> {
    if (200..300).contains(&status) {
        return Ok(());
    }
    let why = said
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("no reason given");
    Err(Refused::Failed(format!("Spotify wouldn't play it: {why}.")))
}

/// The device to play on when none is: the first Spotify lists that can
/// be told what to play.
fn first_device(devices: &Value) -> Option<String> {
    devices
        .get("devices")
        .and_then(Value::as_array)?
        .iter()
        .filter(|device| device.get("is_restricted").and_then(Value::as_bool) != Some(true))
        .find_map(|device| device.get("id")?.as_str().map(String::from))
}

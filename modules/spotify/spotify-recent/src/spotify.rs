//! Spotify's Web API, asked with spotify-player's sign-in, at addresses
//! given rather than fixed, so the tests ask a stand-in on this machine.

use crate::recent::{self, LIKED, Row};
use crate::token::{self, Read, SignIn};
use jiff::Timestamp;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
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
    /// Spotify couldn't be reached, or was too busy to answer; asking
    /// again later may work.
    Unreachable(String),
    /// Nothing is signed in to Spotify Connect to play on.
    NoDevice,
    /// Anything else, said as it is.
    Failed(String),
}

impl Refused {
    /// The exit status it's said by: 2 signed out, 3 nowhere to play, 4
    /// out of reach, 1 anything else.
    pub fn status(&self) -> u8 {
        match self {
            Refused::SignedOut => 2,
            Refused::NoDevice => 3,
            Refused::Unreachable(_) => 4,
            Refused::Failed(_) => 1,
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
    /// first (recent.rs says how).
    pub fn recent(&mut self) -> Result<Vec<Row>, Refused> {
        let (status, player) = self.get("/me/player")?;
        // 204 is Spotify having nothing playing anywhere.
        let player = if status == 200 { player } else { Value::Null };
        let recently = self.must("/me/player/recently-played?limit=50")?;
        let plays = recent::plays(&player, &recently);
        recent::rows(&plays, |kind, uri| self.named(kind, uri))
    }

    /// A playlist's, an artist's or an album's name and who made it, or
    /// none where Spotify won't say.
    fn named(&mut self, kind: &str, uri: &str) -> Result<Option<(String, String)>, Refused> {
        let id = uri.rsplit(':').next().unwrap_or_default();
        let (asked, read): (String, fn(&Value) -> String) = match kind {
            "playlist" => (
                format!("/playlists/{id}?fields=name,owner(display_name)"),
                recent::owner,
            ),
            "artist" => (format!("/artists/{id}"), |_| String::new()),
            "album" => (format!("/albums/{id}"), |album| {
                album
                    .get("artists")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|artist| artist.get("name")?.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }),
            _ => return Ok(None),
        };
        let (status, said) = self.get(&asked)?;
        if status != 200 {
            return Ok(None);
        }
        Ok(said
            .get("name")
            .and_then(Value::as_str)
            .map(|name| (name.to_owned(), read(&said))))
    }

    /// Every one of Joel's playlists, page after page.
    pub fn playlists(&mut self) -> Result<Vec<Row>, Refused> {
        let mut rows = Vec::new();
        let mut next = Some("/me/playlists?limit=50".to_owned());
        while let Some(page) = next {
            let (more, after) = recent::playlists(&self.must(&page)?);
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
        429 => return Err(Refused::Unreachable("Spotify is busy.".to_owned())),
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

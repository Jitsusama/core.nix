//! Says what Spotify played lately, what Joel keeps there and how many
//! songs he likes, and plays one, for Spotify's Recent and its insides in
//! the launcher (modules/spotify/SpotifyProgram.qml).
//!
//! ```text
//! spotify-recent                what played lately, one line a context
//! spotify-recent playlists      every playlist of Joel's
//! spotify-recent albums         every album he saved
//! spotify-recent artists        every artist he follows
//! spotify-recent shows          every show he saved
//! spotify-recent liked          how many songs he likes
//! spotify-recent play URI       plays a playlist, album, artist, show or track
//! ```
//!
//! A line is a Spotify URI, its kind, its name, who made it, when it was
//! last played and how many songs or episodes it holds, apart by tabs,
//! each left empty where Spotify doesn't say. What played lately is one
//! line for each playlist, album or show played from, a track played from
//! nowhere Spotify names counting as its album, what plays now first, its
//! time `playing` or `paused`, then newest first, nine at most. A line of
//! a list says no time, being kept rather than played.
//!
//! It asks Spotify's Web API with spotify-player's sign-in, refreshing it
//! once it's run out and writing it back as spotify-player keeps it
//! (src/token.rs). It's run only as the launcher steps into Spotify,
//! never on a timer, and only ever reads, but for `play`.
//!
//! It exits 0 once it's said what it was asked, 2 while Spotify isn't
//! signed in, 3 when `play` has nowhere to play, 4 while Spotify can't be
//! reached, and 1 otherwise, saying why on standard error.
//! `SPOTIFY_RECENT_API`, `SPOTIFY_RECENT_ACCOUNTS` and
//! `SPOTIFY_RECENT_TOKEN` stand in for Spotify's addresses and the
//! sign-in's file, for the tests.

mod library;
mod recent;
mod spotify;
mod token;

use library::Library;
use recent::Row;
use spotify::Spotify;
use std::io::Write;
use std::process::ExitCode;

/// What it says when asked something it doesn't know: how to ask it.
const USAGE: &str =
    "usage: spotify-recent [playlists | albums | artists | shows | liked | play URI]";

fn main() -> ExitCode {
    let asked: Vec<String> = std::env::args().skip(1).collect();
    let mut spotify = Spotify::from_environment();
    let said = match asked.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        [] => spotify.recent().map(|rows| lines(&rows)),
        ["liked"] => spotify.liked().map(|count| format!("{count}\n")),
        ["play", uri] => spotify.play(uri).map(|()| String::new()),
        [list] if let Some(library) = Library::named(list) => {
            spotify.library(library).map(|rows| lines(&rows))
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(64);
        }
    };
    match said {
        Ok(text) => {
            // The launcher reads it all once it ends, so a reader gone
            // early only means nobody wanted it.
            let _ = std::io::stdout().lock().write_all(text.as_bytes());
            ExitCode::SUCCESS
        }
        Err(refused) => {
            eprintln!("{}", refused.said());
            ExitCode::from(refused.status())
        }
    }
}

fn lines(rows: &[Row]) -> String {
    rows.iter().map(|row| row.line() + "\n").collect()
}

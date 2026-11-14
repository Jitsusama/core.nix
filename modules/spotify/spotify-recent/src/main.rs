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
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit. It exits 0 once it's said what it was asked,
//! 78 while Spotify isn't signed in, 66 when `play` has nowhere to play,
//! 69 while Spotify can't be reached, 75 while Spotify is busy, and 1
//! otherwise, saying why on standard error. The launcher reads those
//! (modules/spotify/Spotify.js). Its lines are the same into a terminal
//! as into a pipe, the launcher's; `--json` says them as fields instead.
//! `SPOTIFY_RECENT_API`, `SPOTIFY_RECENT_ACCOUNTS` and
//! `SPOTIFY_RECENT_TOKEN` stand in for Spotify's addresses and the
//! sign-in's file, for the tests.

mod library;
mod recent;
mod spotify;
mod token;

use library::Library;
use recent::Row;
use serde_json::json;
use spotify::Spotify;
use std::io::Write;
use std::process::ExitCode;
use terminal_kit::{Context, Exit, Failure, Help};

fn help() -> Help {
    Help::new(
        "spotify-recent",
        "what Spotify played lately, and plays one",
    )
    .subcommand(library("playlists", "every playlist of Joel's"))
    .subcommand(library("albums", "every album he saved"))
    .subcommand(library("artists", "every artist he follows"))
    .subcommand(library("shows", "every show he saved"))
    .subcommand(library("liked", "how many songs he likes"))
    .subcommand(
        Help::new("play", "plays a playlist, album, show or track")
            .example("spotify-recent play spotify:album:ID", "plays that album")
            .argument("<uri>", "what to play, as Spotify names it")
            .exit(Exit::Failed, "Spotify refused")
            .exit(Exit::NotFound, "nothing is open to play on")
            .exit(Exit::Unavailable, "Spotify can't be reached")
            .exit(Exit::Config, "Spotify isn't signed in"),
    )
    .example("spotify-recent", "what played lately, a line a context")
    .example("spotify-recent play spotify:album:ID", "plays that album")
    .exit(Exit::Failed, "Spotify refused, or said what it doesn't")
    .exit(Exit::NotFound, "nothing is open to play on")
    .exit(Exit::Unavailable, "Spotify can't be reached")
    .exit(Exit::TryAgain, "Spotify is busy")
    .exit(Exit::Config, "Spotify isn't signed in")
}

/// A subcommand listing part of Joel's library.
fn library(name: &str, about: &str) -> Help {
    Help::new(name, about)
        .example(&format!("spotify-recent {name}"), "a line each")
        .example(
            &format!("spotify-recent {name} --json"),
            "the same, as JSON",
        )
        .exit(Exit::Failed, "Spotify refused, or said what it doesn't")
        .exit(Exit::Unavailable, "Spotify can't be reached")
        .exit(Exit::TryAgain, "Spotify is busy")
        .exit(Exit::Config, "Spotify isn't signed in")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let asked: Vec<&str> = context.args.iter().map(String::as_str).collect();
        let mut spotify = Spotify::from_environment();
        let refused = |refused: spotify::Refused| Failure::new(refused.exit(), refused.said());
        match asked[..] {
            [] => {
                let rows = spotify.recent().map_err(refused)?;
                rows_said(context, &rows);
            }
            ["liked"] => {
                let count = spotify.liked().map_err(refused)?;
                if context.format().is_json() {
                    answered(
                        context,
                        &json!({ "schema": "spotify-recent.liked/1", "count": count }),
                    );
                } else {
                    written(&format!("{count}\n"));
                }
            }
            ["play", uri] => {
                spotify.play(uri).map_err(refused)?;
                if context.format().is_json() {
                    answered(
                        context,
                        &json!({ "schema": "spotify-recent.played/1", "uri": uri }),
                    );
                }
            }
            [list] if let Some(library) = Library::named(list) => {
                let rows = spotify.library(library).map_err(refused)?;
                rows_said(context, &rows);
            }
            _ => {
                return Err(Failure::new(
                    Exit::Usage,
                    format!("spotify-recent doesn't know \"{}\"", asked.join(" ")),
                )
                .try_this("spotify-recent --help"));
            }
        }
        Ok(())
    })
}

/// Rows as lines, apart by tabs as the launcher reads them, or as fields.
fn rows_said(context: &Context, rows: &[Row]) {
    if context.format().is_json() {
        let rows: Vec<_> = rows
            .iter()
            .map(|row| {
                json!({
                    "uri": row.uri,
                    "kind": row.kind,
                    "name": row.name,
                    "detail": row.detail,
                    "at": row.at,
                    "size": row.size,
                })
            })
            .collect();
        answered(
            context,
            &json!({ "schema": "spotify-recent.rows/1", "rows": rows }),
        );
    } else {
        written(&rows.iter().map(|row| row.line() + "\n").collect::<String>());
    }
}

/// Text written whole. The launcher reads it all once it ends, so a reader
/// gone early only means nobody wanted it.
fn written(text: &str) {
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
}

/// The answer as JSON, its reader gone early let go as `written` lets it.
fn answered(context: &Context, answer: &serde_json::Value) {
    let _ = context.answer(answer);
}

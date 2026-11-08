//! What Spotify played lately, as the launcher lists it: one row for each
//! thing Joel played from, newest first, rather than one for each track,
//! since what he goes back to is the playlist, album or artist he was
//! listening to, and Spotify's own Recents in its client lists them so.

use serde_json::Value;

/// How many rows Spotify's Recent holds, as every program's does, so 1 to
/// 9 pick one.
pub const MOST: usize = 9;

/// Liked Songs as the client knows it, whoever's they are: Spotify names
/// the context `spotify:user:<id>:collection` in what it played, which
/// opens nothing in the client, so it's said this way and turned back
/// into the account's own as it's played (`liked`).
pub const LIKED: &str = "spotify:collection:tracks";

/// One row: what it is, as a Spotify URI and its kind, its name, who made
/// it, and when it was last played, in RFC 3339, or `playing` or `paused`
/// for what Spotify has now.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub uri: String,
    pub kind: String,
    pub name: String,
    pub detail: String,
    pub at: String,
}

impl Row {
    /// The row as a line: its fields apart by tabs, each with any tab or
    /// line break in it made a space, so a name can't move a field.
    pub fn line(&self) -> String {
        [&self.uri, &self.kind, &self.name, &self.detail, &self.at]
            .map(|field| plain(field))
            .join("\t")
    }
}

/// A field with tabs and line breaks made spaces, trimmed.
pub fn plain(field: &str) -> String {
    field.replace(['\t', '\n', '\r'], " ").trim().to_owned()
}

/// One track played: where Spotify says it was played from, if it says,
/// and the track's album, which stands in for a track played from nowhere
/// Spotify names, as one played from search is.
#[derive(Clone, Debug)]
pub struct Play {
    context: Option<(String, String)>,
    album: Option<Row>,
    at: String,
}

/// The plays in what Spotify has now, `player` (/me/player, or null when
/// it has nothing), then in what it played lately, `recently`
/// (/me/player/recently-played), newest first.
pub fn plays(player: &Value, recently: &Value) -> Vec<Play> {
    let now = player
        .get("item")
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("track"))
        .map(|track| {
            let at = if player.get("is_playing").and_then(Value::as_bool) == Some(true) {
                "playing"
            } else {
                "paused"
            };
            play(player.get("context"), track, at)
        });
    let mut lately: Vec<Play> = recently
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let at = item.get("played_at")?.as_str()?;
            Some(play(item.get("context"), item.get("track")?, at))
        })
        .collect();
    // Spotify lists them newest first already; sorting keeps it so should
    // it ever not. Its times all share one form, so they sort as text.
    lately.sort_by(|a, b| b.at.cmp(&a.at));
    now.into_iter().chain(lately).collect()
}

fn play(context: Option<&Value>, track: &Value, at: &str) -> Play {
    let text = |value: &Value, field: &str| {
        value
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let context = context
        .filter(|context| !context.is_null())
        .map(|context| (text(context, "type"), text(context, "uri")))
        .filter(|(kind, uri)| !kind.is_empty() && !uri.is_empty());
    let album = track
        .get("album")
        .filter(|album| album.get("uri").and_then(Value::as_str).is_some())
        .map(|album| Row {
            uri: text(album, "uri"),
            kind: "album".to_owned(),
            name: text(album, "name"),
            detail: artists(album),
            at: at.to_owned(),
        });
    Play {
        context,
        album,
        at: at.to_owned(),
    }
}

/// An album's artists, as Spotify credits them, in its order.
fn artists(album: &Value) -> String {
    album
        .get("artists")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|artist| artist.get("name")?.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The rows for `plays`, one a context, newest first, at most nine. A
/// playlist's or an artist's name isn't in what Spotify played, so
/// `named` asks it, once a context, giving its name and who made it, or
/// none where Spotify won't say, as for a playlist it made for Joel and
/// no longer shows to other apps; that play falls back to its album, as
/// one played from nowhere does, so it's still listed and still plays.
/// A play with neither, a local file's, is left out, as nothing plays it.
pub fn rows<E>(
    plays: &[Play],
    mut named: impl FnMut(&str, &str) -> Result<Option<(String, String)>, E>,
) -> Result<Vec<Row>, E> {
    let mut rows: Vec<Row> = Vec::new();
    let mut unnamed: Vec<String> = Vec::new();
    for play in plays {
        if rows.len() == MOST {
            break;
        }
        let row = match &play.context {
            Some((kind, _)) if kind == "collection" => Some(Row {
                uri: LIKED.to_owned(),
                kind: kind.clone(),
                name: "Liked Songs".to_owned(),
                detail: String::new(),
                at: play.at.clone(),
            }),
            Some((kind, uri)) if matches!(kind.as_str(), "playlist" | "album" | "artist") => {
                if rows.iter().any(|row| &row.uri == uri) {
                    continue;
                }
                let own = play.album.as_ref().filter(|album| &album.uri == uri);
                let found = match own {
                    Some(album) => Some((album.name.clone(), album.detail.clone())),
                    None if unnamed.contains(uri) => None,
                    None => named(kind, uri)?,
                };
                if found.is_none() {
                    unnamed.push(uri.clone());
                }
                found
                    .map(|(name, detail)| Row {
                        uri: uri.clone(),
                        kind: kind.clone(),
                        name,
                        detail,
                        at: play.at.clone(),
                    })
                    .or_else(|| play.album.clone())
            }
            _ => play.album.clone(),
        };
        let Some(row) = row else { continue };
        if !rows.iter().any(|kept| kept.uri == row.uri) {
            rows.push(row);
        }
    }
    Ok(rows)
}

/// The playlists on one page of Joel's (/me/playlists), each a row whose
/// last field is how many songs it holds, and the page after it, if any.
/// Spotify's newer answer counts them under `items`, its older under
/// `tracks`, so either is read.
pub fn playlists(page: &Value) -> (Vec<Row>, Option<String>) {
    let rows = page
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|list| {
            let total = ["items", "tracks"]
                .iter()
                .find_map(|field| list.get(field)?.get("total")?.as_u64())
                .unwrap_or(0);
            Some(Row {
                uri: list.get("uri")?.as_str()?.to_owned(),
                kind: "playlist".to_owned(),
                name: list.get("name")?.as_str()?.to_owned(),
                detail: owner(list),
                at: total.to_string(),
            })
        })
        .collect();
    let next = page.get("next").and_then(Value::as_str).map(String::from);
    (rows, next)
}

/// Who made a playlist, by the name Spotify shows for them.
pub fn owner(list: &Value) -> String {
    list.get("owner")
        .and_then(|owner| owner.get("display_name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// What `PUT /me/player/play` is given to play `uri`: a playlist, album
/// or artist as its context, a track alone, and Liked Songs as the
/// account's own collection, `account` being Joel's id; or none for
/// anything Spotify can't play, so nothing is sent.
pub fn playing(uri: &str, account: &str) -> Option<Value> {
    let mut parts = uri.split(':');
    if parts.next() != Some("spotify") {
        return None;
    }
    match (parts.next()?, parts.next()?, parts.next()) {
        ("collection", "tracks", None) => Some(serde_json::json!({
            "context_uri": format!("spotify:user:{account}:collection")
        })),
        ("playlist" | "album" | "artist" | "show", id, None) if !id.is_empty() => {
            Some(serde_json::json!({ "context_uri": uri }))
        }
        ("track" | "episode", id, None) if !id.is_empty() => {
            Some(serde_json::json!({ "uris": [uri] }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    /// The fixture's plays named as a stand-in would name them: the two
    /// playlists and the artist Spotify names, the second playlist one it
    /// won't, counting each time it's asked.
    fn listed(player: &Value) -> (Vec<Row>, Vec<String>) {
        let mut asked = Vec::new();
        let given = plays(
            player,
            &fixture(include_str!("../fixtures/recently-played.json")),
        );
        let rows = rows(&given, |kind, uri| {
            asked.push(uri.to_owned());
            Ok::<_, ()>(match (kind, uri) {
                ("playlist", "spotify:playlist:p1") => Some(("Focus".into(), "Joel".into())),
                ("playlist", "spotify:playlist:p3") => Some(("Road".into(), "Spotify".into())),
                ("playlist", "spotify:playlist:p9") => Some(("Morning".into(), "Joel".into())),
                ("artist", "spotify:artist:r1") => Some(("Artist".into(), String::new())),
                _ => None,
            })
        })
        .unwrap();
        (rows, asked)
    }

    fn uris(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|row| row.uri.as_str()).collect()
    }

    #[test]
    fn what_plays_now_leads_and_repeats_are_dropped() {
        let (rows, asked) = listed(&fixture(include_str!("../fixtures/player.json")));
        assert_eq!(rows[0].name, "Morning");
        assert_eq!(rows[0].at, "playing");
        assert_eq!(rows[1].uri, "spotify:playlist:p1");
        assert_eq!(rows[1].at, "2026-11-07T08:50:00.000Z");
        assert_eq!(
            uris(&rows).iter().filter(|uri| uri.contains("p1")).count(),
            1
        );
        assert_eq!(
            uris(&rows)
                .iter()
                .filter(|uri| **uri == "spotify:album:a2")
                .count(),
            1
        );
        // A context is asked its name once, however often it was played.
        assert_eq!(asked.iter().filter(|uri| uri.contains("p1")).count(), 1);
    }

    #[test]
    fn a_paused_player_says_so() {
        let mut player = fixture(include_str!("../fixtures/player.json"));
        player["is_playing"] = Value::Bool(false);
        assert_eq!(listed(&player).0[0].at, "paused");
    }

    #[test]
    fn a_track_from_nowhere_falls_back_to_its_album() {
        let (rows, _) = listed(&Value::Null);
        let album = rows
            .iter()
            .find(|row| row.uri == "spotify:album:a2")
            .unwrap();
        assert_eq!(album.kind, "album");
        assert_eq!(album.detail, "Singer, Guest");
        // So does one from a playlist Spotify won't name.
        assert!(uris(&rows).contains(&"spotify:album:a4"));
        assert!(!uris(&rows).contains(&"spotify:playlist:p2"));
        // An album played as itself is named from the track, unasked.
        let three = rows
            .iter()
            .find(|row| row.uri == "spotify:album:a3")
            .unwrap();
        assert_eq!(
            (three.name.as_str(), three.detail.as_str()),
            ("Three", "Trio")
        );
    }

    #[test]
    fn liked_songs_are_the_clients_own() {
        let (rows, _) = listed(&Value::Null);
        let liked = rows.iter().find(|row| row.kind == "collection").unwrap();
        assert_eq!(liked.uri, LIKED);
    }

    #[test]
    fn nine_at_most_and_nothing_without_a_context() {
        let (with_now, _) = listed(&fixture(include_str!("../fixtures/player.json")));
        assert_eq!(with_now.len(), MOST);
        assert!(!uris(&with_now).contains(&"spotify:album:a6"));
        let (lately, _) = listed(&Value::Null);
        assert_eq!(lately.len(), MOST);
        assert_eq!(lately.last().unwrap().uri, "spotify:album:a6");
        assert!(!uris(&lately).iter().any(|uri| uri.contains("local")));
    }

    #[test]
    fn a_line_holds_no_tab_but_its_own() {
        let (rows, _) = listed(&Value::Null);
        let album = rows
            .iter()
            .find(|row| row.uri == "spotify:album:a2")
            .unwrap();
        let line = album.line();
        assert_eq!(line.matches('\t').count(), 4);
        assert!(line.contains("\tTab Album Two\t"));
        assert!(!line.contains('\n'));
    }

    #[test]
    fn every_playlist_with_its_count_and_the_next_page() {
        let (rows, next) = playlists(&fixture(include_str!("../fixtures/playlists.json")));
        assert_eq!(
            rows.iter().map(Row::line).collect::<Vec<_>>(),
            [
                "spotify:playlist:p1\tplaylist\tFocus\tJoel\t42",
                "spotify:playlist:p3\tplaylist\tRoad trip\tSpotify\t7"
            ]
        );
        assert_eq!(
            next.as_deref(),
            Some("https://api.spotify.com/v1/me/playlists?offset=2&limit=50")
        );
    }

    #[test]
    fn only_what_spotify_plays_is_sent() {
        assert_eq!(
            playing("spotify:playlist:p1", "joel"),
            Some(serde_json::json!({ "context_uri": "spotify:playlist:p1" }))
        );
        assert_eq!(
            playing(LIKED, "joel"),
            Some(serde_json::json!({ "context_uri": "spotify:user:joel:collection" }))
        );
        assert_eq!(
            playing("spotify:track:t1", "joel"),
            Some(serde_json::json!({ "uris": ["spotify:track:t1"] }))
        );
        assert_eq!(playing("https://example.com", "joel"), None);
        assert_eq!(playing("spotify:playlist:", "joel"), None);
        assert_eq!(playing("spotify:playlist:p1:extra", "joel"), None);
    }
}

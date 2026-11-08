//! What Spotify played lately, as the launcher lists it: one row for each
//! playlist, album or show Joel played from, newest first, rather than one
//! for each track, since what he goes back to is what he was listening to
//! (VKCF3Z). Liked Songs and an artist are left out, each being an inside
//! of Spotify's program of its own, one step in, so Recent never lists
//! them a second time.

use serde_json::Value;

/// How many rows Spotify's Recent holds, as every program's does, so 1 to
/// 9 pick one.
pub const MOST: usize = 9;

/// Liked Songs as the client knows it, whoever's they are: Spotify names
/// the context `spotify:user:<id>:collection` in what it played, which
/// opens nothing in the client, so it's said this way and turned back
/// into the account's own as it's played (`playing`).
pub const LIKED: &str = "spotify:collection:tracks";

/// One row: what it is, as a Spotify URI and its kind, its name, who made
/// it, when it was last played, in RFC 3339, `playing` or `paused` for
/// what Spotify has now, or nothing for a row of a library, and how many
/// songs or episodes it holds, or nothing where Spotify doesn't count.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub uri: String,
    pub kind: String,
    pub name: String,
    pub detail: String,
    pub at: String,
    pub size: String,
}

impl Row {
    /// The row as a line: its fields apart by tabs, each with any tab or
    /// line break in it made a space, so a name can't move a field.
    pub fn line(&self) -> String {
        [
            &self.uri,
            &self.kind,
            &self.name,
            &self.detail,
            &self.at,
            &self.size,
        ]
        .map(|field| plain(field))
        .join("\t")
    }
}

/// A field with tabs and line breaks made spaces, trimmed.
pub fn plain(field: &str) -> String {
    field.replace(['\t', '\n', '\r'], " ").trim().to_owned()
}

/// A text field of `value`, or nothing.
pub fn text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A count of `value`'s as text, or nothing where Spotify doesn't give it.
fn count(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_u64)
        .map(|count| count.to_string())
        .unwrap_or_default()
}

/// An album, its own or a track's, as a row: its artists, as Spotify
/// credits them, in its order, and how many songs it holds. None for one
/// without a URI, as a local file's album is.
pub fn album(album: &Value) -> Option<Row> {
    let artists = album
        .get("artists")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|artist| artist.get("name")?.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Some(Row {
        uri: album.get("uri")?.as_str()?.to_owned(),
        kind: "album".to_owned(),
        name: text(album, "name"),
        detail: artists,
        at: String::new(),
        size: count(album.get("total_tracks")),
    })
}

/// A show, its own or an episode's, as a row: who publishes it and how
/// many episodes it has.
pub fn show(show: &Value) -> Option<Row> {
    Some(Row {
        uri: show.get("uri")?.as_str()?.to_owned(),
        kind: "show".to_owned(),
        name: text(show, "name"),
        detail: text(show, "publisher"),
        at: String::new(),
        size: count(show.get("total_episodes")),
    })
}

/// A playlist as a row: who made it, by the name Spotify shows for them,
/// and how many songs it holds. Spotify's answer counts them under
/// `items` since 2026, as spotify-player 0.25 reads them, and under
/// `tracks` before, so either is read.
pub fn playlist(list: &Value) -> Option<Row> {
    let size = ["items", "tracks"]
        .iter()
        .find_map(|field| list.get(field)?.get("total"))
        .map_or_else(String::new, |total| count(Some(total)));
    Some(Row {
        uri: list.get("uri")?.as_str()?.to_owned(),
        kind: "playlist".to_owned(),
        name: list.get("name")?.as_str()?.to_owned(),
        detail: list
            .get("owner")
            .map(|owner| text(owner, "display_name"))
            .unwrap_or_default(),
        at: String::new(),
        size,
    })
}

/// One thing played: where Spotify says it was played from, if it says,
/// and what it's part of on its own, a track's album or an episode's
/// show, which stands in for one played from nowhere Spotify names, as a
/// track played from search is.
#[derive(Clone, Debug)]
pub struct Play {
    context: Option<(String, String)>,
    own: Option<Row>,
    at: String,
}

/// The plays in what Spotify has now, `player` (/me/player, asked for
/// episodes too, or null when it has nothing), then in what it played
/// lately, `recently` (/me/player/recently-played), newest first. Spotify
/// lists only tracks as played lately, so a show is known only while it
/// plays or is paused.
pub fn plays(player: &Value, recently: &Value) -> Vec<Play> {
    let at = if player.get("is_playing").and_then(Value::as_bool) == Some(true) {
        "playing"
    } else {
        "paused"
    };
    let now = player
        .get("item")
        .and_then(|item| match item.get("type").and_then(Value::as_str) {
            Some("track") => Some(play(player.get("context"), item, at)),
            Some("episode") => Some(Play {
                context: None,
                own: item.get("show").and_then(show),
                at: at.to_owned(),
            }),
            _ => None,
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
    let context = context
        .filter(|context| !context.is_null())
        .map(|context| (text(context, "type"), text(context, "uri")))
        .filter(|(kind, uri)| !kind.is_empty() && !uri.is_empty());
    Play {
        context,
        own: track.get("album").and_then(album),
        at: at.to_owned(),
    }
}

/// The rows for `plays`, one a playlist, album or show, newest first, at
/// most nine. A playlist's name isn't in what Spotify played, so `named`
/// asks it, once a context, giving its row, or none where Spotify won't
/// say, as for a playlist it made for Joel and no longer shows to other
/// apps; that play falls back to its album, as one played from nowhere
/// does, so it's still listed and still plays. A play from Liked Songs or
/// an artist is left out, its home being inside, and so is one with
/// nothing to fall back to, a local file's, as nothing plays it.
pub fn rows<E>(
    plays: &[Play],
    mut named: impl FnMut(&str, &str) -> Result<Option<Row>, E>,
) -> Result<Vec<Row>, E> {
    let mut rows: Vec<Row> = Vec::new();
    let mut unnamed: Vec<String> = Vec::new();
    for play in plays {
        if rows.len() == MOST {
            break;
        }
        let row = match &play.context {
            Some((kind, _)) if matches!(kind.as_str(), "collection" | "artist") => None,
            Some((kind, uri)) if matches!(kind.as_str(), "playlist" | "album" | "show") => {
                if rows.iter().any(|row| &row.uri == uri) {
                    continue;
                }
                let own = play.own.clone().filter(|own| &own.uri == uri);
                let found = match own {
                    Some(own) => Some(own),
                    None if unnamed.contains(uri) => None,
                    None => named(kind, uri)?,
                };
                if found.is_none() {
                    unnamed.push(uri.clone());
                }
                found.or_else(|| play.own.clone())
            }
            _ => play.own.clone(),
        };
        let Some(row) = row else { continue };
        if !rows.iter().any(|kept| kept.uri == row.uri) {
            rows.push(Row {
                at: play.at.clone(),
                ..row
            });
        }
    }
    Ok(rows)
}

/// What `PUT /me/player/play` is given to play `uri`: a playlist, album,
/// artist or show as its context, a track or an episode alone, and Liked
/// Songs as the account's own collection, `account` being Joel's id; or
/// none for anything Spotify can't play, so nothing is sent.
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

    /// A playlist as Spotify names it when asked.
    fn named(name: &str, owner: &str, size: &str) -> Row {
        Row {
            uri: String::new(),
            kind: "playlist".into(),
            name: name.into(),
            detail: owner.into(),
            at: String::new(),
            size: size.into(),
        }
    }

    /// The fixture's plays named as a stand-in would name them: the two
    /// playlists Spotify names, the second playlist one it won't, noting
    /// each one asked.
    fn listed(player: &Value) -> (Vec<Row>, Vec<String>) {
        let mut asked = Vec::new();
        let given = plays(
            player,
            &fixture(include_str!("../fixtures/recently-played.json")),
        );
        let rows = rows(&given, |kind, uri| {
            asked.push(uri.to_owned());
            let found = match (kind, uri) {
                ("playlist", "spotify:playlist:p1") => Some(named("Focus", "Joel", "46")),
                ("playlist", "spotify:playlist:p3") => Some(named("Road", "Spotify", "50")),
                ("playlist", "spotify:playlist:p9") => Some(named("Morning", "Joel", "12")),
                _ => None,
            };
            Ok::<_, ()>(found.map(|row| Row {
                uri: uri.to_owned(),
                ..row
            }))
        })
        .unwrap();
        (rows, asked)
    }

    fn uris(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|row| row.uri.as_str()).collect()
    }

    fn find<'a>(rows: &'a [Row], uri: &str) -> &'a Row {
        rows.iter().find(|row| row.uri == uri).unwrap()
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
    fn a_show_playing_leads_by_its_own_name_and_episodes() {
        let (rows, asked) = listed(&fixture(include_str!("../fixtures/player-episode.json")));
        assert_eq!(
            rows[0],
            Row {
                uri: "spotify:show:s1".into(),
                kind: "show".into(),
                name: "The Show".into(),
                detail: "A Publisher".into(),
                at: "playing".into(),
                size: "212".into(),
            }
        );
        assert!(!asked.iter().any(|uri| uri.contains("show")));
    }

    #[test]
    fn each_row_says_how_many_it_holds() {
        let (rows, _) = listed(&Value::Null);
        assert_eq!(find(&rows, "spotify:playlist:p1").size, "46");
        // An album says it from the track played, unasked.
        let three = find(&rows, "spotify:album:a3");
        assert_eq!(
            (
                three.name.as_str(),
                three.detail.as_str(),
                three.size.as_str()
            ),
            ("Three", "Trio", "10")
        );
        // A track's album that doesn't count its songs says nothing.
        assert_eq!(find(&rows, "spotify:album:a2").size, "");
    }

    #[test]
    fn a_track_from_nowhere_falls_back_to_its_album() {
        let (rows, _) = listed(&Value::Null);
        let album = find(&rows, "spotify:album:a2");
        assert_eq!(album.kind, "album");
        assert_eq!(album.detail, "Singer, Guest");
        // So does one from a playlist Spotify won't name.
        assert!(uris(&rows).contains(&"spotify:album:a4"));
        assert!(!uris(&rows).contains(&"spotify:playlist:p2"));
    }

    #[test]
    fn liked_songs_and_artists_are_left_inside() {
        let (rows, asked) = listed(&Value::Null);
        assert!(
            rows.iter()
                .all(|row| matches!(row.kind.as_str(), "playlist" | "album" | "show"))
        );
        assert!(!uris(&rows).iter().any(|uri| uri.contains("collection")));
        assert!(!uris(&rows).contains(&"spotify:artist:r1"));
        // Nor is either's album listed in its place.
        assert!(!uris(&rows).contains(&"spotify:album:a9"));
        assert!(!uris(&rows).contains(&"spotify:album:a10"));
        assert!(!asked.iter().any(|uri| uri.contains("artist")));
    }

    #[test]
    fn nine_at_most_and_nothing_without_a_context() {
        let (with_now, _) = listed(&fixture(include_str!("../fixtures/player.json")));
        assert_eq!(with_now.len(), MOST);
        assert!(!uris(&with_now).contains(&"spotify:album:a8"));
        let (lately, _) = listed(&Value::Null);
        assert_eq!(lately.len(), MOST);
        assert_eq!(lately.last().unwrap().uri, "spotify:album:a8");
        assert!(!uris(&lately).iter().any(|uri| uri.contains("local")));
    }

    #[test]
    fn a_line_holds_no_tab_but_its_own() {
        let (rows, _) = listed(&Value::Null);
        let line = find(&rows, "spotify:album:a2").line();
        assert_eq!(line.matches('\t').count(), 5);
        assert!(line.contains("\tTab Album Two\t"));
        assert!(!line.contains('\n'));
    }

    #[test]
    fn only_what_spotify_plays_is_sent() {
        assert_eq!(
            playing("spotify:playlist:p1", "joel"),
            Some(serde_json::json!({ "context_uri": "spotify:playlist:p1" }))
        );
        assert_eq!(
            playing("spotify:show:s1", "joel"),
            Some(serde_json::json!({ "context_uri": "spotify:show:s1" }))
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

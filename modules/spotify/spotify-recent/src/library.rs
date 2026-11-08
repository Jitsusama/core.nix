//! What Joel keeps in Spotify, each list an inside of Spotify's program in
//! the launcher, one step in (VKCF3Z): his playlists, the albums and shows
//! he saved, and the artists he follows, each a row as Recent's are, in
//! the order Spotify gives them, his own order for playlists and newest
//! saved first for the rest.

use crate::recent::{self, Row, text};
use serde_json::Value;

/// One of the lists, by what it holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Library {
    Playlists,
    Albums,
    Artists,
    Shows,
}

impl Library {
    /// The list asked by its name on spotify-recent's command line.
    pub fn named(name: &str) -> Option<Library> {
        match name {
            "playlists" => Some(Library::Playlists),
            "albums" => Some(Library::Albums),
            "artists" => Some(Library::Artists),
            "shows" => Some(Library::Shows),
            _ => None,
        }
    }

    /// Its first page, as Spotify's API names it, fifty at a time, the
    /// most it gives.
    pub fn first(self) -> &'static str {
        match self {
            Library::Playlists => "/me/playlists?limit=50",
            Library::Albums => "/me/albums?limit=50",
            Library::Artists => "/me/following?type=artist&limit=50",
            Library::Shows => "/me/shows?limit=50",
        }
    }

    /// The rows on one page of it, and the page after it, if any. The
    /// artists Joel follows come a level down, under `artists`, as
    /// Spotify pages them by a cursor rather than by an offset; a saved
    /// album or show comes under its own name, beside when it was saved.
    pub fn page(self, page: &Value) -> (Vec<Row>, Option<String>) {
        let page = match self {
            Library::Artists => page.get("artists").unwrap_or(&Value::Null),
            _ => page,
        };
        let rows = page
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| match self {
                Library::Playlists => recent::playlist(item),
                Library::Albums => recent::album(item.get("album")?),
                Library::Artists => artist(item),
                Library::Shows => recent::show(item.get("show")?),
            })
            .collect();
        let next = page.get("next").and_then(Value::as_str).map(String::from);
        (rows, next)
    }
}

/// An artist Joel follows as a row, by name alone: Spotify counts no
/// songs of an artist, and their followers and popularity say nothing
/// Joel would play by.
fn artist(artist: &Value) -> Option<Row> {
    Some(Row {
        uri: artist.get("uri")?.as_str()?.to_owned(),
        kind: "artist".to_owned(),
        name: text(artist, "name"),
        detail: String::new(),
        at: String::new(),
        size: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(library: Library, text: &str) -> (Vec<String>, Option<String>) {
        let (rows, next) = library.page(&serde_json::from_str(text).unwrap());
        (rows.iter().map(Row::line).collect(), next)
    }

    #[test]
    fn every_playlist_with_its_count_and_the_next_page() {
        let (rows, next) = lines(
            Library::Playlists,
            include_str!("../fixtures/playlists.json"),
        );
        assert_eq!(
            rows,
            [
                "spotify:playlist:p1\tplaylist\tFocus\tJoel\t\t42",
                "spotify:playlist:p3\tplaylist\tRoad trip\tSpotify\t\t7"
            ]
        );
        assert_eq!(
            next.as_deref(),
            Some("https://api.spotify.com/v1/me/playlists?offset=2&limit=50")
        );
    }

    #[test]
    fn every_saved_album_with_its_artists_and_songs() {
        let (rows, next) = lines(Library::Albums, include_str!("../fixtures/albums.json"));
        assert_eq!(
            rows,
            [
                "spotify:album:b1\talbum\tKid A\tRadiohead\t\t10",
                "spotify:album:b2\talbum\tTogether\tSinger, Guest\t\t"
            ]
        );
        assert_eq!(next, None);
    }

    #[test]
    fn every_artist_followed_a_level_down() {
        let (rows, next) = lines(Library::Artists, include_str!("../fixtures/artists.json"));
        assert_eq!(rows, ["spotify:artist:r1\tartist\tArtist\t\t\t"]);
        assert_eq!(
            next.as_deref(),
            Some("https://api.spotify.com/v1/me/following?type=artist&after=r1&limit=50")
        );
    }

    #[test]
    fn every_saved_show_with_its_publisher_and_episodes() {
        let (rows, _) = lines(Library::Shows, include_str!("../fixtures/shows.json"));
        assert_eq!(
            rows,
            ["spotify:show:s1\tshow\tThe Show\tA Publisher\t\t212"]
        );
    }

    #[test]
    fn each_list_is_asked_by_its_name() {
        for name in ["playlists", "albums", "artists", "shows"] {
            assert!(Library::named(name).is_some(), "{name}");
        }
        assert_eq!(Library::named("liked"), None);
    }
}

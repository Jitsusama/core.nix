//! The one link a meeting is joined by, and which kind of call it is.
//!
//! A meeting made in Google Calendar carries its Meet call as a conference
//! with a video entry point, and an older one as `hangoutLink`. One made
//! elsewhere, or for a Zoom or Tuple call, carries the link in its
//! location or description, often among other links (an agenda, a doc),
//! so only a call's own links count there, the first one found.

use serde::Serialize;
use serde_json::Value;

/// Which call a link joins, so the shell can join a Tuple call through
/// Tuple and say what each is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Meet,
    Tuple,
    Zoom,
    /// A conference Google holds the link of that is none of the others.
    Other,
}

/// A meeting's link and its kind, from the event as the API gives it.
pub fn of(event: &Value) -> Option<(String, Kind)> {
    let video = event
        .pointer("/conferenceData/entryPoints")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|point| point.get("entryPointType").and_then(Value::as_str) == Some("video"))
        .and_then(|point| point.get("uri"))
        .and_then(Value::as_str);
    let given = video.or(event.get("hangoutLink").and_then(Value::as_str));
    if let Some(uri) = given {
        return Some((uri.to_owned(), kind(uri).unwrap_or(Kind::Other)));
    }
    ["location", "description"]
        .iter()
        .filter_map(|field| event.get(*field).and_then(Value::as_str))
        .find_map(first_call)
}

/// The first call's link in some text, which may be HTML.
fn first_call(text: &str) -> Option<(String, Kind)> {
    text.match_indices("https://").find_map(|(at, _)| {
        let rest = &text[at..];
        let end = rest
            .find(|c: char| c.is_whitespace() || "\"'<>()[]".contains(c))
            .unwrap_or(rest.len());
        let url = rest[..end].trim_end_matches(['.', ',', ';']);
        kind(url).map(|kind| (url.to_owned(), kind))
    })
}

/// The kind of call a link is, by its host, or none for any other link.
fn kind(url: &str) -> Option<Kind> {
    let host = url
        .strip_prefix("https://")?
        .split(['/', '?', '#'])
        .next()?
        .to_ascii_lowercase();
    let path = url["https://".len() + host.len()..].to_owned();
    if host == "meet.google.com" && path.len() > 1 {
        Some(Kind::Meet)
    } else if (host == "tuple.app" || host == "production.tuple.app") && path.starts_with("/c/") {
        Some(Kind::Tuple)
    } else if (host == "zoom.us" || host.ends_with(".zoom.us")) && path.starts_with("/j/") {
        Some(Kind::Zoom)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_conference_video_entry_point_comes_first() {
        let event = json!({
            "hangoutLink": "https://meet.google.com/old-link",
            "conferenceData": {"entryPoints": [
                {"entryPointType": "phone", "uri": "tel:+1-555"},
                {"entryPointType": "video", "uri": "https://meet.google.com/abc-defg-hij"}
            ]},
            "description": "https://zoom.us/j/123"
        });
        assert_eq!(
            of(&event),
            Some(("https://meet.google.com/abc-defg-hij".into(), Kind::Meet))
        );
    }

    #[test]
    fn a_hangout_link_stands_in_for_a_conference() {
        let event = json!({"hangoutLink": "https://meet.google.com/abc-defg-hij"});
        assert_eq!(of(&event).unwrap().1, Kind::Meet);
    }

    #[test]
    fn another_conference_is_kept_as_other() {
        let event = json!({"conferenceData": {"entryPoints": [
            {"entryPointType": "video", "uri": "https://teams.example/join/1"}
        ]}});
        assert_eq!(of(&event).unwrap().1, Kind::Other);
    }

    #[test]
    fn a_call_in_the_description_is_found_among_other_links() {
        let event = json!({
            "location": "Room 4",
            "description": "<a href=\"https://docs.example/agenda\">Agenda</a> \
                Join: <a href=\"https://tuple.app/c/joels-room\">https://tuple.app/c/joels-room</a>."
        });
        assert_eq!(
            of(&event),
            Some(("https://tuple.app/c/joels-room".into(), Kind::Tuple))
        );
    }

    #[test]
    fn the_location_is_read_before_the_description() {
        let event = json!({
            "location": "https://example.zoom.us/j/987?pwd=x,",
            "description": "https://meet.google.com/abc-defg-hij"
        });
        assert_eq!(
            of(&event),
            Some(("https://example.zoom.us/j/987?pwd=x".into(), Kind::Zoom))
        );
    }

    #[test]
    fn links_that_are_not_calls_are_not_joined() {
        let event = json!({
            "description": "https://meet.google.com/ https://tuple.app/download https://zoom.us/pricing"
        });
        assert_eq!(of(&event), None);
        assert_eq!(of(&json!({})), None);
    }
}

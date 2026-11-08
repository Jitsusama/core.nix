//! A Meet call Joel is in, as Meet's REST API says it: when its
//! conference began, and who has been in it, each since when and, once
//! gone, until when. The shell names the calls open in windows of their
//! own (ask.rs's `follow`), and each is asked of Meet while it's named
//! (main.rs), so the launcher's call says its people (VKCF3Z's design,
//! 2026-11-08). Reading them needs the `meetings.space.readonly` scope,
//! which a sign-in from before it was asked for lacks, so a call says
//! whether it was granted rather than guessing at nobody.

use serde::Serialize;
use serde_json::Value;

/// One call, by its meeting code.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Call {
    pub code: String,
    /// Whether Joel's sign-in lets this program read who's in calls.
    pub granted: bool,
    /// Whether Meet gave a record of it under way: before it answers, or
    /// where it keeps none Joel may read, as it may not for a guest, who's
    /// in it isn't known, which is never the same as nobody.
    pub known: bool,
    /// When its conference began, in milliseconds since 1970, or none
    /// until Meet says, or while there's no conference under way.
    pub started: Option<i64>,
    /// Everyone in it or who was, by when they first joined.
    pub people: Vec<Person>,
}

/// Someone in a call, by the name Meet shows them by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub name: String,
    /// When they first joined, in milliseconds since 1970.
    pub joined: i64,
    /// When they last left, or none while they're still in.
    pub left: Option<i64>,
}

/// Whether a meeting code is one Meet could have made: letters and the
/// dashes between them, as abc-defg-hij, since it goes into a filter.
pub fn is_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 64
        && code.chars().all(|c| c.is_ascii_lowercase() || c == '-')
        && !code.starts_with('-')
}

/// A time Meet writes, RFC 3339, in milliseconds since 1970.
fn millis(said: Option<&Value>) -> Option<i64> {
    let text = said?.as_str()?;
    let at: jiff::Timestamp = text.parse().ok()?;
    Some(at.as_millisecond())
}

/// The conference under way of a list of conference records, its name,
/// which participants are asked by, and when it began: the one not ended,
/// else none.
pub fn under_way(records: &Value) -> Option<(String, Option<i64>)> {
    let records = records.get("conferenceRecords")?.as_array()?;
    let record = records
        .iter()
        .find(|record| record.get("endTime").is_none())?;
    let name = record.get("name")?.as_str()?.to_owned();
    Some((name, millis(record.get("startTime"))))
}

/// Everyone a list of participants names, by when they first joined. Meet
/// keeps a signed-in person's, an anonymous one's and a phone's name under
/// a field for each.
pub fn people(said: &Value) -> Vec<Person> {
    let mut people: Vec<Person> = said
        .get("participants")
        .and_then(Value::as_array)
        .map(|all| {
            all.iter()
                .filter_map(|one| {
                    let name = ["signedinUser", "anonymousUser", "phoneUser"]
                        .iter()
                        .find_map(|kind| one.get(*kind)?.get("displayName")?.as_str())
                        .filter(|name| !name.is_empty())?;
                    Some(Person {
                        name: name.to_owned(),
                        joined: millis(one.get("earliestStartTime"))?,
                        left: millis(one.get("latestEndTime")),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    people.sort_by_key(|person| person.joined);
    people
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_code_is_letters_and_dashes() {
        assert!(is_code("abc-defg-hij"));
        assert!(!is_code(""));
        assert!(!is_code("abc\" OR end_time IS NULL"));
        assert!(!is_code("-abc"));
    }

    #[test]
    fn the_conference_under_way_is_the_one_not_ended() {
        let records = json!({"conferenceRecords": [
            {"name": "conferenceRecords/old", "startTime": "2026-11-08T14:00:00Z",
             "endTime": "2026-11-08T14:30:00Z"},
            {"name": "conferenceRecords/now", "startTime": "2026-11-08T15:00:00.5Z"}
        ]});
        assert_eq!(
            under_way(&records),
            Some(("conferenceRecords/now".into(), Some(1_794_150_000_500)))
        );
        assert_eq!(under_way(&json!({})), None, "no call under way");
    }

    #[test]
    fn everyone_is_named_by_when_they_joined_and_those_gone_say_when() {
        let said = json!({"participants": [
            {"signedinUser": {"displayName": "Grace Hopper"},
             "earliestStartTime": "2026-11-08T15:02:00Z"},
            {"anonymousUser": {"displayName": "A Guest"},
             "earliestStartTime": "2026-11-08T15:01:00Z",
             "latestEndTime": "2026-11-08T15:15:00Z"},
            {"phoneUser": {"displayName": "+1 555"},
             "earliestStartTime": "2026-11-08T15:03:00Z"},
            {"signedinUser": {}, "earliestStartTime": "2026-11-08T15:04:00Z"}
        ]});
        let people = people(&said);
        let names: Vec<&str> = people.iter().map(|person| person.name.as_str()).collect();
        assert_eq!(names, ["A Guest", "Grace Hopper", "+1 555"]);
        assert_eq!(people[0].left, Some(1_794_150_900_000));
        assert_eq!(people[1].left, None, "still in");
    }
}

//! Mail that Calendar sent about a meeting: an invitation, or someone's
//! answer to one. Calendar writes its subject as "Accepted: Joel / Chad
//! 1:1 @ Every 2 weeks from 11:30am to 12pm on Monday (AST) (Chad
//! Cromwell)", which wraps badly as a row's name (VKCF3Z, through XY9H5C
//! c07ccf58), so it's read here into the meeting, what became of it and
//! when it is: a row names the meeting, says its answer beside it and
//! keeps when it is for its drawer.

use serde::Serialize;
use serde_json::Value;

/// What Calendar's mail says became of the meeting, by the start of its
/// subject, as Calendar writes it in English.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    /// Joel was asked to it.
    Invited,
    /// It changed after he was asked.
    Updated,
    /// Someone's going.
    Accepted,
    /// Someone might go.
    Tentative,
    /// Someone isn't going.
    Declined,
    /// It won't happen.
    Cancelled,
}

/// How each answer's subject starts, longest first where one starts
/// another, so "Updated invitation with note" isn't read as "Updated
/// invitation" with a meeting named "with note".
const STARTS: [(&str, Answer); 11] = [
    ("Updated invitation with note: ", Answer::Updated),
    ("Updated invitation: ", Answer::Updated),
    ("Invitation with note: ", Answer::Invited),
    ("Invitation: ", Answer::Invited),
    ("New event: ", Answer::Invited),
    ("Accepted: ", Answer::Accepted),
    ("Tentatively accepted: ", Answer::Tentative),
    ("Declined: ", Answer::Declined),
    ("Canceled event with note: ", Answer::Cancelled),
    ("Canceled event: ", Answer::Cancelled),
    ("Cancelled event: ", Answer::Cancelled),
];

/// A meeting Calendar's mail is about, as the shell is told it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Invitation {
    pub answer: Answer,
    /// The meeting's title.
    pub meeting: String,
    /// When it is, as Calendar words it: "Every 2 weeks from 11:30am to
    /// 12pm on Monday (AST)".
    pub when: String,
    /// Who the subject names last: whoever answered, or whom an
    /// invitation went to; empty when it names nobody.
    pub who: String,
}

/// The meeting a subject is about, or none for mail that isn't
/// Calendar's: it needs one of Calendar's starts and a meeting before " @
/// ", so "Accepted: the offer" stays mail.
#[must_use]
pub fn read(subject: &str) -> Option<Invitation> {
    let (rest, answer) = STARTS.iter().find_map(|(start, answer)| {
        let head = subject.get(..start.len())?;
        head.eq_ignore_ascii_case(start)
            .then(|| (&subject[start.len()..], *answer))
    })?;
    let (about, who) = named_last(rest.trim());
    let (meeting, when) = about.rsplit_once(" @ ")?;
    let meeting = meeting.trim();
    if meeting.is_empty() {
        return None;
    }
    Some(Invitation {
        answer,
        meeting: meeting.to_owned(),
        when: when.trim().to_owned(),
        who: who.to_owned(),
    })
}

/// The subject without the brackets it ends on, and what they hold: the
/// last bracket's own, so "(AST) (Chad Cromwell)" gives Chad, and "(AST)"
/// alone, which follows a time, gives nobody.
fn named_last(rest: &str) -> (&str, &str) {
    let Some(open) = rest.strip_suffix(')').and_then(|inner| inner.rfind('(')) else {
        return (rest, "");
    };
    let before = rest[..open].trim_end();
    // A time's zone follows its clock; a name follows the zone's bracket.
    if !before.ends_with(')') {
        return (rest, "");
    }
    (before, &rest[open + 1..rest.len() - 1])
}

/// The words a meeting's mail is said in, from the launcher's vocabulary:
/// each answer as its row says it beside the meeting, and the name of the
/// drawer's fact for when it is, Calendar's own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    pub invited: String,
    pub updated: String,
    pub accepted: String,
    pub tentative: String,
    pub declined: String,
    pub cancelled: String,
    pub when: String,
}

impl Words {
    /// Its words from the whole vocabulary, or the first it lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let word = |id: &str, at: &[&str]| -> Result<String, String> {
            let thing = vocabulary["things"]
                .as_array()
                .and_then(|things| things.iter().find(|thing| thing["id"] == id))
                .ok_or_else(|| format!("the vocabulary has no {id}"))?;
            at.iter()
                .fold(thing, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        let said = |answer: &str| word("mail:message", &["said", answer]);
        Ok(Words {
            invited: said("invited")?,
            updated: said("updated")?,
            accepted: said("accepted")?,
            tentative: said("tentative")?,
            declined: said("declined")?,
            cancelled: said("cancelled")?,
            when: word("calendar:when", &["name"])?,
        })
    }

    /// How a row says this answer beside its meeting.
    #[must_use]
    pub fn said(&self, answer: Answer) -> &str {
        match answer {
            Answer::Invited => &self.invited,
            Answer::Updated => &self.updated,
            Answer::Accepted => &self.accepted,
            Answer::Tentative => &self.tentative,
            Answer::Declined => &self.declined,
            Answer::Cancelled => &self.cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::rows;

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's what read gives, to compare with"
    )]
    fn invitation(answer: Answer, meeting: &str, when: &str, who: &str) -> Option<Invitation> {
        Some(Invitation {
            answer,
            meeting: meeting.into(),
            when: when.into(),
            who: who.into(),
        })
    }

    #[test]
    fn an_answer_reads_as_its_meeting_when_it_is_and_who_answered() {
        assert_eq!(
            read(
                "Accepted: Joel / Chad 1:1 @ Every 2 weeks from 11:30am to 12pm on Monday \
                 (AST) (Chad Cromwell)"
            ),
            invitation(
                Answer::Accepted,
                "Joel / Chad 1:1",
                "Every 2 weeks from 11:30am to 12pm on Monday (AST)",
                "Chad Cromwell"
            ),
            "Calendar's answer wasn't read as its meeting"
        );
        assert_eq!(
            read("Declined: Planning @ Thu 12 Nov 2026 2pm - 3pm (AST) (ada@example.com)"),
            invitation(
                Answer::Declined,
                "Planning",
                "Thu 12 Nov 2026 2pm - 3pm (AST)",
                "ada@example.com"
            ),
            "a decline wasn't read with who declined by address"
        );
        assert_eq!(
            read("Tentatively accepted: Planning @ Thu 12 Nov 2026 2pm - 3pm (AST) (Ada)")
                .map(|read| read.answer),
            Some(Answer::Tentative),
            "a maybe wasn't read as one"
        );
    }

    #[test]
    fn an_invitation_reads_whichever_way_calendar_starts_it() {
        for (subject, answer) in [
            ("Invitation: Planning @ Thu (AST) (Joel)", Answer::Invited),
            (
                "Updated invitation: Planning @ Thu (AST) (Joel)",
                Answer::Updated,
            ),
            (
                "Updated invitation with note: Planning @ Thu (AST) (Joel)",
                Answer::Updated,
            ),
            (
                "Canceled event: Planning @ Thu (AST) (Joel)",
                Answer::Cancelled,
            ),
            (
                "Cancelled event: Planning @ Thu (AST) (Joel)",
                Answer::Cancelled,
            ),
            ("invitation: Planning @ Thu (AST) (Joel)", Answer::Invited),
        ] {
            let read = read(subject);
            assert_eq!(
                read.as_ref()
                    .map(|read| (read.answer, read.meeting.as_str())),
                Some((answer, "Planning")),
                "{subject} wasn't read as Planning"
            );
        }
    }

    #[test]
    fn a_meeting_named_with_an_at_or_brackets_keeps_its_name() {
        assert_eq!(
            read("Accepted: Lunch @ the café (upstairs) @ Fri 13 Nov 2026 12pm (AST) (Ada)"),
            invitation(
                Answer::Accepted,
                "Lunch @ the café (upstairs)",
                "Fri 13 Nov 2026 12pm (AST)",
                "Ada"
            ),
            "a meeting's own @ or brackets were taken as when it is"
        );
    }

    #[test]
    fn mail_that_isnt_calendars_is_no_invitation() {
        for subject in [
            "Accepted: the offer",
            "Re: Accepted: Planning @ Thu (AST) (Ada)",
            "Lunch @ noon?",
            "Accepted:  @ Thu (AST) (Ada)",
            "",
        ] {
            assert_eq!(read(subject), None, "{subject:?} was read as an invitation");
        }
    }

    #[test]
    fn an_answers_row_is_named_by_its_meeting_with_its_answer_and_when_it_is() {
        let answer = json!({"id": "m1", "threadId": "t1", "internalDate": "1000",
        "labelIds": ["INBOX", "UNREAD"], "payload": {"headers": [
            {"name": "Subject", "value": "Accepted: Joel / Chad 1:1 @ Every 2 weeks (AST) (Chad)"},
            {"name": "From", "value": "Chad <chad@example.com>"}
        ]}});
        let mail = crate::mail::read(&answer).expect("the answer wasn't read as mail");
        let words = rows::Words::read(&rows::marked::vocabulary()).expect("words didn't read");
        let row = &rows::mail(std::slice::from_ref(&mail), &words)[0];
        assert_eq!(
            row["name"], "Joel / Chad 1:1",
            "the row wasn't named by its meeting: {row}"
        );
        let side: Vec<&Value> = row["side"].as_array().into_iter().flatten().collect();
        assert_eq!(
            side.iter().map(|item| item.get("said")).collect::<Vec<_>>(),
            [Some(&json!("Chad")), Some(&json!("accepted")), None],
            "its side didn't say who, the answer, then when: {side:?}"
        );
        let drawers = rows::mail_drawers(std::slice::from_ref(&mail), &[], &words);
        let drawer = &drawers[0].1;
        assert_eq!(
            (&drawer["title"], &drawer["facts"][0]),
            (
                &json!("Joel / Chad 1:1"),
                &json!({"name": "When", "said": "Every 2 weeks (AST)"})
            ),
            "its drawer didn't say when its meeting is first: {drawer}"
        );
    }

    #[test]
    fn a_subject_naming_nobody_keeps_its_zone_with_when_it_is() {
        assert_eq!(
            read("Invitation: Planning @ Thu 12 Nov 2026 (AST)"),
            invitation(Answer::Invited, "Planning", "Thu 12 Nov 2026 (AST)", ""),
            "a zone's bracket was taken as who it went to"
        );
    }
}

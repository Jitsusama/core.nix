//! Every word Calendar's rows say is the vocabulary's: the rows and drawers
//! of meetings of every shape are made in a vocabulary with each of its
//! words marked, and each word they show must carry the mark but those the
//! meetings say themselves and the clock's figures. A word written into
//! `calendar_rows.rs` shows unmarked and fails here, naming where it shows.

use serde_json::Value;

use super::tests::{HOUR, MINUTE, MONDAY, one, zone};
use super::{Words, drawers, meetings};
use crate::calendar::{Answer, Attachment, Dial, Every, Guest, Meeting, Person, Repeats, Weekday};
use crate::rows::marked::{marked, vocabulary, written_in};

fn guest(name: &str, answer: Answer) -> Guest {
    Guest {
        name: name.into(),
        email: Some(format!("{}@example.com", name.to_lowercase())),
        answer,
    }
}

fn plain(day: u8) -> Weekday {
    Weekday { day, nth: None }
}

/// A rule every `interval` of `every`, with nothing more said.
fn rule(every: Every, interval: u32) -> Repeats {
    Repeats {
        every,
        interval,
        days: vec![],
        month_days: vec![],
        times: None,
        until: None,
    }
}

/// Every way a meeting can repeat: weekdays, on days, on which of a day,
/// on days of the month from either end, so many times and until a day.
fn repeats() -> Vec<Repeats> {
    let mut weekdays = rule(Every::Week, 1);
    weekdays.days = (1..=5).map(plain).collect();
    let mut fortnightly = rule(Every::Week, 2);
    fortnightly.days = vec![plain(1), plain(3), plain(5)];
    fortnightly.until = Some(1_798_747_200_000);
    let mut monthly = rule(Every::Month, 1);
    monthly.days = vec![
        Weekday {
            day: 5,
            nth: Some(-1),
        },
        Weekday {
            day: 2,
            nth: Some(-2),
        },
        Weekday {
            day: 4,
            nth: Some(7),
        },
    ];
    monthly.times = Some(4);
    let mut quarterly = rule(Every::Month, 3);
    quarterly.month_days = vec![1, 2, 3, 11, 22, -1, -3];
    vec![
        weekdays,
        fortnightly,
        monthly,
        quarterly,
        rule(Every::Day, 1),
        rule(Every::Year, 2),
    ]
}

/// One Joel was asked to by Ada, with `asked` guests answering each way.
fn invited(id: &str, start: i64, asked: usize) -> Meeting {
    let mut meeting = one(id, start);
    meeting.holds = false;
    meeting.through = Some("primary".into());
    meeting.answer = Some(Answer::Waiting);
    meeting.organizer = Some(Person {
        name: "Ada".into(),
        email: Some("ada@example.com".into()),
    });
    let answers = [Answer::Yes, Answer::Maybe, Answer::No, Answer::Waiting];
    meeting.guests = ["Grace", "Alan", "Edsger", "Barbara"]
        .iter()
        .zip(answers)
        .take(asked)
        .map(|(name, answer)| guest(name, answer))
        .collect();
    meeting
}

/// Meetings of every shape a row or drawer words: to come and held, each
/// answer, none to four others asked, held by Joel with none to three,
/// all day, untitled, with a dial and its PIN, with one and two files,
/// each way of repeating, a series held twice, and calls in Meet.
fn spread() -> (Vec<Meeting>, Vec<Meeting>) {
    let mut coming: Vec<Meeting> = (0..=4)
        .map(|asked| invited(&format!("asked-{asked}"), MONDAY + HOUR, asked))
        .collect();
    for (at, answer) in [Answer::Yes, Answer::Maybe, Answer::No]
        .into_iter()
        .enumerate()
    {
        let mut answered = invited(&format!("answered-{at}"), MONDAY + 2 * HOUR, 2);
        answered.answer = Some(answer);
        coming.push(answered);
    }
    for held in 0..=3 {
        let mut mine = invited(&format!("mine-{held}"), MONDAY + 3 * HOUR, held);
        mine.holds = true;
        mine.organizer = None;
        coming.push(mine);
    }
    let mut all_day = one("all-day", MONDAY + 10 * HOUR);
    all_day.all_day = true;
    all_day.through = Some("primary".into());
    let mut untitled = one("untitled", MONDAY + 4 * HOUR);
    untitled.title = String::new();
    let mut dialled = invited("dialled", MONDAY + 5 * HOUR, 1);
    dialled.link = Some("https://meet.google.com/abc-defg-hij".into());
    dialled.dial = Some(Dial {
        number: "+1 555-010-0199".into(),
        pin: Some("123456789".into()),
    });
    dialled.place = Some("Room".into());
    dialled.about = Some("Agenda".into());
    coming.extend([all_day, untitled, dialled]);
    for files in 1..=2 {
        let mut filed = one(&format!("files-{files}"), MONDAY + 6 * HOUR);
        filed.attachments = (0..files)
            .map(|at| Attachment {
                title: format!("File{at}"),
                link: format!("https://docs.google.com/document/d/file{at}"),
                id: Some(format!("file{at}")),
                kind: None,
            })
            .collect();
        coming.push(filed);
    }
    for (at, repeats) in repeats().into_iter().enumerate() {
        let mut series = one(&format!("series-{at}"), MONDAY + 7 * HOUR);
        series.repeats = Some(repeats);
        coming.push(series);
    }
    (coming, held())
}

/// Meetings held: a series twice, so its drawer counts one before, and
/// calls in Meet with nobody, one, two and three others.
fn held() -> Vec<Meeting> {
    let mut held = Vec::new();
    for (asked, room) in ["aaa", "bbb", "ccc", "ddd"].into_iter().enumerate() {
        let mut call = invited(&format!("call-{asked}"), MONDAY - HOUR - 30 * MINUTE, asked);
        call.organizer = None;
        call.link = Some(format!("https://meet.google.com/{room}-defg-hij"));
        held.push(call);
    }
    for (at, back) in [2, 26].into_iter().enumerate() {
        let mut weekly = one(&format!("weekly-{at}"), MONDAY - back * HOUR);
        weekly.series = Some("weekly".into());
        held.push(weekly);
    }
    held
}

/// What the meetings say themselves, which no vocabulary words.
fn their_own(meetings: &[Meeting]) -> Vec<String> {
    meetings
        .iter()
        .flat_map(|meeting| {
            let people = meeting
                .organizer
                .iter()
                .flat_map(|person| [Some(person.name.clone()), person.email.clone()])
                .chain(
                    meeting
                        .guests
                        .iter()
                        .flat_map(|guest| [Some(guest.name.clone()), guest.email.clone()]),
                );
            let files = meeting
                .attachments
                .iter()
                .map(|file| Some(file.title.clone()));
            [
                Some(meeting.title.clone()),
                meeting.link.clone(),
                meeting.page.clone(),
                meeting.place.clone(),
                meeting.about.clone(),
                meeting.dial.as_ref().map(|dial| dial.number.clone()),
            ]
            .into_iter()
            .chain(people)
            .chain(files)
            .flatten()
            .collect::<Vec<_>>()
        })
        .collect()
}

/// What a row or drawer shows Joel, each with where it shows.
fn shown(rows: &[Value], drawers: &[(String, Value)]) -> Vec<(String, String)> {
    let mut shown = Vec::new();
    let mut take = |at: String, value: &Value| {
        if let Some(said) = value.as_str() {
            shown.push((at, said.to_owned()));
        }
    };
    for row in rows {
        let key = row["key"].as_str().unwrap_or_default();
        take(format!("{key} name"), &row["name"]);
        take(format!("{key} note"), &row["note"]);
        for side in row["side"].as_array().into_iter().flatten() {
            take(format!("{key} side"), &side["said"]);
        }
        for act in row["acts"].as_array().into_iter().flatten() {
            let named = act["key"].as_str().unwrap_or_default();
            for field in ["sure", "telling", "side"] {
                take(format!("{key} {named} {field}"), &act[field]);
            }
        }
    }
    for (key, drawer) in drawers {
        take(format!("{key} eyebrow"), &drawer["eyebrow"]);
        take(format!("{key} title"), &drawer["title"]);
        for fact in drawer["facts"].as_array().into_iter().flatten() {
            for field in ["name", "said", "qualifier"] {
                take(format!("{key} fact {field}"), &fact[field]);
            }
        }
    }
    shown
}

#[test]
fn every_word_a_row_or_drawer_shows_is_the_vocabularys() {
    let words = Words::read(&marked(&vocabulary())).expect("the marked words didn't read");
    let (coming, held) = spread();
    let rows = meetings(&coming, &held, MONDAY, &zone(), &words);
    let drawn = drawers(&coming, &held, MONDAY, &zone(), &words);
    let own = their_own(&[coming, held].concat());
    let shown = shown(&rows, &drawn);
    assert!(
        shown.len() > 100,
        "only {} words were shown, so the spread reaches too little",
        shown.len()
    );
    let written = written_in(&shown, &own);
    assert!(
        written.is_empty(),
        "words written in calendar_rows.rs, not the vocabulary:\n{}",
        written.join("\n")
    );
}

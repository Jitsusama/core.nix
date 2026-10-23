//! A moment of the journey drawn as lines, top to bottom:
//!
//! - where the journey is, its name in strong and how far along, muted, at
//!   the right;
//! - every step done so far, each behind the done mark with what it
//!   showed, muted, the latest nearest the step at hand;
//! - the step, its title in strong;
//! - the one thing it asks now, said in text as what to do, with its field
//!   below, or how far along it is while it works;
//! - what stopped it, when something did, behind the mark of its kind;
//! - every key that works now and what it does, muted, at the foot.
//!
//! Everything sits two columns in from the edge, and as the screen gets
//! short the steps done go first, the earliest first, then how far along.
//! Something shown for a phone's camera is the one exception: it's alone
//! in the middle of the screen, since the camera copies whatever it sees.

use crate::Ask;
use crate::screen::{Key, Line, Role, Span, Theme, span};

/// Where in the journey the screen is.
pub struct Where<'a> {
    /// What the journey is, as the person would say it: "Install hedgehog".
    pub journey: &'a str,
    /// The step's number, counting from one, and how many there are, or
    /// none while the journey doesn't know yet.
    pub step: usize,
    pub steps: Option<usize>,
    /// What each step done so far showed, in the order they were done.
    pub done: &'a [String],
}

/// What's happening at a moment.
pub enum Moment<'a> {
    /// A step asking for one of the things it needs, with what's been
    /// typed so far, and why an earlier answer wasn't taken.
    Asking {
        title: &'a str,
        ask: &'a Ask,
        typed: &'a str,
        /// Which choice the cursor is on, for a choice.
        chosen: usize,
        refused: Option<&'a str>,
        /// Whether Escape goes back to the ask before.
        back: bool,
    },
    /// A step at work, what the person does meanwhile, if anything, and
    /// how far along it is.
    Working {
        title: &'a str,
        meanwhile: &'a str,
        progress: Option<&'a str>,
    },
    /// A step that can't run here and now, and what to do about it.
    Blocked {
        title: &'a str,
        why: &'a str,
        remedy: &'a str,
    },
    /// A step that ran and didn't finish, and why.
    Failed { title: &'a str, fault: &'a str },
    /// Something for the person's phone, alone on the screen.
    Showing { shown: &'a str },
    /// Every step done.
    Finished { said: &'a str },
    /// What comes next away from the guide, said before it happens, with
    /// what Return does: a reboot card.
    Card {
        title: &'a str,
        said: &'a str,
        then: &'a str,
    },
}

pub struct Drawn {
    pub lines: Vec<Line>,
    /// Where the caret goes, as a row and a column, when something's typed.
    pub caret: Option<(u16, u16)>,
}

const MARGIN: usize = 2;

/// Words wrapped to a width, never cut inside one unless it's longer than
/// the width itself, and then after a dash where it has one, so a recovery
/// key breaks between its groups.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = vec![];
    let mut line = String::new();
    for word in text.split_whitespace() {
        let fits = line.is_empty() || line.chars().count() + 1 + word.chars().count() <= width;
        if !fits {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
        while line.chars().count() > width {
            let chars: Vec<char> = line.chars().collect();
            let cut = chars[..width]
                .iter()
                .rposition(|c| *c == '-')
                .map_or(width, |dash| dash + 1);
            lines.push(chars[..cut].iter().collect());
            line = chars[cut..].iter().collect();
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn indent(spans: Vec<Span>) -> Line {
    let mut line = vec![span(Role::Text, " ".repeat(MARGIN))];
    line.extend(spans);
    line
}

/// Each wrapped line of a text in one role.
fn said(role: Role, text: &str, width: usize) -> Vec<Line> {
    wrap(text, width)
        .into_iter()
        .map(|words| indent(vec![span(role, words)]))
        .collect()
}

/// A line led by a mark, the lines it wraps to lined up under the mark's
/// ink rather than indented from it.
fn marked(mark: Span, role: Role, text: &str, width: usize) -> Vec<Line> {
    let lead = mark.text.chars().count() + 1;
    let mut lines = wrap(text, width.saturating_sub(lead)).into_iter();
    let mut drawn = vec![];
    if let Some(first) = lines.next() {
        drawn.push(indent(vec![mark, span(role, format!(" {first}"))]));
    }
    drawn.extend(lines.map(|rest| indent(vec![span(role, rest)])));
    drawn
}

fn keys(theme: &Theme, keys: &[(Key, &str)]) -> Line {
    let mut spans = vec![];
    for (n, (key, word)) in keys.iter().enumerate() {
        if n > 0 {
            spans.push(span(Role::Muted, "    "));
        }
        spans.push(span(Role::Muted, format!("{} {word}", theme.key(*key))));
    }
    indent(spans)
}

/// Lines in strong, each in the middle of the screen's width, the whole in
/// the middle of its height.
fn alone(shown: &str, width: usize, height: usize) -> Drawn {
    let shown = wrap(shown, width.saturating_sub(2 * MARGIN));
    let mut lines: Vec<Line> = vec![vec![]; height.saturating_sub(shown.len()) / 2];
    for words in shown {
        let lead = width.saturating_sub(words.chars().count()) / 2;
        lines.push(vec![
            span(Role::Text, " ".repeat(lead)),
            span(Role::Strong, words),
        ]);
    }
    Drawn { lines, caret: None }
}

/// A step's steps for the person, each a line of its own. Those that open
/// with their number wrap to lines lined up after it, unlike a mark's, so
/// where one step ends and the next starts is plain; the words before
/// them sit a line apart.
fn steps(how: &str, width: usize) -> Vec<Line> {
    let mut lines = vec![];
    let mut numbered = false;
    for paragraph in how.lines() {
        match paragraph.split_once(". ") {
            Some((number, rest)) if number.chars().all(|c| c.is_ascii_digit()) => {
                if !numbered && !lines.is_empty() {
                    lines.push(vec![]);
                }
                numbered = true;
                let lead = number.chars().count() + 2;
                for (at, words) in wrap(rest, width.saturating_sub(lead))
                    .into_iter()
                    .enumerate()
                {
                    let before = if at == 0 {
                        format!("{number}. ")
                    } else {
                        " ".repeat(lead)
                    };
                    lines.push(indent(vec![
                        span(Role::Text, before),
                        span(Role::Text, words),
                    ]));
                }
            }
            _ => lines.extend(said(Role::Text, paragraph, width)),
        }
    }
    lines
}

pub fn draw(theme: &Theme, at: &Where, moment: &Moment, width: u16, height: u16) -> Drawn {
    if let Moment::Showing { shown } = moment {
        return alone(shown, width as usize, height as usize);
    }
    let width = width as usize;
    let height = height as usize;
    let inside = width.saturating_sub(2 * MARGIN);
    let mut lines: Vec<Line> = vec![vec![]];

    // Where the journey is, its count at the right while both fit.
    let count = at
        .steps
        .map(|steps| format!("{} of {steps}", at.step))
        .unwrap_or_default();
    let name_width = at.journey.chars().count();
    let mut header = indent(vec![span(Role::Strong, at.journey)]);
    if !count.is_empty() && name_width + 4 + count.chars().count() <= inside && height >= 12 {
        let gap = inside - name_width - count.chars().count();
        header.push(span(Role::Muted, format!("{}{count}", " ".repeat(gap))));
    }
    lines.push(header);
    lines.push(vec![]);

    let (body, caret, footer) = step(theme, moment, inside);

    // The steps done, the latest nearest the step at hand, as many as fit
    // with a line apart from it and the keys still at the foot.
    let room = height.saturating_sub(lines.len() + body.len() + 4);
    let history = done(theme, at.done, inside, room);
    if !history.is_empty() {
        lines.extend(history);
        lines.push(vec![]);
    }
    let top = lines.len();
    lines.extend(body);
    let caret = caret.map(|(row, column)| ((top + row) as u16, column));

    // The keys at the foot, a line above the screen's last, or straight
    // after everything else when the screen is too short for that.
    if !footer.is_empty() {
        let foot = height.saturating_sub(2).max(lines.len() + 1);
        lines.resize(foot, vec![]);
        lines.push(footer);
    }
    Drawn { lines, caret }
}

/// The steps done that fit in this many lines, the latest kept first and
/// drawn last, each behind the done mark.
fn done(theme: &Theme, done: &[String], width: usize, room: usize) -> Vec<Line> {
    let mut kept: Vec<Vec<Line>> = vec![];
    let mut used = 0;
    for said in done.iter().rev() {
        let drawn = marked(
            span(Role::Success, theme.mark("done")),
            Role::Muted,
            &capitalised(said),
            width,
        );
        if used + drawn.len() > room {
            break;
        }
        used += drawn.len();
        kept.push(drawn);
    }
    kept.into_iter().rev().flatten().collect()
}

/// The step at hand: its lines, where the caret goes among them, and the
/// keys that work.
fn step(theme: &Theme, moment: &Moment, inside: usize) -> (Vec<Line>, Option<(usize, u16)>, Line) {
    let mut lines: Vec<Line> = vec![];
    let mut caret = None;
    let footer: Line;
    match moment {
        Moment::Asking {
            title,
            ask,
            typed,
            chosen,
            refused,
            back,
        } => {
            lines.extend(said(Role::Strong, title, inside));
            lines.push(vec![]);
            // Every key that works here, Escape only where it goes back.
            let foot = |on: &str| {
                let mut shown = vec![(Key::Return, on)];
                if *back {
                    shown.push((Key::Escape, "back"));
                }
                shown.push((Key::Leave, "leave"));
                keys(theme, &shown)
            };
            let typing =
                |typed: &str| Some((span(Role::Action, theme.mark("cursor")), typed.to_string()));
            let field = match ask {
                Ask::Act { what, .. } => {
                    lines.extend(said(
                        Role::Text,
                        &format!("{what} Then press Return."),
                        inside,
                    ));
                    footer = foot("done");
                    None
                }
                Ask::Secret { name, .. } => {
                    lines.extend(said(
                        Role::Text,
                        &format!("Type {name}, then press Return."),
                        inside,
                    ));
                    lines.push(vec![]);
                    footer = foot("enter it");
                    Some((
                        span(Role::Machine, theme.mark("locked")),
                        hidden(theme, typed),
                    ))
                }
                Ask::SecretOr { name, or, .. } => {
                    lines.extend(said(
                        Role::Text,
                        &format!("Type {name}, then press Return."),
                        inside,
                    ));
                    lines.push(vec![]);
                    lines.extend(said(Role::Text, &format!("Or {or}."), inside));
                    lines.push(vec![]);
                    footer = foot("enter it");
                    Some((
                        span(Role::Machine, theme.mark("locked")),
                        hidden(theme, typed),
                    ))
                }
                Ask::TypeBack { expected, why, .. } => {
                    lines.extend(said(
                        Role::Text,
                        &format!(
                            "{}. To go on, type the word below, then press Return.",
                            capitalised(why)
                        ),
                        inside,
                    ));
                    lines.push(vec![]);
                    lines.extend(said(Role::Strong, expected, inside));
                    lines.push(vec![]);
                    footer = foot("enter it");
                    typing(typed)
                }
                Ask::Show { how, .. } => {
                    lines.extend(steps(how, inside));
                    footer = foot("show it");
                    None
                }
                Ask::Recall { why, .. } => {
                    lines.extend(said(
                        Role::Text,
                        &format!("{}, then press Return.", capitalised(why)),
                        inside,
                    ));
                    lines.push(vec![]);
                    footer = foot("enter it");
                    typing(typed)
                }
                // The choices in a column, the one the keys are on led by
                // the cursor in the accent and in strong, the rest a
                // cursor's width in, so their words line up.
                Ask::Choose {
                    question, choices, ..
                } => {
                    lines.extend(said(Role::Text, question, inside));
                    lines.push(vec![]);
                    let cursor = theme.mark("cursor");
                    let lead = " ".repeat(cursor.chars().count());
                    for (at, choice) in choices.iter().enumerate() {
                        let (mark, role) = if at == *chosen {
                            (span(Role::Accent, cursor), Role::Strong)
                        } else {
                            (span(Role::Text, lead.clone()), Role::Text)
                        };
                        lines.extend(marked(mark, role, choice, inside));
                    }
                    let mut shown = vec![(Key::Arrows, "move"), (Key::Return, "choose")];
                    if *back {
                        shown.push((Key::Escape, "back"));
                    }
                    shown.push((Key::Leave, "leave"));
                    footer = keys(theme, &shown);
                    None
                }
            };
            if let Some((mark, shown)) = field {
                let lead = mark.text.chars().count() + 1;
                caret = Some((lines.len(), (MARGIN + lead + shown.chars().count()) as u16));
                lines.push(indent(vec![mark, span(Role::Text, format!(" {shown}"))]));
            }
            if let Some(refused) = refused {
                lines.push(vec![]);
                lines.extend(marked(
                    span(Role::Alert, theme.mark("failed")),
                    Role::Text,
                    refused,
                    inside,
                ));
            }
        }
        Moment::Working {
            title,
            meanwhile,
            progress,
        } => {
            lines.extend(said(Role::Strong, title, inside));
            lines.push(vec![]);
            if !meanwhile.is_empty() {
                lines.extend(said(Role::Text, meanwhile, inside));
                lines.push(vec![]);
            }
            if let Some(progress) = progress {
                lines.extend(marked(
                    span(Role::Accent, theme.mark("running")),
                    Role::Text,
                    progress,
                    inside,
                ));
                // Only a step at work can be stopped; the log is left with
                // the keys it says.
                footer = keys(theme, &[(Key::Leave, "stop")]);
            } else {
                footer = vec![];
            }
        }
        Moment::Blocked { title, why, remedy } => {
            lines.extend(said(Role::Strong, title, inside));
            lines.push(vec![]);
            lines.extend(marked(
                span(Role::Warning, theme.mark("caution")),
                Role::Text,
                &capitalised(why),
                inside,
            ));
            lines.extend(said(
                Role::Text,
                &format!("{}.", capitalised(remedy)),
                inside,
            ));
            // A step that stops can hold a boot back, so the way out is
            // shown beside the way on.
            footer = keys(theme, &[(Key::Return, "look again"), (Key::Leave, "leave")]);
        }
        Moment::Failed { title, fault } => {
            lines.extend(said(Role::Strong, title, inside));
            lines.push(vec![]);
            lines.extend(marked(
                span(Role::Alert, theme.mark("failed")),
                Role::Text,
                &capitalised(fault),
                inside,
            ));
            footer = keys(
                theme,
                &[
                    (Key::Return, "try again"),
                    (Key::Tab, "the log"),
                    (Key::Leave, "leave"),
                ],
            );
        }
        // Drawn alone, by draw.
        Moment::Showing { .. } => footer = vec![],
        Moment::Finished { said: text } => {
            lines.extend(marked(
                span(Role::Success, theme.mark("done")),
                Role::Text,
                text,
                inside,
            ));
            footer = vec![];
        }
        Moment::Card {
            title,
            said: text,
            then,
        } => {
            lines.extend(marked(
                span(Role::Success, theme.mark("done")),
                Role::Strong,
                title,
                inside,
            ));
            lines.push(vec![]);
            lines.extend(said(Role::Text, text, inside));
            footer = keys(theme, &[(Key::Return, then)]);
        }
    }
    (lines, caret, footer)
}

/// What's been typed of a secret: a dot for each character, never the
/// characters.
fn hidden(theme: &Theme, typed: &str) -> String {
    theme.mark("hidden").repeat(typed.chars().count())
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::plain;

    fn theme() -> Theme {
        Theme::from(
            "text=37 strong=97 muted=90 accent=34 alert=31 warning=93 success=32 action=93 machine=91 voice=36",
            "cursor ▸ >\ndone ✓ v\nfailed ✕ x\ncaution ! !\nlocked L %\nhidden • *\nrunning → >",
            true,
        )
    }

    const DONE: &[String] = &[];

    fn history() -> Vec<String> {
        vec![
            "the keyboard types Colemak".into(),
            "the vault is open, holding 12 files".into(),
        ]
    }

    fn at() -> Where<'static> {
        Where {
            journey: "Install hedgehog",
            step: 3,
            steps: Some(13),
            done: DONE,
        }
    }

    fn text(drawn: &Drawn) -> Vec<String> {
        drawn
            .lines
            .iter()
            .map(plain)
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    #[test]
    fn a_secret_shows_its_name_and_only_its_length() {
        let ask = Ask::Secret {
            id: "pin",
            name: "the Nano's PIN".into(),
        };
        let moment = Moment::Asking {
            title: "Enrol a YubiKey",
            ask: &ask,
            typed: "123456",
            chosen: 0,
            refused: None,
            back: false,
        };
        let drawn = draw(&theme(), &at(), &moment, 80, 24);
        let shown = text(&drawn);
        assert!(
            shown.contains(&"  Type the Nano's PIN, then press Return.".to_string()),
            "{shown:#?}"
        );
        assert!(shown.contains(&"  % ******".to_string()), "{shown:#?}");
        assert!(!shown.concat().contains("123456"));
        assert_eq!(
            drawn.caret,
            Some((
                shown
                    .iter()
                    .position(|line| line.contains("******"))
                    .unwrap() as u16,
                10
            ))
        );
        assert_eq!(shown.last().unwrap(), "  Return enter it    Ctrl+C leave");
        assert_eq!(shown.len(), 23, "the keys sit a line above the foot");
    }

    #[test]
    fn the_header_says_where_and_how_far_along() {
        let ask = Ask::Act {
            id: "plugged",
            what: "Plug in the Nano.".into(),
        };
        let moment = Moment::Asking {
            title: "Enrol a YubiKey",
            ask: &ask,
            typed: "",
            chosen: 0,
            refused: None,
            back: false,
        };
        let shown = text(&draw(&theme(), &at(), &moment, 40, 24));
        assert_eq!(
            shown[1],
            format!("  Install hedgehog{}3 of 13", " ".repeat(36 - 16 - 7))
        );
    }

    #[test]
    fn every_step_done_stays_on_the_screen_the_latest_nearest() {
        let ask = Ask::Act {
            id: "plugged",
            what: "Plug in the Nano.".into(),
        };
        let moment = Moment::Asking {
            title: "Enrol a YubiKey",
            ask: &ask,
            typed: "",
            chosen: 0,
            refused: None,
            back: false,
        };
        let done = history();
        let at = Where {
            done: &done,
            ..at()
        };
        let shown = text(&draw(&theme(), &at, &moment, 60, 24));
        assert_eq!(shown[3], "  v The keyboard types Colemak", "{shown:#?}");
        assert_eq!(
            shown[4], "  v The vault is open, holding 12 files",
            "{shown:#?}"
        );
        assert_eq!(shown[5], "");
        assert_eq!(shown[6], "  Enrol a YubiKey");
        // A short screen gives up the earliest first.
        let short = text(&draw(&theme(), &at, &moment, 60, 11));
        assert!(!short.concat().contains("Colemak"), "{short:#?}");
        assert!(short.concat().contains("The vault is open"), "{short:#?}");
    }

    #[test]
    fn escape_is_shown_only_where_it_goes_back() {
        let ask = Ask::Secret {
            id: "again",
            name: "the same password again".into(),
        };
        let moment = |back| Moment::Asking {
            title: "Choose the account's password",
            ask: &ask,
            typed: "",
            chosen: 0,
            refused: None,
            back,
        };
        let first = text(&draw(&theme(), &at(), &moment(false), 80, 24));
        assert_eq!(first.last().unwrap(), "  Return enter it    Ctrl+C leave");
        let later = text(&draw(&theme(), &at(), &moment(true), 80, 24));
        assert_eq!(
            later.last().unwrap(),
            "  Return enter it    Esc back    Ctrl+C leave"
        );
    }

    #[test]
    fn a_secret_with_another_way_in_says_both_on_one_screen() {
        let ask = Ask::SecretOr {
            id: "passphrase",
            name: "the stick's passphrase, the seven words in 1Password on your phone".into(),
            or: "plug in the Nano or the 5Ci, type its PIN and press Return, then touch the key when it blinks".into(),
        };
        let moment = Moment::Asking {
            title: "Open the stick's vault",
            ask: &ask,
            typed: "ab",
            chosen: 0,
            refused: None,
            back: false,
        };
        let drawn = draw(&theme(), &at(), &moment, 120, 24);
        let shown = text(&drawn);
        assert!(shown.contains(&"  Type the stick's passphrase, the seven words in 1Password on your phone, then press Return.".to_string()), "{shown:#?}");
        assert!(shown.contains(&"  Or plug in the Nano or the 5Ci, type its PIN and press Return, then touch the key when it blinks.".to_string()), "{shown:#?}");
        let field = shown
            .iter()
            .position(|line| line == "  % **")
            .expect("one field");
        assert_eq!(drawn.caret, Some((field as u16, 6)));
    }

    #[test]
    fn a_step_at_work_says_how_far_along_it_is() {
        let moment = Moment::Working {
            title: "Install the system",
            meanwhile: "",
            progress: Some("3.1 GB of 9.5 GB, 15.5 MB a second, about 7 minutes left"),
        };
        let shown = text(&draw(&theme(), &at(), &moment, 80, 24));
        assert_eq!(shown[3], "  Install the system");
        assert_eq!(
            shown[5],
            "  > 3.1 GB of 9.5 GB, 15.5 MB a second, about 7 minutes left"
        );
    }

    #[test]
    fn keys_are_named_the_same_on_a_console_as_in_a_terminal() {
        let console = Theme::from("", "", true);
        let terminal = Theme::from("", "", false);
        for key in [Key::Return, Key::Tab, Key::Escape, Key::Arrows, Key::Leave] {
            assert_eq!(console.key(key), terminal.key(key));
        }
        assert_eq!(terminal.key(Key::Return), "Return");
    }

    #[test]
    fn a_short_screen_gives_up_what_came_before_then_the_count() {
        let ask = Ask::Act {
            id: "plugged",
            what: "Plug in the Nano.".into(),
        };
        let moment = Moment::Asking {
            title: "Enrol a YubiKey",
            ask: &ask,
            typed: "",
            chosen: 0,
            refused: None,
            back: false,
        };
        let medium = text(&draw(&theme(), &at(), &moment, 80, 14));
        assert!(
            !medium.concat().contains("the vault is open"),
            "{medium:#?}"
        );
        assert!(medium[1].ends_with("3 of 13"));
        let short = text(&draw(&theme(), &at(), &moment, 80, 10));
        assert_eq!(short[1], "  Install hedgehog");
    }

    #[test]
    fn what_to_type_back_stands_alone_above_its_field() {
        let ask = Ask::TypeBack {
            id: "machine",
            expected: "hedgehog".into(),
            why: "installing wipes SK hynix HFM128GDGTNG-87A0A, 128 GB, and everything on it"
                .into(),
        };
        let moment = Moment::Asking {
            title: "Confirm the disk to wipe",
            ask: &ask,
            typed: "pen",
            chosen: 0,
            refused: Some("\"hedgehoge\" isn't hedgehog, so nothing was wiped"),
            back: false,
        };
        let drawn = draw(&theme(), &at(), &moment, 60, 30);
        let shown = text(&drawn);
        let expected = shown
            .iter()
            .position(|line| line == "  hedgehog")
            .expect("hedgehog alone");
        assert_eq!(shown[expected + 2], "  > pen");
        assert!(
            shown.contains(&"  x \"hedgehoge\" isn't hedgehog, so nothing was wiped".to_string()),
            "{shown:#?}"
        );
        let strong = &drawn.lines[expected][1];
        assert_eq!(strong.role, Role::Strong);
    }

    #[test]
    fn a_mark_leads_and_what_it_wraps_to_lines_up_under_its_ink() {
        let moment = Moment::Blocked {
            title: "Make the machine's Secure Boot keys",
            why: "the firmware's Secure Boot is in user mode, so it won't take new keys",
            remedy: "clear the firmware's Secure Boot keys in its setup, then restart from the stick",
        };
        let shown = text(&draw(&theme(), &at(), &moment, 50, 30));
        let first = shown
            .iter()
            .position(|line| line.starts_with("  ! "))
            .unwrap();
        // Under the mark's ink, not indented from it.
        for below in &shown[first + 1..first + 4] {
            assert!(
                below.starts_with("  ") && !below.starts_with("   "),
                "{shown:#?}"
            );
        }
        assert!(
            shown.iter().all(|line| line.chars().count() <= 48),
            "{shown:#?}"
        );
        assert_eq!(shown.last().unwrap(), "  Return look again    Ctrl+C leave");
    }

    #[test]
    fn a_choice_leads_with_the_cursor_and_the_rest_line_up_under_its_words() {
        let ask = Ask::Choose {
            id: "repair",
            question: "What should be done?".into(),
            choices: vec!["Open a root shell inside it".into(), "Restart".into()],
        };
        let moment = Moment::Asking {
            title: "The machine's disk is open",
            ask: &ask,
            typed: "",
            chosen: 1,
            refused: None,
            back: false,
        };
        let drawn = draw(&theme(), &at(), &moment, 60, 24);
        let shown = text(&drawn);
        assert!(
            shown.contains(&"    Open a root shell inside it".to_string()),
            "{shown:#?}"
        );
        assert!(shown.contains(&"  > Restart".to_string()), "{shown:#?}");
        let row = shown.iter().position(|line| line == "  > Restart").unwrap();
        assert_eq!(drawn.lines[row][1].role, Role::Accent);
        assert_eq!(
            shown.last().unwrap(),
            "  ↑↓ move    Return choose    Ctrl+C leave"
        );
        assert_eq!(drawn.caret, None);
    }

    #[test]
    fn what_a_phone_reads_is_alone_in_the_middle_of_the_screen() {
        let words = "abacus abdomen abdominal abide abiding ability ablaze";
        let drawn = draw(&theme(), &at(), &Moment::Showing { shown: words }, 80, 20);
        let shown = text(&drawn);
        let inked: Vec<&String> = shown.iter().filter(|line| !line.is_empty()).collect();
        assert_eq!(inked, [&format!("{}{words}", " ".repeat((80 - 53) / 2))]);
        assert_eq!(shown.len(), 10);
        assert_eq!(drawn.lines[9][1].role, Role::Strong);
        assert_eq!(drawn.caret, None);
    }

    #[test]
    fn steps_for_the_phone_are_a_line_each_their_words_after_their_number() {
        let ask = Ask::Show {
            id: "shown",
            how: crate::words::keeping(
                "the recovery key",
                "hedgehog disk recovery key",
                "It opens the disk",
            ),
            shown: "abacus abdomen abdominal abide abiding ability ablaze".into(),
        };
        let moment = Moment::Asking {
            title: "Keep the recovery key",
            ask: &ask,
            typed: "",
            chosen: 0,
            refused: None,
            back: false,
        };
        let shown = text(&draw(&theme(), &at(), &moment, 60, 40));
        let first = shown
            .iter()
            .position(|line| line.starts_with("  1. On your iPhone"))
            .unwrap_or_else(|| panic!("{shown:#?}"));
        assert_eq!(shown[first - 1], "", "{shown:#?}");
        assert_eq!(shown[first + 1], "     words.", "{shown:#?}");
        for number in 1..=5 {
            assert!(
                shown
                    .iter()
                    .any(|line| line.starts_with(&format!("  {number}. "))),
                "{shown:#?}"
            );
        }
        assert!(!shown.concat().contains("abacus"), "{shown:#?}");
        assert_eq!(shown.last().unwrap(), "  Return show it    Ctrl+C leave");
    }

    #[test]
    fn words_wrap_between_themselves() {
        assert_eq!(wrap("one two three four", 9), ["one two", "three", "four"]);
        assert_eq!(wrap("abcdefghijk", 4), ["abcd", "efgh", "ijk"]);
        assert_eq!(
            wrap("aaaaaaaa-bbbbbbbb-cccccccc-dddddddd", 20),
            ["aaaaaaaa-bbbbbbbb-", "cccccccc-dddddddd"]
        );
    }
}

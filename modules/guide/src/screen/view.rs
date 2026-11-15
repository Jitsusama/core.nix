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

#[derive(Clone)]
pub struct Drawn {
    pub lines: Vec<Line>,
    /// Where the caret goes, as a row and a column, when something's typed.
    pub caret: Option<(u16, u16)>,
    /// The screen's width and height it was laid out for.
    pub size: (u16, u16),
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
fn alone(shown: &str, size: (u16, u16)) -> Drawn {
    let (width, height) = (size.0 as usize, size.1 as usize);
    let shown = wrap(shown, width.saturating_sub(2 * MARGIN));
    let mut lines: Vec<Line> = vec![vec![]; height.saturating_sub(shown.len()) / 2];
    for words in shown {
        let lead = width.saturating_sub(words.chars().count()) / 2;
        lines.push(vec![
            span(Role::Text, " ".repeat(lead)),
            span(Role::Strong, words),
        ]);
    }
    Drawn {
        lines,
        caret: None,
        size,
    }
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
    let size = (width, height);
    if let Moment::Showing { shown } = moment {
        return alone(shown, size);
    }
    let width = width as usize;
    let height = height as usize;
    let inside = width.saturating_sub(2 * MARGIN);
    let mut lines: Vec<Line> = vec![vec![]];

    // Where the journey is, its count at the right while both fit.
    let count = at
        .steps
        .map_or_default(|steps| format!("{} of {steps}", at.step));
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
    let caret = caret.map(|(row, column)| (super::cell(top + row), column));

    // The keys at the foot, a line above the screen's last, or straight
    // after everything else when the screen is too short for that.
    if !footer.is_empty() {
        let foot = height.saturating_sub(2).max(lines.len() + 1);
        lines.resize(foot, vec![]);
        lines.push(footer);
    }
    Drawn { lines, caret, size }
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
            let (field, foot) = asked(theme, ask, typed, *chosen, *back, inside, &mut lines);
            footer = foot;
            if let Some((mark, shown)) = field {
                let lead = mark.text.chars().count() + 1;
                caret = Some((
                    lines.len(),
                    super::cell(MARGIN + lead + shown.chars().count()),
                ));
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
            footer = working(theme, meanwhile, *progress, inside, &mut lines);
        }
        Moment::Blocked { title, why, remedy } => {
            lines.extend(said(Role::Strong, title, inside));
            lines.push(vec![]);
            footer = blocked(theme, why, remedy, inside, &mut lines);
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
                &capitalised(text),
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

/// A step at work: what to do meanwhile and how far along it is, added
/// to `lines`, then the keys that work.
fn working(
    theme: &Theme,
    meanwhile: &str,
    progress: Option<&str>,
    inside: usize,
    lines: &mut Vec<Line>,
) -> Line {
    if !meanwhile.is_empty() {
        lines.extend(said(Role::Text, meanwhile, inside));
        lines.push(vec![]);
    }
    let Some(progress) = progress else {
        return vec![];
    };
    lines.extend(marked(
        span(Role::Accent, theme.mark("running")),
        Role::Text,
        progress,
        inside,
    ));
    // Only a step at work can be stopped; the log is left with the keys it
    // says.
    keys(theme, &[(Key::Leave, "stop")])
}

/// What's in the way of a step and what to do about it, added to `lines`,
/// then the keys that work.
fn blocked(theme: &Theme, why: &str, remedy: &str, inside: usize, lines: &mut Vec<Line>) -> Line {
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
    // A step that stops can hold a boot back, so the way out is shown
    // beside the way on.
    keys(theme, &[(Key::Return, "look again"), (Key::Leave, "leave")])
}

/// What one ask says to do, added to `lines`, then its field, if it has
/// one, as its mark and what's shown in it, and the keys that work.
fn asked(
    theme: &Theme,
    ask: &Ask,
    typed: &str,
    chosen: usize,
    back: bool,
    inside: usize,
    lines: &mut Vec<Line>,
) -> (Option<(Span, String)>, Line) {
    // Every key that works here, Escape only where it goes back.
    let foot = |on: &str| {
        let mut shown = vec![(Key::Return, on)];
        if back {
            shown.push((Key::Escape, "back"));
        }
        shown.push((Key::Leave, "leave"));
        keys(theme, &shown)
    };
    let typing = |typed: &str| Some((span(Role::Action, theme.mark("cursor")), typed.to_string()));
    match ask {
        Ask::Act { what, .. } => {
            lines.extend(said(
                Role::Text,
                &format!("{what} Then press Return."),
                inside,
            ));
            (None, foot("done"))
        }
        Ask::Confirm { what, verb, .. } => {
            lines.extend(said(
                Role::Text,
                &format!("{what} Press Return to {verb}."),
                inside,
            ));
            (None, foot(verb))
        }
        Ask::Secret { name, .. } | Ask::SecretOr { name, .. } => {
            lines.extend(said(
                Role::Text,
                &format!("Type {name}, then press Return."),
                inside,
            ));
            lines.push(vec![]);
            if let Ask::SecretOr { or, .. } = ask {
                lines.extend(said(Role::Text, &format!("Or {or}."), inside));
                lines.push(vec![]);
            }
            let field = (
                span(Role::Machine, theme.mark("locked")),
                hidden(theme, typed),
            );
            (Some(field), foot("enter it"))
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
            (typing(typed), foot("enter it"))
        }
        Ask::Show { how, .. } => {
            lines.extend(steps(how, inside));
            (None, foot("show it"))
        }
        Ask::Recall { why, .. } => {
            lines.extend(said(
                Role::Text,
                &format!("{}, then press Return.", capitalised(why)),
                inside,
            ));
            lines.push(vec![]);
            (typing(typed), foot("enter it"))
        }
        // The choices in a column, the one the keys are on led by the
        // cursor in the accent and in strong, the rest a cursor's width in,
        // so their words line up.
        Ask::Choose {
            question, choices, ..
        } => {
            lines.extend(said(Role::Text, question, inside));
            lines.push(vec![]);
            let cursor = theme.mark("cursor");
            let lead = " ".repeat(cursor.chars().count());
            for (at, choice) in choices.iter().enumerate() {
                let (mark, role) = if at == chosen {
                    (span(Role::Accent, cursor), Role::Strong)
                } else {
                    (span(Role::Text, lead.clone()), Role::Text)
                };
                lines.extend(marked(mark, role, choice, inside));
            }
            let mut shown = vec![(Key::Arrows, "move"), (Key::Return, "choose")];
            if back {
                shown.push((Key::Escape, "back"));
            }
            shown.push((Key::Leave, "leave"));
            (None, keys(theme, &shown))
        }
    }
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

/// A drawing's lines as plain text, each with its trailing spaces cut, as
/// the view's tests and the walk's compare them.
#[cfg(test)]
pub fn text(drawn: &Drawn) -> Vec<String> {
    drawn
        .lines
        .iter()
        .map(crate::screen::plain)
        .map(|line| line.trim_end().to_string())
        .collect()
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;

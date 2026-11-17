//! What the person sees and types into: one step at a time, everything it
//! needs asked one thing at a time before it starts, and the keys beside
//! what they do. It's drawn in the theme's roles and marks, which this
//! build was given (theme.nix), with the console's stand-ins on a console.
//!
//! `view` draws a moment as lines of text in roles and is all the screen
//! there is to test; `terminal` puts lines on a real one and reads its
//! keys; `run` walks a journey through them.

pub mod run;
pub mod terminal;
pub mod view;

use std::sync::OnceLock;

use terminal_kit::ink::{Colours, Ink, Pen};

/// The colours the kit decided for stdout, where the screen is drawn,
/// from `--color`, `NO_COLOR`, `FORCE_COLOR` and whether it's a terminal.
/// main says it once, before any command runs; until then, as in a test,
/// the screen draws plainly.
static DECIDED: OnceLock<Colours> = OnceLock::new();

/// Says once which colours the screen draws in, as the kit decided them.
pub fn decide_colours(colours: Colours) {
    let _ = DECIDED.set(colours);
}

/// A row or a column as a terminal takes one: the low 16 bits of where it
/// is, all a screen has, so one past them wraps round as it always has.
fn cell(at: usize) -> u16 {
    let [low, high, ..] = at.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// What a piece of text means, which says how it's drawn: text in its
/// three weights, the accent where the keys go, and the hues marks wear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Text,
    Strong,
    Muted,
    Accent,
    Alert,
    Warning,
    Success,
    /// A mark for something to do.
    Action,
    /// A mark for the machine itself: a padlock, a disk.
    Machine,
    /// A mark for the guide speaking for itself.
    Voice,
}

impl Role {
    pub const ALL: &[Role] = &[
        Role::Text,
        Role::Strong,
        Role::Muted,
        Role::Accent,
        Role::Alert,
        Role::Warning,
        Role::Success,
        Role::Action,
        Role::Machine,
        Role::Voice,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Role::Text => "text",
            Role::Strong => "strong",
            Role::Muted => "muted",
            Role::Accent => "accent",
            Role::Alert => "alert",
            Role::Warning => "warning",
            Role::Success => "success",
            Role::Action => "action",
            Role::Machine => "machine",
            Role::Voice => "voice",
        }
    }
}

/// The marks the screen draws, by the theme's names for them.
pub const MARKS: &[&str] = &[
    "cursor", "done", "failed", "caution", "locked", "hidden", "running",
];

/// The theme as the screen draws it.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Each role's SGR number, once each; none draws everything in the
    /// terminal's own. The pen is given the same list, so what `missing`
    /// checks is what the screen draws.
    roles: Vec<(Role, String)>,
    /// Each mark by name, as drawn here: the mark, or on a console its
    /// stand-in.
    marks: Vec<(String, String)>,
    /// What draws each role: the terminal kit's pen, given the roles'
    /// codes, so the guide spells no escape of its own.
    pen: Pen,
}

impl Theme {
    /// From the roles and marks as theme.nix writes them, in the sixteen
    /// colours, as a test reads them.
    pub fn from(roles: &str, marks: &str, console: bool) -> Theme {
        Theme::inked(
            roles,
            marks,
            Ink {
                colours: Colours::Sixteen,
                console,
            },
        )
    }

    /// From the roles and marks as theme.nix writes them, drawn in this
    /// ink; a role given twice keeps its first code.
    pub fn inked(roles: &str, marks: &str, ink: Ink) -> Theme {
        let console = ink.console;
        let mut kept: Vec<(Role, String)> = Vec::new();
        let given = roles
            .split_whitespace()
            .filter_map(|pair| pair.split_once('='))
            .filter_map(|(name, code)| {
                Role::ALL
                    .iter()
                    .find(|role| role.name() == name)
                    .map(|role| (*role, code.to_string()))
            });
        for (role, code) in given {
            if !kept.iter().any(|(found, _)| *found == role) {
                kept.push((role, code));
            }
        }
        let roles = kept;
        let marks = marks
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let (name, mark, stand_in) = (parts.next()?, parts.next()?, parts.next()?);
                Some((
                    name.to_string(),
                    if console { stand_in } else { mark }.to_string(),
                ))
            })
            .collect();
        let pen = Pen::coded(ink, roles_coded(&roles));
        Theme { roles, marks, pen }
    }

    /// A span's words as the terminal draws them, in its role's colour.
    pub fn drawn(&self, span: &Span) -> String {
        self.pen.role(span.role.name(), &span.text)
    }

    /// The theme this build was given, in the colours the kit decided.
    pub fn built(console: bool) -> Theme {
        Theme::inked(
            option_env!("GUIDE_ROLES").unwrap_or_default(),
            option_env!("GUIDE_GLYPHS").unwrap_or_default(),
            Ink {
                colours: DECIDED.get().copied().unwrap_or(Colours::None),
                console,
            },
        )
    }

    pub fn sgr(&self, role: Role) -> Option<&str> {
        self.roles
            .iter()
            .find(|(found, _)| *found == role)
            .map(|(_, code)| code.as_str())
    }

    /// A mark by its name in the theme, or nothing when the build was
    /// given none, which `guide tools` refuses.
    pub fn mark(&self, name: &str) -> &str {
        self.marks
            .iter()
            .find(|(found, _)| found == name)
            .map_or("", |(_, mark)| mark.as_str())
    }

    /// What's missing from it that the screen draws.
    pub fn missing(&self) -> Vec<String> {
        let roles = Role::ALL
            .iter()
            .filter(|role| self.sgr(**role).is_none())
            .map(|role| format!("the {} role", role.name()));
        let marks = MARKS
            .iter()
            .filter(|mark| self.mark(mark).is_empty())
            .map(|mark| format!("the {mark} mark"));
        roles.chain(marks).collect()
    }

    /// A key as it's drawn: by the name on its keycap, the same on the
    /// console as in a terminal, so it reads the same wherever the guide
    /// runs; the arrows by their glyphs, which every font has
    /// (../../console/glyphs.txt).
    pub fn key(&self, key: Key) -> &'static str {
        match key {
            Key::Return => "Return",
            Key::Tab => "Tab",
            Key::Escape => "Esc",
            Key::Arrows => "↑↓",
            Key::Leave => "Ctrl+C",
        }
    }
}

/// The keys the screen names beside what they do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Return,
    Tab,
    Escape,
    /// Up and down together, for moving through a list.
    Arrows,
    /// Ctrl+C, which leaves the guide.
    Leave,
}

/// A run of text in one role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub role: Role,
    pub text: String,
}

/// A line on the screen, from its left edge.
pub type Line = Vec<Span>;

pub fn span(role: Role, text: impl Into<String>) -> Span {
    Span {
        role,
        text: text.into(),
    }
}

fn roles_coded(roles: &[(Role, String)]) -> Vec<(&'static str, &str)> {
    roles
        .iter()
        .map(|(role, code)| (role.name(), code.as_str()))
        .collect()
}

/// A line's text without its roles, as a test reads it.
pub fn plain(line: &Line) -> String {
    line.iter().map(|span| span.text.as_str()).collect()
}

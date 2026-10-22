//! The real terminal: the screen taken over while the guide runs and given
//! back as it was, each line drawn in its role's colour, and each key read
//! as it's pressed.

use std::io::{Stdout, Write, stdout};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, queue, terminal};

use crate::Fault;
use crate::screen::Theme;
use crate::screen::view::Drawn;

/// A key as the guide reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Press {
    Char(char),
    Return,
    Escape,
    Backspace,
    Tab,
    Up,
    Down,
    /// Ctrl+C: leave the guide.
    Leave,
}

pub trait Keys {
    fn press(&mut self) -> Result<Press, Fault>;

    /// A key pressed within this long, or none, for a screen that looks at
    /// the machine again by itself; by default it waits for a key.
    fn press_within(&mut self, _wait: std::time::Duration) -> Result<Option<Press>, Fault> {
        self.press().map(Some)
    }
}

pub trait Screen {
    /// Its width and height, in characters.
    fn size(&self) -> (u16, u16);
    fn show(&mut self, drawn: &Drawn, theme: &Theme) -> Result<(), Fault>;

    /// Something that draws on the same screen from another thread, for a
    /// step at work, or none where nothing can.
    fn painter(&self) -> Option<Box<dyn Paint>> {
        None
    }
}

/// Draws on a screen from a thread of its own.
pub trait Paint: Send {
    fn size(&self) -> (u16, u16);
    fn paint(&mut self, drawn: &Drawn, theme: &Theme) -> Result<(), Fault>;
}

/// Where keys come from and lines go: one terminal, or a test's stand-in.
pub trait Io: Keys + Screen {}

impl<T: Keys + Screen> Io for T {}

fn broken(why: std::io::Error) -> Fault {
    Fault::unreadable("the terminal", why)
}

/// The terminal the guide runs in, held in raw mode on its own screen
/// until it's dropped.
pub struct Terminal {
    out: Stdout,
}

impl Terminal {
    pub fn open() -> Result<Terminal, Fault> {
        let mut out = stdout();
        terminal::enable_raw_mode().map_err(broken)?;
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide).map_err(broken)?;
        Ok(Terminal { out })
    }

    /// Whether it's the Linux console, whose font has none of the marks
    /// and draws their stand-ins instead.
    pub fn is_console() -> bool {
        std::env::var("TERM").is_ok_and(|term| term == "linux")
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(self.out, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

/// What a key event means here, if anything.
fn pressed(event: Event) -> Option<Press> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    Some(match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Press::Leave,
        KeyCode::Char(c) => Press::Char(c),
        KeyCode::Enter => Press::Return,
        KeyCode::Esc => Press::Escape,
        KeyCode::Backspace => Press::Backspace,
        KeyCode::Tab => Press::Tab,
        KeyCode::Up => Press::Up,
        KeyCode::Down => Press::Down,
        _ => return None,
    })
}

impl Keys for Terminal {
    fn press(&mut self) -> Result<Press, Fault> {
        loop {
            if let Some(press) = pressed(event::read().map_err(broken)?) {
                return Ok(press);
            }
        }
    }

    fn press_within(&mut self, wait: std::time::Duration) -> Result<Option<Press>, Fault> {
        let until = std::time::Instant::now() + wait;
        loop {
            let left = until.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() || !event::poll(left).map_err(broken)? {
                return Ok(None);
            }
            if let Some(press) = pressed(event::read().map_err(broken)?) {
                return Ok(Some(press));
            }
        }
    }
}

impl Screen for Terminal {
    fn size(&self) -> (u16, u16) {
        terminal::size().unwrap_or((80, 24))
    }

    fn show(&mut self, drawn: &Drawn, theme: &Theme) -> Result<(), Fault> {
        paint(&mut self.out, drawn, theme)
    }

    fn painter(&self) -> Option<Box<dyn Paint>> {
        Some(Box::new(Painter { out: stdout() }))
    }
}

/// The terminal's screen, drawn from another thread. Stdout's own lock
/// keeps each frame whole.
struct Painter {
    out: Stdout,
}

impl Paint for Painter {
    fn size(&self) -> (u16, u16) {
        terminal::size().unwrap_or((80, 24))
    }

    fn paint(&mut self, drawn: &Drawn, theme: &Theme) -> Result<(), Fault> {
        paint(&mut self.out, drawn, theme)
    }
}

/// Lines drawn from the top, each in its role's colour, the rest of the
/// screen cleared, and the caret put where something's typed.
fn paint(out: &mut Stdout, drawn: &Drawn, theme: &Theme) -> Result<(), Fault> {
    let mut out = out.lock();
    queue!(out, cursor::Hide, cursor::MoveTo(0, 0)).map_err(broken)?;
    for (row, line) in drawn.lines.iter().enumerate() {
        queue!(
            out,
            cursor::MoveTo(0, row as u16),
            terminal::Clear(terminal::ClearType::CurrentLine)
        )
        .map_err(broken)?;
        for span in line {
            match theme.sgr(span.role) {
                Some(code) => write!(out, "\x1b[{code}m{}\x1b[0m", span.text),
                None => write!(out, "{}", span.text),
            }
            .map_err(broken)?;
        }
    }
    queue!(
        out,
        cursor::MoveTo(0, drawn.lines.len() as u16),
        terminal::Clear(terminal::ClearType::FromCursorDown)
    )
    .map_err(broken)?;
    if let Some((row, column)) = drawn.caret {
        queue!(out, cursor::MoveTo(column, row), cursor::Show).map_err(broken)?;
    }
    out.flush().map_err(broken)
}

//! The outline of the page's headings: asked for, moved through and acted on.

use super::Spot;
use crate::{App, keys, keys_logged, outline, prompt, search};

impl App {
    /// Asks for gO's outline set as it stands now.
    pub(crate) fn ask_outline(&mut self) {
        // Given up by the selection, words reading on are back at their
        // start at once.
        let moved = match (self.reads_on.as_ref(), self.outline.as_ref()) {
            (Some(on), Some(outline)) => on.strip.said != outline.rows[outline.chosen].words,
            (Some(_), None) => true,
            _ => false,
        };
        if moved {
            self.stop_reading();
        }
        let px_per_pt = self.prompt_px_per_pt();
        let (Some(outline), Some(prompt), Some(gl)) = (
            self.outline.as_ref(),
            self.prompt.as_ref(),
            self.gl.as_ref(),
        ) else {
            return;
        };
        if keys_logged() {
            eprintln!(
                "outline: {} rows, chosen {:?}",
                outline.rows.len(),
                outline.rows[outline.chosen].words
            );
        }
        prompt.ask(prompt::Ask {
            serial: 0,
            keys: false,
            labels: Vec::new(),
            outline: outline
                .shown()
                .map(|(row, chosen)| (row.level, row.words.clone(), chosen))
                .collect(),
            menu: None,
            note: String::new(),
            mark: "",
            typed: String::new(),
            cursor: false,
            count: None,
            width_pt: f64::from(gl.width) / px_per_pt,
            px_per_pt,
            page: self.width,
        });
    }

    /// Puts gO's outline away.
    fn close_outline(&mut self) {
        self.stop_reading();
        self.outline = None;
        self.outline_set = None;
        if let (Some(gl), Some(old)) = (self.gl.as_ref(), self.outline_canvas.take()) {
            gl.forget(old);
        }
        if keys_logged() {
            eprintln!("outline: closed");
        }
    }

    /// A key while gO's outline shows, as Neovim's location list takes
    /// them in normal mode: j and k, gg and G, Ctrl+D and Ctrl+U and the
    /// screen's keys through its rows, / and ? then n and N through its
    /// words, Return to the heading chosen, and ZZ or Space w away: Escape
    /// closes nothing, as Joel asked, and q does nothing, since nothing
    /// records.
    pub(crate) fn outline_key(&mut self, key: keys::Key, held: bool) {
        if let Some(typing) = self.outline_typing.as_mut() {
            let said = typing.key(key, held);
            match said {
                search::Typed::Changed => self.ask_prompt(),
                search::Typed::Done => {
                    let typing = self.outline_typing.take();
                    self.prompt_shown = false;
                    if let (Some(typing), Some(outline)) = (typing, self.outline.as_mut()) {
                        let query = if typing.typed.is_empty() {
                            outline.searched.as_ref().map(|(query, _)| query.clone())
                        } else {
                            Some(typing.typed)
                        };
                        if let Some(query) = query {
                            outline.find(&query, typing.forward, 1);
                            outline.searched = Some((query, typing.forward));
                        }
                    }
                    self.ask_outline();
                }
                search::Typed::Cancelled => {
                    self.outline_typing = None;
                    self.prompt_shown = false;
                }
                search::Typed::Nothing => {}
            }
        } else {
            // Return takes the row chosen, as in his location list; the
            // page's own keys take nothing of it, so it drops a key
            // waiting for its second, as any key vi doesn't know does.
            let pressed = self.keys.press(key, false);
            if key == keys::Key::Return {
                // Only a fresh press goes, as a held key never leaves.
                if !held {
                    self.outline_go();
                }
            } else if let Some((act, count)) = pressed {
                self.outline_act(act, count);
            }
        }
        if !self.frame_pending {
            self.draw();
        }
    }

    /// Return in gO's outline: to the heading chosen, as a jump.
    fn outline_go(&mut self) {
        let Some(outline) = self.outline.as_ref() else {
            return;
        };
        let row = outline.rows[outline.chosen].clone();
        self.close_outline();
        if let outline::To::Command(line) = &row.to {
            // An earlier line is run, not gone to, so it's no jump.
            self.remember_line(line);
            self.run_line(line);
            return;
        }
        self.remember(self.heading() / self.px_per_pt());
        match row.to {
            outline::To::Command(_) => {}
            outline::To::Heading(top) => self.land(top),
            outline::To::Line(top) => self.bring(top),
            outline::To::Jump(file, pt, aim) => self.revisit(Spot { file, pt, aim }),
        }
        if keys_logged() {
            eprintln!(
                "outline went to {:?}: heading {:.1} pt",
                row.words,
                self.heading() / self.px_per_pt()
            );
        }
    }

    fn outline_act(&mut self, act: keys::Act, count: Option<u32>) {
        use keys::Act;
        let Some(outline) = self.outline.as_mut() else {
            return;
        };
        let n = count.unwrap_or(1);
        let rows = |each: usize| isize::try_from(n as usize * each).unwrap_or(isize::MAX);
        match act {
            Act::Down | Act::ScrollDown => outline.by(rows(1)),
            Act::Up | Act::ScrollUp => outline.by(-rows(1)),
            Act::HalfDown => outline.by(rows(outline::SHOWN / 2)),
            Act::HalfUp => outline.by(-rows(outline::SHOWN / 2)),
            Act::ScreenDown => outline.by(rows(outline::SHOWN)),
            Act::ScreenUp => outline.by(-rows(outline::SHOWN)),
            Act::First => outline.choose(count.map_or(0, |n| n.saturating_sub(1) as usize)),
            Act::Last => {
                outline.choose(count.map_or(usize::MAX, |n| n.saturating_sub(1) as usize));
            }
            Act::SearchForward | Act::SearchBackward => {
                self.outline_typing = Some(search::Typing::new(act == Act::SearchForward, 0.0));
                self.ask_prompt();
                return;
            }
            Act::Next | Act::Previous => {
                if let Some((query, forward)) = outline.searched.clone() {
                    outline.find(&query, forward == (act == Act::Next), n);
                }
            }
            Act::Close => {
                self.close_outline();
                return;
            }
            // Escape among the rest, which has already dropped a count or a
            // key waiting, and closes nothing.
            _ => return,
        }
        self.ask_outline();
    }
}

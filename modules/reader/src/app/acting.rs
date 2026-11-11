//! Acting on a key: what each does to the view, the page or the file, and the
//! keys as vi takes them.

use super::{Following, Landing, Reload, Spot, folder_of};
use crate::theme::PT_A_PX;
use crate::{App, ex, keys, keys_logged, line_pt, links, moves, outline, page, place, search};
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym};

impl App {
    /// Goes to a place on the jump list without adding to it: on this page
    /// as a jump, or on another file, set in this one's place.
    pub(crate) fn revisit(&mut self, spot: Spot) {
        if !links::same(&spot.file, &self.file) {
            self.open(spot.file, Landing::At(spot.pt, spot.aim));
            return;
        }
        self.aim = Some(spot.aim);
        // Back on the file shown before another took its place: the other
        // is let go, and the reloader set on this one again.
        if self.following.take().is_some() {
            self.let_incoming_go();
            self.set_on(&self.file);
        }
        self.jump_to(spot.pt * self.px_per_pt());
    }

    /// Sets another file to take this one's place in the window once its
    /// bands cover the view, where `landing` says.
    pub(crate) fn open(&mut self, file: std::path::PathBuf, landing: Landing) {
        self.let_incoming_go();
        self.set_on(&file);
        self.following = Some(Following { file, landing });
    }

    /// Asks the page set again for the window's width, when that width
    /// takes another page than the one asked for last: a window narrower
    /// than the whole page's 540 pt, or one widened back past it. The page
    /// shown stays until the new one is set, with the view on the same
    /// words.
    pub(crate) fn fit_window(&mut self) {
        let width = page::Width::for_window(f64::from(self.size.0) * PT_A_PX);
        if width == self.width {
            return;
        }
        // Taken as asked only once the reloader has it, so a page is never
        // held to a width nothing will set.
        if let Some(to) = self.reload_to.as_ref()
            && to.send(Reload::Fit(width)).is_ok()
        {
            self.width = width;
        }
    }

    /// Moves the reloader and the watch to `file`, which the reloader sets
    /// at once and then at each save.
    pub(crate) fn set_on(&self, file: &std::path::Path) {
        if let Some(to) = self.reload_to.as_ref() {
            let _ = to.send(Reload::Open(file.to_string_lossy().into_owned()));
        }
        if let Some(watch) = self.watch.as_ref()
            && let Err(error) = watch.follow(file)
        {
            eprintln!("reader: can't watch {} for saves: {error}", file.display());
        }
    }

    /// Lets go of a page set to take the page's place, and its bands.
    pub(crate) fn let_incoming_go(&mut self) {
        if let Some(old) = self.incoming.take()
            && let Some(gl) = self.gl.as_ref()
        {
            for (_, texture) in old.textures {
                gl.forget(texture);
            }
        }
    }

    /// Does what a key asks, `count` times where a count means anything.
    pub(crate) fn act(&mut self, act: keys::Act, count: Option<u32>) {
        use keys::Act;
        let from = (self.heading() / self.px_per_pt(), self.cursor_pt());
        let screen_pt = self.screen_px() / self.px_per_pt();
        self.leapt = false;
        // The prompt stays while n and N go on, as Neovim's search count.
        if !matches!(
            act,
            Act::Next | Act::Previous | Act::SearchForward | Act::SearchBackward
        ) {
            self.prompt_shown = false;
        }
        // Any key closes the key list, as which-key's, and does what it does,
        // and Space ? again too; ZZ and Space w close only the list, and
        // Escape closes nothing, as Joel asked. While it's taller than the
        // window, the keys that move the view scroll it instead.
        let listed = self.keys_shown;
        if listed && matches!(act, Act::Rest | Act::Close) {
            if act == Act::Close {
                self.show_keys(false);
            }
            self.acted(act, count, from, screen_pt);
            return;
        }
        if listed && self.scroll_keys(act, count) {
            if !self.frame_pending {
                self.draw();
            }
            return;
        }
        if listed {
            self.show_keys(false);
        }
        // Selecting, the keys move the selection's cursor and the view
        // follows it; a key that moves the view takes the cursor with it.
        let selecting = self.selection.is_some();
        let handled = !(listed && act == Act::Rest) && self.select_act(act, count);
        if !handled {
            self.view_act(act, count, listed);
            if selecting {
                self.after_view_moved(matches!(act, Act::Next | Act::Previous));
            }
        }
        self.acted(act, count, from, screen_pt);
    }

    /// Ctrl+E and Ctrl+Y a line, Ctrl+D and Ctrl+U half a screen, and
    /// Ctrl+F and Ctrl+B a screen, `n` times. Ctrl+D and Ctrl+U take the
    /// cursor as far as the view, as vi's scroll option has it; the others
    /// leave it on its line while that stays in view.
    fn scroll(&mut self, act: keys::Act, n: f64) {
        use keys::Act;
        let (screen, line) = (self.screen_px(), line_pt() * self.px_per_pt());
        let (at, here) = (self.heading(), self.cursor_pt());
        let by = match act {
            Act::ScrollDown => n * line,
            Act::ScrollUp => -n * line,
            Act::HalfDown => n * screen / 2.0,
            Act::HalfUp => -n * screen / 2.0,
            Act::ScreenDown => n * screen,
            _ => -n * screen,
        };
        match act {
            Act::ScrollDown | Act::ScrollUp => self.step_to(at + by),
            _ => self.jump_to(at + by),
        }
        let carried = if matches!(act, Act::HalfDown | Act::HalfUp) {
            by / self.px_per_pt()
        } else {
            0.0
        };
        self.keep_cursor(here + carried);
    }

    /// Does what a key asks of the view, at rest or of one it moves while
    /// selecting.
    fn view_act(&mut self, act: keys::Act, count: Option<u32>, listed: bool) {
        use keys::Act;
        let n = f64::from(count.unwrap_or(1));
        let line = line_pt() * self.px_per_pt();
        let at = self.heading();
        let here = self.cursor_pt();
        match act {
            Act::Down => self.step_cursor(count_of(count)),
            Act::Up => self.step_cursor(-count_of(count)),
            Act::ScrollDown
            | Act::ScrollUp
            | Act::HalfDown
            | Act::HalfUp
            | Act::ScreenDown
            | Act::ScreenUp => self.scroll(act, n),
            // gg and a count's G take the cursor to the view's top line, and G
            // alone to its bottom one, as vi's go to a line and to the last.
            Act::First => {
                self.leap(count.map_or(0.0, |n| f64::from(n.saturating_sub(1)) * line));
                self.aim = None;
            }
            Act::Last => {
                self.leap(count.map_or(f64::MAX, |n| f64::from(n.saturating_sub(1)) * line));
                self.aim = count.is_none().then_some(f64::INFINITY);
            }
            // A mark is the cursor's line, as vi's is, and going to it puts
            // the cursor back on that line.
            Act::SetMark(letter) => {
                self.marks.insert(letter, here);
            }
            Act::GoToMark(letter) => {
                if let Some(&pt) = self.marks.get(&letter) {
                    self.remember(at / self.px_per_pt());
                    self.bring(pt);
                }
            }
            Act::LastJump => {
                if let Some(spot) = self.last_jump.clone() {
                    self.leap(spot.pt * self.px_per_pt());
                    self.aim = Some(spot.aim);
                }
            }
            Act::JumpBack => {
                if self.jump_at > 0 {
                    if self.jump_at == self.jumps.len() {
                        self.jumps.push(Spot {
                            file: self.file.clone(),
                            pt: at / self.px_per_pt(),
                            aim: here,
                        });
                    }
                    self.jump_at -= 1;
                    self.revisit(self.jumps[self.jump_at].clone());
                }
            }
            Act::JumpForward => {
                if self.jump_at + 1 < self.jumps.len() {
                    self.jump_at += 1;
                    self.revisit(self.jumps[self.jump_at].clone());
                }
            }
            Act::Close => self.exit = true,
            Act::Links => self.show_links(),
            // A tenth larger or smaller a step, as a browser's, the view on
            // the same words, within half and three times its own size.
            Act::ZoomIn | Act::ZoomOut | Act::ZoomBack => {
                let step = match act {
                    Act::ZoomIn => 1.1_f64.powf(n),
                    Act::ZoomOut => 1.1_f64.powf(-n),
                    _ => 1.0 / self.zoom,
                };
                self.jump = None;
                self.set_zoom((self.zoom * step).clamp(ZOOM_LEAST, ZOOM_MOST));
                self.clamp();
            }
            Act::SearchForward | Act::SearchBackward => {
                self.typing = Some(search::Typing::new(
                    act == Act::SearchForward,
                    at / self.px_per_pt(),
                ));
                self.refind();
            }
            Act::Next | Act::Previous => self.go_on(act == Act::Next, count.unwrap_or(1)),
            Act::ShowKeys => self.show_keys(!listed),
            Act::AgainCommand => self.again_command(),
            Act::Change(_) => self.say(ex::CANNOT_CHANGE),
            // Escape at rest does nothing, as Vim's does; :noh puts the
            // search's marks away. The page is set to fit the window's
            // width, so there's never anything beside it to move to; h and
            // l move only a selection's cursor, which takes them, with its
            // own keys, before they get here.
            Act::Rest
            | Act::Left
            | Act::Right
            | Act::Select
            | Act::SelectPieces
            | Act::WordOn
            | Act::WordBack
            | Act::WordEnd
            | Act::LineStart
            | Act::LineEnd
            | Act::OtherEnd
            | Act::Object(_)
            | Act::Copy
            | Act::CopyPiece
            | Act::CopyParagraph
            | Act::CopyAParagraph
            | Act::CopyToParagraphEnd => {}
            other => self.act_as_vi(other, count),
        }
    }

    /// After a key: a move longer than a screen kept as a jump, from where
    /// the view's top and the cursor's line were, and what it did logged for
    /// the tests.
    fn acted(&mut self, act: keys::Act, count: Option<u32>, from: (f64, f64), screen_pt: f64) {
        use keys::Act;
        let (from_pt, from_aim) = from;
        // A move longer than a screen is a jump, as vi's are, however it
        // was made; going through the jumps never adds to them.
        let to_pt = self.heading() / self.px_per_pt();
        if !self.leapt
            && !matches!(
                act,
                Act::JumpBack | Act::JumpForward | Act::ZoomIn | Act::ZoomOut | Act::ZoomBack
            )
            && (to_pt - from_pt).abs() > screen_pt + 0.5
        {
            self.remember_at(from_pt, from_aim);
        }
        if keys_logged() {
            // What it selected and where the cursor is go first, so they're
            // there once the key is.
            self.log_selection();
            eprintln!("cursor: {:.1} pt", self.cursor_pt());
            eprintln!(
                "key {act:?} {count:?}: heading {:.1} pt",
                self.heading() / self.px_per_pt()
            );
            if matches!(
                act,
                Act::Next
                    | Act::Previous
                    | Act::SearchForward
                    | Act::SearchBackward
                    | Act::SameWordOn
                    | Act::SameWordBack
                    | Act::Rest
            ) {
                self.log_search(&format!("{act:?}"));
            }
        }
        if !self.frame_pending {
            self.draw();
        }
    }

    /// ]b and [b: the markdown file a count on or back in this one's
    /// folder, by name, round its ends.
    pub(crate) fn file_beside(&mut self, on: bool, times: u32) {
        let files = moves::markdown_files(folder_of(&self.file));
        if let Some(file) = moves::neighbour(&files, &self.file, times, on) {
            if keys_logged() {
                eprintln!("file: {}", file.display());
            }
            self.remember(self.heading() / self.px_per_pt());
            self.open(file, Landing::Kept);
        }
    }

    /// Ctrl+]: the first link from the cursor's line down, in view.
    fn follow_link_here(&mut self) {
        let from = self.cursor_pt();
        let (_, foot) = self.view_pt();
        let link = links::links(&self.painter.page().frame)
            .into_iter()
            .filter(|link| link.baseline_pt > from && link.top_pt < foot)
            .min_by(|a, b| {
                a.baseline_pt
                    .round()
                    .total_cmp(&b.baseline_pt.round())
                    .then(a.left_pt.total_cmp(&b.left_pt))
            });
        match link {
            Some(link) => self.follow(&link),
            None => self.say("No link from the line here"),
        }
    }

    /// The keys vi moves its cursor and the view by that the page took on
    /// last: paragraphs and headings, the lines in view, the view on a
    /// line, brackets, the word here, the files beside this one, and links,
    /// the outline and the command line.
    fn act_as_vi(&mut self, act: keys::Act, count: Option<u32>) {
        use keys::Act;
        let times = count.unwrap_or(1);
        let which = |act| match act {
            Act::ViewTop | Act::PlaceTop => moves::InView::Top,
            Act::ViewMiddle | Act::PlaceMiddle => moves::InView::Middle,
            _ => moves::InView::Bottom,
        };
        match act {
            Act::ParagraphOn | Act::ParagraphBack | Act::HeadingOn | Act::HeadingBack => {
                let blocks = place::blocks(&self.painter.page().frame);
                let to = moves::block_from(
                    &blocks,
                    self.words.as_deref().unwrap_or_default(),
                    self.cursor_pt(),
                    times,
                    matches!(act, Act::ParagraphOn | Act::HeadingOn),
                    matches!(act, Act::HeadingOn | Act::HeadingBack),
                );
                self.bring(to);
            }
            Act::ViewTop | Act::ViewMiddle | Act::ViewBottom => {
                let (top, foot) = self.view_pt();
                if let Some(lines) = self.words.as_deref()
                    && let Some(i) = moves::line_in_view(lines, top, foot, which(act), times)
                {
                    self.aim = Some(moves::top_of(&lines[i]));
                }
            }
            // With a count, zt goes to that line of the file, as vi's.
            Act::PlaceTop | Act::PlaceMiddle | Act::PlaceBottom => {
                let (top, foot) = self.view_pt();
                let line = match count {
                    Some(n) => {
                        let blocks = place::blocks(&self.painter.page().frame);
                        moves::source_line(&blocks, n as usize).and_then(|pt| self.line_from(pt))
                    }
                    None => self.cursor(),
                };
                if let Some((i, lines)) = line.zip(self.words.as_deref()) {
                    let to = moves::placed(&lines[i], foot - top, which(act));
                    self.aim = Some(moves::top_of(&lines[i]));
                    self.step_to(to * self.px_per_pt());
                }
            }
            Act::Bracket => {
                let line = match count {
                    Some(n) => self
                        .words
                        .as_deref()
                        .and_then(|lines| moves::percent(lines, n)),
                    None => self
                        .cursor()
                        .zip(self.words.as_deref())
                        .and_then(|(i, lines)| moves::bracket(lines, i)),
                };
                if let Some(i) = line {
                    self.reach(i);
                }
            }
            Act::SameWordOn | Act::SameWordBack => self.same_word(act == Act::SameWordOn, times),
            Act::FileOn | Act::FileBack => self.file_beside(act == Act::FileOn, times),
            Act::FollowLink => self.follow_link_here(),
            Act::FollowFile => self.follow_file(),
            Act::Outline => {
                let blocks = place::blocks(&self.painter.page().frame);
                self.outline = outline::Outline::new(&blocks, self.view_pt().0);
                if self.outline.is_some() {
                    self.ask_outline();
                } else {
                    self.say("No headings on this page");
                }
            }
            Act::Command => {
                self.commanding = Some(search::Typing::command(self.heading() / self.px_per_pt()));
                // The menu shows the moment : is typed.
                self.narrow_menu();
                self.ask_prompt();
            }
            _ => {}
        }
    }

    /// Takes a key pressed, or repeated while `held`. Typing a search or a
    /// command takes repeats as presses, as a held Backspace goes on; a
    /// link's label and a page's keys take only those that move.
    pub(crate) fn key(&mut self, event: &KeyEvent, held: bool) {
        if let Some(key) = key_of(event, self.ctrl, self.shift) {
            if self.labelling.is_some() {
                if !held {
                    self.label_key(key);
                }
                return;
            }
            // What a command said stays until the next key, as Neovim's.
            if self.said.take().is_some() {
                self.prompt_shown = false;
                if !self.frame_pending {
                    self.draw();
                }
            }
            if let Some(typing) = self.commanding.as_mut() {
                if let Some(line) = keys::line_key(key, true).filter(|line| line.menus()) {
                    self.menu_key(line);
                    return;
                }
                let said = typing.key(key, held);
                self.commanded(said);
                return;
            }
            if self.outline.is_some() {
                self.outline_key(key, held);
                return;
            }
            if let Some(typing) = self.typing.as_mut() {
                let said = typing.key(key, held);
                self.typed(said);
                return;
            }
            let selecting = self.selection.is_some();
            let done = if held {
                self.keys.held(key, selecting)
            } else {
                self.keys.press(key, selecting)
            };
            if let Some((act, count)) = done {
                self.act(act, count);
            }
        }
    }
}

/// A key's count as lines to move by: one when none was typed.
fn count_of(count: Option<u32>) -> isize {
    isize::try_from(count.unwrap_or(1)).unwrap_or(isize::MAX)
}

/// How far the page zooms: half its own size, and three times it.
const ZOOM_LEAST: f64 = 0.5;
const ZOOM_MOST: f64 = 3.0;

/// A key as the reader's table knows it: by what it types, so Colemak's
/// letters are vi's, with Ctrl named apart since it changes what's typed.
fn key_of(event: &KeyEvent, ctrl: bool, shift: bool) -> Option<keys::Key> {
    Some(match event.keysym {
        Keysym::Escape => keys::Key::Escape,
        Keysym::Return | Keysym::KP_Enter => keys::Key::Return,
        Keysym::BackSpace => keys::Key::Backspace,
        Keysym::Up => keys::Key::Up,
        Keysym::Down => keys::Key::Down,
        Keysym::Left => keys::Key::Left,
        Keysym::Right => keys::Key::Right,
        Keysym::Page_Up => keys::Key::PageUp,
        Keysym::Page_Down => keys::Key::PageDown,
        Keysym::F1 => keys::Key::F1,
        Keysym::Tab if shift => keys::Key::BackTab,
        Keysym::Tab => keys::Key::Tab,
        Keysym::ISO_Left_Tab => keys::Key::BackTab,
        sym => {
            let typed = sym.key_char()?;
            if ctrl {
                keys::Key::Ctrl(typed.to_ascii_lowercase())
            } else {
                keys::Key::Char(typed)
            }
        }
    })
}

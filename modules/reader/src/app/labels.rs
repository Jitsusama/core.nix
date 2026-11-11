//! The keys list and the links' labels: shown, scrolled and drawn, a label
//! typed to follow its link, and a prompt's answer taken.

use super::{Landing, desktop_open, painted};
use crate::{App, keys, keys_logged, links, page, prompt};

impl App {
    /// Shows or hides the key list from its top, asking for it set at this
    /// scale and for this window's width if it isn't yet.
    pub(crate) fn show_keys(&mut self, shown: bool) {
        if shown && !self.keys_shown {
            self.keys_scroll = 0.0;
        }
        if shown != self.keys_shown && keys_logged() {
            eprintln!("keys: {}", if shown { "shown" } else { "away" });
        }
        self.keys_shown = shown;
        let px_per_pt = self.prompt_px_per_pt();
        let width_pt = self.gl.as_ref().map_or(0.0, |gl| f64::from(gl.width)) / px_per_pt;
        if shown
            && ((self.keys_px - px_per_pt).abs() > f64::EPSILON
                || (self.keys_width_pt - width_pt).abs() > f64::EPSILON)
            && let Some(prompt) = self.prompt.as_ref()
        {
            self.keys_px = px_per_pt;
            self.keys_width_pt = width_pt;
            prompt.ask(prompt::Ask {
                serial: 0,
                keys: true,
                labels: Vec::new(),
                outline: Vec::new(),
                menu: None,
                note: String::new(),
                mark: "",
                typed: String::new(),
                cursor: false,
                count: None,
                width_pt,
                px_per_pt,
                page: self.width,
            });
        }
    }

    /// How far the key list can scroll, in device pixels: what of it stands
    /// past the window's height, none while it fits.
    fn keys_overflow(&self) -> f64 {
        let tall = self.keys_canvas.as_ref().map_or(0, |it| it.height);
        (f64::from(tall) - self.screen_px()).max(0.0)
    }

    /// Scrolls a key list taller than the window by a key that moves the
    /// view, as which-key scrolls its own, so every key in it can be read;
    /// any other key, or one while it fits, is the list's to close.
    pub(crate) fn scroll_keys(&mut self, act: keys::Act, count: Option<u32>) -> bool {
        use keys::Act;
        let most = self.keys_overflow();
        if most <= 0.0 {
            return false;
        }
        let row = page::pitch_pt() * self.prompt_px_per_pt();
        let screen = self.screen_px();
        let times = f64::from(count.unwrap_or(1));
        let by = match act {
            Act::Down | Act::ScrollDown => row * times,
            Act::Up | Act::ScrollUp => -row * times,
            Act::HalfDown => screen / 2.0 * times,
            Act::HalfUp => -screen / 2.0 * times,
            Act::ScreenDown => screen * times,
            Act::ScreenUp => -screen * times,
            Act::First => -most,
            Act::Last => most,
            _ => return false,
        };
        self.keys_scroll = (self.keys_scroll + by).clamp(0.0, most);
        true
    }

    /// Space ?'s key list, centred, as his which-key, on whole pixels so its
    /// type is sharp; taller than the window, from its top down as far as
    /// it's scrolled.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "GL places in f32, and the list and the window are a few thousand pixels"
    )]
    pub(crate) fn draw_keys(&mut self) {
        let screen = self.screen_px() as f32;
        let Some(gl) = self.gl.as_ref() else { return };
        if self.keys_shown
            && let Some(canvas) = painted(gl, &mut self.keys_canvas, self.keys_set.take())
        {
            let (width, height) = (canvas.width as f32, canvas.height as f32);
            let x = ((gl.width as f32 - width) / 2.0).round().max(0.0);
            let y = if height > screen {
                let most = height - screen;
                -(self.keys_scroll as f32).clamp(0.0, most).round()
            } else {
                ((screen - height) / 2.0).round()
            };
            gl.draw(canvas, x, y, width, height);
        }
    }

    /// g x's labels, each over its link's first letters with its letters'
    /// baseline on the link's, on whole pixels so its letters are sharp; a
    /// newly set batch is painted first.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "points and pixels as f32, as GL takes them, within a page's few hundred thousand"
    )]
    pub(crate) fn draw_labels(&mut self) {
        if self.labelling.is_none() {
            return;
        }
        let px = self.px_per_pt();
        let offset = self.offset;
        let left = self.page_x();
        let Some(gl) = self.gl.as_ref() else { return };
        if let Some(set) = self.labels_set.take() {
            for (old, _) in self.label_canvases.drain(..) {
                gl.forget(old);
            }
            for label in set {
                if let Ok(canvas) = gl.canvas(label.width, label.height) {
                    gl.paint(&canvas, &label.pixels);
                    self.label_canvases.push((canvas, label.baseline_px));
                }
            }
        }
        if let Some((_, shown)) = self.labelling.as_ref() {
            for (link, (canvas, baseline_px)) in shown.iter().zip(&self.label_canvases) {
                let (width, height) = (canvas.width as f32, canvas.height as f32);
                let x = (link.left_pt * px).round() as f32 + left;
                let y = links::label_top(link, *baseline_px, px, offset);
                gl.draw(canvas, x, y, width, height);
            }
        }
    }

    /// Labels every link in view, as flash labels a jump: a label's letters
    /// typed follow it, and anything else puts them away.
    pub(crate) fn show_links(&mut self) {
        let px = self.px_per_pt();
        let top = self.heading() / px;
        let foot = top + self.screen_px() / px;
        let shown: Vec<links::Link> = links::links(&self.painter.page().frame)
            .into_iter()
            .filter(|link| link.top_pt + link.height_pt > top && link.top_pt < foot)
            .collect();
        if keys_logged() {
            eprintln!("links: {} in view", shown.len());
        }
        if shown.is_empty() {
            return;
        }
        let labelling = links::Labelling::new(shown.len());
        if let Some(prompt) = self.prompt.as_ref() {
            prompt.ask(prompt::Ask {
                serial: 0,
                keys: false,
                labels: labelling.labels.clone(),
                outline: Vec::new(),
                menu: None,
                note: String::new(),
                mark: "",
                typed: String::new(),
                cursor: false,
                count: None,
                width_pt: 0.0,
                px_per_pt: px,
                page: self.width,
            });
        }
        self.labelling = Some((labelling, shown));
    }

    /// A key while the labels show: a letter of one, or else they go.
    pub(crate) fn label_key(&mut self, key: keys::Key) {
        let Some((labelling, shown)) = self.labelling.as_mut() else {
            return;
        };
        let chosen = match key {
            keys::Key::Char(letter) => match labelling.key(letter) {
                links::Typed::Waiting => return,
                links::Typed::Chosen(at) => shown.get(at).cloned(),
                links::Typed::Gone => None,
            },
            _ => None,
        };
        self.labelling = None;
        for (old, _) in self.label_canvases.drain(..) {
            if let Some(gl) = self.gl.as_ref() {
                gl.forget(old);
            }
        }
        if let Some(link) = chosen {
            self.follow(&link);
        }
        if !self.frame_pending {
            self.draw();
        }
    }

    /// Goes where a link points: to a heading here, as a jump Ctrl+O comes
    /// back from, or anywhere else as the desktop opens it.
    pub(crate) fn follow(&mut self, link: &links::Link) {
        if keys_logged() {
            eprintln!("link: {}", link.href);
        }
        match links::target(&link.href, &self.file) {
            // A link to the page itself, with no heading, goes to its top,
            // and one to a heading puts the cursor on it, as gg and ]] do.
            links::Target::Here(anchor) if anchor.is_empty() => {
                self.leap(0.0);
                self.aim = None;
            }
            links::Target::Here(anchor) => {
                let heading = self
                    .words
                    .as_ref()
                    .and_then(|lines| links::heading_at(lines, &anchor));
                match heading {
                    Some(pt) => {
                        self.remember(self.heading() / self.px_per_pt());
                        self.land(pt.max(0.0));
                    }
                    None => eprintln!("reader: no heading here is #{anchor}"),
                }
            }
            links::Target::Page(file, heading) => {
                if !file.is_file() {
                    eprintln!("reader: there's no {}", file.display());
                    return;
                }
                self.remember(self.heading() / self.px_per_pt());
                self.open(file, heading.map_or(Landing::Kept, Landing::Heading));
            }
            links::Target::Elsewhere(to) => desktop_open(&to),
        }
    }

    pub(crate) fn prompt_arrived(&mut self, mut set: prompt::Set) {
        if set.outline {
            if self.outline.is_some() || self.menu_shows() {
                self.read_on(set.reading.take());
                self.outline_set = Some(set);
                if !self.frame_pending {
                    self.draw();
                }
            }
            return;
        }
        if !set.labels.is_empty() {
            if self.labelling.is_some() {
                self.labels_set = Some(set.labels);
                if !self.frame_pending {
                    self.draw();
                }
            }
            return;
        }
        if set.keys {
            self.keys_set = Some(set);
            if self.keys_shown && !self.frame_pending {
                self.draw();
            }
            return;
        }
        if set.serial == self.prompt_serial {
            self.prompt_set = Some(set);
            if !self.frame_pending {
                self.draw();
            }
        }
    }
}

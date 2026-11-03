//! Draws the page's bands off the main thread. The main thread asks for a
//! band by its key and carries on; the band comes back through the loop's
//! channel when it's drawn, so nothing on screen ever waits on drawing.

use std::sync::Arc;
use std::time::Instant;

use smithay_client_toolkit::reexports::calloop::channel::Sender;
use typst_layout::Page;

use crate::tiles;

/// A band's height in device pixels: small enough that uploading one costs
/// a sliver of a frame, and a screen holds four or so.
pub const BAND_PX: u32 = 512;

/// Which band: the zoom it was drawn at, in thousandths, and its place down
/// the page.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Key {
    pub zoom: u32,
    pub index: u32,
}

pub struct Band {
    pub key: Key,
    /// Which page it was drawn from: the first screen's, then the whole.
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub drawn_ms: f64,
}

pub struct Painter {
    page: Arc<Page>,
    /// Counts the pages it has been given.
    pub version: u32,
    pool: rayon::ThreadPool,
    done: Sender<Band>,
    /// Device pixels per point at zoom 1: the buffer's scale over 0.75 points
    /// a logical pixel.
    pub base: f64,
}

impl Painter {
    pub fn new(page: Page, base: f64, done: Sender<Band>) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .thread_name(|n| format!("painter-{n}"))
            .build()
            .expect("the painters' threads");
        Painter {
            page: Arc::new(page),
            version: 0,
            pool,
            done,
            base,
        }
    }

    /// Draws from this page from now on; bands asked before are of the last.
    pub fn replace(&mut self, page: Page) {
        self.page = Arc::new(page);
        self.version += 1;
    }

    /// Takes a page whose bands were drawn already, as `version`, while the
    /// page before it showed.
    pub fn take(&mut self, page: Arc<Page>, version: u32) {
        self.page = page;
        self.version = version;
    }

    /// The page it draws from now.
    pub fn page(&self) -> Arc<Page> {
        Arc::clone(&self.page)
    }

    /// How tall the page is in points.
    pub fn height_pt(&self) -> f64 {
        self.page.frame.size().y.to_pt()
    }

    /// Where a band's foot falls down the page, in points.
    pub fn foot_pt(&self, key: Key) -> f64 {
        f64::from(key.index + 1) * f64::from(BAND_PX) / self.scale(key.zoom)
    }

    /// Device pixels per point at a zoom.
    pub fn scale(&self, zoom: u32) -> f64 {
        self.base * f64::from(zoom) / 1000.0
    }

    /// How tall the page is in device pixels at a zoom.
    pub fn height_px(&self, zoom: u32) -> f64 {
        self.page.frame.size().y.to_pt() * self.scale(zoom)
    }

    /// How many bands cover the page at a zoom.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a band's number and size are whole pixels within a page's few hundred thousand"
    )]
    pub fn bands(&self, zoom: u32) -> u32 {
        (self.height_px(zoom) / f64::from(BAND_PX)).ceil() as u32
    }

    /// Draws a band and sends it back; the asker doesn't wait.
    pub fn ask(&self, key: Key) {
        self.ask_from(Arc::clone(&self.page), self.version, key);
    }

    /// Draws a band of a page that isn't shown yet, as `version`.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a band's number and size are whole pixels within a page's few hundred thousand"
    )]
    pub fn ask_from(&self, page: Arc<Page>, version: u32, key: Key) {
        let done = self.done.clone();
        let scale = self.scale(key.zoom);
        self.pool.spawn(move || {
            let start = Instant::now();
            let height_pt = f64::from(BAND_PX) / scale;
            let top_pt = f64::from(key.index) * height_pt;
            let pixmap = tiles::draw(&page, top_pt, height_pt, scale as f32);
            let band = Band {
                key,
                version,
                width: pixmap.width(),
                height: pixmap.height(),
                pixels: pixmap.take(),
                drawn_ms: start.elapsed().as_secs_f64() * 1000.0,
            };
            // The loop has gone when the window has: nothing to tell.
            let _ = done.send(band);
        });
    }
}

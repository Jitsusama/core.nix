//! screen-text: reads the words in Joel's screenshots on the NPU, so they
//! can be found by them and copied out (the glossary's screenshot text).
//!
//! `PaddleOCR`'s `PP-OCRv5` does it in two models: one finds where each line
//! of text is (find.rs), the other reads each line (read.rs). Both are
//! compiled at fixed shapes, as the NPU needs, at its background priority,
//! since nobody waits on a screenshot's words.
//!
//! ```text
//! screen-text read MODELS PICTURE...
//! ```
//!
//! prints each picture's lines in reading order. MODELS is a folder
//! holding the detector (det.onnx), the recognizer (rec.onnx) and the
//! characters the recognizer knows, one a line (characters.txt), as
//! modules/screen-text/models.nix makes it. `SCREEN_TEXT_DEVICE` names the
//! device, the NPU unless it says otherwise, and `SCREEN_TEXT_CACHE` where
//! `OpenVINO` keeps what it compiles, so the next start only reads it.

mod find;
mod picture;
mod read;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use dictation::openvino::Core;

use crate::find::Finder;
use crate::picture::Picture;
use crate::read::Reader;

const USAGE: &str = "usage: screen-text read MODELS PICTURE...";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let done = match args.split_first() {
        Some((ask, rest)) if ask == "read" && rest.len() >= 2 => {
            read_all(Path::new(&rest[0]), &rest[1..])
        }
        _ => Err(USAGE.to_owned()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("screen-text: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Both models, compiled for the device named.
pub struct Models {
    finder: Finder,
    reader: Reader,
    // Last, so it's let go after what it compiled.
    _core: Core,
}

impl Models {
    pub fn load(folder: &Path) -> Result<Self, String> {
        let device = std::env::var("SCREEN_TEXT_DEVICE").unwrap_or_else(|_| "NPU".to_owned());
        let cache = std::env::var_os("SCREEN_TEXT_CACHE").map(PathBuf::from);
        let core = Core::new(cache.as_deref())?;
        let finder = Finder::load(&core, &folder.join("det.onnx"), &device)?;
        let reader = Reader::load(
            &core,
            &folder.join("rec.onnx"),
            &folder.join("characters.txt"),
            &device,
        )?;
        Ok(Self {
            finder,
            reader,
            _core: core,
        })
    }

    /// The lines of text in a picture, in reading order.
    pub fn lines(&mut self, picture: &Picture) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        for place in read::in_order(self.finder.find(picture)?) {
            if let Some(line) = self.reader.read(picture, place)? {
                lines.push(line.text.trim().to_owned());
            }
        }
        Ok(lines)
    }
}

fn read_all(models: &Path, pictures: &[String]) -> Result<(), String> {
    let began = Instant::now();
    let mut models = Models::load(models)?;
    eprintln!("screen-text: loaded in {} ms", began.elapsed().as_millis());
    for path in pictures {
        let began = Instant::now();
        let picture = Picture::read(Path::new(path))?;
        let lines = models.lines(&picture)?;
        eprintln!(
            "screen-text: {path}, {} lines in {} ms",
            lines.len(),
            began.elapsed().as_millis()
        );
        for line in lines {
            println!("{line}");
        }
    }
    Ok(())
}

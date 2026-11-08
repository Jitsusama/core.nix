//! Every meaning embedded, kept by the text it's the meaning of, so a
//! thing is embedded once however often it's asked about, and a restart
//! embeds only what's new.
//!
//! The file opens with a line naming the model, so a new model's
//! meanings never sit beside an old one's: a file naming another is read
//! as empty, and written over at the next save. Then, for each text, its
//! length in bytes and the text, then its meaning's numbers, each as four
//! bytes, little-endian.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::text::WIDTH;

/// The meanings kept, by text, and where.
pub struct Store {
    meanings: HashMap<String, Vec<f32>>,
    file: PathBuf,
    model: String,
    /// Whether a meaning came since the last save.
    unsaved: bool,
}

impl Store {
    /// The meanings `file` holds for `model`, or none.
    pub fn read(file: &Path, model: &str) -> Self {
        let meanings = std::fs::read(file)
            .ok()
            .and_then(|bytes| parse(&bytes, model))
            .unwrap_or_default();
        Self {
            meanings,
            file: file.to_owned(),
            model: model.to_owned(),
            unsaved: false,
        }
    }

    pub fn get(&self, text: &str) -> Option<&[f32]> {
        self.meanings.get(text).map(Vec::as_slice)
    }

    pub fn holds(&self, text: &str) -> bool {
        self.meanings.contains_key(text)
    }

    pub fn len(&self) -> usize {
        self.meanings.len()
    }

    pub fn put(&mut self, text: &str, meaning: Vec<f32>) {
        self.meanings.insert(text.to_owned(), meaning);
        self.unsaved = true;
    }

    /// Lets go of every meaning but those of `texts`, as what's known
    /// changes, so the file holds only what's asked about.
    pub fn keep_only<'a>(&mut self, texts: impl Iterator<Item = &'a str>) {
        let kept: std::collections::HashSet<&str> = texts.collect();
        let before = self.meanings.len();
        self.meanings.retain(|text, _| kept.contains(text.as_str()));
        self.unsaved |= self.meanings.len() != before;
    }

    /// Writes the meanings out whole, when any changed: into a file
    /// beside it, then renamed over it, so nothing reads half of it.
    pub fn save(&mut self) {
        if !self.unsaved {
            return;
        }
        let mut bytes = format!("{}\n", self.model).into_bytes();
        let mut texts: Vec<&String> = self.meanings.keys().collect();
        texts.sort();
        for text in texts {
            let length = u32::try_from(text.len()).unwrap_or(u32::MAX);
            bytes.extend_from_slice(&length.to_le_bytes());
            bytes.extend_from_slice(text.as_bytes());
            for value in &self.meanings[text] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        let name = self.file.file_name().unwrap_or_default().to_string_lossy();
        let part = self.file.with_file_name(format!(".{name}.part"));
        let written = self
            .file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&part, &bytes))
            .and_then(|()| std::fs::rename(&part, &self.file));
        match written {
            Ok(()) => self.unsaved = false,
            Err(why) => eprintln!("meaning: keeping {}: {why}", self.file.display()),
        }
    }
}

/// The meanings in a file's bytes, or none when it names another model
/// or is cut short.
fn parse(bytes: &[u8], model: &str) -> Option<HashMap<String, Vec<f32>>> {
    let line = bytes.iter().position(|&byte| byte == b'\n')?;
    if bytes[..line] != *model.as_bytes() {
        return None;
    }
    let mut meanings = HashMap::new();
    let mut at = line + 1;
    let take = |at: &mut usize, count: usize| -> Option<&[u8]> {
        let slice = bytes.get(*at..*at + count)?;
        *at += count;
        Some(slice)
    };
    while at < bytes.len() {
        let length = u32::from_le_bytes(take(&mut at, 4)?.try_into().ok()?);
        let text = std::str::from_utf8(take(&mut at, usize::try_from(length).ok()?)?).ok()?;
        let meaning = take(&mut at, WIDTH * 4)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|four| f32::from_le_bytes(*four))
            .collect();
        meanings.insert(text.to_owned(), meaning);
    }
    Some(meanings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meaning(seed: f32) -> Vec<f32> {
        (0..WIDTH)
            .map(|i| seed + f32::from(u16::try_from(i).unwrap()))
            .collect()
    }

    #[test]
    fn what_was_kept_reads_back_for_the_same_model() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("meanings");
        let mut store = Store::read(&file, "bge");
        store.put("Brightness", meaning(1.0));
        store.put("Wi-Fi", meaning(2.0));
        store.save();
        let read = Store::read(&file, "bge");
        assert_eq!(read.len(), 2);
        assert_eq!(read.get("Wi-Fi"), Some(meaning(2.0).as_slice()));
    }

    #[test]
    fn another_models_meanings_are_never_read() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("meanings");
        let mut store = Store::read(&file, "bge");
        store.put("Brightness", meaning(1.0));
        store.save();
        assert_eq!(Store::read(&file, "siglip").len(), 0);
    }

    #[test]
    fn a_file_cut_short_reads_as_nothing_rather_than_half() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("meanings");
        let mut store = Store::read(&file, "bge");
        store.put("Brightness", meaning(1.0));
        store.save();
        let bytes = std::fs::read(&file).unwrap();
        std::fs::write(&file, &bytes[..bytes.len() - 1]).unwrap();
        assert_eq!(Store::read(&file, "bge").len(), 0);
    }

    #[test]
    fn only_what_is_still_known_is_kept() {
        let folder = tempfile::tempdir().unwrap();
        let mut store = Store::read(&folder.path().join("meanings"), "bge");
        store.put("Brightness", meaning(1.0));
        store.put("Gone", meaning(2.0));
        store.keep_only(["Brightness"].into_iter());
        assert!(store.holds("Brightness") && !store.holds("Gone"));
    }
}

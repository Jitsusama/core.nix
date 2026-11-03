//! The Typst world pages are set in: fonts from the theme's folders
//! plus Typst's own maths face, `MiTeX`'s scope, and pictures made in memory.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook, FontInfo};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::{FontPath, FontStore};

/// Every face in the font folders, as kept in the font book: its file, its
/// index in that file, and what Typst knows of it.
type Faces = Vec<(PathBuf, u32, FontInfo)>;

/// The faces in `dirs`, read from the book kept for them when there is one,
/// and otherwise scanned and kept. Scanning reads every face of every file,
/// 30 MB for the reader's folders, which a build's memory pressure leaves
/// to come from the disk: 384 ms of a first frame. The book is keyed by the
/// folders and when each last changed, so a store path's never goes stale
/// and a folder changed in place is scanned again.
fn faces(dirs: &[String]) -> Faces {
    let mut key = DefaultHasher::new();
    for dir in dirs {
        dir.hash(&mut key);
        std::fs::metadata(dir)
            .and_then(|m| m.modified())
            .ok()
            .hash(&mut key);
    }
    let book = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .map(|cache| {
            cache
                .join("reader")
                .join(format!("fonts-{:016x}.json", key.finish()))
        });
    if let Some(faces) = book
        .as_ref()
        .and_then(|book| std::fs::read(book).ok())
        .and_then(|text| serde_json::from_slice(&text).ok())
    {
        return faces;
    }
    let faces: Faces = dirs
        .iter()
        .flat_map(|dir| typst_kit::fonts::scan(Path::new(dir)))
        .map(|(path, info)| (path.path, path.index, info))
        .collect();
    if let (Some(book), Ok(text)) = (book, serde_json::to_vec(&faces)) {
        // Written whole and moved into place, so a reader starting beside
        // this one never reads half a book.
        let fresh = book.with_extension(format!("{}.part", std::process::id()));
        if book
            .parent()
            .is_some_and(|dir| std::fs::create_dir_all(dir).is_ok())
            && std::fs::write(&fresh, text).is_ok()
        {
            let _ = std::fs::rename(&fresh, &book);
        }
    }
    faces
}

pub fn id(path: &str) -> FileId {
    FileId::new(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(path).expect("a virtual path"),
    ))
}

pub struct PageWorld {
    library: LazyHash<Library>,
    fonts: FontStore,
    main: Source,
    sources: HashMap<FileId, Source>,
    files: Mutex<HashMap<FileId, Bytes>>,
    /// Whether the specs of `MiTeX` were found, so equations can be set.
    mitex: bool,
}

impl PageWorld {
    pub fn new(font_dirs: &[String], mitex_dir: &Path) -> Self {
        let mut fonts = FontStore::new();
        fonts.extend(typst_kit::fonts::embedded());
        fonts.extend(
            faces(font_dirs)
                .into_iter()
                .map(|(path, index, info)| (FontPath { path, index }, info)),
        );
        let mut sources = HashMap::new();
        let names = [
            "specs/mod.typ",
            "specs/prelude.typ",
            "specs/latex/standard.typ",
        ];
        let specs: Option<Vec<String>> = names
            .iter()
            .map(|name| std::fs::read_to_string(mitex_dir.join(name)).ok())
            .collect();
        // Without MiTeX's specs the page still sets, each equation shown as
        // its source, broken, since one missing piece never takes the page.
        let mitex = specs.is_some();
        let specs = specs.unwrap_or_else(|| {
            eprintln!(
                "reader: no MiTeX in {}, so equations show as written",
                mitex_dir.display()
            );
            vec![
                "#let mitex-scope = (:)\n".to_string(),
                String::new(),
                String::new(),
            ]
        });
        for (name, text) in names.iter().zip(specs) {
            let file = id(&format!("/mitex/{name}"));
            sources.insert(file, Source::new(file, text));
        }
        Self {
            library: LazyHash::new(Library::default()),
            fonts,
            main: Source::new(id("/main.typ"), String::new()),
            sources,
            files: Mutex::new(HashMap::new()),
            mitex,
        }
    }

    /// Whether the specs of `MiTeX` were there to set equations with.
    pub fn sets_equations(&self) -> bool {
        self.mitex
    }

    /// Replaces the page's program, keeping what Typst memoised.
    pub fn set_main(&mut self, text: &str) {
        self.main.replace(text);
    }

    pub fn put_file(&self, path: &str, bytes: Vec<u8>) {
        self.files
            .lock()
            .unwrap()
            .insert(id(path), Bytes::new(bytes));
    }
}

impl World for PageWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, file: FileId) -> FileResult<Source> {
        if file == self.main.id() {
            return Ok(self.main.clone());
        }
        self.sources
            .get(&file)
            .cloned()
            .ok_or(FileError::AccessDenied)
    }

    fn file(&self, file: FileId) -> FileResult<Bytes> {
        self.files
            .lock()
            .unwrap()
            .get(&file)
            .cloned()
            .ok_or(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

//! git's objects: loose files, packs found through their indexes or one
//! multi-pack-index, and the stores an `alternates` file names. Nothing is
//! read whole: an index is searched where it lies on disk, a few bytes at a
//! time, so a repository of hundreds of millions of objects costs what one
//! of a hundred does. Finding an object in a pack's index is the work of
//! `pack_index`; reading it, loose or from a pack through deltas, is here.

use crate::hash::Oid;
use crate::pack_index::{Index, Midx};
use crate::stamp::{Stamp, Stamps};
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Commit,
    Tree,
    Blob,
    Tag,
}

impl Kind {
    fn from_pack(code: u8) -> Option<Kind> {
        match code {
            1 => Some(Kind::Commit),
            2 => Some(Kind::Tree),
            3 => Some(Kind::Blob),
            4 => Some(Kind::Tag),
            _ => None,
        }
    }

    fn from_name(name: &[u8]) -> Option<Kind> {
        match name {
            b"commit" => Some(Kind::Commit),
            b"tree" => Some(Kind::Tree),
            b"blob" => Some(Kind::Blob),
            b"tag" => Some(Kind::Tag),
            _ => None,
        }
    }
}

pub struct Object {
    pub kind: Kind,
    pub data: Vec<u8>,
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

struct Pack {
    data: File,
    /// Its own index, for a pack the multi-pack-index doesn't cover.
    index: Option<Index>,
}

/// One object store: its packs and the folder of loose objects.
struct Store {
    dir: PathBuf,
    midx: Option<Midx>,
    /// Packs by their `.idx` name; the multi-pack-index's come first, in its
    /// order.
    packs: Vec<Pack>,
    midx_packs: usize,
    /// The own indexes of the packs the multi-pack-index covers, in its
    /// order, read only once it can't answer a lookup.
    covered: OnceLock<Vec<Option<Index>>>,
}

impl Store {
    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "git names a pack's index .idx, in lowercase"
    )]
    fn open(dir: &Path) -> Store {
        let pack_dir = dir.join("pack");
        let midx = Midx::open(&pack_dir.join("multi-pack-index")).ok();
        let mut packs = Vec::new();
        let mut covered = std::collections::HashSet::new();
        let mut midx_packs = 0;
        if let Some(midx) = &midx {
            for name in &midx.packs {
                let data = pack_dir.join(name.replace(".idx", ".pack"));
                // A pack the index names but that's gone ends the index's use.
                match File::open(&data) {
                    Ok(data) => packs.push(Pack { data, index: None }),
                    Err(_) => {
                        return Store::without_midx(dir);
                    }
                }
                covered.insert(name.clone());
            }
            midx_packs = packs.len();
        }
        let mut names: Vec<String> = std::fs::read_dir(&pack_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.ends_with(".idx") && !covered.contains(name))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        for name in names {
            let index = Index::open(&pack_dir.join(&name));
            let data = File::open(pack_dir.join(name.replace(".idx", ".pack")));
            if let (Ok(index), Ok(data)) = (index, data) {
                packs.push(Pack {
                    data,
                    index: Some(index),
                });
            }
        }
        Store {
            dir: dir.to_path_buf(),
            midx,
            packs,
            midx_packs,
            covered: OnceLock::new(),
        }
    }

    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "git names a pack's index .idx, in lowercase"
    )]
    fn without_midx(dir: &Path) -> Store {
        let pack_dir = dir.join("pack");
        let mut names: Vec<String> = std::fs::read_dir(&pack_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.ends_with(".idx"))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        let packs = names
            .into_iter()
            .filter_map(|name| {
                let index = Index::open(&pack_dir.join(&name)).ok()?;
                let data = File::open(pack_dir.join(name.replace(".idx", ".pack"))).ok()?;
                Some(Pack {
                    data,
                    index: Some(index),
                })
            })
            .collect();
        Store {
            dir: dir.to_path_buf(),
            midx: None,
            packs,
            midx_packs: 0,
            covered: OnceLock::new(),
        }
    }

    /// Where `oid` lies in this store's packs. An entry the
    /// multi-pack-index refuses, or one naming a pack past those it covers,
    /// can't say where the object is, so its packs' own indexes are read
    /// instead. What any index refuses is kept in `refused` rather than
    /// returned, the first kept standing, for the caller to let stand only
    /// once no other store, nor the loose objects, holds the object: a
    /// broken entry neither hides an object nor passes for a missing one.
    /// The packs in `bad` held a copy that wouldn't unpack, and are passed
    /// over as git passes over a copy it has marked bad.
    fn locate(
        &self,
        oid: &Oid,
        bad: &[usize],
        refused: &mut Option<io::Error>,
    ) -> Option<(usize, u64)> {
        if let Some(midx) = &self.midx {
            let found = match midx.locate(oid) {
                // The multi-pack-index names one copy of each object, so
                // past a bad one only the packs it doesn't cover are read.
                Ok(Some((pack, offset))) if pack < self.midx_packs => {
                    (!bad.contains(&pack)).then_some((pack, offset))
                }
                Ok(None) => None,
                // The entry names a pack it doesn't cover, so the packs it
                // does are read; should none hold the object, that stands.
                Ok(Some(_)) => {
                    refused.get_or_insert_with(|| {
                        invalid("multi-pack-index names a pack past those it covers")
                    });
                    self.in_covered(oid, bad)
                }
                Err(error) => {
                    refused.get_or_insert(error);
                    self.in_covered(oid, bad)
                }
            };
            if found.is_some() {
                return found;
            }
        }
        for (at, pack) in self.packs.iter().enumerate().skip(self.midx_packs) {
            if bad.contains(&at) {
                continue;
            }
            match pack.index.as_ref().map(|index| index.offset(oid)) {
                Some(Ok(Some(offset))) => return Some((at, offset)),
                Some(Err(error)) => {
                    refused.get_or_insert(error);
                }
                Some(Ok(None)) | None => {}
            }
        }
        None
    }

    /// Where `oid` lies by the own indexes of the packs the
    /// multi-pack-index covers, read only once it can't answer and has
    /// left its own refusal standing. That refusal is what's said should
    /// none hold the object, so one that won't open or read is passed over
    /// rather than hiding the packs after it. A pack keeps its place
    /// whether or not its index opens, since the multi-pack-index numbers
    /// them. The packs in `bad` are passed over, as `locate` says.
    fn in_covered(&self, oid: &Oid, bad: &[usize]) -> Option<(usize, u64)> {
        let indexes = self.covered.get_or_init(|| {
            let pack_dir = self.dir.join("pack");
            self.midx
                .iter()
                .flat_map(|midx| &midx.packs)
                .map(|name| Index::open(&pack_dir.join(name)).ok())
                .collect()
        });
        indexes.iter().enumerate().find_map(|(at, index)| {
            if bad.contains(&at) {
                return None;
            }
            let offset = index.as_ref()?.offset(oid).ok()??;
            Some((at, offset))
        })
    }

    fn loose(&self, oid: &Oid) -> io::Result<Option<Object>> {
        let hex = oid.hex();
        let path = self.dir.join(&hex[..2]).join(&hex[2..]);
        let raw = match std::fs::read(path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let data = miniz_oxide::inflate::decompress_to_vec_zlib(&raw)
            .map_err(|_| invalid("loose object"))?;
        let nul = data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| invalid("loose header"))?;
        let space = data[..nul]
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| invalid("loose header"))?;
        let kind = Kind::from_name(&data[..space]).ok_or_else(|| invalid("loose kind"))?;
        Ok(Some(Object {
            kind,
            data: data[nul + 1..].to_vec(),
        }))
    }
}

/// Inflates the zlib stream at `offset`, which inflates to `size` bytes.
fn inflate_at(file: &File, mut offset: u64, size: usize) -> io::Result<Vec<u8>> {
    let mut state = InflateState::new_boxed(DataFormat::Zlib);
    let mut out = vec![0; size];
    let mut written = 0;
    let mut chunk = vec![0; (size + 64).clamp(256, 1 << 16)];
    loop {
        let got = file.read_at(&mut chunk, offset)?;
        if got == 0 {
            return Err(invalid("pack ends inside an object"));
        }
        let mut input = &chunk[..got];
        while !input.is_empty() {
            let result = inflate(&mut state, input, &mut out[written..], MZFlush::None);
            written += result.bytes_written;
            input = &input[result.bytes_consumed..];
            offset += result.bytes_consumed as u64;
            match result.status {
                Ok(MZStatus::StreamEnd) if written == size => return Ok(out),
                Ok(MZStatus::StreamEnd) => return Err(invalid("pack object's size")),
                _ if written == size => return Ok(out),
                Ok(_) | Err(MZError::Buf) => {}
                Err(_) => return Err(invalid("pack object's stream")),
            }
            if result.bytes_consumed == 0 && result.bytes_written == 0 {
                break;
            }
        }
    }
}

/// Applies a delta to its base.
pub fn apply_delta(base: &[u8], delta: &[u8]) -> io::Result<Vec<u8>> {
    let mut at = 0;
    let mut size = || -> io::Result<usize> {
        let mut value = 0usize;
        let mut shift = 0;
        loop {
            let byte = *delta.get(at).ok_or_else(|| invalid("delta size"))?;
            at += 1;
            value |= usize::from(byte & 0x7f) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
    };
    let base_size = size()?;
    let out_size = size()?;
    if base_size != base.len() {
        return Err(invalid("delta's base size"));
    }
    let mut out = Vec::with_capacity(out_size);
    while at < delta.len() {
        let cmd = delta[at];
        at += 1;
        if cmd & 0x80 != 0 {
            let mut offset = 0usize;
            let mut len = 0usize;
            for bit in 0..4 {
                if cmd & (1 << bit) != 0 {
                    offset |= usize::from(*delta.get(at).ok_or_else(|| invalid("delta copy"))?)
                        << (8 * bit);
                    at += 1;
                }
            }
            for bit in 0..3 {
                if cmd & (0x10 << bit) != 0 {
                    len |= usize::from(*delta.get(at).ok_or_else(|| invalid("delta copy"))?)
                        << (8 * bit);
                    at += 1;
                }
            }
            if len == 0 {
                len = 0x10000;
            }
            let piece = base
                .get(offset..offset + len)
                .ok_or_else(|| invalid("delta copy range"))?;
            out.extend_from_slice(piece);
        } else if cmd != 0 {
            let len = usize::from(cmd);
            let piece = delta
                .get(at..at + len)
                .ok_or_else(|| invalid("delta insert"))?;
            out.extend_from_slice(piece);
            at += len;
        } else {
            return Err(invalid("delta opcode"));
        }
    }
    if out.len() != out_size {
        return Err(invalid("delta's result size"));
    }
    Ok(out)
}

/// Every store a repository reads, its own first.
pub struct Odb {
    stores: Vec<Store>,
    pub stamps: Stamps,
    /// Bases read for the deltas over them, by where each lies, and how
    /// many bytes they hold.
    bases: HashMap<Place, Arc<(Kind, Vec<u8>)>>,
    base_bytes: usize,
    /// Trees already read, which the staged comparison asks for again on
    /// every question while a folder differs from HEAD, and how many bytes
    /// they hold.
    trees: HashMap<Oid, Vec<u8>>,
    tree_bytes: usize,
    /// The copies that wouldn't unpack, and what each said, passed over
    /// from then on as git passes over a copy it has marked bad for as
    /// long as it runs. Each copy is unpacked to its failure once, so a
    /// chain of failing deltas, each with several copies, costs one
    /// unpacking a copy rather than one for every path down to it.
    bad: HashMap<Place, (io::ErrorKind, String)>,
}

/// The most deltas a chain may run through to its base, its bases named
/// and placed alike, before it's refused rather than read: more than the
/// 4095 git's pack-objects writes at most, so a chain an older git wrote
/// deeper reads too.
const CHAIN_LIMIT: usize = 10_000;

/// How many bytes of trees are kept before starting again.
const TREE_BUDGET: usize = 32 << 20;

/// How many bytes of bases are kept before starting again: git's own
/// `core.deltaBaseCacheLimit` by default.
const BASE_BUDGET: usize = 96 << 20;

/// Where an object sits in a pack: the store, the pack and its offset.
type Place = (usize, usize, u64);

impl Odb {
    pub fn open(objects: &Path) -> Odb {
        let mut dirs = vec![objects.to_path_buf()];
        let mut at = 0;
        while at < dirs.len() && dirs.len() < 16 {
            let alternates = dirs[at].join("info/alternates");
            if let Ok(text) = std::fs::read_to_string(&alternates) {
                for line in text
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                {
                    let dir = dirs[at].join(line);
                    if !dirs.contains(&dir) {
                        dirs.push(dir);
                    }
                }
            }
            at += 1;
        }
        let mut stamps = Vec::new();
        for dir in &dirs {
            for watched in [
                dir.join("pack"),
                dir.join("pack/multi-pack-index"),
                dir.join("info/alternates"),
            ] {
                stamps.push((watched.clone(), Stamp::of(&watched)));
            }
        }
        Odb {
            stores: dirs.iter().map(|dir| Store::open(dir)).collect(),
            stamps,
            bases: HashMap::new(),
            base_bytes: 0,
            trees: HashMap::new(),
            tree_bytes: 0,
            bad: HashMap::new(),
        }
    }

    /// The object, or None when no store here has it (a partial clone's
    /// promised object, say).
    pub fn read(&mut self, oid: &Oid) -> io::Result<Option<Object>> {
        match self.tree(oid) {
            Some(tree) => Ok(Some(tree)),
            None => self.find(oid),
        }
    }

    /// The tree `oid`, if it's one already read.
    fn tree(&self, oid: &Oid) -> Option<Object> {
        self.trees.get(oid).map(|data| Object {
            kind: Kind::Tree,
            data: data.clone(),
        })
    }

    /// Keeps `object`, read as `oid`, should it be a tree.
    fn keep_tree(&mut self, oid: &Oid, object: &Object) {
        if object.kind != Kind::Tree {
            return;
        }
        if self.tree_bytes + object.data.len() > TREE_BUDGET {
            self.trees.clear();
            self.tree_bytes = 0;
        }
        self.tree_bytes += object.data.len();
        self.trees.insert(*oid, object.data.clone());
    }

    /// Keeps `object`, read at `place` as a delta's base, should it be one
    /// deltas build on most: trees and commits.
    fn keep_base(&mut self, place: Place, object: &Object) {
        if object.kind == Kind::Blob {
            return;
        }
        if object.data.len() > BASE_BUDGET {
            return;
        }
        if self.base_bytes + object.data.len() > BASE_BUDGET {
            self.bases.clear();
            self.base_bytes = 0;
        }
        self.base_bytes += object.data.len();
        if let Some(was) = self
            .bases
            .insert(place, Arc::new((object.kind, object.data.clone())))
        {
            self.base_bytes -= was.1.len();
        }
    }
}

/// A tree's entries, in its own order.
pub struct TreeEntry<'a> {
    pub mode: u32,
    pub name: &'a [u8],
    pub oid: Oid,
}

pub fn tree_entries(data: &[u8]) -> io::Result<Vec<TreeEntry<'_>>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let space = data[at..]
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| invalid("tree entry"))?;
        let mode = std::str::from_utf8(&data[at..at + space])
            .ok()
            .and_then(|m| u32::from_str_radix(m, 8).ok())
            .ok_or_else(|| invalid("tree mode"))?;
        at += space + 1;
        let nul = data[at..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| invalid("tree name"))?;
        let name = &data[at..at + nul];
        at += nul + 1;
        let oid = Oid::from_slice(data.get(at..at + 20).ok_or_else(|| invalid("tree oid"))?)
            .ok_or_else(|| invalid("tree oid"))?;
        at += 20;
        out.push(TreeEntry { mode, name, oid });
    }
    Ok(out)
}

/// What the engine reads from a commit.
pub struct Commit {
    pub tree: Oid,
    pub parents: Vec<Oid>,
    pub time: i64,
    pub encoding: String,
    pub summary: String,
}

pub fn parse_commit(data: &[u8]) -> io::Result<Commit> {
    let mut tree = None;
    let mut parents = Vec::new();
    let mut time = 0;
    let mut encoding = String::new();
    let mut lines = data.split(|&b| b == b'\n');
    let mut at = 0;
    for line in lines.by_ref() {
        at += line.len() + 1;
        if line.is_empty() {
            break;
        }
        if let Some(hex) = line.strip_prefix(b"tree ") {
            tree = Oid::from_hex(hex);
        } else if let Some(hex) = line.strip_prefix(b"parent ") {
            parents.push(Oid::from_hex(hex).ok_or_else(|| invalid("commit parent"))?);
        } else if let Some(who) = line.strip_prefix(b"committer ") {
            // "Name <mail> 1700000000 +0000"
            let close = who.iter().rposition(|&b| b == b'>').unwrap_or(0);
            time = std::str::from_utf8(&who[close + 1..])
                .ok()
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|t| t.parse().ok())
                .unwrap_or(0);
        } else if let Some(name) = line.strip_prefix(b"encoding ") {
            encoding = String::from_utf8_lossy(name).into_owned();
        }
    }
    let message = data.get(at.min(data.len())..).unwrap_or_default();
    // libgit2's summary: the first paragraph, its lines joined by spaces.
    let text = String::from_utf8_lossy(message);
    let summary = text
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Commit {
        tree: tree.ok_or_else(|| invalid("commit without a tree"))?,
        parents,
        time,
        encoding,
        summary,
    })
}

mod chain;

#[cfg(test)]
#[path = "odb_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "odb_delta_tests.rs"]
mod delta_tests;

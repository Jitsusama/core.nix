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

impl Pack {
    /// The pack beside the index `name` in `pack_dir`, or None where either
    /// won't open, the pack opened only once its index has and only where
    /// it's a file, so a FIFO in its place holds nothing up.
    fn indexed(pack_dir: &Path, name: &str) -> Option<Pack> {
        let index = Index::open(&pack_dir.join(name)).ok()?;
        let data = crate::regular_file::present(&pack_dir.join(name.replace(".idx", ".pack")));
        Some(Pack {
            data: data.ok()?,
            index: Some(index),
        })
    }
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
                // A pack the index names but that's gone, or isn't a file,
                // ends the index's use.
                match crate::regular_file::present(&data) {
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
            // The pack is opened only once its index has, and only as a file.
            if let Some(pack) = Pack::indexed(&pack_dir, &name) {
                packs.push(pack);
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
            .filter_map(|name| Pack::indexed(&pack_dir, &name))
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
}

#[cfg(test)]
thread_local! {
    /// How many bytes this thread's objects and deltas may hold at once,
    /// what a growth held and what it grows to both, as an allocator moving
    /// it would hold them, before more is refused, as an allocator out of
    /// memory would refuse it, which the tests set since nothing here asks
    /// for more than they can give.
    static STARVED: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Holds this thread's objects and deltas to `cap` bytes, or frees them,
/// for tests beyond the store's own.
#[cfg(test)]
pub(crate) fn starve(cap: Option<usize>) {
    STARVED.with(|starved| starved.set(cap));
}

/// The memory an object or delta was refused: what was growing, the bytes
/// it held and those it would have held, which a mark it leaves waits for.
#[derive(Debug)]
struct Starved {
    what: &'static str,
    held: usize,
    total: usize,
}

impl std::fmt::Display for Starved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no memory to grow {} to {} bytes", self.what, self.total)
    }
}

impl std::error::Error for Starved {}

/// Makes room for `more` bytes past what `out` holds, refused as `what`
/// and the bytes it would then hold should the memory not be there.
fn reserve(out: &mut Vec<u8>, more: usize, what: &'static str) -> io::Result<()> {
    let total = out.len().saturating_add(more);
    let held = out.capacity();
    let refused = || io::Error::new(io::ErrorKind::OutOfMemory, Starved { what, held, total });
    #[cfg(test)]
    if total > held
        && STARVED
            .with(std::cell::Cell::get)
            .is_some_and(|cap| held.saturating_add(total) > cap)
    {
        return Err(refused());
    }
    out.try_reserve_exact(more).map_err(|_| refused())
}

/// Whether what holds `held` bytes could grow to `total` now: the climb a
/// copy was refused, asked again as it was refused, since growing what's
/// held can be refused where as much from nothing is given. Nothing is
/// written, so nothing is touched but what the allocator lends.
fn room_for(held: usize, total: usize) -> bool {
    let mut probe = Vec::new();
    reserve(&mut probe, held, "room").is_ok() && reserve(&mut probe, total, "room").is_ok()
}

/// Where an object inflated is stored, which its refusals name.
#[derive(Clone, Copy)]
enum Stored {
    Packed,
    Loose,
}

impl Stored {
    fn name(self) -> &'static str {
        match self {
            Stored::Packed => "pack object",
            Stored::Loose => "loose object",
        }
    }

    /// What its memory refused is said to have been growing.
    fn growing(self) -> &'static str {
        match self {
            Stored::Packed => "a pack object",
            Stored::Loose => "a loose object",
        }
    }
}

/// Inflates the zlib stream at `offset`, which inflates to `size` bytes,
/// its refusals naming where the object is `stored`.
///
/// What it inflates to is grown as the stream fills it rather than made
/// at the size its header says, so a size no stream fills, however large,
/// takes at most twice what the stream gives, or 64 KiB, before it ends.
/// Memory the allocator won't give for it fails the read rather than
/// ending the program, and the copy is marked until it can be had. As git's
/// `unpack_compressed_entry` does, it makes room for one byte past the
/// size, and takes the object only where its stream ends there, so a
/// stream giving more, or ending wrongly, is refused.
fn inflate_at(file: &File, offset: u64, size: usize, stored: Stored) -> io::Result<Vec<u8>> {
    inflate_ending(file, offset, size, stored).map(|(data, _)| data)
}

/// The same, and the offset just past the stream's end.
fn inflate_ending(
    file: &File,
    mut offset: u64,
    size: usize,
    stored: Stored,
) -> io::Result<(Vec<u8>, u64)> {
    let wrong = |part: &str| invalid(&format!("{}'s {part}", stored.name()));
    let mut state = InflateState::new_boxed(DataFormat::Zlib);
    let room = size.checked_add(1).ok_or_else(|| wrong("size"))?;
    let mut out = Vec::new();
    let mut filled = 0;
    let mut chunk = vec![0; size.saturating_add(64).clamp(256, 1 << 16)];
    loop {
        let got = file.read_at(&mut chunk, offset)?;
        if got == 0 {
            return Err(wrong("stream ends early"));
        }
        let mut input = &chunk[..got];
        // Read on past the chunk's end too, while the stream still holds
        // what it inflated past the room made for it.
        loop {
            if filled == out.len() {
                grow_toward(&mut out, room, stored.growing())?;
            }
            let result = inflate(&mut state, input, &mut out[filled..], MZFlush::None);
            filled += result.bytes_written;
            input = &input[result.bytes_consumed..];
            offset += result.bytes_consumed as u64;
            match result.status {
                Ok(MZStatus::StreamEnd) if filled == size => {
                    out.truncate(size);
                    return Ok((out, offset));
                }
                Ok(MZStatus::StreamEnd) => return Err(wrong("size")),
                _ if filled > size => return Err(wrong("size")),
                Ok(_) | Err(MZError::Buf) => {}
                Err(_) => return Err(wrong("stream")),
            }
            if result.bytes_consumed == 0 && result.bytes_written == 0 {
                break;
            }
        }
    }
}

/// Makes more room at the end of `out`, which is to hold `size` bytes:
/// twice what it holds, or 64 KiB more while it holds less, never past
/// `size`, and refused as `what` should the memory not be there.
fn grow_toward(out: &mut Vec<u8>, size: usize, what: &'static str) -> io::Result<()> {
    let len = out.len();
    let to = size.min(len.saturating_mul(2).max(len.saturating_add(1 << 16)));
    reserve(out, to - len, what)?;
    out.resize(to, 0);
    Ok(())
}

/// The size a delta says its base holds, or None where it can't be read,
/// which is the delta's own fault.
pub fn delta_base_size(delta: &[u8]) -> Option<usize> {
    delta_size(delta, &mut 0).ok()
}

/// The size in `delta` at `at`, read past, refused once it would run past
/// a word.
fn delta_size(delta: &[u8], at: &mut usize) -> io::Result<usize> {
    let mut value = 0usize;
    let mut shift = 0;
    loop {
        let byte = *delta.get(*at).ok_or_else(|| invalid("delta size"))?;
        if shift > usize::BITS - 7 {
            return Err(invalid("delta size"));
        }
        *at += 1;
        value |= usize::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
}

/// Applies a delta to its base.
///
/// Its sizes are refused once they'd run past a word, and a command
/// writing past the size it says it makes is refused before it's written,
/// as git's `patch_delta` refuses one, so a delta takes no more memory
/// than the size it says, and the bytes its commands give.
pub fn apply_delta(base: &[u8], delta: &[u8]) -> io::Result<Vec<u8>> {
    let mut at = 0;
    let base_size = delta_size(delta, &mut at)?;
    let out_size = delta_size(delta, &mut at)?;
    if base_size != base.len() {
        return Err(invalid("delta's base size"));
    }
    let mut out = Vec::new();
    reserve(&mut out, out_size.min(TRUSTED_SIZE), "a delta")?;
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
            write_within(&mut out, piece, out_size)?;
        } else if cmd != 0 {
            let len = usize::from(cmd);
            let piece = delta
                .get(at..at + len)
                .ok_or_else(|| invalid("delta insert"))?;
            write_within(&mut out, piece, out_size)?;
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

/// Writes a delta's `piece` to `out`, refused should it run `out` past the
/// size the delta says it makes, or need memory that isn't there: copies
/// from a small base can still claim far more than a pack holds.
fn write_within(out: &mut Vec<u8>, piece: &[u8], size: usize) -> io::Result<()> {
    if piece.len() > size - out.len() {
        return Err(invalid("delta's result size"));
    }
    if out.capacity() - out.len() < piece.len() {
        // Twice what it holds, never past `size`, as `grow_toward` grows,
        // rather than the doubling of what it was made with.
        let more = piece.len().max(out.len()).min(size - out.len());
        reserve(out, more, "a delta")?;
    }
    out.extend_from_slice(piece);
    Ok(())
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
    /// The copies that wouldn't unpack, and why, passed over from then on
    /// as git passes over a copy it has marked bad for as long as it runs.
    /// Each copy is unpacked to its failure once an answer, so a chain of
    /// failing deltas, each with several copies, costs one unpacking a
    /// copy rather than one for every path down to it.
    bad: HashMap<Place, Mark>,
}

/// Why a copy is marked bad, and how long the mark lasts.
struct Mark {
    kind: io::ErrorKind,
    said: String,
    lasts: Lasts,
}

/// How long a copy's mark lasts, each answer asking again what it waits on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lasts {
    /// While the packs hold: the copy's own bytes won't read, its data
    /// corrupt or its deltas running past their sizes, through a
    /// repository started afresh after an error too, which keeps its store.
    WhileThePacksHold,
    /// Until what held `held` bytes can grow to `total`, the memory the copy
    /// was refused. Climbing back to a refusal costs what the climb cost
    /// the first time, as much as a stream will inflate, so a copy that
    /// inflates toward all the memory there is costs that once, not every
    /// answer.
    UntilThereIsRoom { held: usize, total: usize },
    /// That answer alone: an I/O error, or a fault that may be a base's
    /// found by name, which the next answer may find written or rewritten.
    ThisAnswer,
}

impl Mark {
    /// The mark a copy leaves that won't read for `error`, which is its
    /// `own` bytes' fault, rather than a base's found elsewhere, or not.
    fn of(error: &io::Error, own: bool) -> Mark {
        let starved = error
            .get_ref()
            .and_then(|cause| cause.downcast_ref::<Starved>());
        let lasts = if let Some(starved) = starved {
            Lasts::UntilThereIsRoom {
                held: starved.held,
                total: starved.total,
            }
        } else if own && error.kind() == io::ErrorKind::InvalidData {
            Lasts::WhileThePacksHold
        } else {
            Lasts::ThisAnswer
        };
        Mark {
            kind: error.kind(),
            said: error.to_string(),
            lasts,
        }
    }
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

/// How much of the size a delta says it makes is made room for before
/// anything is written: past it, room grows only as bytes come.
const TRUSTED_SIZE: usize = 1 << 20;

/// Where an object sits in a pack: the store, the pack and its offset.
type Place = (usize, usize, u64);

impl Odb {
    pub fn open(objects: &Path) -> Odb {
        let mut dirs = vec![objects.to_path_buf()];
        let mut at = 0;
        while at < dirs.len() && dirs.len() < 16 {
            let alternates = dirs[at].join("info/alternates");
            // Read only as a file, as every other file of git's is.
            if let Ok(Some(bytes)) = crate::regular_file::read(&alternates)
                && let Ok(text) = String::from_utf8(bytes)
            {
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

    /// The store `kept` from a repository started afresh, while it reads
    /// `objects` and its packs are as it read them, so the copies whose
    /// marks last stay marked and its caches stay warm; else `objects`
    /// opened again.
    pub fn reopen(kept: Option<Odb>, objects: &Path) -> Odb {
        match kept {
            Some(odb)
                if odb.stores.first().is_some_and(|store| store.dir == objects)
                    && crate::stamp::still(&odb.stamps) =>
            {
                odb
            }
            _ => Odb::open(objects),
        }
    }

    /// Forgets the marks that last one answer, and those refused memory
    /// that can now be had, as the next answer begins, so the copies they
    /// marked are read again.
    pub fn forget_passing_marks(&mut self) {
        self.bad.retain(|_, mark| match mark.lasts {
            Lasts::WhileThePacksHold => true,
            Lasts::UntilThereIsRoom { held, total } => !room_for(held, total),
            Lasts::ThisAnswer => false,
        });
    }

    /// The stamps of the packs every store watches, which move once one is
    /// added or taken away.
    pub fn pack_stamps(&self) -> Stamps {
        self.stamps.clone()
    }

    /// The stamps that move once `oid` may be written loose: the folder each
    /// store's loose copy of it would lie in, which git makes if it must
    /// and renames a new object into. With `pack_stamps`, they move once it
    /// may be found.
    pub fn folders_of(&self, oid: &Oid) -> Stamps {
        let hex = oid.hex();
        let mut stamps = Stamps::new();
        for store in &self.stores {
            let folder = store.dir.join(&hex[..2]);
            let stamp = Stamp::of(&folder);
            stamps.push((folder, stamp));
        }
        stamps
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
        self.keep_tree_within(oid, object, TREE_BUDGET);
    }

    /// The same, keeping no more than `budget` bytes of trees, and no tree
    /// larger.
    fn keep_tree_within(&mut self, oid: &Oid, object: &Object, budget: usize) {
        if object.kind != Kind::Tree || object.data.len() > budget {
            return;
        }
        if self.tree_bytes + object.data.len() > budget {
            self.trees.clear();
            self.tree_bytes = 0;
        }
        self.tree_bytes += object.data.len();
        self.trees.insert(*oid, object.data.clone());
    }

    /// Keeps `object`, read at `place` as a delta's base, should it be one
    /// deltas build on most: trees and commits.
    fn keep_base(&mut self, place: Place, object: &Object) {
        self.keep_base_within(place, object, BASE_BUDGET);
    }

    /// The same, keeping no more than `budget` bytes of bases, and no base
    /// larger.
    fn keep_base_within(&mut self, place: Place, object: &Object, budget: usize) {
        if object.kind == Kind::Blob || object.data.len() > budget {
            return;
        }
        if self.base_bytes + object.data.len() > budget {
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

mod chain;
mod kind;
mod loose;
mod parse;

pub use parse::{TreeEntry, parse_commit, tree_entries};

#[cfg(test)]
#[path = "odb_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "odb_delta_tests.rs"]
mod delta_tests;

#[cfg(test)]
#[path = "odb_bound_tests.rs"]
mod bound_tests;

//! What `git status` would say, counted: changes staged against HEAD's
//! tree, changes in the worktree against the index, conflicts, and files
//! git doesn't track or ignore. Each count stops at the limit gitstatusd
//! was started with, since the prompt asks only whether there are any.
//! Telling whether a file's contents changed, by turning it into a blob
//! as `git add` would, is `convert`'s; the walk for untracked files is
//! `untracked`'s.

use crate::attrs;
use crate::fsmonitor::Changed;
use crate::hash::Oid;
use crate::index::{CacheTree, Entry, Index};
use crate::odb::{Kind, Odb, TreeEntry, tree_entries};
use crate::pattern::{Pattern, parse_file};
use crate::stamp::Stamp;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod convert;
mod untracked;

use convert::{Conversion, conversion_of, same_contents};
pub use convert::{Crlf, Driver, autocrlf, filter_drivers};
pub use untracked::untracked;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Counts {
    pub staged: usize,
    pub staged_new: usize,
    pub staged_deleted: usize,
    pub unstaged: usize,
    pub unstaged_deleted: usize,
    pub conflicted: usize,
    pub untracked: usize,
    pub skip_worktree: usize,
    pub assume_unchanged: usize,
}

/// How many of each to count before stopping; None counts them all.
#[derive(Clone, Copy)]
pub struct Limits {
    pub staged: Option<usize>,
    pub unstaged: Option<usize>,
    pub conflicted: Option<usize>,
    pub untracked: Option<usize>,
}

fn full(count: usize, limit: Option<usize>) -> bool {
    limit.is_some_and(|limit| count >= limit)
}

/// What a scan may stop at, as gitstatusd has it: one kind of change is
/// counted on while the kind it's paired with, staged with conflicted and
/// unstaged with untracked, isn't full yet.
pub fn stop_at(
    limit: Option<usize>,
    partner: usize,
    partner_limit: Option<usize>,
) -> Option<usize> {
    if full(partner, partner_limit) {
        limit
    } else {
        None
    }
}

/// The counts as they're answered: each at most its limit, and what was
/// new or deleted at most the count it's part of.
pub fn settle(counts: &mut Counts, limits: Limits) {
    let cap = |count: usize, limit: Option<usize>| limit.map_or(count, |limit| count.min(limit));
    counts.staged = cap(counts.staged, limits.staged);
    counts.unstaged = cap(counts.unstaged, limits.unstaged);
    counts.conflicted = cap(counts.conflicted, limits.conflicted);
    counts.untracked = cap(counts.untracked, limits.untracked);
    counts.staged_new = counts.staged_new.min(counts.staged);
    counts.staged_deleted = counts.staged_deleted.min(counts.staged);
    counts.unstaged_deleted = counts.unstaged_deleted.min(counts.unstaged);
}

/// How many paths are in conflict.
pub fn conflicts(index: &Index) -> usize {
    let mut last: Option<&[u8]> = None;
    let mut count = 0;
    for entry in index.entries.iter().filter(|e| e.stage != 0) {
        if last != Some(entry.path.as_slice()) {
            count += 1;
            last = Some(&entry.path);
        }
    }
    count
}

fn bump(count: &mut usize, limit: Option<usize>) {
    if !full(*count, limit) {
        *count += 1;
    }
}

/// The settings that change how a file in the worktree becomes a blob.
pub struct Settings {
    /// What core.autocrlf makes of the line endings of a file no attribute
    /// speaks for: Auto where it is true or input, else Binary.
    pub autocrlf: Crlf,
    pub filemode: bool,
    pub trust_ctime: bool,
    pub show_untracked: bool,
    /// The filter drivers that do anything on the way in, by name.
    pub filters: BTreeMap<String, Driver>,
}

/// Files read once and kept until their stamp moves: ignore and attributes
/// files, by path.
#[derive(Default)]
pub struct Cache {
    ignores: HashMap<PathBuf, (Option<Stamp>, Arc<Vec<Pattern>>)>,
    attributes: HashMap<PathBuf, (Option<Stamp>, Arc<attrs::File>)>,
    /// Content checks already made, by path, the stat they were made at and
    /// how the file was turned into a blob.
    contents: HashMap<Vec<u8>, (StatKey, Oid, Conversion, bool)>,
    /// Folders as last listed, by path in the worktree. A folder's stamp
    /// moves whenever an entry is added, removed or renamed in it, so its
    /// listing stands until then.
    listings: HashMap<Vec<u8>, (Stamp, Arc<Listing>)>,
    /// What each file came to, while a file system monitor says which
    /// files to look at again, with the index entry it was judged against.
    verdicts: HashMap<Vec<u8>, (Identity, Outcome)>,
    /// What those verdicts were reached under: what turns a file into a
    /// blob, and the attributes files outside the worktree.
    basis: Option<Basis>,
    /// Folders whose listing, and ignore files whose patterns, were checked
    /// against the disk since the monitor last lost track, by path in the
    /// worktree: only these can stand on its word that nothing changed.
    checked: HashSet<Vec<u8>>,
    /// How many untracked files each folder walked held, with the ignore
    /// patterns above it then, while the monitor names nothing in it.
    walked: HashMap<Vec<u8>, (Vec<Patterns>, usize)>,
    /// The index those counts were made against.
    walked_index: Option<Arc<Index>>,
}

/// One ignore file's patterns, as read.
type Patterns = Arc<Vec<Pattern>>;

/// What a file in the worktree came to, against its index entry.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Clean,
    Changed,
    Deleted,
}

/// An index entry as it was when its file was judged.
type Identity = (Oid, u32, u16, crate::index::StatData);

fn identity(entry: &Entry) -> Identity {
    (entry.oid, entry.mode, entry.extended, entry.stat.clone())
}

type Basis = (
    Crlf,
    bool,
    bool,
    BTreeMap<String, Driver>,
    [Option<Stamp>; 3],
);

/// What a folder holds: each entry by its path in the worktree, with whether
/// it's a folder, and whether there's an ignore file or a repository of its
/// own there.
#[derive(Default)]
struct Listing {
    entries: Vec<(Vec<u8>, bool)>,
    ignore_file: bool,
    repository: bool,
}

type StatKey = (i64, i64, i64, i64, u64, u64);

fn stat_key(meta: &std::fs::Metadata) -> StatKey {
    (
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
        meta.ino(),
        meta.size(),
    )
}

impl Cache {
    fn ignore_file(&mut self, path: &Path) -> Arc<Vec<Pattern>> {
        let stamp = Stamp::of(path);
        if let Some((seen, patterns)) = self.ignores.get(path)
            && *seen == stamp
            && !stamp.is_some_and(|s| s.is_recent())
        {
            return patterns.clone();
        }
        let read = std::fs::read(path)
            .map(|text| parse_file(&text, true))
            .unwrap_or_default();
        // The same patterns read again are the same file, so what was
        // worked out under them still stands.
        let patterns = match self.ignores.get(path) {
            Some((_, seen)) if **seen == read => seen.clone(),
            _ => Arc::new(read),
        };
        self.ignores
            .insert(path.to_path_buf(), (stamp, patterns.clone()));
        patterns
    }

    fn attributes_file(&mut self, path: &Path, macros: bool) -> Arc<attrs::File> {
        let stamp = Stamp::of(path);
        if let Some((seen, file)) = self.attributes.get(path)
            && *seen == stamp
            && !stamp.is_some_and(|s| s.is_recent())
        {
            return file.clone();
        }
        let file = Arc::new(
            std::fs::read(path)
                .map(|text| attrs::File::parse(&text, macros))
                .unwrap_or_default(),
        );
        self.attributes
            .insert(path.to_path_buf(), (stamp, file.clone()));
        file
    }

    /// Forgets what stood on a file system monitor's word, once it says it
    /// can't tell what changed: what it would have named since is lost.
    pub fn lost_track(&mut self) {
        self.checked.clear();
        self.walked.clear();
        self.verdicts.clear();
    }

    /// Lists a folder, `dir` empty or ending in a slash. While a monitor
    /// says nothing in it has changed, the listing stands unasked.
    fn listing(&mut self, workdir: &Path, dir: &[u8], changed: Option<&Changed>) -> Arc<Listing> {
        if let Some(changed) = changed
            && !changed.touches_dir(dir)
            && self.checked.contains(dir)
            && let Some((_, listing)) = self.listings.get(dir)
        {
            return listing.clone();
        }
        let path = workdir.join(os(dir));
        let Some(stamp) = Stamp::of(&path) else {
            return Arc::default();
        };
        self.checked.insert(dir.to_vec());
        if let Some((seen, listing)) = self.listings.get(dir)
            && *seen == stamp
            && !stamp.is_recent()
        {
            return listing.clone();
        }
        let mut listing = Listing::default();
        for entry in std::fs::read_dir(&path).into_iter().flatten().flatten() {
            let name = entry.file_name();
            let name = name.as_encoded_bytes();
            let kind = entry.file_type().ok();
            if name == b".git" {
                listing.repository = true;
                continue;
            }
            if name == b".gitignore" && kind.is_some_and(|t| !t.is_dir()) {
                listing.ignore_file = true;
            }
            let mut path = dir.to_vec();
            path.extend_from_slice(name);
            listing
                .entries
                .push((path, kind.is_some_and(|t| t.is_dir())));
        }
        if self.listings.len() > 200_000 {
            self.listings.clear();
            self.walked.clear();
            self.checked.clear();
            self.checked.insert(dir.to_vec());
        }
        let listing = Arc::new(listing);
        self.listings.insert(dir.to_vec(), (stamp, listing.clone()));
        listing
    }
}

pub struct Paths<'a> {
    pub workdir: &'a Path,
    /// Where `info/exclude` and `info/attributes` are: the shared git folder.
    pub common: &'a Path,
    pub excludes_file: Option<PathBuf>,
    /// git's system-wide attributes, read before every other file.
    pub system_attributes: Option<PathBuf>,
    pub attributes_file: Option<PathBuf>,
    /// Whether a file system monitor watches the worktree.
    pub monitored: bool,
    /// What it says changed since the last question; `None` while anything
    /// may have.
    pub changed: Option<&'a Changed>,
}

/// Staged changes: HEAD's tree against the index, skipping every folder
/// whose cached tree is HEAD's own.
pub fn staged(
    odb: &mut Odb,
    index: &Index,
    head: Option<Oid>,
    limits: Limits,
    counts: &mut Counts,
) -> io::Result<()> {
    let Some(head) = head else {
        for _ in index
            .entries
            .iter()
            .filter(|e| e.stage == 0 && !e.intent_to_add())
        {
            if full(counts.staged, limits.staged) {
                break;
            }
            counts.staged += 1;
            counts.staged_new += 1;
        }
        return Ok(());
    };
    compare_tree(odb, index, b"", head, index.tree.as_ref(), limits, counts)
}

fn compare_tree(
    odb: &mut Odb,
    index: &Index,
    prefix: &[u8],
    tree: Oid,
    cache: Option<&CacheTree>,
    limits: Limits,
    counts: &mut Counts,
) -> io::Result<()> {
    if full(counts.staged, limits.staged) {
        return Ok(());
    }
    if let Some(cache) = cache
        && cache.entries >= 0
        && cache.oid == Some(tree)
    {
        return Ok(());
    }
    let object = odb
        .read(&tree)?
        .ok_or_else(|| io::Error::other("HEAD's tree is missing"))?;
    if object.kind != Kind::Tree {
        return Err(io::Error::other("not a tree"));
    }
    let entries = tree_entries(&object.data)?;
    let range = index.under(prefix);
    let mut seen = vec![false; entries.len()];
    let by_name: HashMap<&[u8], usize> = entries
        .iter()
        .enumerate()
        .map(|(at, e)| (e.name, at))
        .collect();
    let mut at = range.start;
    while at < range.end {
        if full(counts.staged, limits.staged) {
            return Ok(());
        }
        let entry = &index.entries[at];
        let rest = &entry.path[prefix.len()..];
        let slash = rest.iter().position(|&c| c == b'/');
        let name = slash.map_or(rest, |s| &rest[..s]);
        let in_tree = by_name.get(name).copied();
        if let Some(at_tree) = in_tree {
            seen[at_tree] = true;
        }
        let tree_entry = in_tree.map(|at| &entries[at]);
        let is_folder = slash.is_some() && !entry.is_sparse_dir();
        if entry.is_sparse_dir() {
            // A whole folder outside the checkout, as one tree.
            match tree_entry {
                Some(t) if t.mode == 0o040_000 => {
                    if t.oid != entry.oid {
                        bump(&mut counts.staged, limits.staged);
                    }
                }
                _ => {
                    bump(&mut counts.staged, limits.staged);
                    counts.staged_new += 1;
                }
            }
            at += 1;
        } else if is_folder {
            let mut sub = prefix.to_vec();
            sub.extend_from_slice(name);
            sub.push(b'/');
            let sub_range = index.under(&sub);
            match tree_entry {
                Some(t) if t.mode == 0o040_000 => {
                    let child = cache.and_then(|c| c.child(name));
                    compare_tree(odb, index, &sub, t.oid, child, limits, counts)?;
                }
                other => {
                    if other.is_some() {
                        // A file in HEAD that's a folder in the index.
                        bump(&mut counts.staged, limits.staged);
                        counts.staged_deleted += 1;
                    }
                    for entry in &index.entries[sub_range.clone()] {
                        if entry.stage == 0 && !entry.intent_to_add() {
                            bump(&mut counts.staged, limits.staged);
                            counts.staged_new += 1;
                        }
                    }
                }
            }
            at = sub_range.end.max(at + 1);
        } else {
            compare_file(odb, entry, tree_entry, limits, counts)?;
            // Every stage of one path is one entry to the tree.
            let mut next = at + 1;
            while next < range.end && index.entries[next].path == entry.path {
                next += 1;
            }
            at = next;
        }
    }
    for (at_tree, t) in entries.iter().enumerate() {
        if seen[at_tree] {
            continue;
        }
        if t.mode == 0o040_000 {
            count_deleted(odb, t.oid, limits, counts)?;
        } else {
            bump(&mut counts.staged, limits.staged);
            counts.staged_deleted += 1;
        }
    }
    Ok(())
}

/// Every file of a tree HEAD has and the index doesn't.
/// Counts a file in the index as staged against what HEAD's tree has at
/// its path, if anything.
fn compare_file(
    odb: &mut Odb,
    entry: &Entry,
    tree_entry: Option<&TreeEntry>,
    limits: Limits,
    counts: &mut Counts,
) -> io::Result<()> {
    if entry.stage == 0 && !entry.intent_to_add() {
        match tree_entry {
            Some(t) if t.mode == 0o040_000 => {
                bump(&mut counts.staged, limits.staged);
                counts.staged_new += 1;
                count_deleted(odb, t.oid, limits, counts)?;
            }
            Some(t) => {
                if t.oid != entry.oid || t.mode != entry.mode {
                    bump(&mut counts.staged, limits.staged);
                }
            }
            None => {
                bump(&mut counts.staged, limits.staged);
                counts.staged_new += 1;
            }
        }
    }
    // Neither a conflict, whose tree side is still to be decided, nor an
    // intent to add, unchanged and staged, counts.
    Ok(())
}

fn count_deleted(odb: &mut Odb, tree: Oid, limits: Limits, counts: &mut Counts) -> io::Result<()> {
    let object = odb
        .read(&tree)?
        .ok_or_else(|| io::Error::other("tree missing"))?;
    let subtrees: Vec<Oid> = {
        let entries = tree_entries(&object.data)?;
        for _ in entries.iter().filter(|e| e.mode != 0o040_000) {
            bump(&mut counts.staged, limits.staged);
            counts.staged_deleted += 1;
        }
        entries
            .iter()
            .filter(|e| e.mode == 0o040_000)
            .map(|e| e.oid)
            .collect()
    };
    for sub in subtrees {
        if full(counts.staged, limits.staged) {
            break;
        }
        count_deleted(odb, sub, limits, counts)?;
    }
    Ok(())
}

enum Verdict {
    Clean,
    Changed,
    Deleted,
    /// The stat moved but the size didn't: only the contents can say.
    Look(std::fs::Metadata),
}

fn later_or_same(a: (u32, u32), b: (i64, i64)) -> bool {
    let a = (i64::from(a.0), i64::from(a.1));
    a >= b
}

/// A stat field's low 32 bits, all the index keeps of it, as git does.
fn low_32(field: u64) -> u32 {
    let [a, b, c, d, ..] = field.to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}

/// A time's seconds and nanoseconds as the index keeps them.
fn kept_time(seconds: i64, nanoseconds: i64) -> (u32, u32) {
    (
        low_32(seconds.cast_unsigned()),
        low_32(nanoseconds.cast_unsigned()),
    )
}

fn judge(workdir: &Path, entry: &Entry, written: (i64, i64), settings: &Settings) -> Verdict {
    let path = workdir.join(os(&entry.path));
    let Ok(meta) = std::fs::symlink_metadata(&path) else {
        return Verdict::Deleted;
    };
    let kind = entry.mode & 0o170_000;
    let file_type = meta.file_type();
    let type_matches = match kind {
        0o120_000 => file_type.is_symlink(),
        0o100_000 => file_type.is_file(),
        _ => true,
    };
    if !type_matches {
        // A file become a folder is a change of type, as gitstatusd has
        // it, rather than git's deletion beside a new untracked folder.
        return Verdict::Changed;
    }
    if entry.intent_to_add() {
        return Verdict::Changed;
    }
    if kind == 0o100_000 && settings.filemode {
        let executable = meta.permissions().mode() & 0o100 != 0;
        if executable != (entry.mode & 0o100 != 0) {
            return Verdict::Changed;
        }
    }
    let stat = &entry.stat;
    let moved = stat.mtime != kept_time(meta.mtime(), meta.mtime_nsec())
        || (settings.trust_ctime && stat.ctime != kept_time(meta.ctime(), meta.ctime_nsec()))
        || stat.ino != low_32(meta.ino())
        || stat.uid != meta.uid()
        || stat.gid != meta.gid()
        || stat.size != low_32(meta.size());
    if !moved && !later_or_same(stat.mtime, written) {
        return Verdict::Clean;
    }
    if moved && stat.size != 0 && stat.size != low_32(meta.size()) {
        return Verdict::Changed;
    }
    Verdict::Look(meta)
}

/// A path from its bytes, which on Unix any bytes are.
fn os(bytes: &[u8]) -> &std::ffi::OsStr {
    std::os::unix::ffi::OsStrExt::from_bytes(bytes)
}

/// Changes in the worktree against the index, and conflicts.
pub fn unstaged(
    odb: &mut Odb,
    index: &Index,
    paths: &Paths,
    settings: &Settings,
    cache: &mut Cache,
    limits: Limits,
    counts: &mut Counts,
) -> io::Result<()> {
    let mut checked = Vec::new();
    let mut last_conflict: Option<&[u8]> = None;
    for entry in &index.entries {
        if entry.stage != 0 {
            if last_conflict != Some(entry.path.as_slice()) {
                bump(&mut counts.conflicted, limits.conflicted);
                last_conflict = Some(&entry.path);
            }
            continue;
        }
        if entry.is_sparse_dir() {
            continue;
        }
        if entry.skip_worktree() {
            counts.skip_worktree += 1;
            continue;
        }
        if entry.assume_valid() {
            counts.assume_unchanged += 1;
            continue;
        }
        if entry.mode == 0o160_000 {
            // Submodules are left to their own repositories, as gitstatusd
            // leaves them.
            continue;
        }
        checked.push(entry);
    }
    if paths.monitored {
        // A change to how files become blobs changes every verdict without
        // touching a file the monitor would name.
        let basis = (
            settings.autocrlf,
            settings.filemode,
            settings.trust_ctime,
            settings.filters.clone(),
            [
                Stamp::of(&paths.common.join("info/attributes")),
                paths.attributes_file.as_deref().and_then(Stamp::of),
                paths.system_attributes.as_deref().and_then(Stamp::of),
            ],
        );
        if cache.basis.as_ref() != Some(&basis) || paths.changed.is_some_and(|c| c.attributes) {
            cache.verdicts.clear();
            cache.basis = Some(basis);
        }
    } else if !cache.verdicts.is_empty() {
        cache.verdicts = HashMap::new();
        cache.basis = None;
    }
    // What's known already: a file the monitor didn't name, judged against
    // the same index entry.
    let known: Vec<Option<Outcome>> = checked
        .iter()
        .map(|entry| {
            let changed = paths.changed?;
            if changed.touches(&entry.path) {
                return None;
            }
            cache
                .verdicts
                .get(&entry.path)
                .filter(|(seen, _)| *seen == identity(entry))
                .map(|(_, outcome)| *outcome)
        })
        .collect();
    let unknown: Vec<&Entry> = checked
        .iter()
        .zip(&known)
        .filter(|(_, known)| known.is_none())
        .map(|(entry, _)| *entry)
        .collect();
    let mut verdicts = judge_all(&unknown, paths, index, settings).into_iter();
    for (entry, known) in checked.iter().zip(known) {
        // Unwatched, a full count is the end; watched, every verdict is
        // worth keeping for next time.
        if !paths.monitored && full(counts.unstaged, limits.unstaged) {
            break;
        }
        let outcome = if let Some(outcome) = known {
            outcome
        } else {
            let verdict = verdicts.next().unwrap_or(Verdict::Changed);
            let outcome = settle_verdict(odb, entry, verdict, paths, settings, cache)?;
            if paths.monitored {
                cache
                    .verdicts
                    .insert(entry.path.clone(), (identity(entry), outcome));
            }
            outcome
        };
        match outcome {
            Outcome::Clean => {}
            Outcome::Changed => counts.unstaged += 1,
            Outcome::Deleted => {
                counts.unstaged += 1;
                counts.unstaged_deleted += 1;
            }
        }
    }
    Ok(())
}

/// Each file's stat against its index entry, many at once across threads.
fn judge_all(
    entries: &[&Entry],
    paths: &Paths,
    index: &Index,
    settings: &Settings,
) -> Vec<Verdict> {
    let checked = entries;
    if checked.len() > 2048 {
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
        let chunk = checked.len().div_ceil(threads);
        std::thread::scope(|scope| {
            let handles: Vec<_> = checked
                .chunks(chunk)
                .map(|part| {
                    scope.spawn(move || {
                        part.iter()
                            .map(|e| judge(paths.workdir, e, index.written, settings))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_default())
                .collect()
        })
    } else {
        checked
            .iter()
            .map(|e| judge(paths.workdir, e, index.written, settings))
            .collect()
    }
}

/// What a stat's verdict comes to, looking at the contents where it can't
/// say by itself.
fn settle_verdict(
    odb: &mut Odb,
    entry: &Entry,
    verdict: Verdict,
    paths: &Paths,
    settings: &Settings,
    cache: &mut Cache,
) -> io::Result<Outcome> {
    Ok(match verdict {
        Verdict::Clean => Outcome::Clean,
        Verdict::Changed => Outcome::Changed,
        Verdict::Deleted => Outcome::Deleted,
        Verdict::Look(meta) => {
            let key = stat_key(&meta);
            let conversion = conversion_of(&entry.path, paths, settings, cache);
            let same = match cache.contents.get(&entry.path) {
                Some((seen, oid, how, same))
                    if *seen == key && *oid == entry.oid && *how == conversion =>
                {
                    *same
                }
                _ => {
                    let same = same_contents(odb, entry, &meta, paths, &conversion)?;
                    // A file changed this second may change again unseen.
                    let recent = Stamp::from_meta(&meta).is_recent();
                    if !recent {
                        cache
                            .contents
                            .insert(entry.path.clone(), (key, entry.oid, conversion, same));
                    }
                    same
                }
            };
            if same {
                Outcome::Clean
            } else {
                Outcome::Changed
            }
        }
    })
}

//! How `git add` would turn a file in the worktree into a blob, as git's
//! convert.c does: through a filter, with its line endings and its ident
//! collapsed, as its attributes and the settings say. A file whose stat
//! moved is told from one whose contents did by comparing that blob with
//! the index's.

use super::{Cache, Paths, Settings, os};
use crate::attrs::{self, State};
use crate::hash::{Oid, Sha256, to_hex};
use crate::index::Entry;
use crate::odb::Odb;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;

/// Whether the file in the worktree is what the index holds, once turned
/// into a blob the way `git add` would turn it.
pub(super) fn same_contents(
    odb: &mut Odb,
    entry: &Entry,
    meta: &std::fs::Metadata,
    paths: &Paths,
    conversion: &Conversion,
) -> io::Result<bool> {
    let path = paths.workdir.join(os(&entry.path));
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(&path)?;
        return Ok(Oid::of("blob", target.as_os_str().as_encoded_bytes()) == entry.oid);
    }
    let Ok(raw) = std::fs::read(&path) else {
        return Ok(false);
    };
    match conversion.filter {
        Filter::None => {}
        Filter::Lfs => return matches_lfs_pointer(odb, entry, &raw),
        // Another filter's clean step, or a change of encoding, would have
        // to run; it can't, so the file counts as changed.
        Filter::Other => return Ok(false),
    }
    let mut data = raw;
    if conversion.crlf != Crlf::Binary
        && data.contains(&b'\r')
        && !(conversion.crlf == Crlf::Auto && (is_binary(&data) || index_has_cr(odb, entry)?))
    {
        data = crlf_to_lf(&data);
    }
    if conversion.ident {
        data = collapse_ident(&data);
    }
    Ok(Oid::of("blob", &data) == entry.oid)
}

/// How `git add` would turn a file into a blob, from its attributes and
/// the settings: what a content check's answer rests on besides the file.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Conversion {
    filter: Filter,
    crlf: Crlf,
    ident: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    None,
    Lfs,
    Other,
}

pub(super) fn conversion_of(
    path: &[u8],
    paths: &Paths,
    settings: &Settings,
    cache: &mut Cache,
) -> Conversion {
    let attrs = attributes_of(path, paths, cache);
    let filter = match attrs.get("filter") {
        Some(State::Value(filter)) if filter == "lfs" => Filter::Lfs,
        Some(State::Value(_)) => Filter::Other,
        _ if attrs.contains_key("working-tree-encoding") => Filter::Other,
        _ => Filter::None,
    };
    Conversion {
        filter,
        crlf: converts_crlf(&attrs, settings),
        ident: attrs.get("ident") == Some(&State::Set),
    }
}

fn attributes_of(path: &[u8], paths: &Paths, cache: &mut Cache) -> HashMap<String, State> {
    let mut files: Vec<(Vec<u8>, Arc<attrs::File>)> = Vec::new();
    if let Some(global) = &paths.attributes_file {
        files.push((Vec::new(), cache.attributes_file(global, true)));
    }
    let mut dir: Vec<u8> = Vec::new();
    files.push((
        Vec::new(),
        cache.attributes_file(&paths.workdir.join(".gitattributes"), true),
    ));
    let parts: Vec<&[u8]> = path.split(|&c| c == b'/').collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if !dir.is_empty() {
            dir.push(b'/');
        }
        dir.extend_from_slice(part);
        let file = paths.workdir.join(os(&dir)).join(".gitattributes");
        files.push((dir.clone(), cache.attributes_file(&file, false)));
    }
    files.push((
        Vec::new(),
        cache.attributes_file(&paths.common.join("info/attributes"), true),
    ));
    let borrowed: Vec<(&[u8], &attrs::File)> = files
        .iter()
        .map(|(base, file)| (base.as_slice(), &**file))
        .collect();
    attrs::resolve(&borrowed, path)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Crlf {
    Binary,
    Text,
    Auto,
}

/// convert.c's choice of what to do with line endings on the way in.
fn converts_crlf(attrs: &HashMap<String, State>, settings: &Settings) -> Crlf {
    let text = attrs.get("text").or_else(|| attrs.get("crlf"));
    match text {
        Some(State::Unset) => Crlf::Binary,
        Some(State::Set) => Crlf::Text,
        Some(State::Value(v)) if v == "auto" => Crlf::Auto,
        Some(State::Value(v)) if v == "input" => Crlf::Text,
        _ => {
            if attrs.contains_key("eol") {
                return Crlf::Text;
            }
            match settings
                .autocrlf
                .as_deref()
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("input" | "true" | "yes" | "on" | "1") => Crlf::Auto,
                _ => Crlf::Binary,
            }
        }
    }
}

/// convert.c's guess at whether contents are text: no NUL, no lone CR, and
/// few unprintable bytes.
fn is_binary(data: &[u8]) -> bool {
    let (mut printable, mut nonprintable, mut lone_cr, mut nul) = (0usize, 0usize, 0usize, 0usize);
    let mut at = 0;
    while at < data.len() {
        let c = data[at];
        if c == b'\r' {
            if data.get(at + 1) == Some(&b'\n') {
                at += 2;
            } else {
                lone_cr += 1;
                at += 1;
            }
            continue;
        }
        match c {
            b'\n' => {}
            127 => nonprintable += 1,
            8 | b'\t' | 0x1b | 0x0c => printable += 1,
            0 => {
                nul += 1;
                nonprintable += 1;
            }
            c if c < 32 => nonprintable += 1,
            _ => printable += 1,
        }
        at += 1;
    }
    if data.last() == Some(&0x1a) {
        nonprintable = nonprintable.saturating_sub(1);
    }
    lone_cr > 0 || nul > 0 || (printable >> 7) < nonprintable
}

fn index_has_cr(odb: &mut Odb, entry: &Entry) -> io::Result<bool> {
    Ok(odb
        .read(&entry.oid)?
        .is_some_and(|blob| blob.data.contains(&b'\r')))
}

fn crlf_to_lf(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for (at, &c) in data.iter().enumerate() {
        if c == b'\r' && data.get(at + 1) == Some(&b'\n') {
            continue;
        }
        out.push(c);
    }
    out
}

/// `$Id: anything $` back to `$Id$`, as the ident attribute asks.
fn collapse_ident(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut at = 0;
    while at < data.len() {
        if data[at..].starts_with(b"$Id:") {
            let rest = &data[at + 4..];
            let end = rest.iter().position(|&c| c == b'$' || c == b'\n');
            if let Some(end) = end
                && rest[end] == b'$'
            {
                out.extend_from_slice(b"$Id$");
                at += 4 + end + 1;
                continue;
            }
        }
        out.push(data[at]);
        at += 1;
    }
    out
}

/// Whether the index's blob is the LFS pointer git-lfs would write for
/// these contents.
fn matches_lfs_pointer(odb: &mut Odb, entry: &Entry, raw: &[u8]) -> io::Result<bool> {
    let mut sha = Sha256::new();
    sha.update(raw);
    let pointer = format!(
        "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize {}\n",
        to_hex(&sha.finish()),
        raw.len()
    );
    if Oid::of("blob", pointer.as_bytes()) == entry.oid {
        return Ok(true);
    }
    // A pointer with extensions, or one written differently: compare what
    // it says rather than its bytes.
    let Some(blob) = odb.read(&entry.oid)? else {
        return Ok(false);
    };
    let text = String::from_utf8_lossy(&blob.data);
    let wanted_oid = pointer.lines().nth(1).unwrap_or_default();
    let wanted_size = pointer.lines().nth(2).unwrap_or_default();
    Ok(
        text.starts_with("version https://git-lfs.github.com/spec/v1")
            && text.lines().any(|l| l == wanted_oid)
            && text.lines().any(|l| l == wanted_size),
    )
}

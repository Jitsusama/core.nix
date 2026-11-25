//! How `git add` would turn a file in the worktree into a blob, as git's
//! convert.c does: through a filter, with its line endings and its ident
//! collapsed, as its attributes and the settings say. A file whose stat
//! moved is told from one whose contents did by comparing that blob with
//! the index's.

use super::{Cache, Paths, Settings, os};
use crate::attrs::{self, State};
use crate::config::Config;
use crate::hash::{Oid, Sha256, to_hex};
use crate::index::Entry;
use crate::odb::Odb;
use std::collections::{BTreeSet, HashMap};
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
        Filter::Lfs => return matches_lfs_pointer(odb, &entry.oid, &raw),
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    Conversion {
        filter: filter_of(&attrs, settings),
        crlf: converts_crlf(&attrs, settings),
        ident: attrs.get("ident") == Some(&State::Set),
    }
}

/// The filters git runs on a file's way in, by name: those config gives a
/// clean or a process command, since one with a smudge alone, or none,
/// leaves a file as it is.
pub fn filters_git_runs(config: &Config) -> BTreeSet<String> {
    config
        .subsections("filter")
        .into_iter()
        .filter(|name| {
            ["clean", "process"].iter().any(|step| {
                config
                    .get(&format!("filter.{name}.{step}"))
                    .is_some_and(|command| !command.is_empty())
            })
        })
        .map(str::to_owned)
        .collect()
}

/// What runs on a file's way in before its line endings, as convert.c
/// has it: the filter its attributes name, where git has a clean step for
/// it, since one it has none for leaves the file as it is, or else a
/// change of encoding from any but UTF-8.
fn filter_of(attrs: &HashMap<String, State>, settings: &Settings) -> Filter {
    match attrs.get("filter") {
        Some(State::Value(name)) if settings.filters.contains(name) => {
            if name == "lfs" {
                Filter::Lfs
            } else {
                Filter::Other
            }
        }
        _ => match attrs.get("working-tree-encoding") {
            Some(State::Value(encoding))
                if encoding.is_empty()
                    || encoding.eq_ignore_ascii_case("utf-8")
                    || encoding.eq_ignore_ascii_case("utf8") =>
            {
                Filter::None
            }
            // git refuses an encoding set or unset with no name, so no
            // answer of its can be matched.
            Some(_) => Filter::Other,
            None => Filter::None,
        },
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

/// The pointer git-lfs writes for these contents.
fn lfs_pointer(raw: &[u8]) -> String {
    let mut sha = Sha256::new();
    sha.update(raw);
    format!(
        "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize {}\n",
        to_hex(&sha.finish()),
        raw.len()
    )
}

/// Whether the index's blob, `oid`, is the LFS pointer git-lfs would write
/// for these contents.
fn matches_lfs_pointer(odb: &mut Odb, oid: &Oid, raw: &[u8]) -> io::Result<bool> {
    let pointer = lfs_pointer(raw);
    if Oid::of("blob", pointer.as_bytes()) == *oid {
        return Ok(true);
    }
    // A pointer with extensions, or one written differently: compare what
    // it says rather than its bytes.
    let Some(blob) = odb.read(oid)? else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_runs_a_filter_only_with_a_clean_or_process_command() {
        let config = Config::parsed(
            "[filter \"lfs\"]\n process = git-lfs filter-process\n\
             [filter \"Crypt\"]\n clean = crypt in\n smudge = crypt out\n\
             [filter \"view\"]\n smudge = cat\n\
             [filter \"blank\"]\n clean =\n\
             [filter \"unset\"]\n clean = cat\n clean =\n\
             [filter \"twice\"]\n clean = cat\n smudge = cat\n\
             [filter \"a.b\"]\n clean = cat\n\
             [filters]\n x = y\n",
        );
        assert_eq!(
            filters_git_runs(&config),
            ["Crypt", "a.b", "lfs", "twice"]
                .map(str::to_string)
                .into_iter()
                .collect::<BTreeSet<_>>(),
            "took a filter only smudging, with an empty or emptied command, or another \
             section's keys, or missed one with a dot or a capital in its name"
        );
    }

    fn attrs(pairs: &[(&str, State)]) -> HashMap<String, State> {
        pairs
            .iter()
            .map(|(name, state)| ((*name).to_string(), state.clone()))
            .collect()
    }

    fn value(text: &str) -> State {
        State::Value(text.to_string())
    }

    fn settings(autocrlf: Option<&str>, filters: &[&str]) -> Settings {
        Settings {
            autocrlf: autocrlf.map(str::to_string),
            filemode: true,
            trust_ctime: true,
            show_untracked: true,
            filters: filters
                .iter()
                .map(|name| (*name).to_string())
                .collect::<BTreeSet<_>>(),
        }
    }

    #[test]
    fn a_filter_counts_only_where_git_has_a_clean_step_for_it() {
        let none = settings(None, &[]);
        let both = settings(None, &["lfs", "crypt"]);
        for (filter, settings, wanted) in [
            (value("crypt"), &none, Filter::None),
            (value("lfs"), &none, Filter::None),
            (value("crypt"), &both, Filter::Other),
            (value("lfs"), &both, Filter::Lfs),
            (value("Crypt"), &both, Filter::None),
            (State::Set, &both, Filter::None),
            (State::Unset, &both, Filter::None),
        ] {
            let attrs = attrs(&[("filter", filter.clone())]);
            assert_eq!(
                filter_of(&attrs, settings),
                wanted,
                "filter {filter:?} with clean steps for {:?}",
                settings.filters
            );
        }
    }

    #[test]
    fn only_an_encoding_other_than_utf8_counts() {
        let settings = settings(None, &[]);
        for (encoding, wanted) in [
            (value("UTF-8"), Filter::None),
            (value("utf-8"), Filter::None),
            (value("Utf8"), Filter::None),
            (value(""), Filter::None),
            (value("UTF-16"), Filter::Other),
            (value("utf-8-bom"), Filter::Other),
            (State::Set, Filter::Other),
            (State::Unset, Filter::Other),
        ] {
            let attrs = attrs(&[("working-tree-encoding", encoding.clone())]);
            assert_eq!(
                filter_of(&attrs, &settings),
                wanted,
                "working-tree-encoding {encoding:?}"
            );
        }
        assert_eq!(
            filter_of(&attrs(&[]), &settings),
            Filter::None,
            "a file with no attributes ran through something"
        );
    }

    #[test]
    fn a_filter_git_runs_wins_over_the_encoding() {
        let attrs = attrs(&[
            ("filter", value("lfs")),
            ("working-tree-encoding", value("UTF-16")),
        ]);
        assert_eq!(
            filter_of(&attrs, &settings(None, &["lfs"])),
            Filter::Lfs,
            "an LFS file with an encoding wasn't read as LFS"
        );
        assert_eq!(
            filter_of(&attrs, &settings(None, &[])),
            Filter::Other,
            "an encoding beside a filter git doesn't run was dropped"
        );
    }

    #[test]
    fn line_endings_follow_the_attributes_then_autocrlf() {
        for (pairs, autocrlf, wanted) in [
            (vec![("text", State::Unset)], Some("true"), Crlf::Binary),
            (vec![("text", State::Set)], None, Crlf::Text),
            (vec![("text", value("auto"))], None, Crlf::Auto),
            (vec![("crlf", value("input"))], None, Crlf::Text),
            (vec![("crlf", State::Unset)], Some("true"), Crlf::Binary),
            (
                vec![("text", State::Set), ("crlf", State::Unset)],
                None,
                Crlf::Text,
            ),
            (vec![("eol", value("lf"))], None, Crlf::Text),
            (vec![], Some("Input"), Crlf::Auto),
            (vec![], Some("true"), Crlf::Auto),
            (vec![], Some("false"), Crlf::Binary),
            (vec![], None, Crlf::Binary),
        ] {
            let attrs = attrs(&pairs);
            assert_eq!(
                converts_crlf(&attrs, &settings(autocrlf, &[])),
                wanted,
                "attributes {pairs:?} with autocrlf {autocrlf:?}"
            );
        }
    }

    #[test]
    fn contents_are_binary_as_convert_c_guesses() {
        for (data, wanted) in [
            (&b"text\r\nlines\n"[..], false),
            (b"a lone\rcr", true),
            (b"a nul\0", true),
            (b"tabs\tand\x1b escapes\x08\x0c", false),
            (b"one control \x01 in a short line", true),
            (b"\x01\x02", true),
            (b"\x7f", true),
            (b"ends in a DOS end of file\x1a", false),
            (b"", false),
        ] {
            assert_eq!(
                is_binary(data),
                wanted,
                "{:?}",
                String::from_utf8_lossy(data)
            );
        }
        let mut long = vec![b'x'; 128];
        long.push(1);
        assert!(
            !is_binary(&long),
            "one control byte in 128 printable ones read as binary"
        );
        long.push(2);
        assert!(
            is_binary(&long),
            "two control bytes in 128 printable ones read as text"
        );
    }

    #[test]
    fn only_a_cr_before_an_lf_goes() {
        assert_eq!(
            crlf_to_lf(b"a\r\nb\rc\r\r\n\r"),
            b"a\nb\rc\r\n\r".to_vec(),
            "line endings came out wrong"
        );
    }

    #[test]
    fn an_ident_collapses_only_when_closed_on_its_line() {
        for (data, wanted) in [
            (&b"$Id: 1234 $ and $Id: 5678 $"[..], &b"$Id$ and $Id$"[..]),
            (b"$Id$ stays", b"$Id$ stays"),
            (b"$Id: open\n$ not closed", b"$Id: open\n$ not closed"),
            (b"$Id: never closed", b"$Id: never closed"),
            (b"$Ident: 1 $", b"$Ident: 1 $"),
        ] {
            assert_eq!(
                collapse_ident(data),
                wanted.to_vec(),
                "{:?}",
                String::from_utf8_lossy(data)
            );
        }
    }

    /// An object store holding these blobs as loose objects.
    fn store(name: &str, blobs: &[&[u8]]) -> std::path::PathBuf {
        let objects =
            std::env::temp_dir().join(format!("git-engine-convert-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&objects);
        for blob in blobs {
            let hex = Oid::of("blob", blob).hex();
            let dir = objects.join(&hex[..2]);
            std::fs::create_dir_all(&dir).expect("made no object folder");
            let mut raw = format!("blob {}\0", blob.len()).into_bytes();
            raw.extend_from_slice(blob);
            let packed = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
            std::fs::write(dir.join(&hex[2..]), packed).expect("wrote no object");
        }
        objects
    }

    #[test]
    fn an_lfs_pointer_matches_by_what_it_says() {
        assert_eq!(
            lfs_pointer(b"hello\n"),
            "version https://git-lfs.github.com/spec/v1\n\
             oid sha256:5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03\n\
             size 6\n",
            "the pointer isn't the one git-lfs writes"
        );
        let extended = b"version https://git-lfs.github.com/spec/v1\n\
            ext-0-foo sha256:00\n\
            oid sha256:5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03\n\
            size 6\n";
        let other = b"version https://git-lfs.github.com/spec/v1\n\
            oid sha256:5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03\n\
            size 7\n";
        let objects = store("lfs", &[extended, other]);
        let mut odb = Odb::open(&objects);
        let exact = Oid::of("blob", lfs_pointer(b"hello\n").as_bytes());
        for (oid, wanted, what) in [
            (exact, true, "the pointer git-lfs writes"),
            (
                Oid::of("blob", extended),
                true,
                "a pointer with an extension",
            ),
            (Oid::of("blob", other), false, "a pointer of another size"),
            (
                Oid::of("blob", b"absent"),
                false,
                "a pointer the store lacks",
            ),
        ] {
            assert_eq!(
                matches_lfs_pointer(&mut odb, &oid, b"hello\n").expect("read no blob"),
                wanted,
                "{what}"
            );
        }
        let _ = std::fs::remove_dir_all(&objects);
    }
}

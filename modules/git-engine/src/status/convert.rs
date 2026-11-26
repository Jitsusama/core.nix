//! How `git add` would turn a file in the worktree into a blob, as git's
//! convert.c does: through a filter, with its line endings and its ident
//! collapsed, as its attributes and the settings say. A file whose stat
//! moved is told from one whose contents did by comparing that blob with
//! the index's.

use super::lfs::{is_git_lfs, matches_lfs_pointer};
use super::{Cache, Paths, Settings, os};
use crate::attrs::{self, State};
use crate::config::Config;
use crate::hash::Oid;
use crate::index::Entry;
use crate::odb::Odb;
use std::collections::{BTreeMap, HashMap};
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

/// What git does with a filter driver on a file's way in, for one that
/// does anything: a driver neither running nor required leaves the file
/// as it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Driver {
    /// Its process or clean command runs, and is git-lfs's own, which turns
    /// a file into the pointer git-lfs writes for it.
    Lfs,
    /// Its process or clean command runs, and is some other.
    Runs,
    /// It has neither, but is required, so git refuses the file.
    Refuses,
}

/// The filter drivers that do anything on a file's way in, by name, as
/// convert.c's `apply_filter` has it: a process command, where one is set,
/// runs in place of the clean one, so an empty one runs neither, and a
/// required driver that runs nothing makes git refuse the file.
pub fn filter_drivers(config: &Config) -> BTreeMap<String, Driver> {
    config
        .subsections("filter")
        .into_iter()
        .filter_map(|name| {
            let step = |step: &str| config.get(&format!("filter.{name}.{step}"));
            let (runs, verb) = match step("process") {
                Some(process) => (process, "filter-process"),
                None => (step("clean").unwrap_or_default(), "clean"),
            };
            let driver = if is_git_lfs(runs, verb) {
                Driver::Lfs
            } else if !runs.is_empty() {
                Driver::Runs
            } else if config.get_bool(&format!("filter.{name}.required")) == Some(true) {
                Driver::Refuses
            } else {
                return None;
            };
            Some((name.to_owned(), driver))
        })
        .collect()
}

/// What runs on a file's way in before its line endings, as convert.c
/// has it: the filter its attributes name, then a change of encoding from
/// any but UTF-8, made to the filter's output. The engine changes no
/// encoding, so a file whose encoding git would change, or one git
/// refuses, counts as changed before its filter is looked at.
fn filter_of(attrs: &HashMap<String, State>, settings: &Settings) -> Filter {
    match attrs.get("working-tree-encoding") {
        Some(State::Value(encoding))
            if encoding.is_empty()
                || encoding.eq_ignore_ascii_case("utf-8")
                || encoding.eq_ignore_ascii_case("utf8") => {}
        // git refuses an encoding set or unset with no name, so no answer
        // of its can be matched.
        Some(_) => return Filter::Other,
        None => {}
    }
    match attrs.get("filter") {
        Some(State::Value(name)) => match settings.filters.get(name) {
            Some(Driver::Lfs) => Filter::Lfs,
            Some(_) => Filter::Other,
            None => Filter::None,
        },
        _ => Filter::None,
    }
}

fn attributes_of(path: &[u8], paths: &Paths, cache: &mut Cache) -> HashMap<String, State> {
    let mut files: Vec<(Vec<u8>, Arc<attrs::File>)> = Vec::new();
    if let Some(system) = &paths.system_attributes {
        files.push((Vec::new(), cache.attributes_file(system, true)));
    }
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

/// What happens to a file's CRLF line endings on the way in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Crlf {
    /// They stay.
    Binary,
    /// They become LF.
    Text,
    /// They become LF unless the file reads as binary.
    Auto,
}

/// What core.autocrlf makes of a file no attribute speaks for: input asks
/// for CRLF to become LF as true does, and a bare key is true.
pub fn autocrlf(config: &Config) -> Crlf {
    if config
        .get("core.autocrlf")
        .is_some_and(|value| value.eq_ignore_ascii_case("input"))
        || config.get_bool("core.autocrlf") == Some(true)
    {
        Crlf::Auto
    } else {
        Crlf::Binary
    }
}

/// convert.c's choice of what to do with line endings on the way in: the
/// text attribute, or crlf where text says nothing it knows, then an eol
/// of lf or crlf, which makes a file text, then core.autocrlf. Any other
/// eol, set or unset, says nothing.
fn converts_crlf(attrs: &HashMap<String, State>, settings: &Settings) -> Crlf {
    let said = |name: &str| match attrs.get(name)? {
        State::Unset => Some(Crlf::Binary),
        State::Set => Some(Crlf::Text),
        State::Value(v) if v == "auto" => Some(Crlf::Auto),
        State::Value(v) if v == "input" => Some(Crlf::Text),
        State::Value(_) => None,
    };
    if let Some(crlf) = said("text").or_else(|| said("crlf")) {
        return crlf;
    }
    match attrs.get("eol") {
        Some(State::Value(eol)) if eol == "lf" || eol == "crlf" => Crlf::Text,
        _ => settings.autocrlf,
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
             [filter \"emptied\"]\n clean = cat\n process =\n\
             [filter \"must\"]\n smudge = cat\n required\n\
             [filter \"loose\"]\n smudge = cat\n required = false\n\
             [filter \"sure\"]\n clean = cat\n required\n\
             [filter \"pathed\"]\n clean = /bin/git-lfs clean -- %f\n\
             [filter \"wrapped\"]\n process = git-lfs-wrapper filter-process\n\
             [filter \"crossed\"]\n process = git-lfs clean\n\
             [filter \"crossedthrough\"]\n process = git lfs clean\n\
             [filter \"piped\"]\n clean = git-lfs clean -- %f | tr a b\n\
             [filter \"bare\"]\n clean = git-lfs clean\n\
             [filter \"named\"]\n clean = git-lfs clean %f\n\
             [filter \"through\"]\n process = git lfs filter-process\n\
             [filter \"cleaned\"]\n clean = /usr/bin/git lfs clean -- %f\n\
             [filter \"lfsx\"]\n process = git lfsx filter-process\n\
             [filter \"either\"]\n clean = git-lfs clean -- %f\n process = crypt\n\
             [filter \"skipping\"]\n process = git-lfs filter-process --skip\n\
             [filter \"skipclean\"]\n clean = git-lfs clean --skip -- %f\n\
             [filter \"skipafter\"]\n process = git-lfs filter-process --skip x\n\
             [filter \"skipnamed\"]\n process = git-lfs filter-process --skip %f\n\
             [filter \"skipdashed\"]\n process = git-lfs filter-process --skip -- %f\n\
             [filters]\n x = y\n",
        );
        let runs = |name: &str| (name.to_owned(), Driver::Runs);
        let lfs = |name: &str| (name.to_owned(), Driver::Lfs);
        assert_eq!(
            filter_drivers(&config),
            BTreeMap::from([
                runs("Crypt"),
                runs("a.b"),
                runs("crossed"),
                runs("crossedthrough"),
                runs("piped"),
                lfs("bare"),
                lfs("named"),
                lfs("lfs"),
                lfs("pathed"),
                runs("wrapped"),
                ("must".to_owned(), Driver::Refuses),
                runs("sure"),
                runs("twice"),
                lfs("through"),
                lfs("cleaned"),
                runs("lfsx"),
                runs("either"),
                lfs("skipping"),
                runs("skipclean"),
                runs("skipafter"),
                lfs("skipnamed"),
                lfs("skipdashed"),
            ]),
            "took a filter only smudging, with an empty or emptied command, an empty \
             process over a clean one, or another section's keys; missed one with a dot \
             or a capital in its name; misjudged a required one; took for git-lfs's \
             a command other than its process or clean, one whose process runs \
             another, or one piping it on; or missed git-lfs run through git, bare, \
             given the file's name or its process told to skip smudging"
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

    fn settings(autocrlf: bool, filters: &[(&str, Driver)]) -> Settings {
        Settings {
            autocrlf: if autocrlf { Crlf::Auto } else { Crlf::Binary },
            filemode: true,
            trust_ctime: true,
            show_untracked: true,
            filters: filters
                .iter()
                .map(|(name, driver)| ((*name).to_string(), *driver))
                .collect(),
        }
    }

    #[test]
    fn a_filter_counts_only_where_git_has_a_clean_step_for_it() {
        let none = settings(false, &[]);
        let both = settings(false, &[("lfs", Driver::Lfs), ("crypt", Driver::Runs)]);
        let other = settings(false, &[("lfs", Driver::Runs)]);
        let refusing = settings(
            false,
            &[("lfs", Driver::Refuses), ("must", Driver::Refuses)],
        );
        for (filter, settings, wanted) in [
            (value("crypt"), &none, Filter::None),
            (value("lfs"), &none, Filter::None),
            (value("crypt"), &both, Filter::Other),
            (value("lfs"), &both, Filter::Lfs),
            // A driver named lfs whose command is another's is no LFS.
            (value("lfs"), &other, Filter::Other),
            (value("Crypt"), &both, Filter::None),
            (State::Set, &both, Filter::None),
            (State::Unset, &both, Filter::None),
            // git refuses these files, so they count as changed.
            (value("must"), &refusing, Filter::Other),
            (value("lfs"), &refusing, Filter::Other),
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
        let settings = settings(false, &[]);
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
    fn an_encoding_counts_over_any_filter() {
        let lfs = settings(false, &[("lfs", Driver::Lfs)]);
        for (encoding, settings, wanted) in [
            ("UTF-16", &lfs, Filter::Other),
            ("UTF-16", &settings(false, &[]), Filter::Other),
            ("utf-8", &lfs, Filter::Lfs),
        ] {
            let attrs = attrs(&[
                ("filter", value("lfs")),
                ("working-tree-encoding", value(encoding)),
            ]);
            assert_eq!(
                filter_of(&attrs, settings),
                wanted,
                "an LFS file with encoding {encoding} and filters {:?}: git re-encodes what \
                 the filter gives, so no pointer can be matched",
                settings.filters
            );
        }
    }

    #[test]
    fn line_endings_follow_the_attributes_then_autocrlf() {
        for (pairs, autocrlf, wanted) in [
            (vec![("text", State::Unset)], true, Crlf::Binary),
            (vec![("text", State::Set)], false, Crlf::Text),
            (vec![("text", value("auto"))], false, Crlf::Auto),
            (vec![("crlf", value("input"))], false, Crlf::Text),
            (vec![("crlf", State::Unset)], true, Crlf::Binary),
            (
                vec![("text", State::Set), ("crlf", State::Unset)],
                false,
                Crlf::Text,
            ),
            // A text git doesn't know says nothing, so crlf is read.
            (
                vec![("text", value("foo")), ("crlf", State::Set)],
                false,
                Crlf::Text,
            ),
            (
                vec![("text", value("foo")), ("crlf", State::Unset)],
                true,
                Crlf::Binary,
            ),
            (vec![("text", value("foo"))], true, Crlf::Auto),
            (vec![("eol", value("lf"))], false, Crlf::Text),
            (vec![("eol", value("crlf"))], false, Crlf::Text),
            // An eol makes text only of a file whose text is unsaid.
            (
                vec![("text", value("auto")), ("eol", value("lf"))],
                false,
                Crlf::Auto,
            ),
            (
                vec![("text", State::Unset), ("eol", value("crlf"))],
                true,
                Crlf::Binary,
            ),
            // An eol other than lf or crlf says nothing either.
            (vec![("eol", value("foo"))], false, Crlf::Binary),
            (vec![("eol", value("foo"))], true, Crlf::Auto),
            (vec![("eol", State::Set)], false, Crlf::Binary),
            (vec![("eol", State::Unset)], false, Crlf::Binary),
            (vec![("eol", State::Unset)], true, Crlf::Auto),
            (vec![], true, Crlf::Auto),
            (vec![], false, Crlf::Binary),
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
    fn autocrlf_reads_as_git_reads_it() {
        for (text, wanted) in [
            ("", Crlf::Binary),
            ("[core]\n autocrlf\n", Crlf::Auto),
            ("[core]\n autocrlf = Input\n", Crlf::Auto),
            ("[core]\n autocrlf = yes\n", Crlf::Auto),
            ("[core]\n autocrlf = false\n", Crlf::Binary),
            ("[core]\n autocrlf = true\n autocrlf = off\n", Crlf::Binary),
            ("[core]\n autocrlf = maybe\n", Crlf::Binary),
        ] {
            assert_eq!(autocrlf(&Config::parsed(text)), wanted, "config {text:?}");
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
            (b"a DOS end of file\x1a mid-way", true),
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
        long.truncate(129);
        long.push(0x1a);
        assert!(
            !is_binary(&long),
            "a DOS end of file counted against a file at the threshold"
        );
        let mut nul = vec![b'x'; 200];
        nul.insert(100, 0);
        assert!(
            is_binary(&nul),
            "a NUL among 200 printable bytes read as text"
        );
        let mut high = vec![0x80; 200];
        high.push(1);
        assert!(
            !is_binary(&high),
            "bytes past 0x7f weren't counted printable, as convert.c counts them"
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
}

//! What a check reads only because `checks.nix` gathers every folder's.
//!
//! To name every check, `checks.nix` imports each folder's `checks.nix`,
//! so a check traced alone reads all 162 of them, though its derivation
//! depends only on its own file and what that names. Over the last 100
//! merges on 12 November about 18 changed one, and each such gate
//! evaluated every check again. The flake says which file gives each check
//! (`legacyPackages.<system>.checkFiles`), and a check's record keeps, of
//! the gathered files, its own and those a file it read names by path: a
//! folder names each gathered file under it, as `${../..}` or a fileset of
//! `../.` copies them, while a fileset's `root` copies nothing of its own,
//! nor does a folder a fileset's filter takes only QML or another kind
//! than Nix's from, as quickshell's checks find the shell's files under
//! modules/.
//! A file using the flake's whole source as a path or the outputs holding
//! every check, and a check `checks.nix` gives itself, keep them all. But
//! `flake.nix` and `checks.nix`, which every check reads, make the outputs:
//! their use of the checks makes others, so only their paths count.
//!
//! A gathered file may gather files of its own, as
//! `modules/quickshell/checks.nix` imports 39 for its 182 checks, and those
//! every check reads too. So a file every check read, which a gathered file
//! names by path, is kept as a gathered one is: by the checks whose own
//! files name it. One that `checks.nix` or `flake.nix` names stays in all.
//!
//! Only the names `self` and `core` are seen: a file handed the flake as a
//! function's argument of another name, which used its checks, would lose
//! the gathered files it truly depends on. None does on 13 November.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use crate::evaluating;
use crate::reads::Reads;

/// The file a check `checks.nix` gives itself is said to come from.
const ROOT: &str = "checks.nix";

/// The files that make the flake's outputs, every check read by each.
const MAKING: [&str; 2] = [ROOT, "flake.nix"];

/// The file that gives each check, by its name, from the flake at
/// `source`; none when it can't be said, as of a base from before the
/// flake said it.
pub fn files(source: &str, system: &str) -> Option<BTreeMap<String, String>> {
    let of = format!("path:{source}#legacyPackages.{system}.checkFiles");
    let asked = ["eval", "--json", &of, evaluating::UNCACHED];
    let said = crate::run("nix", &evaluating::pure(&asked)).ok()?;
    serde_json::from_str(&said).ok()
}

/// Leave out of each check's reads, in `each`, the gathered files, those
/// `files` names and those they gather in turn, that neither give the check
/// nor are named by a file it read, each read from `source`.
pub fn narrow(source: &Path, files: &BTreeMap<String, String>, each: &mut BTreeMap<&str, Reads>) {
    let listed: BTreeSet<&str> = files
        .values()
        .map(String::as_str)
        .filter(|file| *file != ROOT)
        .collect();
    let floor: BTreeSet<String> = each
        .values()
        .map(|reads| &reads.read)
        .reduce(|one, other| if one.len() <= other.len() { one } else { other })
        .into_iter()
        .flatten()
        .filter(|file| {
            Path::new(file).extension().is_some_and(|it| it == "nix")
                && ![ROOT, "flake.nix"].contains(&file.as_str())
                && !listed.contains(file.as_str())
                && each.values().all(|reads| reads.read.contains(*file))
        })
        .cloned()
        .collect();
    let everyones: BTreeSet<&str> = floor.iter().map(String::as_str).collect();
    let mut gathered = listed.clone();
    for file in &listed {
        let named = named_by(source, file, &everyones, &listed).unwrap_or_default();
        gathered.extend(
            floor
                .iter()
                .filter(|one| named.contains(*one))
                .map(String::as_str),
        );
    }
    // Many checks read the same files, so each is read once.
    let mut named: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (name, reads) in each.iter_mut() {
        let Some(own) = files.get(*name).filter(|own| *own != ROOT) else {
            continue;
        };
        let mut kept = BTreeSet::from([own.clone()]);
        let mut left: Vec<String> = reads
            .read
            .iter()
            .filter(|file| {
                Path::new(file).extension().is_some_and(|it| it == "nix")
                    && !gathered.contains(file.as_str())
            })
            .cloned()
            .chain([own.clone()])
            .collect();
        let mut looked = BTreeSet::new();
        while let Some(file) = left.pop() {
            if !looked.insert(file.clone()) {
                continue;
            }
            let these = named.entry(file.clone()).or_insert_with(|| {
                named_by(source, &file, &gathered, &listed)
                    .unwrap_or_else(|| gathered.iter().map(|one| (*one).to_owned()).collect())
            });
            for one in these.iter() {
                if kept.insert(one.clone()) {
                    left.push(one.clone());
                }
            }
        }
        reads
            .read
            .retain(|file| !gathered.contains(file.as_str()) || kept.contains(file));
    }
}

/// The files of `gathered` that `file` names by path, read from `source`,
/// but those of `listed` the root names exactly, which is its gathering;
/// none when it can't be read, or uses the whole source, so may need any.
fn named_by(
    source: &Path,
    file: &str,
    gathered: &BTreeSet<&str>,
    listed: &BTreeSet<&str>,
) -> Option<BTreeSet<String>> {
    let text = fs::read_to_string(source.join(file)).ok()?;
    let code = code(&text);
    if takes_whole(&code, MAKING.contains(&file)) {
        return None;
    }
    let here = file.rsplit_once('/').map_or("", |(folder, _)| folder);
    let mut found = BTreeSet::new();
    for path in paths(&code) {
        let Some(at) = resolved(here, &path) else {
            continue;
        };
        for one in gathered {
            let under = at.is_empty() || one.starts_with(&format!("{at}/"));
            // The root's list of files is the gathering itself.
            if under || (*one == at && (file != ROOT || !listed.contains(one))) {
                found.insert((*one).to_owned());
            }
        }
    }
    Some(found)
}

/// Whether `code` uses the flake's own source, `self` or the `core` it's
/// given as, as a path, or the outputs that hold every check: its checks,
/// its legacy packages or any it names by a variable, as a check of which
/// checks exist does. Its modules, library and packages gather none. The
/// files `making` the outputs use its checks for others no check reads,
/// as `flake.nix` makes the subtests of them, so only a path counts there.
fn takes_whole(code: &str, making: bool) -> bool {
    ["self", "core"].iter().any(|name| {
        code.match_indices(name).any(|(at, _)| {
            let before = &code[..at];
            let after = &code[at + name.len()..];
            let alone = !before.ends_with(is_word) && !after.starts_with(is_word);
            let rest = after.trim_start();
            let lead = before.trim_end();
            let output = |named: &str| {
                rest.strip_prefix(named)
                    .is_some_and(|it| !it.starts_with(is_word))
            };
            let path = before.ends_with("${")
                || rest.starts_with('+')
                || output(".outPath")
                || ["src =", "path =", "toString"]
                    .iter()
                    .any(|word| lead.ends_with(word));
            let checks = output(".checks") || output(".legacyPackages") || rest.starts_with(".${");
            alone && (path || (checks && !making))
        })
    })
}

/// Whether a character may be part of a name.
fn is_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || "_-'".contains(character)
}

/// Whether a character may be part of a name or a path.
fn is_named(character: char) -> bool {
    is_word(character) || "./+".contains(character)
}

/// Each relative path literal in `code` that copies or reads what it
/// names: all but those that only place others.
fn paths(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in code.match_indices("./") {
        let start = code[..at].trim_end_matches('.').len();
        if at - start > 1 || code[..start].ends_with(is_named) {
            continue;
        }
        let path: String = code[start..]
            .chars()
            .take_while(|character| is_named(*character) && *character != '\'')
            .collect();
        if !places(&code[..start]) && !found.contains(&path) {
            found.push(path);
        }
    }
    found
}

/// Whether what comes before a path, `before`, makes it one that only
/// places other paths, copying nothing itself: a fileset's root, which
/// copies only what the fileset names, or what a path is taken from. Or
/// a folder a fileset's filter takes only files of another kind from.
fn places(before: &str) -> bool {
    if before.trim_end().ends_with("removePrefix") || filters_out_nix(before) {
        return true;
    }
    let Some(named) = before.trim_end().strip_suffix('=') else {
        return false;
    };
    let named = named.trim_end();
    let near = (before.len().saturating_sub(400)..before.len())
        .find(|at| before.is_char_boundary(*at))
        .unwrap_or(before.len());
    named.ends_with("root")
        && !named[..named.len() - 4].ends_with(is_named)
        && before[near..].contains("toSource")
}

/// Whether `before` ends by handing what follows to a fileset's filter
/// taking only files of one kind other than Nix's, as
/// `fileFilter (file: file.hasExt "qml")` does. What it gives holds no
/// gathered file, which is Nix, however many sit beside what it takes. A
/// file added or removed there is still seen, as the filter lists the
/// folder and the trace sees the listing. Any other predicate, or another
/// use of the same folder, still names every file under it.
fn filters_out_nix(before: &str) -> bool {
    let Some((_, filter)) = before
        .trim_end()
        .strip_suffix(')')
        .and_then(|call| call.rsplit_once("fileFilter"))
    else {
        return false;
    };
    let filter: String = filter
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let Some((name, asked)) = filter
        .strip_prefix('(')
        .and_then(|filter| filter.split_once(':'))
    else {
        return false;
    };
    let kind = asked
        .strip_prefix(name)
        .and_then(|asked| asked.strip_prefix(".hasExt\""))
        .and_then(|asked| asked.strip_suffix('"'));
    !name.is_empty()
        && name.chars().all(is_word)
        && kind.is_some_and(|kind| !kind.is_empty() && kind.chars().all(is_word) && kind != "nix")
}

/// The path `path` names from the folder `here`, both from the
/// repository's root, which is the empty path; none above the root.
fn resolved(here: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = here.split('/').filter(|part| !part.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// `text`, a Nix file, with its comments gone and its strings' text gone
/// but what they interpolate, kept as `${...}`, so a path or a name counts
/// only where it's code.
fn code(text: &str) -> String {
    let mut scanner = Scanner {
        text: text.as_bytes(),
        at: 0,
        out: Vec::new(),
    };
    scanner.code(false);
    String::from_utf8_lossy(&scanner.out).into_owned()
}

/// Where a scan of a Nix file has got to, and what it has kept.
struct Scanner<'a> {
    text: &'a [u8],
    at: usize,
    out: Vec<u8>,
}

impl Scanner<'_> {
    fn starts(&self, with: &str) -> bool {
        self.text[self.at..].starts_with(with.as_bytes())
    }

    /// Keep code up to the end of the text, or, `inside` an
    /// interpolation, past the brace that closes it.
    fn code(&mut self, inside: bool) {
        let mut depth = 0;
        while let Some(&byte) = self.text.get(self.at) {
            if self.starts("/*") {
                self.at = self.text[self.at + 2..]
                    .windows(2)
                    .position(|two| two == b"*/")
                    .map_or(self.text.len(), |found| self.at + 2 + found + 2);
                continue;
            }
            self.at += 1;
            match byte {
                b'{' if inside => depth += 1,
                b'}' if inside && depth == 0 => return,
                b'}' if inside => depth -= 1,
                b'#' => {
                    while self.text.get(self.at).is_some_and(|next| *next != b'\n') {
                        self.at += 1;
                    }
                    continue;
                }
                b'"' => {
                    self.string(false);
                    continue;
                }
                b'\'' if self.starts("'") => {
                    self.at += 1;
                    self.string(true);
                    continue;
                }
                _ => {}
            }
            self.out.push(byte);
        }
    }

    /// Pass over a string, `indented` or in quotes, keeping only what it
    /// interpolates, each as `${...}` with a space either side. A kind of
    /// file a fileset's filter asks for (`hasExt "qml"`) is kept whole, in
    /// its quotes, when it interpolates nothing.
    fn string(&mut self, indented: bool) {
        let kind = !indented && self.out.trim_ascii_end().ends_with(b"hasExt");
        let from = self.at;
        let mut plain = true;
        let mut ended = None;
        self.out.push(b' ');
        while self.at < self.text.len() {
            if self.starts("$${") || (indented && (self.starts("''$") || self.starts("'''"))) {
                self.at += 3;
            } else if indented && self.starts("''\\") {
                self.at += 4;
            } else if !indented && self.starts("\\") {
                self.at += 2;
            } else if self.starts("${") {
                plain = false;
                self.at += 2;
                self.out.extend_from_slice(b"${");
                self.code(true);
                self.out.extend_from_slice(b"} ");
            } else if (indented && self.starts("''")) || (!indented && self.starts("\"")) {
                ended = Some(self.at);
                self.at += if indented { 2 } else { 1 };
                break;
            } else {
                self.at += 1;
            }
        }
        if let Some(end) = ended.filter(|_| kind && plain) {
            let text = self.text;
            self.out.push(b'"');
            self.out.extend_from_slice(&text[from..end]);
            self.out.push(b'"');
        }
        self.out.push(b' ');
    }
}

#[cfg(test)]
mod tests;

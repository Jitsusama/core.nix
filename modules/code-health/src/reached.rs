//! The code files nothing reaches, read from the files alone: no other
//! file names it, imports it as its language does, or takes it with a
//! folder holding it, so the quality standard's rule 11 (no file nothing
//! reaches) is held to a count. It reads generously, so a file it names is
//! dead but a dead file may pass: one mentioned only in prose still counts
//! as reached, and an export nothing imports is knip's and the compilers'
//! to find, not this.
//!
//! A program's entry, a test its runner finds by name, and a crate's
//! `tests`, `benches` and `examples` are reached by how they're run.

use std::collections::{HashMap, HashSet};

use crate::language::{Language, language};
use crate::measure::{File, RUN_BY_NAME, extension, is_code};

/// Every code file in `paths` that no file of `files` reaches.
pub fn unreached(paths: &[String], files: &[File]) -> Vec<String> {
    let reach = Reach::of(files);
    let crates: HashSet<&str> = paths
        .iter()
        .filter_map(|path| match path.rsplit_once('/') {
            Some((folder, "Cargo.toml")) => Some(folder),
            None if path == "Cargo.toml" => Some(""),
            _ => None,
        })
        .collect();
    paths
        .iter()
        .filter(|path| is_code(path) && !run_as_it_is(path, &crates))
        .filter(|path| !reach.reaches(path))
        .cloned()
        .collect()
}

/// Whether a file is reached by how it's run, with nothing naming it: a
/// program's entry, a test its runner finds by its name, or a file of a
/// crate's `tests`, `benches`, `examples` or `src/bin`.
fn run_as_it_is(path: &str, crates: &HashSet<&str>) -> bool {
    let (folder, name) = split(path);
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let test = matches!(extension(stem), "test" | "spec")
        || stem.ends_with("_test")
        || stem.ends_with("-test")
        || (extension(name) == "rs" && stem.ends_with("_tests"))
        || (language(name) == Some(Language::Python) && stem.starts_with("test_"));
    let (above, last) = split(folder);
    let in_crate = matches!(last, "tests" | "benches" | "examples") && crates.contains(above)
        || (last == "bin" && split(above).1 == "src");
    RUN_BY_NAME.contains(&name) || name == "conftest.py" || test || in_crate
}

/// A path's folder and its last part, the folder of a top file being `""`.
fn split(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

/// What every file reaches, each by the files reaching it.
#[derive(Default)]
struct Reach<'a> {
    /// File names, extension and all, named anywhere.
    names: HashMap<&'a str, HashSet<&'a str>>,
    /// Rust modules declared with `mod name;`.
    modules: HashMap<&'a str, HashSet<&'a str>>,
    /// The parts of what Python imports and the last part of what
    /// TypeScript and JavaScript import, as modules.
    imports: HashMap<String, HashSet<&'a str>>,
    /// QML types made, as `Name {`.
    types: HashMap<&'a str, HashSet<&'a str>>,
    /// Paths named, resolved from the repository's top: a file, a file
    /// without its extension, or a folder taken whole.
    paths: HashMap<String, HashSet<&'a str>>,
}

impl<'a> Reach<'a> {
    fn of(files: &'a [File]) -> Self {
        let mut reach = Reach::default();
        for file in files {
            let by = file.path.as_str();
            let (here, _) = split(by);
            for token in file.text.split(|c: char| !is_path_char(c)) {
                if token.contains('/') {
                    reach.path(here, token, by);
                }
                let name = split(token).1;
                if name.trim_matches('.').contains('.') {
                    reach.names.entry(name).or_default().insert(by);
                }
            }
            for line in file.text.lines() {
                reach.line(line.trim(), by);
            }
        }
        reach
    }

    /// What one line reaches as its language does.
    fn line(&mut self, line: &'a str, by: &'a str) {
        let declared = line
            .trim_start_matches("pub ")
            .trim_start_matches("pub(crate) ")
            .strip_prefix("mod ")
            .and_then(|rest| rest.strip_suffix(';'));
        if let Some(module) = declared {
            self.modules.entry(module.trim()).or_default().insert(by);
        }
        // Python's `import a.b` and `from a.b import c, d`.
        let python = line
            .strip_prefix("import ")
            .or_else(|| line.strip_prefix("from "));
        if let Some(rest) = python {
            let (module, names) = rest.split_once(" import ").unwrap_or((rest, ""));
            let names = names
                .split(',')
                .filter_map(|name| name.split_whitespace().next());
            for part in module.split(['.', ' ', ',']).chain(names) {
                if !part.is_empty() {
                    self.imports.entry(part.to_owned()).or_default().insert(by);
                }
            }
        }
        // TypeScript's and JavaScript's `from "./a.js"` and `require("a")`.
        for (at, _) in line.match_indices(['"', '\'']) {
            let before = line[..at].trim_end();
            if before.ends_with("from") || before.ends_with("import") || before.ends_with('(') {
                let rest = &line[at + 1..];
                let spec = rest.split(['"', '\'']).next().unwrap_or_default();
                let last = split(spec).1;
                let module = last.rsplit_once('.').map_or(last, |(stem, _)| stem);
                for module in [last, module] {
                    self.imports
                        .entry(module.to_owned())
                        .or_default()
                        .insert(by);
                }
            }
        }
        // QML's `Name {`.
        if matches!(extension(by), "qml" | "js") {
            for part in line.split('{').rev().skip(1) {
                let name = part
                    .trim_end()
                    .rsplit(|c: char| !c.is_alphanumeric())
                    .next();
                if let Some(name) = name.filter(|name| name.starts_with(char::is_uppercase)) {
                    self.types.entry(name).or_default().insert(by);
                }
            }
        }
    }

    /// A path named in a file in `here`: from `here` when it starts with a
    /// dot, else from `here` and from the top. A path to the folder naming
    /// it, or one above, reaches nothing, since every file is in those.
    fn path(&mut self, here: &str, token: &str, by: &'a str) {
        let bases: &[&str] = if token.starts_with('.') {
            &[here]
        } else {
            &[here, ""]
        };
        for base in bases {
            let mut parts: Vec<&str> = base.split('/').filter(|part| !part.is_empty()).collect();
            for part in token.split('/') {
                match part {
                    ".." => {
                        parts.pop();
                    }
                    "." | "" => {}
                    part => parts.push(part),
                }
            }
            let resolved = parts.join("/");
            let above = format!("{here}/").starts_with(&format!("{resolved}/"));
            if !resolved.is_empty() && !above {
                self.paths.entry(resolved).or_default().insert(by);
            }
        }
    }

    /// Whether any file but `path` itself reaches it.
    fn reaches(&self, path: &str) -> bool {
        let other = |by: Option<&HashSet<&str>>| by.is_some_and(|by| by.iter().any(|b| *b != path));
        let (_, name) = split(path);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        let bare = path.rsplit_once('.').map_or(path, |(bare, _)| bare);
        let by_language = match language(path) {
            Some(Language::Rust) => other(self.modules.get(stem)),
            Some(Language::Script | Language::Python) => other(self.imports.get(stem)),
            Some(Language::Qml) => other(self.types.get(stem)),
            _ => false,
        };
        let folders = path
            .match_indices('/')
            .any(|(at, _)| other(self.paths.get(&path[..at])));
        other(self.names.get(name))
            || by_language
            || folders
            || other(self.paths.get(path))
            || other(self.paths.get(bare))
    }
}

/// Whether a character can be part of a path as code names one.
fn is_path_char(c: char) -> bool {
    c.is_alphanumeric() || "_.-/".contains(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::file;

    fn dead(files: &[File]) -> Vec<String> {
        let paths: Vec<String> = files.iter().map(|file| file.path.clone()).collect();
        unreached(&paths, files)
    }

    #[test]
    fn a_file_nothing_names_is_unreached() {
        let files = [
            file("src/main.rs", "fn main() {}\n"),
            file("src/orphan.rs", "pub fn left() {}\n"),
            file("lib/named.sh", "echo\n"),
            file("README.md", "Run lib/named.sh to start.\n"),
        ];
        assert_eq!(dead(&files), ["src/orphan.rs"]);
    }

    #[test]
    fn a_file_reached_as_its_language_reaches_it_is_not() {
        let files = [
            file("src/main.rs", "mod reader;\npub(crate) mod writer;\n"),
            file("src/reader.rs", "\n"),
            file("src/writer.rs", "\n"),
            file(
                "web/app.ts",
                "import { a } from \"./parts/a.js\";\nimport b from '../b';\n",
            ),
            file("web/parts/a.ts", "\n"),
            file("b.ts", "\n"),
            file("py/run.py", "from tally import count\nimport plain.sums\n"),
            file("py/tally.py", "\n"),
            file("py/sums.py", "\n"),
            file("ui/Main.qml", "Item {\n    Badge { }\n}\n"),
            file("ui/Badge.qml", "\n"),
            file("ui/Unused.qml", "\n"),
        ];
        assert_eq!(
            dead(&files),
            ["web/app.ts", "py/run.py", "ui/Main.qml", "ui/Unused.qml"]
        );
    }

    #[test]
    fn a_file_in_a_folder_taken_whole_is_reached() {
        let files = [
            file("tests/holders.nix", "{ src = ./holders; }\n"),
            file("tests/holders/stand-in.py", "\n"),
            file(
                "package.json",
                "{ \"exports\": { \"./auth/setup\": \"./dist/x.js\" } }\n",
            ),
            file("auth/setup.ts", "\n"),
        ];
        assert_eq!(dead(&files), ["tests/holders.nix"]);
    }

    #[test]
    fn a_folder_naming_its_own_parent_reaches_nothing_in_it() {
        let files = [
            file("checks/all.nix", "{ here = ./.; up = ../checks; }\n"),
            file("checks/lost.nix", "{ }\n"),
        ];
        assert_eq!(dead(&files), ["checks/all.nix", "checks/lost.nix"]);
    }

    #[test]
    fn entries_and_tests_are_reached_by_how_they_run() {
        let files = [
            file("Cargo.toml", "[package]\n"),
            file("src/main.rs", "\n"),
            file("tests/cli.rs", "\n"),
            file("src/bin/extra.rs", "\n"),
            file("web/reader.test.ts", "\n"),
            file("py/test_tally.py", "\n"),
            file("checks/opens-vm-test.py", "\n"),
            file("flake.nix", "{ }\n"),
        ];
        assert!(dead(&files).is_empty(), "{:?}", dead(&files));
    }
}

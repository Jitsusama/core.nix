//! What a repository's files say of its health, read from the files alone
//! so every measure is tested without git: each is a count where more is
//! worse, so cadence holds it against the days before it as it does any.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

use crate::{attributes, lints, notes, reached, stale, terraform};

/// A file of the repository: its path from the folder read, and its text.
pub struct File {
    pub path: String,
    pub text: String,
}

/// A file at `path` holding `text`, for the tests of every measure.
#[cfg(test)]
pub fn file(path: &str, text: &str) -> File {
    File {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

/// Past this many lines a code file is long, and needs to say why.
pub const LONG: usize = 800;
/// Past this many entries a folder is crowded, and needs to say why.
pub const CROWDED: usize = 40;
/// How many lines a code file's opening comment is looked for in.
const OPENING: usize = 40;
/// How many lines in a row, the same in two places, are a duplicate: fewer
/// than eight would count a function's usual opening or a list's tail.
const SAME: usize = 8;
/// How many names each measure's day record keeps, so it stays readable;
/// its counts by file are of every name.
pub const NAMED: usize = 200;
/// The list of the points of docs/daemons.md each daemon still breaks,
/// as the daemons-agree check keeps it.
pub const DAEMONS_WAITING: &str = "tests/rules/daemons-agree-waiting.nix";

/// The files a tool runs by their name alone, so nothing need name them
/// to reach them (reached.rs), and a folder is entered from (notes.rs).
pub const RUN_BY_NAME: &[&str] = &[
    "main.rs",
    "lib.rs",
    "mod.rs",
    "build.rs",
    "flake.nix",
    "default.nix",
    "shell.nix",
    "index.ts",
    "index.js",
    "__init__.py",
];

/// The extensions of the files counted as code.
const CODE: &[&str] = &[
    "bash", "c", "cc", "cjs", "cpp", "go", "h", "js", "lua", "mjs", "nix", "py", "qml", "rb", "rs",
    "sh", "tf", "ts", "tsx",
];

/// A repository's figures, by measure, and what each counted, by name.
#[derive(Debug, Default, PartialEq)]
pub struct Health {
    pub figures: BTreeMap<&'static str, f64>,
    pub named: BTreeMap<&'static str, Vec<String>>,
}

impl Health {
    pub fn put(&mut self, measure: &'static str, names: Vec<String>, count: usize) {
        self.figures.insert(measure, figure(count));
        self.named.insert(measure, names);
    }
}

/// A path's extension, past its last dot, or `""` where it has none.
pub fn extension(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or("", |(_, extension)| extension)
}

/// Whether a path is a code file this reads.
pub fn is_code(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let generated = name.ends_with(".min.js") || path.split('/').any(|part| part == "node_modules");
    let extension = name.rsplit_once('.').map(|(_, extension)| extension);
    !generated && extension.is_some_and(|extension| CODE.contains(&extension))
}

/// Whether a path's text is read by a measure: code, a folder's note, a
/// quarantine, a lint config, or any file of words or settings that can
/// name another file and so reach it (reached.rs).
pub fn is_read(path: &str) -> bool {
    let mentions = matches!(
        extension(path),
        "md" | "json" | "jsonc" | "toml" | "yaml" | "yml"
    ) && !path.split('/').any(|part| part == "node_modules");
    is_code(path)
        || is_note(path)
        || is_quarantine(path)
        || lints::is_config(path)
        || terraform::is_terraform(path)
        || mentions
}

/// Whether a path is a folder's note, which says why it's crowded.
pub fn is_note(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name == "README.md" || name == "AGENTS.md"
}

/// Whether a path is a list of tests put aside for failing at random, as
/// a merge net keeps in `quarantine.json`.
pub fn is_quarantine(path: &str) -> bool {
    path.rsplit('/').next() == Some("quarantine.json")
}

/// Every measure of `files`, given `paths`, every file's path, so a folder
/// is counted by everything in it and not only what's read.
pub fn measure(paths: &[String], files: &[File], roots: &[String]) -> Health {
    let mut health = Health::default();
    let code: Vec<&File> = files.iter().filter(|file| is_code(&file.path)).collect();

    let long: Vec<String> = code
        .iter()
        .filter(|file| file.text.lines().count() > LONG && !says(file, "Long because"))
        .map(|file| file.path.clone())
        .collect();
    let count = long.len();
    health.put("long-files", long, count);

    let suppressions: Vec<String> = code
        .iter()
        .flat_map(|file| {
            // Each line with the file past it, since an attribute can run on.
            let mut from = 0;
            file.text
                .split_inclusive('\n')
                .enumerate()
                .filter_map(move |(number, line)| {
                    let rest = &file.text[from..];
                    from += line.len();
                    suppresses(&file.path, rest).then(|| format!("{}:{}", file.path, number + 1))
                })
        })
        .collect();
    let count = suppressions.len();
    health.put("suppressions", suppressions, count);

    let crowded = crowded(paths, files);
    let count = crowded.len();
    health.put("crowded-folders", crowded, count);

    let (lines, places) = duplicated(&code);
    health.put("duplicated-lines", places, lines);

    // A repository keeping no quarantine has no figure for it, rather
    // than a nought that would say its tests were all trusted.
    let lists: Vec<&File> = files
        .iter()
        .filter(|file| is_quarantine(&file.path))
        .collect();
    if !lists.is_empty() {
        let quarantined: Vec<String> = lists
            .iter()
            .flat_map(|file| quarantined(&file.text))
            .collect();
        let count = quarantined.len();
        health.put("quarantined-tests", quarantined, count);
    }

    let turned_off = lints::turned_off(files);
    let count = turned_off.len();
    health.put("lints-off", turned_off, count);

    let unreached = reached::unreached(paths, files);
    let count = unreached.len();
    health.put("unreached-files", unreached, count);

    let bare = notes::without_notes(paths, files, roots);
    let count = bare.len();
    health.put("folders-without-notes", bare, count);

    let gone = stale::stale(paths, files);
    let count = gone.len();
    health.put("stale-paths", gone, count);

    if let Some((named, lines)) = vm_test_lines(&code) {
        health.put("vm-test-lines", named, lines);
    }

    // Likewise a repository holding no daemons to docs/daemons.md.
    if let Some(list) = files.iter().find(|file| file.path == DAEMONS_WAITING) {
        let points = waiting_points(&list.text);
        let count = points.len();
        health.put("daemons-waiting", points, count);
    }
    health
}

/// The lines of a repository's VM tests, less blanks and comments, and each
/// test by its lines, most first; none when it keeps no VM tests, rather
/// than a nought that would say it keeps them cheap. A VM test is the
/// dearest level a test can be at, so a case moved down to a unit test, as
/// the quality standard's rule 6 has it, shows here as fewer lines.
fn vm_test_lines(code: &[&File]) -> Option<(Vec<String>, usize)> {
    let mut tests: Vec<(usize, &str)> = code
        .iter()
        .filter(|file| file.path.ends_with("vm-test.py"))
        .map(|file| {
            let lines = file
                .text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .count();
            (lines, file.path.as_str())
        })
        .collect();
    if tests.is_empty() {
        return None;
    }
    tests.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let total = tests.iter().map(|(lines, _)| lines).sum();
    let named = tests
        .into_iter()
        .map(|(lines, path)| format!("{path}: {lines} lines"))
        .collect();
    Some((named, total))
}

/// The points each daemon waits on, as `daemon: point`, from a list of
/// `daemon = [ "point" ... ];` entries, one or several lines each.
fn waiting_points(text: &str) -> Vec<String> {
    let mut points = Vec::new();
    let mut daemon: Option<String> = None;
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
    {
        let rest = match line.split_once(" = [") {
            Some((name, rest)) => {
                daemon = Some(name.trim_matches('"').to_owned());
                rest
            }
            None => line,
        };
        if let Some(name) = &daemon {
            let quoted = rest.split('"').skip(1).step_by(2);
            points.extend(quoted.map(|point| format!("{name}: {point}")));
        }
        if rest.contains(']') {
            daemon = None;
        }
    }
    points
}

/// The tests a quarantine list holds, by name: a list of entries, each
/// naming its test. An entry with no name still counts, as "unnamed".
fn quarantined(text: &str) -> Vec<String> {
    let entries: Vec<serde_json::Value> = serde_json::from_str(text).unwrap_or_default();
    entries
        .iter()
        .map(|entry| {
            entry
                .get("test")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unnamed")
                .to_owned()
        })
        .collect()
}

/// Whether a file's opening says `words`, as a sentence of its own.
fn says(file: &File, words: &str) -> bool {
    file.text
        .lines()
        .take(OPENING)
        .any(|line| sentence(line, words))
}

/// Whether a note says `words` anywhere, its lines joined and each word
/// whole, as core's every-long-file-says-why reads a folder's reason, so
/// a folder that rule passes isn't counted crowded here.
fn mentions(file: &File, words: &str) -> bool {
    let joined = file.text.split_whitespace().collect::<Vec<_>>().join(" ");
    let word = |c: char| c.is_alphanumeric() || c == '_';
    joined.match_indices(words).any(|(at, _)| {
        let before = joined[..at].chars().next_back();
        let after = joined[at + words.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// Whether `words` open a sentence in a line: at its start, after its
/// comment mark, or after a sentence's end.
fn sentence(line: &str, words: &str) -> bool {
    line.match_indices(words).any(|(at, _)| {
        let before = line[..at].trim_end();
        let opens = before.is_empty()
            || before.ends_with(['.', ':', ';', '!', '?'])
            || before.chars().all(|c| "#/*!-; ".contains(c));
        let after = line[at + words.len()..].chars().next();
        opens && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Whether a line of a code file turns a linter off, by its language's
/// own way of saying so, so a string naming one in other code isn't one.
/// `rest` is the file from the line on.
fn suppresses(path: &str, rest: &str) -> bool {
    let line = rest.lines().next().unwrap_or_default();
    let extension = path.rsplit_once('.').map_or("", |(_, extension)| extension);
    // A mark is a comment's only where no quote before it is left open.
    let comment = |mark: &str, markers: &[&str]| {
        line.match_indices(mark).any(|(at, _)| {
            let before = &line[..at];
            let quoted = ['"', '\'']
                .iter()
                .any(|quote| before.matches(*quote).count() % 2 == 1);
            let said = line[at + mark.len()..].trim_start();
            !quoted && markers.iter().any(|marker| said.starts_with(marker))
        })
    };
    match extension {
        "rs" => attributes::turns_a_rust_lint_off(rest),
        "py" => comment(
            "#",
            &["noqa", "type: ignore", "pyright: ignore", "pylint: disable"],
        ),
        "sh" | "bash" => comment("#", &["shellcheck disable"]),
        "qml" => comment("//", &["qmllint disable"]),
        "js" | "mjs" | "cjs" | "ts" | "tsx" => comment(
            "//",
            &[
                "biome-ignore",
                "eslint-disable",
                "@ts-ignore",
                "@ts-expect-error",
                "@ts-nocheck",
            ],
        ),
        "nix" => comment("#", &["deadnix: skip"]),
        // Terraform comments start with either mark, and trivy still reads
        // tfsec's older form.
        "tf" => {
            let markers = ["tflint-ignore", "trivy:ignore", "tfsec:ignore"];
            comment("#", &markers) || comment("//", &markers)
        }
        _ => false,
    }
}

/// Each folder holding more than `CROWDED` entries whose note doesn't say
/// why, the files and folders right in it counted alike.
fn crowded(paths: &[String], files: &[File]) -> Vec<String> {
    let mut entries: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for path in paths {
        let mut rest = path.as_str();
        while let Some((folder, _)) = rest.rsplit_once('/') {
            let entry = &path[..rest.len()];
            entries.entry(folder).or_default().insert(entry);
            rest = folder;
        }
        entries.entry("").or_default().insert(&path[..rest.len()]);
    }
    let reasoned: BTreeSet<&str> = files
        .iter()
        .filter(|file| is_note(&file.path) && mentions(file, "Many entries because"))
        .map(|file| file.path.rsplit_once('/').map_or("", |(folder, _)| folder))
        .collect();
    entries
        .into_iter()
        .filter(|(folder, inside)| inside.len() > CROWDED && !reasoned.contains(folder))
        .map(|(folder, _)| {
            if folder.is_empty() {
                ".".to_owned()
            } else {
                folder.to_owned()
            }
        })
        .collect()
}

/// The lines of code that repeat `SAME` or more lines found earlier, in
/// the same file or another, and where each run of them begins. Blank
/// lines and lines of punctuation alone are passed over, so a run of
/// closing braces is no duplicate.
fn duplicated(code: &[&File]) -> (usize, Vec<String>) {
    let mut first: HashMap<u64, (usize, usize)> = HashMap::new();
    let mut repeated: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut places = Vec::new();
    for (index, file) in code.iter().enumerate() {
        let kept: Vec<(usize, &str)> = file
            .text
            .lines()
            .enumerate()
            .map(|(number, line)| (number, line.trim()))
            .filter(|(_, line)| line.chars().filter(|c| c.is_alphanumeric()).count() >= 3)
            .collect();
        let mut run_ends = 0;
        for start in 0..kept.len().saturating_sub(SAME - 1) {
            let window = &kept[start..start + SAME];
            let mut hasher = DefaultHasher::new();
            for (_, line) in window {
                line.hash(&mut hasher);
            }
            let key = hasher.finish();
            match first.get(&key) {
                Some(&(at, line)) if (at, line) != (index, window[0].0) => {
                    if start >= run_ends {
                        places.push(format!(
                            "{}:{} repeats {}:{}",
                            file.path,
                            window[0].0 + 1,
                            code[at].path,
                            line + 1
                        ));
                    }
                    repeated.extend(window.iter().map(|(number, _)| (index, *number)));
                    run_ends = start + SAME;
                }
                Some(_) => {}
                None => {
                    first.insert(key, (index, window[0].0));
                }
            }
        }
    }
    (repeated.len(), places)
}

/// A count as a figure; no repository holds four billion of anything.
fn figure(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(count: usize, each: &str) -> String {
        let lines: Vec<String> = (0..count).map(|n| format!("{each} {n}\n")).collect();
        lines.concat()
    }

    fn paths(files: &[File]) -> Vec<String> {
        files.iter().map(|file| file.path.clone()).collect()
    }

    /// A whole repository's health, read from no folder in particular.
    fn measure(paths: &[String], files: &[File]) -> Health {
        super::measure(paths, files, &[])
    }

    #[test]
    fn a_long_file_counts_only_without_its_reason() {
        let files = [
            file("long.rs", &lines(LONG + 1, "let a =")),
            file(
                "said.rs",
                &format!("// Long because a table.\n{}", lines(LONG, "let b =")),
            ),
            file("short.rs", &lines(LONG, "let c =")),
            file("long.md", &lines(LONG + 1, "words")),
        ];
        let health = measure(&paths(&files), &files);
        assert_eq!(
            health.named["long-files"],
            ["long.rs"],
            "a long file was missed, one with its reason or not code was counted"
        );
        assert!(
            (health.figures["long-files"] - 1.0).abs() < f64::EPSILON,
            "the figure didn't match the files named: {health:?}"
        );
    }

    #[test]
    fn a_reason_must_open_a_sentence() {
        assert!(
            sentence("# Long because it's a table.", "Long because"),
            "a reason opening the comment wasn't found"
        );
        assert!(
            sentence("// It grew. Long because it's a table.", "Long because"),
            "a reason after a sentence's end wasn't found"
        );
        assert!(
            !sentence("// not Long because of anything", "Long because"),
            "words inside a sentence were taken for its reason"
        );
    }

    #[test]
    fn a_suppression_is_read_in_its_own_languages_way() {
        let files = [
            file("a.py", "x = 1  # noqa: E501\ny = '# noqa'\n"),
            file(
                "b.sh",
                "# shellcheck disable=SC2086\necho \"# shellcheck\"\n",
            ),
            file(
                "c.rs",
                "#[expect(clippy::x, reason = \"y\")]\nlet s = \"#[allow(x)]\";\n",
            ),
            file("d.ts", "// biome-ignore lint: why\nconst a = 1;\n"),
            file("e.md", "# noqa\n"),
        ];
        let health = measure(&paths(&files), &files);
        assert_eq!(
            health.named["suppressions"],
            ["a.py:1", "b.sh:1", "c.rs:1", "d.ts:1"],
            "a suppression was missed, or a string naming one was counted"
        );
    }

    #[test]
    fn a_terraform_suppression_is_read_in_each_tools_way() {
        let files = [file(
            "main.tf",
            concat!(
                "# tflint-ignore: terraform_unused_declarations\n",
                "// tflint-ignore: all\n",
                "#trivy:ignore:GCP-0072 hosts reach each other\n",
                "# tfsec:ignore:google-compute-no-public-ip\n",
                "description = \"# tflint-ignore: x\"\n",
            ),
        )];
        let health = measure(&paths(&files), &files);
        assert_eq!(
            health.named["suppressions"],
            ["main.tf:1", "main.tf:2", "main.tf:3", "main.tf:4"],
            "a Terraform suppression was missed, or a string naming one was counted"
        );
    }

    #[test]
    fn a_crowded_folder_counts_only_without_its_reason() {
        let mut all: Vec<String> = (0..=CROWDED).map(|n| format!("many/{n}.txt")).collect();
        all.extend((0..=CROWDED).map(|n| format!("said/{n}/x.txt")));
        all.push("said/README.md".to_owned());
        all.extend((0..CROWDED).map(|n| format!("few/{n}.txt")));
        let notes = [file(
            "said/README.md",
            "Many entries because each is a case.\n",
        )];
        let health = measure(&all, &notes);
        assert_eq!(
            health.named["crowded-folders"],
            ["many"],
            "a crowded folder was missed, or one with its reason or few entries was counted"
        );
    }

    #[test]
    fn a_crowded_folders_reason_is_read_anywhere_in_its_note_as_cores_rule_reads_it() {
        let late = format!(
            "{}Many entries because each is a case.\n",
            "A line.\n".repeat(OPENING)
        );
        let notes = [
            file("late/README.md", &late),
            file(
                "wrapped/README.md",
                "Here are cases:\n\n- Many entries\n  because each is one.\n",
            ),
            file("agents/AGENTS.md", "(Many entries because each is one.)\n"),
            file("after/README.md", "Too Many entries becausewords.\n"),
            file("before/README.md", "TooMany entries because each is one.\n"),
            file("under/README.md", "_Many entries because_ each is one.\n"),
        ];
        let mut all = Vec::new();
        for note in &notes {
            let (folder, _) = note.path.split_once('/').unwrap();
            all.extend((0..=CROWDED).map(|n| format!("{folder}/{n}.txt")));
            all.push(note.path.clone());
        }
        let health = measure(&all, &notes);
        assert_eq!(
            health.named["crowded-folders"],
            ["after", "before", "under"],
            "a reason past the opening, wrapped or in AGENTS.md was missed, or one joined to a \
             word was taken"
        );
    }

    #[test]
    fn a_repeated_run_is_counted_once_where_it_repeats() {
        let shared = lines(SAME + 2, "let shared =");
        let files = [
            file("a.rs", &format!("fn a() {{\n{shared}}}\n")),
            file("b.rs", &format!("fn b() {{\n}}\n{shared}")),
            file("c.rs", &lines(SAME * 2, "let own =")),
        ];
        let health = measure(&paths(&files), &files);
        assert!(
            (health.figures["duplicated-lines"] - figure(SAME + 2)).abs() < f64::EPSILON,
            "the repeated lines weren't counted once each: {health:?}"
        );
        assert_eq!(
            health.named["duplicated-lines"],
            ["b.rs:3 repeats a.rs:2"],
            "a repeated run wasn't named once, where it repeats"
        );
    }

    #[test]
    fn each_quarantined_test_is_counted_by_its_name() {
        let files = [file(
            "tools/net/quarantine.json",
            r#"[{"test": "vm-a", "reason": "flaky"}, {"test": "b"}]"#,
        )];
        let health = measure(&paths(&files), &files);
        assert_eq!(
            health.named["quarantined-tests"],
            ["vm-a", "b"],
            "a quarantined test was missed or misnamed"
        );
        assert!(
            (health.figures["quarantined-tests"] - 2.0).abs() < f64::EPSILON,
            "the figure didn't match the tests named: {health:?}"
        );
    }

    #[test]
    fn a_repository_keeping_no_quarantine_has_no_figure_for_it() {
        let files = [file("a.rs", "fn a() {}\n")];
        let health = measure(&paths(&files), &files);
        assert!(
            !health.figures.contains_key("quarantined-tests"),
            "a quarantine nobody keeps was counted as empty: {health:?}"
        );
    }

    #[test]
    fn each_point_a_daemon_waits_on_is_counted_by_its_daemon() {
        let list = "# Waiting.\n{\n  agents = [ \"shown\" ];\n  calls = [\n    \"held\"\n    \"named\"\n  ];\n}\n";
        let files = [file(DAEMONS_WAITING, list)];
        let health = measure(&paths(&files), &files);
        assert_eq!(
            health.named["daemons-waiting"],
            ["agents: shown", "calls: held", "calls: named"],
            "a point a daemon waits on was missed or misnamed"
        );
        assert!(
            (health.figures["daemons-waiting"] - 3.0).abs() < f64::EPSILON,
            "the figure didn't match the points named: {health:?}"
        );
    }

    #[test]
    fn a_repository_holding_no_daemons_to_the_convention_has_no_figure_for_it() {
        let files = [file("a.rs", "fn a() {}\n")];
        let health = measure(&paths(&files), &files);
        assert!(
            !health.figures.contains_key("daemons-waiting"),
            "a daemons list nobody keeps was counted as empty: {health:?}"
        );
    }

    #[test]
    fn a_vm_tests_lines_are_counted_less_blanks_and_comments() {
        let files = [
            file(
                "tests/a-vm-test.py",
                "# Proves a.\nstart_all()\n\nmachine.succeed('a')\n",
            ),
            file("tests/b-vm-test.py", &"machine.succeed('b')\n".repeat(3)),
            file("tests/c_session.py", "x = 1\n"),
        ];
        let health = measure(&paths(&files), &files);
        assert_eq!(
            health.named["vm-test-lines"],
            ["tests/b-vm-test.py: 3 lines", "tests/a-vm-test.py: 2 lines"],
            "a VM test was missed, misread or out of order"
        );
        assert!(
            (health.figures["vm-test-lines"] - 5.0).abs() < f64::EPSILON,
            "the figure didn't add the lines named: {health:?}"
        );
        let none = [file("a.rs", "fn a() {}\n")];
        assert!(
            !measure(&paths(&none), &none)
                .figures
                .contains_key("vm-test-lines"),
            "a repository with no VM tests was counted as keeping them cheap"
        );
    }

    #[test]
    fn the_standards_measures_are_each_read() {
        let files = [
            file("README.md", "# A repository\n"),
            file(
                "biome.json",
                "{ \"linter\": { \"rules\": { \"preset\": \"all\", \"noVoid\": \"off\" } } }\n",
            ),
            file("src/main.rs", "//! The program.\n"),
            file("src/orphan.rs", "\n"),
            file("docs/a.md", "# A\n"),
            file("docs/b.md", "# B\n"),
            file("src/lib.rs", "//! Beside src/words/gone.json.\n"),
            file("src/words/README.md", "# Words\n"),
        ];
        let health = measure(&paths(&files), &files);
        for (measure, named) in [
            ("lints-off", "biome.json: noVoid"),
            ("unreached-files", "src/orphan.rs"),
            ("folders-without-notes", "docs"),
            ("stale-paths", "src/lib.rs:1: src/words/gone.json"),
        ] {
            assert_eq!(health.named[measure], [named], "{measure} misread");
            assert!(
                (health.figures[measure] - 1.0).abs() < f64::EPSILON,
                "{measure}'s figure didn't match what it named: {health:?}"
            );
        }
    }

    #[test]
    fn braces_alone_repeat_nothing() {
        let braces = "}\n".repeat(SAME * 3);
        let files = [file("a.rs", &braces), file("b.rs", &braces)];
        let health = measure(&paths(&files), &files);
        assert!(
            health.figures["duplicated-lines"].abs() < f64::EPSILON,
            "lines of punctuation alone were taken for a duplicate: {health:?}"
        );
    }
}

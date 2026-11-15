//! What repos shows a person, and what it answers a machine, drawn from
//! the record alone (model.rs). Each view is a function of the record, the
//! time now, home, a pen and the room, so its tests set every width
//! without a terminal. A page shows each fact once, in layers, as net's
//! do: the collection as a table, one repository's facts and its
//! checkouts, then every tree cut from it.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use terminal_kit::layout::{self, Align, ITEMS, Room};
use terminal_kit::{Exit, Failure, Pen, flags, time};

use repos::model::{Head, Record, Repository, Tree, TreeKind};
use repos::read;

/// How many trees one repository's page lists, the newest; the rest are
/// `repos NAME trees`'s.
const NEWEST: usize = 5;

/// A count of a thing as a person says it: `1 tree`, `169 trees`.
fn counted(count: usize, thing: &str) -> String {
    match count {
        1 => format!("1 {thing}"),
        _ => format!("{count} {thing}s"),
    }
}

/// How long ago a time was, as a table's column says it, under a label
/// that already says when: `4 m`.
fn age(now: u64, then: Option<u64>) -> String {
    then.map(|then| time::span(now.saturating_sub(then)))
        .unwrap_or_default()
}

/// A path as a person reads it, from home.
fn shown(path: &str, home: Option<&Path>) -> String {
    read::shown(Path::new(path), home)
}

/// A repository's name as the collection shows it: its own, or its owner's
/// and its own where another repository goes by the same name.
fn named(record: &Record, repository: &Repository) -> String {
    let shared = record
        .repositories
        .iter()
        .filter(|other| other.name.eq_ignore_ascii_case(&repository.name))
        .count()
        > 1;
    match (&repository.owner, shared) {
        (Some(owner), true) => format!("{owner}/{}", repository.name),
        _ => repository.name.clone(),
    }
}

/// The trees of a repository, every checkout's, the newest first.
fn trees_of(repository: &Repository) -> Vec<&Tree> {
    let mut trees: Vec<&Tree> = repository
        .checkouts
        .iter()
        .flat_map(|checkout| &checkout.trees)
        .collect();
    trees.sort_by(|a, b| b.written_at.cmp(&a.written_at).then(a.path.cmp(&b.path)));
    trees
}

/// How many of a repository's trees are worktrees and snapshots still on
/// the disk, and how many are gone: `160 worktrees · 9 snapshots · 3 gone`.
fn tree_census(trees: &[&Tree]) -> String {
    let present = |kind: TreeKind| {
        trees
            .iter()
            .filter(|tree| tree.present && tree.kind == kind)
            .count()
    };
    let gone = trees.iter().filter(|tree| !tree.present).count();
    let mut said = vec![
        counted(present(TreeKind::Worktree), "worktree"),
        counted(present(TreeKind::Snapshot), "snapshot"),
    ];
    if gone > 0 {
        said.push(format!("{gone} gone from the disk"));
    }
    said.join(ITEMS)
}

/// The folder most trees still on the disk lie in, which a table of
/// trees says once above itself: the deepest holding more than half of
/// them, so a few cut somewhere else for a moment, as in /tmp, are said
/// whole rather than taking the folder away from the rest. None where
/// they share nothing below the root.
fn shared_folder(trees: &[&Tree]) -> Option<PathBuf> {
    let present: Vec<&Path> = trees
        .iter()
        .filter(|tree| tree.present)
        .map(|tree| Path::new(&tree.path))
        .collect();
    let mut held: std::collections::BTreeMap<&Path, usize> = std::collections::BTreeMap::new();
    for path in &present {
        for folder in path.ancestors().skip(1) {
            *held.entry(folder).or_default() += 1;
        }
    }
    held.into_iter()
        .filter(|(folder, count)| count * 2 > present.len() && folder.components().count() > 2)
        .max_by_key(|(folder, _)| folder.components().count())
        .map(|(folder, _)| folder.to_path_buf())
}

/// Each tree's own name under the shared folder, without what every name
/// beside it repeats: the harness names a tree for its kind, host, owner
/// and repository before its branch, so the part through the repository's
/// name goes, and among names that don't say it, the start every one of
/// their kind shares. A tree outside the folder is said whole.
fn own_names(
    trees: &[&Tree],
    folder: Option<&Path>,
    name: &str,
    home: Option<&Path>,
) -> Vec<String> {
    let through = format!("{name}-");
    let mut names: Vec<(Option<String>, bool)> = trees
        .iter()
        .map(|tree| {
            let path = Path::new(&tree.path);
            let Some(under) = folder.and_then(|folder| path.strip_prefix(folder).ok()) else {
                return (Some(shown(&tree.path, home)), true);
            };
            let under = under.to_string_lossy().into_owned();
            match under.split_once(&through) {
                Some((_, own)) if !own.is_empty() => (Some(own.to_owned()), true),
                _ => (Some(under), false),
            }
        })
        .collect();
    for kind in [TreeKind::Worktree, TreeKind::Snapshot] {
        let of_kind: Vec<usize> = (0..trees.len())
            .filter(|at| trees[*at].kind == kind && !names[*at].1)
            .collect();
        if of_kind.len() < 2 {
            continue;
        }
        let words: Vec<&str> = of_kind
            .iter()
            .filter_map(|at| names[*at].0.as_deref())
            .collect();
        let first = words[0];
        let mut shared = first.len();
        for word in &words[1..] {
            shared = shared.min(
                first
                    .char_indices()
                    .zip(word.chars())
                    .find(|((_, a), b)| a != b)
                    .map_or(first.len().min(word.len()), |((at, _), _)| at),
            );
        }
        let Some(start) = first.get(..shared) else {
            continue;
        };
        let cut = start.rfind(['-', '/']).map_or(0, |at| at + 1);
        if cut == 0 || words.iter().any(|word| word.len() <= cut) {
            continue;
        }
        for at in of_kind {
            if let Some(word) = names[at].0.as_mut() {
                word.replace_range(..cut, "");
            }
        }
    }
    names.into_iter().filter_map(|(word, _)| word).collect()
}

/// A table of trees, under the line saying the folder they share.
fn trees_table(
    trees: &[&Tree],
    all: &[&Tree],
    name: &str,
    now: u64,
    home: Option<&Path>,
    pen: &Pen,
    room: Room,
) -> Vec<String> {
    if trees.is_empty() {
        return Vec::new();
    }
    let folder = shared_folder(all);
    let names = own_names(trees, folder.as_deref(), name, home);
    let mut lines: Vec<String> = folder
        .iter()
        .map(|folder| {
            pen.role(
                "muted",
                &format!("trees under {}", shown(&folder.to_string_lossy(), home)),
            )
        })
        .collect();
    lines.extend(layout::table_in(
        &TREES,
        &tree_rows(trees, &names, now),
        pen,
        room,
        &TREES_GIVE_WAY,
    ));
    lines
}

/// Whether a table's columns all fit the room at their full width.
fn whole(columns: &[(&str, Align)], rows: &[Vec<String>], room: Room) -> bool {
    room.cells().is_none_or(|cells| {
        let widths: usize = (0..columns.len())
            .map(|at| {
                rows.iter()
                    .filter_map(|row| row.get(at))
                    .map(|cell| layout::width(cell))
                    .chain([layout::width(columns[at].0)])
                    .max()
                    .unwrap_or(0)
            })
            .sum();
        widths + layout::GAP * columns.len().saturating_sub(1) <= cells
    })
}

/// The collection's head: how many, on which machine, read from where.
fn head(record: &Record, home: Option<&Path>, pen: &Pen) -> Vec<String> {
    let roots: Vec<String> = record.roots.iter().map(|root| shown(root, home)).collect();
    let roots: Vec<&str> = roots.iter().map(String::as_str).collect();
    vec![format!(
        "{}{}",
        pen.title("repositories"),
        pen.role(
            "muted",
            &format!(
                "{ITEMS}{} on {}{ITEMS}under {}",
                record.repositories.len(),
                record.machine,
                flags::and(&roots)
            )
        )
    )]
}

/// The collection's columns. The key goes first where they don't fit,
/// since the name already says which, and goes whole, since a key cut
/// short names nothing; then the checkouts.
const REPOSITORIES: [(&str, Align); 5] = [
    ("repository", Align::Left),
    ("trees", Align::Right),
    ("checkouts", Align::Right),
    ("written", Align::Right),
    ("key", Align::Left),
];
const KEY: usize = 4;
const REPOSITORIES_GIVE_WAY: [usize; 1] = [2];

/// `repos`: every repository as one line of a table, the one written to
/// last first, and the commands that go further.
pub fn repositories(
    record: &Record,
    now: u64,
    home: Option<&Path>,
    pen: &Pen,
    room: Room,
) -> String {
    let rows: Vec<Vec<String>> = record
        .repositories
        .iter()
        .map(|repository| {
            let trees = trees_of(repository);
            vec![
                named(record, repository),
                trees.iter().filter(|tree| tree.present).count().to_string(),
                repository.checkouts.len().to_string(),
                age(now, repository.written_at),
                repository.key.clone(),
            ]
        })
        .collect();
    let table = if rows.is_empty() {
        vec![pen.role("muted", "no checkout is there")]
    } else if whole(&REPOSITORIES, &rows, room) {
        layout::table_in(&REPOSITORIES, &rows, pen, room, &REPOSITORIES_GIVE_WAY)
    } else {
        let rows: Vec<Vec<String>> = rows.into_iter().map(|row| row[..KEY].to_vec()).collect();
        layout::table_in(
            &REPOSITORIES[..KEY],
            &rows,
            pen,
            room,
            &REPOSITORIES_GIVE_WAY,
        )
    };
    let further = [
        (
            "repos NAME",
            "one repository: its remote, trunk and checkouts",
        ),
        (
            "repos NAME trees",
            "every tree cut from it, the newest first",
        ),
    ]
    .map(|(command, shows)| vec![command.to_owned(), shows.to_owned()]);
    layout::joined(vec![
        head(record, home, pen),
        table,
        layout::table_in(
            &[("further", Align::Left), ("shows", Align::Left)],
            &further,
            pen,
            room,
            &[],
        ),
    ])
}

/// One repository's title: its name, then its key.
fn title(repository: &Repository, pen: &Pen) -> Vec<String> {
    vec![format!(
        "{}{}",
        pen.title(&repository.name),
        pen.role("muted", &format!("{ITEMS}{}", repository.key))
    )]
}

/// The trees' columns: each one's own folder's name, which tells it from
/// the rest and so is never cut, its kind, when it was written, and where
/// it's at, which comes last so a long branch is shortened in its middle
/// rather than sending the rest away. Where it's narrow, the kind goes
/// first, then the time.
const TREES: [(&str, Align); 4] = [
    ("tree", Align::Left),
    ("kind", Align::Left),
    ("written", Align::Right),
    ("at", Align::Left),
];
const TREES_GIVE_WAY: [usize; 2] = [1, 2];

fn tree_rows(trees: &[&Tree], names: &[String], now: u64) -> Vec<Vec<String>> {
    trees
        .iter()
        .zip(names)
        .map(|(tree, name)| {
            let kind = match (tree.present, tree.kind) {
                (false, _) => "gone",
                (true, TreeKind::Worktree) => "worktree",
                (true, TreeKind::Snapshot) => "snapshot",
            };
            vec![
                name.clone(),
                kind.to_owned(),
                age(now, tree.written_at),
                tree.head.said(),
            ]
        })
        .collect()
}

/// The checkouts' columns: each one's folder, where its own files are, its
/// trees still on the disk, as the collection counts them, and when it was
/// written. Where it's narrow, the trees go first.
const CHECKOUTS: [(&str, Align); 4] = [
    ("checkout", Align::Left),
    ("at", Align::Left),
    ("trees", Align::Right),
    ("written", Align::Right),
];
const CHECKOUTS_GIVE_WAY: [usize; 2] = [2, 1];

/// `repos NAME`: one repository in full: its facts, each checkout as a
/// line, and its newest trees.
pub fn repository(
    repository: &Repository,
    now: u64,
    home: Option<&Path>,
    pen: &Pen,
    room: Room,
) -> String {
    let trees = trees_of(repository);
    let remotes: Vec<String> = repository
        .remotes
        .iter()
        .map(|remote| format!("{}{ITEMS}{}", remote.name, remote.url))
        .collect();
    let written = repository
        .written_at
        .map(|then| format!("{} ago", time::span(now.saturating_sub(then))));
    let checkouts = repository.checkouts.len().to_string();
    let census = tree_census(&trees);
    let mut facts: Vec<(&str, Option<&str>)> = if remotes.is_empty() {
        vec![("remote", Some("none, it's only on this machine"))]
    } else {
        remotes
            .iter()
            .map(|remote| ("remote", Some(remote.as_str())))
            .collect()
    };
    facts.extend([
        ("trunk", repository.trunk.as_deref()),
        ("written", written.as_deref()),
        ("checkouts", Some(checkouts.as_str())),
        ("trees", Some(census.as_str())),
    ]);
    let checkout_rows: Vec<Vec<String>> = repository
        .checkouts
        .iter()
        .map(|checkout| {
            let at = checkout
                .head
                .as_ref()
                .map_or_else(|| "bare".to_owned(), Head::said);
            vec![
                shown(&checkout.path, home),
                at,
                checkout
                    .trees
                    .iter()
                    .filter(|tree| tree.present)
                    .count()
                    .to_string(),
                age(now, checkout.written_at),
            ]
        })
        .collect();
    // The page counts what the collection counts, the trees still on the
    // disk, so its newest and its more add up to the same number; the
    // gone ones are `repos NAME trees`'s.
    let present: Vec<&Tree> = trees.iter().copied().filter(|tree| tree.present).collect();
    let newest = &present[..present.len().min(NEWEST)];
    let more = (present.len() > NEWEST).then(|| {
        let name = quoted(&repository.name);
        vec![pen.role(
            "muted",
            &format!("{} more: repos {name} trees", present.len() - NEWEST),
        )]
    });
    layout::joined(vec![
        title(repository, pen),
        layout::facts(&facts, "unknown", pen, room),
        layout::table_in(&CHECKOUTS, &checkout_rows, pen, room, &CHECKOUTS_GIVE_WAY),
        trees_table(newest, &trees, &repository.name, now, home, pen, room),
        more.unwrap_or_default(),
    ])
}

/// `repos NAME trees`: every tree cut from a repository, the newest first.
pub fn trees(
    repository: &Repository,
    now: u64,
    home: Option<&Path>,
    pen: &Pen,
    room: Room,
) -> String {
    let trees = trees_of(repository);
    let census = vec![pen.role("muted", &tree_census(&trees))];
    let table = trees_table(&trees, &trees, &repository.name, now, home, pen, room);
    layout::joined(vec![title(repository, pen), census, table])
}

/// The repository a word names: its key, its name in any case, or its
/// owner and name. Two repositories can share a name, as a fork does its
/// upstream's, and then the word is refused with each one's key to choose
/// by.
pub fn find<'a>(record: &'a Record, word: &str) -> Result<&'a Repository, Failure> {
    let all = &record.repositories;
    if let Some(repository) = all.iter().find(|repository| repository.key == word) {
        return Ok(repository);
    }
    let whose = |repository: &Repository| {
        repository
            .owner
            .as_ref()
            .is_some_and(|owner| format!("{owner}/{}", repository.name).eq_ignore_ascii_case(word))
    };
    if let Some(repository) = all.iter().find(|repository| whose(repository)) {
        return Ok(repository);
    }
    let sharing: Vec<&Repository> = all
        .iter()
        .filter(|repository| repository.name.eq_ignore_ascii_case(word))
        .collect();
    match sharing.as_slice() {
        [repository] => return Ok(repository),
        [first, ..] => {
            let keys: Vec<&str> = sharing
                .iter()
                .map(|repository| repository.key.as_str())
                .collect();
            return Err(Failure::new(
                Exit::Usage,
                format!("{} repositories are called {word}", sharing.len()),
            )
            .because(format!("they're {}", flags::and(&keys)))
            .try_this(format!("repos {}", quoted(&first.key))));
        }
        [] => {}
    }
    let names: Vec<String> = all
        .iter()
        .map(|repository| repository.name.to_lowercase())
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let failure = Failure::new(Exit::NotFound, format!("no repository is called {word}"));
    Err(match flags::nearest(&word.to_lowercase(), &names) {
        Some(near) => failure.try_this(format!("repos {}", quoted(near))),
        None => failure.try_this("repos"),
    })
}

/// A name as a shell takes it as one word.
fn quoted(name: &str) -> String {
    if name.contains(|c: char| c.is_whitespace() || "'\"$`\\".contains(c)) {
        format!("'{}'", name.replace('\'', r"'\''"))
    } else {
        name.to_owned()
    }
}

/// The names of the record's times, which an answer says in RFC 3339
/// (docs/commands.md, The Answer), where the record keeps seconds.
const TIMES: [&str; 2] = ["read_at", "written_at"];

/// A value as an answer says it: each time in RFC 3339.
fn answered(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    let value = match (&value, key.as_str()) {
                        (Value::Number(seconds), key) if TIMES.contains(&key) => seconds
                            .as_i64()
                            .map_or(value.clone(), |seconds| json!(time::rfc3339(seconds))),
                        _ => answered(value),
                    };
                    (key, value)
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(answered).collect()),
        other => other,
    }
}

/// `repos --json`: the whole record, as `repos.repositories/1`.
pub fn repositories_json(record: &Record) -> Value {
    answered(json!(record))
}

/// `repos NAME --json`, as `repos.repository/1`.
pub fn repository_json(record: &Record, repository: &Repository) -> Value {
    answered(json!({
        "schema": "repos.repository/1",
        "read_at": record.read_at,
        "machine": record.machine,
        "repository": repository,
    }))
}

/// `repos NAME trees --json`, as `repos.trees/1`: every tree, the newest
/// first, each naming the checkout it was cut from.
pub fn trees_json(record: &Record, repository: &Repository) -> Value {
    let mut trees: Vec<Value> = repository
        .checkouts
        .iter()
        .flat_map(|checkout| {
            checkout.trees.iter().map(|tree| {
                let mut tree = json!(tree);
                tree["checkout"] = json!(checkout.path);
                tree
            })
        })
        .collect();
    trees.sort_by(|a, b| {
        b["written_at"]
            .as_u64()
            .cmp(&a["written_at"].as_u64())
            .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
    });
    answered(json!({
        "schema": "repos.trees/1",
        "read_at": record.read_at,
        "machine": record.machine,
        "key": repository.key,
        "trees": trees,
    }))
}

#[cfg(test)]
mod tests;

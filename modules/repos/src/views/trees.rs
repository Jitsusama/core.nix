//! The trees of a repository as a person reads them: the folder most of
//! them share said once, each by where it's at and the change open there,
//! its own name only where that says something else, and those gone from
//! the disk after the rest. A repository's page shows its newest this way,
//! and `repos NAME trees` shows every one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use terminal_kit::Pen;
use terminal_kit::layout::{self, Align, ITEMS, Room};

use repos::model::{Change, Head, Repository, Tree, TreeKind};
use repos::when::Clock;

use super::{at, change_name, counted, moment, shown, table, title};

/// The trees of a repository, every checkout's, the newest first.
pub(super) fn trees_of(repository: &Repository) -> Vec<&Tree> {
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
pub(super) fn tree_census(trees: &[&Tree]) -> String {
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

/// The change whose branch a tree is checked out at, if any.
pub(crate) fn change_at<'a>(repository: &'a Repository, tree: &Tree) -> Option<&'a Change> {
    let branch = tree.head.branch.as_deref()?;
    repository
        .changes
        .iter()
        .find(|change| change.branch == branch)
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
    let mut held: BTreeMap<&Path, usize> = BTreeMap::new();
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

/// Whether a tree's own name only says again where it's at: a branch's
/// words, parted at slashes, dashes, dots or underscores, give or take
/// two at either end, as an owner's name before a branch is; or the
/// start of the commit it's pinned to. Such a name is said once, as the
/// branch or commit.
fn restates(name: &str, head: &Head) -> bool {
    let words = |said: &str| -> Vec<String> {
        said.to_lowercase()
            .split(['/', '-', '.', '_'])
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect()
    };
    let name = words(name);
    match (&head.branch, &head.commit) {
        (Some(branch), _) => {
            let branch = words(branch);
            let (shorter, longer) = if name.len() <= branch.len() {
                (&name, &branch)
            } else {
                (&branch, &name)
            };
            !shorter.is_empty()
                && longer.len() - shorter.len() <= 2
                && longer
                    .windows(shorter.len())
                    .any(|run| run == shorter.as_slice())
        }
        (None, Some(commit)) => commit
            .get(..7)
            .is_some_and(|short| name.iter().any(|word| word.contains(short))),
        (None, None) => false,
    }
}

/// The trees' columns: where each is at, its branch or the commit it's
/// pinned to, which names it; the change open at its branch; when it was
/// written; and its own name, only where that says something else, which
/// comes last so it's cut rather than sending the rest away. Where it's
/// narrow, the time goes first, then the change.
const TREES: [(&str, Align); 4] = [
    ("at", Align::Left),
    ("change", Align::Left),
    ("written", Align::Right),
    ("tree", Align::Left),
];
const TREES_GIVE_WAY: [usize; 2] = [2, 1];

/// A table of trees still on the disk, under the line saying the folder
/// they share, then each gone one, muted, as its name and that it's gone.
pub(super) fn trees_table(
    trees: &[&Tree],
    repository: &Repository,
    clock: &Clock,
    home: Option<&Path>,
    pen: &Pen,
    room: Room,
) -> Vec<String> {
    if trees.is_empty() {
        return Vec::new();
    }
    let folder = shared_folder(trees);
    let names = own_names(trees, folder.as_deref(), &repository.name, home);
    let (present, gone): (Vec<_>, Vec<_>) =
        trees.iter().zip(&names).partition(|(tree, _)| tree.present);
    let rows: Vec<Vec<String>> = present
        .iter()
        .map(|(tree, name)| {
            let mut own: Vec<&str> = Vec::new();
            if !restates(name, &tree.head) {
                own.push(name);
            }
            if tree.kind == TreeKind::Snapshot && tree.head.branch.is_some() {
                own.push("snapshot");
            }
            vec![
                at(&tree.head),
                change_at(repository, tree)
                    .map(change_name)
                    .unwrap_or_default(),
                moment(clock, tree.written_at),
                own.join(ITEMS),
            ]
        })
        .collect();
    let mut lines: Vec<String> = folder
        .iter()
        .map(|folder| {
            pen.role(
                "muted",
                &format!("trees under {}", shown(&folder.to_string_lossy(), home)),
            )
        })
        .collect();
    if !rows.is_empty() {
        lines.extend(table(&TREES, rows, &TREES_GIVE_WAY, pen, room));
    }
    let gone: Vec<String> = gone
        .iter()
        .map(|(_, name)| {
            let line = format!("{name}{ITEMS}gone from the disk");
            let line = room
                .cells()
                .map_or_else(|| line.clone(), |cells| layout::cut(&line, cells));
            pen.role("muted", &line)
        })
        .collect();
    layout::joined(vec![lines, gone])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `repos NAME trees`: every tree cut from a repository, the newest first,
/// then those gone from the disk.
pub fn trees(
    repository: &Repository,
    clock: &Clock,
    home: Option<&Path>,
    pen: &Pen,
    room: Room,
) -> String {
    let trees = trees_of(repository);
    let census = vec![pen.role("muted", &tree_census(&trees))];
    let table = trees_table(&trees, repository, clock, home, pen, room);
    layout::joined(vec![title(repository, pen), census, table])
}

#[cfg(test)]
mod tests;

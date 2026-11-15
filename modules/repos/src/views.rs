//! What repos shows a person, and what it answers a machine, drawn from
//! the record alone (model.rs). Each view is a function of the record, a
//! clock, home, a pen and the room, so its tests set every width and the
//! time zone without a terminal. A page shows each fact once, in layers,
//! as net's do: the collection as a table, one repository's facts, its
//! checkouts, its newest changes and trees, then every change or tree.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use terminal_kit::layout::{self, Align, ITEMS, Room};
use terminal_kit::{Exit, Failure, Pen, flags, time};

use repos::github::Seen;
use repos::model::{Change, ChecksState, Head, Record, Repository, Review, Tree, TreeKind};
use repos::read;
use repos::when::Clock;

/// How many trees, and how many changes, one repository's page lists, the
/// newest; the rest are `repos NAME trees`'s and `repos NAME changes`'s.
const NEWEST: usize = 5;

/// A count of a thing as a person says it: `1 tree`, `169 trees`.
fn counted(count: usize, thing: &str) -> String {
    match count {
        1 => format!("1 {thing}"),
        _ => format!("{count} {thing}s"),
    }
}

/// When a time was, as a table's column says it, under a label that
/// already says what happened then: `10:21`, `yesterday 23:40`, `Wed`.
fn moment(clock: &Clock, then: Option<u64>) -> String {
    then.map(|then| clock.moment(then)).unwrap_or_default()
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

/// A table's column: its label and how its cells align.
type Column<'a> = (&'a str, Align);

/// A table's columns, rows and the columns that give way, as `filled`
/// leaves them.
type Filled<'a> = (Vec<Column<'a>>, Vec<Vec<String>>, Vec<usize>);

/// A table's columns without those no row fills, so a column a source
/// left empty, as GitHub's changes where no refresh has run, takes no
/// room. The first column names each record and always stays. The columns
/// that give way are renumbered to match.
fn filled<'a>(columns: &[Column<'a>], rows: Vec<Vec<String>>, gives_way: &[usize]) -> Filled<'a> {
    let kept: Vec<usize> = (0..columns.len())
        .filter(|at| {
            *at == 0
                || rows
                    .iter()
                    .any(|row| row.get(*at).is_some_and(|cell| !cell.is_empty()))
        })
        .collect();
    let place = |at: usize| kept.iter().position(|kept| *kept == at);
    (
        kept.iter().map(|at| columns[*at]).collect(),
        rows.into_iter()
            .map(|row| {
                kept.iter()
                    .map(|at| row.get(*at).cloned().unwrap_or_default())
                    .collect()
            })
            .collect(),
        gives_way.iter().filter_map(|at| place(*at)).collect(),
    )
}

/// A table fitted to the room, without the columns no row fills.
fn table(
    columns: &[(&str, Align)],
    rows: Vec<Vec<String>>,
    gives_way: &[usize],
    pen: &Pen,
    room: Room,
) -> Vec<String> {
    let (columns, rows, gives_way) = filled(columns, rows, gives_way);
    layout::table_in(&columns, &rows, pen, room, &gives_way)
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

/// How a repository's open changes stand: `36 open · 2 failing · 3
/// pending · 4 approved`, each part that's nought giving way.
fn change_census(changes: &[Change]) -> String {
    let checks = |state: ChecksState| {
        changes
            .iter()
            .filter(|change| change.checks.state == state)
            .count()
    };
    let approved = changes
        .iter()
        .filter(|change| change.review == Review::Approved)
        .count();
    let mut said = vec![format!("{} open", changes.len())];
    for (count, what) in [
        (checks(ChecksState::Failing), "failing"),
        (checks(ChecksState::Pending), "pending"),
        (approved, "approved"),
    ] {
        if count > 0 {
            said.push(format!("{count} {what}"));
        }
    }
    said.join(ITEMS)
}

/// Why GitHub couldn't read a repository, where it couldn't.
fn unread<'a>(record: &'a Record, repository: &Repository) -> Option<&'a str> {
    let opening = format!("{}: ", repository.key);
    record
        .sources
        .iter()
        .filter(|source| source.name == "github")
        .flat_map(|source| &source.unread)
        .find_map(|why| why.strip_prefix(&opening))
}

/// A change as a table names it: its number where GitHub holds it, else
/// its branch.
fn change_name(change: &Change) -> String {
    change.github.as_ref().map_or_else(
        || change.branch.clone(),
        |github| format!("#{}", github.number),
    )
}

/// The change whose branch a tree is checked out at, if any.
fn change_at<'a>(repository: &'a Repository, tree: &Tree) -> Option<&'a Change> {
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
fn trees_table(
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
                tree.head.said(),
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

/// The collection's head: how many, on which machine, read from where,
/// and when GitHub was last asked, where it has been.
fn head(record: &Record, clock: &Clock, home: Option<&Path>, pen: &Pen) -> Vec<String> {
    let roots: Vec<String> = record.roots.iter().map(|root| shown(root, home)).collect();
    let roots: Vec<&str> = roots.iter().map(String::as_str).collect();
    let mut lines = vec![format!(
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
    )];
    for github in record
        .sources
        .iter()
        .filter(|source| source.name == "github")
    {
        let read = match github.read_at {
            Some(then) => format!("changes from GitHub {}", clock.at(then)),
            None => "no changes from GitHub".to_owned(),
        };
        let unread = match github.unread.len() {
            0 => String::new(),
            unread => format!("{ITEMS}{unread} it couldn't read"),
        };
        lines.push(pen.role("muted", &(read + &unread)));
    }
    lines
}

/// The collection's columns. The key goes first where they don't fit,
/// since the name already says which, and goes whole, since a key cut
/// short names nothing; then the checkouts, when it was written, and the
/// trees, so the changes stay longest beside the name.
const REPOSITORIES: [(&str, Align); 6] = [
    ("repository", Align::Left),
    ("changes", Align::Right),
    ("trees", Align::Right),
    ("checkouts", Align::Right),
    ("written", Align::Right),
    ("key", Align::Left),
];
const REPOSITORIES_GIVE_WAY: [usize; 3] = [3, 4, 2];

/// A repository's open changes as the collection counts them: `36`, and
/// how many fail where any do, `36 · 2 failing`. Nothing where none are,
/// since a part that's nought gives way; `not read` where GitHub couldn't
/// read it, so the head line's count of those can be found. The kit's
/// table inks every cell alike, so it isn't muted.
fn changes_cell(record: &Record, repository: &Repository) -> String {
    if unread(record, repository).is_some() {
        return "not read".to_owned();
    }
    let changes = &repository.changes;
    let failing = changes
        .iter()
        .filter(|change| change.checks.state == ChecksState::Failing)
        .count();
    match (changes.len(), failing) {
        (0, _) => String::new(),
        (open, 0) => open.to_string(),
        (open, failing) => format!("{open}{ITEMS}{failing} failing"),
    }
}

/// `repos`: every repository as one line of a table, the one written to
/// last first, and the commands that go further.
pub fn repositories(
    record: &Record,
    clock: &Clock,
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
                changes_cell(record, repository),
                trees.iter().filter(|tree| tree.present).count().to_string(),
                repository.checkouts.len().to_string(),
                moment(clock, repository.written_at),
                repository.key.clone(),
            ]
        })
        .collect();
    let table = if rows.is_empty() {
        vec![pen.role("muted", "no checkout is there")]
    } else {
        let (columns, rows, gives_way) = filled(&REPOSITORIES, rows, &REPOSITORIES_GIVE_WAY);
        if whole(&columns, &rows, room) {
            layout::table_in(&columns, &rows, pen, room, &gives_way)
        } else {
            let keyless = columns.len() - 1;
            let rows: Vec<Vec<String>> = rows
                .into_iter()
                .map(|row| row[..keyless].to_vec())
                .collect();
            layout::table_in(&columns[..keyless], &rows, pen, room, &gives_way)
        }
    };
    let further = [
        (
            "repos NAME",
            "one repository: its remote, trunk and checkouts",
        ),
        (
            "repos NAME changes",
            "its open changes, with their checks and review",
        ),
        (
            "repos NAME trees",
            "every tree cut from it, the newest first",
        ),
    ]
    .map(|(command, shows)| vec![command.to_owned(), shows.to_owned()]);
    layout::joined(vec![
        head(record, clock, home, pen),
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

/// The changes' columns: each one's number, how its checks stand, where
/// its review is, when it was updated, and its title, last, so a long one
/// is cut rather than sending the rest away. Where it's narrow, the time
/// goes first, then the review.
const CHANGES: [(&str, Align); 5] = [
    ("change", Align::Left),
    ("checks", Align::Left),
    ("review", Align::Left),
    ("updated", Align::Right),
    ("title", Align::Left),
];
const CHANGES_GIVE_WAY: [usize; 2] = [3, 2];

fn change_rows(changes: &[Change], clock: &Clock) -> Vec<Vec<String>> {
    changes
        .iter()
        .map(|change| {
            let checks = match change.checks.state {
                ChecksState::Passing => "passing".to_owned(),
                ChecksState::Failing => format!("{} failing", change.checks.failed.len()),
                ChecksState::Pending => "pending".to_owned(),
                ChecksState::None => String::new(),
            };
            let review = match (change.draft, change.review) {
                (true, _) => "draft",
                (false, Review::Approved) => "approved",
                (false, Review::ChangesAsked) => "changes asked",
                (false, Review::Waiting) => "waiting",
                (false, Review::None) => "",
            };
            vec![
                change_name(change),
                checks,
                review.to_owned(),
                clock.moment(change.updated_at),
                change.title.clone(),
            ]
        })
        .collect()
}

/// `repos NAME`: one repository in full: its facts, each checkout as a
/// line, and its newest changes and trees.
pub fn repository(
    record: &Record,
    repository: &Repository,
    clock: &Clock,
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
    let written = repository.written_at.map(|then| clock.at(then));
    let checkouts = repository.checkouts.len().to_string();
    let census = tree_census(&trees);
    let changes = match (repository.changes.is_empty(), unread(record, repository)) {
        (false, _) => Some(change_census(&repository.changes)),
        (true, Some(why)) => Some(why.to_owned()),
        (true, None) => None,
    };
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
    ]);
    if let Some(changes) = changes.as_deref() {
        facts.push(("changes", Some(changes)));
    }
    facts.push(("trees", Some(census.as_str())));
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
                moment(clock, checkout.written_at),
            ]
        })
        .collect();
    let name = quoted(&repository.name);
    let more = |count: usize, what: &str| {
        if count > NEWEST {
            let more = format!("{} more: repos {name} {what}", count - NEWEST);
            vec![pen.role("muted", &more)]
        } else {
            Vec::new()
        }
    };
    let newest_changes = &repository.changes[..repository.changes.len().min(NEWEST)];
    // The page counts what the collection counts, the trees still on the
    // disk, so its newest and its more add up to the same number; the
    // gone ones are `repos NAME trees`'s.
    let present: Vec<&Tree> = trees.iter().copied().filter(|tree| tree.present).collect();
    let newest_trees = &present[..present.len().min(NEWEST)];
    layout::joined(vec![
        title(repository, pen),
        layout::facts(&facts, "unknown", pen, room),
        layout::table_in(&CHECKOUTS, &checkout_rows, pen, room, &CHECKOUTS_GIVE_WAY),
        if newest_changes.is_empty() {
            Vec::new()
        } else {
            table(
                &CHANGES,
                change_rows(newest_changes, clock),
                &CHANGES_GIVE_WAY,
                pen,
                room,
            )
        },
        more(repository.changes.len(), "changes"),
        trees_table(newest_trees, repository, clock, home, pen, room),
        more(present.len(), "trees"),
    ])
}

/// `repos NAME changes`: every change open on a repository's remote, the
/// one updated last first.
pub fn changes(
    record: &Record,
    repository: &Repository,
    clock: &Clock,
    pen: &Pen,
    room: Room,
) -> String {
    let census = match (repository.changes.is_empty(), unread(record, repository)) {
        (false, _) => change_census(&repository.changes),
        (true, Some(why)) => why.to_owned(),
        (true, None) if record.sources.iter().any(|source| source.name == "github") => {
            "none open".to_owned()
        }
        (true, None) => "no source has read its changes: repos refresh".to_owned(),
    };
    let table = if repository.changes.is_empty() {
        Vec::new()
    } else {
        table(
            &CHANGES,
            change_rows(&repository.changes, clock),
            &CHANGES_GIVE_WAY,
            pen,
            room,
        )
    };
    layout::joined(vec![
        title(repository, pen),
        vec![pen.role("muted", &census)],
        table,
    ])
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
const TIMES: [&str; 3] = ["read_at", "updated_at", "written_at"];

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

/// `repos refresh --json`: what GitHub was seen to hold, as
/// `repos.github/1`.
pub fn seen_json(seen: &Seen) -> Value {
    answered(json!(seen))
}

/// What a refresh saw, for a person: how much, and what it couldn't.
pub fn seen(seen: &Seen, pen: &Pen) -> String {
    let open: usize = seen.changes.values().map(Vec::len).sum();
    let read = seen.changes.len();
    std::iter::once(format!(
        "GitHub: {open} open changes in {read} repositories"
    ))
    .chain(seen.unread.iter().map(|why| pen.role("muted", why)))
    .map(|line| line + "\n")
    .collect()
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

/// `repos NAME changes --json`, as `repos.changes/1`: every open change,
/// the one updated last first, read when GitHub was last asked.
pub fn changes_json(record: &Record, repository: &Repository) -> Value {
    let github = record.sources.iter().find(|source| source.name == "github");
    answered(json!({
        "schema": "repos.changes/1",
        "read_at": github.and_then(|github| github.read_at),
        "machine": record.machine,
        "key": repository.key,
        "unread": unread(record, repository),
        "changes": repository.changes,
    }))
}

/// `repos NAME trees --json`, as `repos.trees/1`: every tree, the newest
/// first, each naming the checkout it was cut from and the change open at
/// its branch.
pub fn trees_json(record: &Record, repository: &Repository) -> Value {
    let mut trees: Vec<Value> = repository
        .checkouts
        .iter()
        .flat_map(|checkout| {
            checkout.trees.iter().map(|tree| {
                let mut said = json!(tree);
                said["checkout"] = json!(checkout.path);
                if let Some(github) =
                    change_at(repository, tree).and_then(|change| change.github.as_ref())
                {
                    said["change"] = json!(github);
                }
                said
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

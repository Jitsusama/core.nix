//! What repos shows a person, and what it answers a machine, drawn from
//! the record alone (model.rs). Each view is a function of the record, a
//! clock, home, a pen and the room, so its tests set every width and the
//! time zone without a terminal. A page shows each fact once, in layers,
//! as net's do: the collection as a table, one repository's facts, its
//! checkouts, its newest changes and trees, then every change or tree.

use std::path::Path;

use serde_json::{Map, Value, json};
use terminal_kit::failure::shell_word;
use terminal_kit::layout::{self, Align, ITEMS, Room};
use terminal_kit::{Exit, Failure, Pen, flags, time};

use repos::github::Seen;
use repos::model::{
    COUNTED_TO, Change, ChecksState, Head, Record, Repository, Review, Through, Tree, Upstream,
};
use repos::read;
use repos::upstreams::Counted;
use repos::when::Clock;

mod trees;

pub(crate) use trees::change_at;
pub use trees::trees;
use trees::{tree_census, trees_of, trees_table};

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

/// Where a head is, as a table's at column says it: its branch or commit,
/// then how far it is from the branch it follows, where it's apart and
/// that's been counted (`main · 3 behind · 1 ahead`, `100+ behind` past
/// the count's end), or that the branch it follows is gone. A head even
/// with its upstream, or not yet counted, says only where it is.
fn at(head: &Head) -> String {
    let far = |count: u32, way: &str| match count {
        0 => None,
        COUNTED_TO.. => Some(format!("{COUNTED_TO}+ {way}")),
        count => Some(format!("{count} {way}")),
    };
    let apart: Vec<String> = match &head.upstream {
        Some(upstream) if upstream.commit.is_none() => vec!["upstream gone".to_owned()],
        Some(Upstream {
            count: Some(count), ..
        }) => [far(count.behind, "behind"), far(count.ahead, "ahead")]
            .into_iter()
            .flatten()
            .collect(),
        _ => Vec::new(),
    };
    std::iter::once(head.said())
        .chain(apart)
        .collect::<Vec<_>>()
        .join(ITEMS)
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

/// A repository's changes as a page counts them, and the way they were
/// read where it isn't the App's, as most are: `3 open · through gh's
/// sign-in`.
fn with_way(census: String, repository: &Repository) -> String {
    match repository.read_through {
        Some(Through::Gh) => format!("{census}{ITEMS}through gh's sign-in"),
        Some(Through::App) | None => census,
    }
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
    _login: Option<&str>,
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
        (false, _) => Some(with_way(change_census(&repository.changes), repository)),
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
            let at = checkout.head.as_ref().map_or_else(|| "bare".to_owned(), at);
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
    let name = shell_word(&repository.name);
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
        (false, _) => with_way(change_census(&repository.changes), repository),
        (true, Some(why)) => why.to_owned(),
        (true, None) if record.sources.iter().any(|source| source.name == "github") => {
            with_way("none open".to_owned(), repository)
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
            .try_this(format!("repos {}", shell_word(&first.key))));
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
        Some(near) => failure.try_this(format!("repos {}", shell_word(near))),
        None => failure.try_this("repos"),
    })
}

/// The names of the record's times, which an answer says in RFC 3339
/// (docs/commands.md, The Answer), where the record keeps seconds.
const TIMES: [&str; 4] = ["counted_at", "read_at", "updated_at", "written_at"];

/// A value as an answer says it: each time in RFC 3339.
pub(crate) fn answered(value: Value) -> Value {
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

/// `repos count --json`: what the counting counted, as
/// `repos.upstreams/1`.
pub fn counting_json(counted: &Counted) -> Value {
    answered(json!(counted))
}

/// What a counting counted, for a person: how many heads it counted apart
/// from their branches, and how many of those are behind. Heads at the
/// same two commits, as a checkout and its tree can be, count once.
pub fn counting(kept: &Counted) -> String {
    let behind = kept
        .counts
        .values()
        .filter(|count| count.behind > 0)
        .count();
    format!(
        "{} counted apart from their branches{ITEMS}{behind} behind\n",
        counted(kept.counts.len(), "head")
    )
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
        "read_through": repository.read_through,
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
mod apart_tests;
#[cfg(test)]
mod tests;

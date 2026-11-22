//! The rows of Code, the launcher's place for every repository on the
//! machine (docs/glossary.md's Code words), each listed once however many
//! checkouts of it lie on the disk, as `repos` reads them
//! (modules/repos). The one Joel was in last comes first, by zoxide's
//! last visit to a folder of its checkouts or trees (visited.rs), since a
//! list of things returned to moves by his own acts alone and git's
//! writes are mostly agents' (RSCH-20261030-GB56HX); those he hasn't
//! visited follow A to Z. Each is said beside it
//! by how many changes are open on its remote, how far the checkout
//! written to last is behind and ahead of the branch it follows (as repos'
//! timer last counted), how many of its tracked files that checkout has
//! changed and not committed (as the daemon last counted), how many trees
//! are cut from it and when git last wrote in it; it's found by its name,
//! its owner and its key. Return opens its page where a remote keeps it on
//! GitHub or GitLab, else copies its checkout's path; inside are Open Its
//! Page, Open Its Open Changes (while any are), Copy Its Path, New
//! Terminal Here and Edit Here, Forget Gone Trees where git lists trees
//! whose folders are gone and Gather Its Store where upkeep is due
//! (tend.rs), then Pull,
//! which ask this daemon to start kitty in it, with Neovim on it for Edit
//! Here (code.rs), and shut the
//! launcher as they're asked so the new window takes the keys, then, as a row
//! of its own (`inside`), Open a File by Name, which steps into its
//! checkout's files (`files`), each found by its name and the folder it's
//! in and opened by Return in its default program, and Switch Branch,
//! which steps into its checkout's branches (`branches`), each checked out
//! by Return (switch.rs); its drawer names its
//! key, its trunk and the checkout written to last, at its branch.

use ready_rows::order::by_name;
use repos::model::{COUNTED_TO, Checkout, Head, Record, Repository};
use repos::read::shown;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

/// The place these rows are of.
pub const PLACE: &str = "code";

/// The words a repository's row and drawer are said in, from the
/// vocabulary (words/code.json). One the vocabulary hasn't is said by its
/// id, so the rows are served all the same.
#[derive(Clone, Debug)]
pub struct Words {
    /// What a repository is, in its drawer's eyebrow.
    kind: String,
    /// How many trees are cut from it: `1 tree`, `{count} trees`.
    tree: String,
    trees: String,
    /// A repository no remote keeps, in its drawer's eyebrow.
    local: String,
    /// How many changes are open on its remote: `1 change`,
    /// `{count} changes`.
    change: String,
    changes: String,
    /// How far its checkout's branch has fallen behind the branch it
    /// follows: `{count} behind`, and how many commits it has that branch
    /// lacks, not yet pushed: `{count} ahead`.
    behind: String,
    ahead: String,
    /// How many tracked files its checkout has changed and not committed:
    /// `{count} uncommitted`, git's word, since a change is work put up for
    /// review.
    uncommitted: String,
    /// The labels of its drawer's facts.
    key: String,
    trunk: String,
    checkout: String,
    branch: String,
    remote: String,
    /// The row inside a repository that steps into its files, and its
    /// glyph.
    find: String,
    find_glyph: String,
    /// The glyph a file's row wears.
    file_glyph: String,
    /// The row inside a repository that steps into its branches, and its
    /// glyph.
    branches: String,
    branches_glyph: String,
    /// The glyph a branch's row wears, and what its side says of the one
    /// the checkout is on and of one another tree holds: `in {tree}`.
    branch_glyph: String,
    branch_here: String,
    branch_elsewhere: String,
}

impl Words {
    /// Its words from the vocabulary, each it hasn't said by its id.
    #[must_use]
    pub fn read(vocabulary: &Value) -> Words {
        let things: Vec<&Value> = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        let of = |id: &str| things.iter().find(|thing| thing["id"] == id);
        let name = |id: &str| {
            of(id)
                .and_then(|thing| thing["name"].as_str())
                .unwrap_or(id)
                .to_owned()
        };
        let glyph = |id: &str| {
            of(id)
                .and_then(|thing| thing["glyph"].as_str())
                .unwrap_or(id)
                .to_owned()
        };
        let said_of = |id: &str, field: &str| {
            of(id)
                .and_then(|thing| thing["said"][field].as_str())
                .unwrap_or(id)
                .to_owned()
        };
        let said = |field: &str| said_of("code:repository", field);
        Words {
            kind: said("kind"),
            tree: said("tree"),
            trees: said("trees"),
            local: said("local"),
            change: said("change"),
            changes: said("changes"),
            behind: said("behind"),
            ahead: said("ahead"),
            uncommitted: said("uncommitted"),
            key: name("code:key"),
            trunk: name("code:trunk"),
            checkout: name("code:checkout"),
            branch: name("code:branch"),
            remote: name("code:remote"),
            find: name("verb:code:find"),
            find_glyph: glyph("verb:code:find"),
            file_glyph: glyph("code:file"),
            branches: name("verb:code:branches"),
            branches_glyph: glyph("verb:code:branches"),
            branch_glyph: glyph("code:branch"),
            branch_here: said_of("code:branch", "here"),
            branch_elsewhere: said_of("code:branch", "elsewhere"),
        }
    }

    fn counted(&self, count: usize) -> String {
        Words::count(&self.tree, &self.trees, count)
    }

    fn changes_open(&self, count: usize) -> String {
        Words::count(&self.change, &self.changes, count)
    }

    /// How far a head is behind and ahead of the branch it follows, where
    /// it's been counted, each only where it's apart: `100+` past the
    /// count's end. Behind comes first, since it's what a Pull mends.
    fn apart(&self, head: &Head) -> Vec<String> {
        let Some(count) = head.upstream.as_ref().and_then(|upstream| upstream.count) else {
            return Vec::new();
        };
        [(count.behind, &self.behind), (count.ahead, &self.ahead)]
            .into_iter()
            .filter_map(|(far, words)| Words::up_to(words, far))
            .collect()
    }

    /// A count said only where it's above none, as `100+` past the
    /// count's end.
    fn up_to(words: &str, count: u32) -> Option<String> {
        let count = match count {
            0 => return None,
            far if far >= COUNTED_TO => format!("{COUNTED_TO}+"),
            far => far.to_string(),
        };
        Some(words.replace("{count}", &count))
    }

    fn count(one: &str, many: &str, count: usize) -> String {
        if count == 1 {
            one.to_owned()
        } else {
            many.replace("{count}", &count.to_string())
        }
    }
}

/// Every repository's row, the one Joel visited last first by `visited`
/// (zoxide's folders, each with when he was last in it), then those he
/// hasn't A to Z by name; paths are said from `home` as `~`, and
/// `uncommitted` says how many tracked files each checkout counted has
/// changed and not committed, by its path.
#[must_use]
pub fn rows(
    record: &Record,
    home: Option<&Path>,
    words: &Words,
    uncommitted: &BTreeMap<String, u32>,
    visited: &[(String, u64)],
) -> Vec<Value> {
    let last = last_visits(record, visited);
    let mut ordered: Vec<(&Repository, Option<u64>)> = record
        .repositories
        .iter()
        .map(|repository| (repository, last.get(repository.key.as_str()).copied()))
        .collect();
    ordered.sort_by(|(one, one_at), (other, other_at)| {
        other_at
            .cmp(one_at)
            .then_with(|| by_name(&one.name, &other.name))
            .then_with(|| one.key.cmp(&other.key))
    });
    ordered
        .into_iter()
        .map(|(repository, _)| row(repository, home, words, uncommitted))
        .collect()
}

/// When Joel was last in each repository, by its key: each visited folder
/// counts for the repository whose checkout or tree holds it most
/// closely, so a checkout nested in another's folder keeps its own visits.
fn last_visits<'a>(record: &'a Record, visited: &[(String, u64)]) -> BTreeMap<&'a str, u64> {
    let places: Vec<(&str, &str)> = record
        .repositories
        .iter()
        .flat_map(|repository| {
            repository.checkouts.iter().flat_map(move |checkout| {
                std::iter::once(checkout.path.as_str())
                    .chain(checkout.trees.iter().map(|tree| tree.path.as_str()))
                    .map(move |path| (path, repository.key.as_str()))
            })
        })
        .collect();
    let mut last: BTreeMap<&str, u64> = BTreeMap::new();
    for (folder, at) in visited {
        let holder = places
            .iter()
            .filter(|(path, _)| holds(path, folder))
            .max_by_key(|(path, _)| path.len());
        if let Some((_, key)) = holder {
            let latest = last.entry(key).or_default();
            *latest = (*latest).max(*at);
        }
    }
    last
}

/// Whether `folder` is `path` or lies below it: `/s/beta-old` isn't in
/// `/s/beta`, though its name begins the same.
fn holds(path: &str, folder: &str) -> bool {
    let path = path.trim_end_matches('/');
    folder
        .strip_prefix(path)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The rows inside the repositories' rows, served beside them: for each
/// with a checkout, Open a File by Name, which steps into its files, and
/// where it has files of its own, Switch Branch, which steps into its
/// branches.
#[must_use]
pub fn inside(record: &Record, words: &Words) -> Vec<Value> {
    record
        .repositories
        .iter()
        .flat_map(|repository| {
            let checkout = worked_in(repository);
            let find = checkout.map(|checkout| (FILES, &words.find, &words.find_glyph, checkout));
            let switch = checkout
                .filter(|checkout| !checkout.bare)
                .map(|checkout| (BRANCHES, &words.branches, &words.branches_glyph, checkout));
            find.into_iter()
                .chain(switch)
                .map(|(end, name, glyph, checkout)| {
                    let parent = format!("row:{PLACE}:{}", repository.key);
                    json!({
                        "key": format!("{parent}{end}"),
                        // Its own, so the shell tells one repository's apart.
                        "thing": format!("repository:{}{end}", repository.key),
                        "place": PLACE,
                        "kind": "act",
                        "parent": parent,
                        "depth": 1,
                        "name": name,
                        "glyph": glyph,
                        "hue": "action",
                        "side": [],
                        "held": {"path": checkout.path},
                        "acts": [{"key": "verb:more", "step": "rows"}],
                    })
                })
        })
        .collect()
}

/// What ends the key of Open a File by Name's row, after its
/// repository's row's.
pub const FILES: &str = ":files";

/// What ends the key of Switch Branch's row, after its repository's
/// row's.
pub const BRANCHES: &str = ":branches";

/// The rows inside Switch Branch's row of key `parent`, one for each
/// branch `listed` says as switch.rs lists them,
/// `{name, at_s, here, elsewhere, path}`, in its order.
#[must_use]
pub fn branches(parent: &str, listed: &[Value], words: &Words) -> Vec<Value> {
    let repository = parent
        .strip_prefix(&format!("row:{PLACE}:"))
        .and_then(|rest| rest.strip_suffix(BRANCHES))
        .unwrap_or(parent);
    listed
        .iter()
        .filter_map(|one| {
            let name = one["name"].as_str()?;
            let path = one["path"].as_str()?;
            let here = one["here"].as_bool().unwrap_or(false);
            let mut side = Vec::new();
            if here {
                side.push(json!({"said": words.branch_here}));
            } else if let Some(tree) = one["elsewhere"].as_str() {
                side.push(json!({"said": words.branch_elsewhere.replace("{tree}", tree)}));
            }
            if let Some(at) = one["at_s"].as_u64() {
                side.push(json!({"at": at.saturating_mul(1000), "as": "came", "keep": 1}));
            }
            // The one it's on is no choice. One another tree holds keeps
            // its act, so pressing it says which tree (switch.rs).
            let acts = if here {
                json!([])
            } else {
                json!([{
                    "key": "verb:code:switch",
                    "ask": {"ask": "code:switch", "path": path, "branch": name},
                    "streams": true,
                }])
            };
            Some(json!({
                "key": format!("{parent}:{name}"),
                "thing": format!("branch:{repository}:{name}"),
                "place": PLACE,
                "kind": "branch",
                "parent": parent,
                "depth": 2,
                "name": name,
                "glyph": words.branch_glyph,
                "hue": "program",
                "side": side,
                "held": {"path": path, "branch": name},
                "acts": acts,
            }))
        })
        .collect()
}

/// The rows inside Open a File by Name's row of key `parent`, one for each
/// file `listed` says as repositories.rs lists them, `{path, at, within}`,
/// in its order; Return opens one in its default program.
#[must_use]
pub fn files(parent: &str, listed: &[Value], words: &Words) -> Vec<Value> {
    listed
        .iter()
        .filter_map(|one| {
            let path = one["path"].as_str()?;
            let within = one["within"].as_str().unwrap_or_default();
            let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
            let mut side = vec![json!({"at": one["at"], "as": "came"})];
            if !within.is_empty() {
                side.push(json!({"said": within, "keep": 1}));
            }
            Some(json!({
                "key": format!("{parent}:{path}"),
                "thing": format!("file:{path}"),
                "place": PLACE,
                "kind": "file",
                "parent": parent,
                "depth": 2,
                "name": name,
                "glyph": words.file_glyph,
                "hue": "program",
                "side": side,
                "keywords": within,
                "held": {"path": path},
                "acts": [{"key": "verb:code:file:open", "open": path}],
            }))
        })
        .collect()
}

/// Where a repository's page is, from its key, for the forges whose
/// pages are at their host and path; none for any other.
#[must_use]
pub fn page(key: &str) -> Option<String> {
    let (place, path) = key.split_once(':')?;
    let host = match place {
        "github" => "github.com",
        "gitlab" => "gitlab.com",
        _ => return None,
    };
    Some(format!("https://{host}/{path}"))
}

/// Where a repository's open changes are listed on its forge, from its
/// key; none where it has no page.
#[must_use]
pub fn changes_page(key: &str) -> Option<String> {
    let list = match key.split_once(':')?.0 {
        "github" => "pulls",
        "gitlab" => "-/merge_requests",
        _ => return None,
    };
    page(key).map(|page| format!("{page}/{list}"))
}

/// The checkout a person works in: the one written to last with files of
/// its own, else the first.
pub fn worked_in(repository: &Repository) -> Option<&Checkout> {
    repository
        .checkouts
        .iter()
        .find(|checkout| !checkout.bare)
        .or_else(|| repository.checkouts.first())
}

/// Whether a checkout has files of its own at a branch that follows
/// another, which is what Pull brings it up to.
fn follows(checkout: &Checkout) -> bool {
    !checkout.bare
        && checkout
            .head
            .as_ref()
            .is_some_and(|head| head.upstream.is_some())
}

/// Forget Gone Trees and Gather Its Store, each offered only where the
/// checkout read wants it, and asking only of a checkout's own folder, as
/// Pull does, saying in the row's foot how it went (tend.rs).
fn tending(checkout: &Checkout) -> Vec<Value> {
    if checkout.bare {
        return Vec::new();
    }
    let gone = checkout.trees.iter().any(|tree| !tree.present);
    let due = checkout.weight.is_some_and(|weight| weight.due);
    [("forget", gone), ("gather", due)]
        .into_iter()
        .filter(|(_, wanted)| *wanted)
        .map(|(act, _)| {
            json!({
                "key": format!("verb:code:{act}"),
                "ask": {"ask": format!("code:{act}"), "path": checkout.path},
                "streams": true,
            })
        })
        .collect()
}

fn row(
    repository: &Repository,
    home: Option<&Path>,
    words: &Words,
    uncommitted: &BTreeMap<String, u32>,
) -> Value {
    let trees: usize = repository
        .checkouts
        .iter()
        .map(|checkout| checkout.trees.iter().filter(|tree| tree.present).count())
        .sum();
    let mut side = Vec::new();
    // Said first and short, since a row's side gives way past its share.
    if !repository.changes.is_empty() {
        side.push(json!({"said": words.changes_open(repository.changes.len())}));
    }
    let checkout = worked_in(repository);
    if let Some(head) = checkout.and_then(|checkout| checkout.head.as_ref()) {
        side.extend(
            words
                .apart(head)
                .into_iter()
                .map(|said| json!({"said": said})),
        );
    }
    // What a Pull or a Switch would be refused over, after how far apart.
    if let Some(said) = checkout
        .filter(|checkout| !checkout.bare)
        .and_then(|checkout| uncommitted.get(&checkout.path))
        .and_then(|count| Words::up_to(&words.uncommitted, *count))
    {
        side.push(json!({"said": said}));
    }
    if trees > 0 {
        side.push(json!({"said": words.counted(trees)}));
    }
    if let Some(written) = repository.written_at {
        let ms = i64::try_from(written.saturating_mul(1000)).unwrap_or(i64::MAX);
        side.push(json!({"at": ms, "as": "came", "keep": 1}));
    }
    let path = checkout.map(|checkout| checkout.path.clone());
    let paths: Vec<String> = repository
        .checkouts
        .iter()
        .map(|checkout| shown(Path::new(&checkout.path), home))
        .collect();
    let keywords = std::iter::once(repository.key.clone())
        .chain(repository.owner.clone())
        .chain(paths)
        .collect::<Vec<_>>()
        .join(" ");
    let page = page(&repository.key);
    let mut acts = Vec::new();
    if let Some(page) = &page {
        acts.push(json!({"key": "verb:code:page", "open": page}));
    }
    if let Some(list) = changes_page(&repository.key)
        && !repository.changes.is_empty()
    {
        acts.push(json!({"key": "verb:code:changes", "open": list}));
    }
    if let Some(path) = &path {
        acts.push(json!({"key": "verb:code:path", "copy": path}));
        acts.push(json!({
            "key": "verb:code:terminal",
            "ask": {"ask": "code:terminal", "path": path},
            "closes": true,
        }));
        acts.push(json!({
            "key": "verb:code:edit",
            "ask": {"ask": "code:edit", "path": path},
            "closes": true,
        }));
    }
    if let Some(checkout) = checkout {
        acts.extend(tending(checkout));
    }
    // Pull asks only of a checkout's own folder, never a tree cut from it,
    // and says in the row's foot how it went (pull.rs).
    if let Some(checkout) = checkout.filter(|checkout| follows(checkout)) {
        acts.push(json!({
            "key": "verb:code:pull",
            "ask": {"ask": "code:pull", "path": checkout.path},
            "streams": true,
        }));
    }
    json!({
        "key": format!("row:{PLACE}:{}", repository.key),
        "thing": format!("repository:{}", repository.key),
        "place": PLACE,
        "kind": "repository",
        "name": repository.name,
        "glyph": "branch",
        "hue": "program",
        "side": side,
        "keywords": keywords,
        "held": {"key": repository.key, "path": path},
        "acts": acts,
        "drawer": drawer(repository, checkout, home, words),
    })
}

/// A repository's drawer: whose it is, titled by its name, by its key, its
/// trunk and the checkout written to last at its branch, and, folded under
/// the nerd's, the remote its key comes from.
fn drawer(
    repository: &Repository,
    checkout: Option<&Checkout>,
    home: Option<&Path>,
    words: &Words,
) -> Value {
    let owner = repository.owner.as_deref().unwrap_or(&words.local);
    let fact = |label: &str, value: &str| json!({"label": label, "value": value});
    let mut facts = vec![fact(&words.key, &repository.key)];
    if let Some(trunk) = &repository.trunk {
        facts.push(fact(&words.trunk, trunk));
    }
    if let Some(checkout) = checkout {
        facts.push(fact(
            &words.checkout,
            &shown(Path::new(&checkout.path), home),
        ));
        if let Some(head) = checkout.head.as_ref().map(repos::model::Head::said)
            && !head.is_empty()
        {
            facts.push(fact(&words.branch, &head));
        }
    }
    let mut drawer = json!({
        "eyebrow": [
            {"said": owner, "keep": 2},
            {"said": words.kind, "keep": 1},
        ],
        "title": repository.name,
        "body": "",
        "facts": facts,
    });
    if let Some(remote) = repository.remotes.first() {
        drawer["nerd"] = json!([fact(&words.remote, &remote.url)]);
    }
    drawer
}

#[cfg(test)]
mod tests;

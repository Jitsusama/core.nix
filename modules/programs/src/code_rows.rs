//! The rows of Code, the launcher's place for every repository on the
//! machine (docs/glossary.md's Code words), each listed once however many
//! checkouts of it lie on the disk, as `repos` reads them
//! (modules/repos). The one written to last comes first, said beside it
//! by how many trees are cut from it and when git last wrote in it; it's
//! found by its name, its owner and its key. Return opens its page where
//! a remote keeps it on GitHub or GitLab, else copies its checkout's
//! path; inside are Open Its Page, Copy Its Path and New Terminal Here,
//! which asks this daemon to start kitty in it (code.rs), and shuts the
//! launcher as it's asked so the new window takes the keys; its drawer names
//! its key, its trunk and the checkout written to last, at its branch.

use repos::model::{Checkout, Record, Repository};
use repos::read::shown;
use serde_json::{Value, json};
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
    /// The labels of its drawer's facts.
    key: String,
    trunk: String,
    checkout: String,
    branch: String,
    remote: String,
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
        let said = |field: &str| {
            of("code:repository")
                .and_then(|thing| thing["said"][field].as_str())
                .unwrap_or("code:repository")
                .to_owned()
        };
        Words {
            kind: said("kind"),
            tree: said("tree"),
            trees: said("trees"),
            local: said("local"),
            key: name("code:key"),
            trunk: name("code:trunk"),
            checkout: name("code:checkout"),
            branch: name("code:branch"),
            remote: name("code:remote"),
        }
    }

    fn counted(&self, count: usize) -> String {
        if count == 1 {
            self.tree.clone()
        } else {
            self.trees.replace("{count}", &count.to_string())
        }
    }
}

/// Every repository's row, in the record's order, the one written to
/// last first; paths are said from `home` as `~`.
#[must_use]
pub fn rows(record: &Record, home: Option<&Path>, words: &Words) -> Vec<Value> {
    record
        .repositories
        .iter()
        .map(|repository| row(repository, home, words))
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

/// The checkout a person works in: the one written to last with files of
/// its own, else the first.
fn worked_in(repository: &Repository) -> Option<&Checkout> {
    repository
        .checkouts
        .iter()
        .find(|checkout| !checkout.bare)
        .or_else(|| repository.checkouts.first())
}

fn row(repository: &Repository, home: Option<&Path>, words: &Words) -> Value {
    let trees: usize = repository
        .checkouts
        .iter()
        .map(|checkout| checkout.trees.iter().filter(|tree| tree.present).count())
        .sum();
    let mut side = Vec::new();
    if trees > 0 {
        side.push(json!({"said": words.counted(trees)}));
    }
    if let Some(written) = repository.written_at {
        let ms = i64::try_from(written.saturating_mul(1000)).unwrap_or(i64::MAX);
        side.push(json!({"at": ms, "as": "came", "keep": 1}));
    }
    let checkout = worked_in(repository);
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
    if let Some(path) = &path {
        acts.push(json!({"key": "verb:code:path", "copy": path}));
        acts.push(json!({
            "key": "verb:code:terminal",
            "ask": {"ask": "code:terminal", "path": path},
            "closes": true,
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
mod tests {
    use super::*;
    use repos::model::{Head, Remote, SCHEMA, Tree, TreeKind};

    fn words() -> Words {
        Words::read(&json!({"things": [
            {"id": "code:repository", "said": {
                "kind": "repository", "tree": "1 tree", "trees": "{count} trees",
                "local": "no remote",
            }},
            {"id": "code:key", "name": "Key"},
            {"id": "code:trunk", "name": "Trunk"},
            {"id": "code:checkout", "name": "Checkout"},
            {"id": "code:branch", "name": "Branch"},
            {"id": "code:remote", "name": "Remote"},
        ]}))
    }

    fn tree(path: &str, present: bool) -> Tree {
        Tree {
            path: path.to_owned(),
            kind: TreeKind::Worktree,
            head: Head {
                branch: Some("work".to_owned()),
                commit: None,
            },
            present,
            written_at: Some(10),
        }
    }

    fn checkout(path: &str, bare: bool, trees: Vec<Tree>) -> Checkout {
        Checkout {
            path: path.to_owned(),
            bare,
            head: (!bare).then(|| Head {
                branch: Some("main".to_owned()),
                commit: None,
            }),
            written_at: Some(20),
            trees,
        }
    }

    fn core() -> Repository {
        Repository {
            key: "github:Jitsusama/core.nix".to_owned(),
            name: "core.nix".to_owned(),
            owner: Some("Jitsusama".to_owned()),
            remotes: vec![Remote {
                name: "origin".to_owned(),
                url: "git@github.com:Jitsusama/core.nix.git".to_owned(),
            }],
            trunk: Some("main".to_owned()),
            written_at: Some(1_700_000_000),
            checkouts: vec![
                checkout(
                    "/home/j/src/core.nix",
                    false,
                    vec![tree("/t/a", true), tree("/t/b", true), tree("/t/c", false)],
                ),
                checkout("/home/j/old/core.nix", false, vec![tree("/t/d", true)]),
            ],
            changes: Vec::new(),
            read_through: None,
        }
    }

    fn record(repositories: Vec<Repository>) -> Record {
        Record {
            schema: SCHEMA.to_owned(),
            read_at: 0,
            machine: "here".to_owned(),
            roots: Vec::new(),
            sources: Vec::new(),
            repositories,
        }
    }

    #[test]
    fn a_repository_is_one_row_said_by_its_trees_and_when_it_was_written() {
        let rows = rows(&record(vec![core()]), Some(Path::new("/home/j")), &words());
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row["key"], "row:code:github:Jitsusama/core.nix");
        assert_eq!(row["place"], "code");
        assert_eq!(row["name"], "core.nix");
        assert_eq!(
            row["side"],
            json!([{"said": "3 trees"}, {"at": 1_700_000_000_000_i64, "as": "came", "keep": 1}]),
            "a gone tree isn't counted, and every checkout's trees are"
        );
        assert_eq!(
            row["keywords"],
            "github:Jitsusama/core.nix Jitsusama ~/src/core.nix ~/old/core.nix"
        );
    }

    #[test]
    fn return_opens_its_page_and_copy_and_a_terminal_take_the_checkout_written_to_last() {
        let rows = rows(&record(vec![core()]), None, &words());
        assert_eq!(
            rows[0]["acts"],
            json!([
                {"key": "verb:code:page", "open": "https://github.com/Jitsusama/core.nix"},
                {"key": "verb:code:path", "copy": "/home/j/src/core.nix"},
                {"key": "verb:code:terminal", "closes": true,
                 "ask": {"ask": "code:terminal", "path": "/home/j/src/core.nix"}},
            ])
        );
    }

    #[test]
    fn a_repository_with_no_checkout_opens_no_terminal() {
        let mut repository = core();
        repository.checkouts.clear();
        let rows = rows(&record(vec![repository]), None, &words());
        assert_eq!(
            rows[0]["acts"],
            json!([{"key": "verb:code:page", "open": "https://github.com/Jitsusama/core.nix"}]),
            "a terminal was offered with nowhere to open it"
        );
    }

    #[test]
    fn a_repository_with_no_remote_copies_its_path_and_counts_no_trees() {
        let local = Repository {
            key: "local:~/scratch/thing".to_owned(),
            name: "thing".to_owned(),
            owner: None,
            remotes: Vec::new(),
            trunk: None,
            written_at: None,
            checkouts: vec![checkout("/home/j/scratch/thing", false, Vec::new())],
            changes: Vec::new(),
            read_through: None,
        };
        let rows = rows(&record(vec![local]), Some(Path::new("/home/j")), &words());
        let row = &rows[0];
        assert_eq!(row["side"], json!([]), "nothing to say beside it");
        assert_eq!(
            row["acts"],
            json!([
                {"key": "verb:code:path", "copy": "/home/j/scratch/thing"},
                {"key": "verb:code:terminal", "closes": true,
                 "ask": {"ask": "code:terminal", "path": "/home/j/scratch/thing"}},
            ])
        );
        assert_eq!(row["drawer"]["eyebrow"][0]["said"], "no remote");
        assert!(row["drawer"].get("nerd").is_none(), "no remote to name");
    }

    #[test]
    fn a_bare_checkout_is_passed_over_for_one_with_files() {
        let mut repository = core();
        repository
            .checkouts
            .insert(0, checkout("/home/j/src/big.git", true, Vec::new()));
        let rows = rows(&record(vec![repository]), None, &words());
        assert_eq!(rows[0]["held"]["path"], "/home/j/src/core.nix");
    }

    #[test]
    fn its_drawer_names_its_key_trunk_checkout_branch_and_remote() {
        let rows = rows(&record(vec![core()]), Some(Path::new("/home/j")), &words());
        let drawer = &rows[0]["drawer"];
        assert_eq!(drawer["title"], "core.nix");
        assert_eq!(
            drawer["eyebrow"],
            json!([{"said": "Jitsusama", "keep": 2}, {"said": "repository", "keep": 1}])
        );
        assert_eq!(
            drawer["facts"],
            json!([
                {"label": "Key", "value": "github:Jitsusama/core.nix"},
                {"label": "Trunk", "value": "main"},
                {"label": "Checkout", "value": "~/src/core.nix"},
                {"label": "Branch", "value": "main"},
            ])
        );
        assert_eq!(
            drawer["nerd"],
            json!([{"label": "Remote", "value": "git@github.com:Jitsusama/core.nix.git"}])
        );
    }

    #[test]
    fn only_github_and_gitlab_have_pages() {
        assert_eq!(
            page("gitlab:group/sub/thing").as_deref(),
            Some("https://gitlab.com/group/sub/thing")
        );
        assert_eq!(page("git.example.org:thing"), None);
        assert_eq!(page("local:~/x"), None);
    }

    #[test]
    fn one_tree_is_said_alone() {
        let mut repository = core();
        repository.checkouts.truncate(1);
        repository.checkouts[0].trees.truncate(1);
        let rows = rows(&record(vec![repository]), None, &words());
        assert_eq!(rows[0]["side"][0], json!({"said": "1 tree"}));
    }
}

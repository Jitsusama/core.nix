//! The record as the daemon holds it (serve.rs): split by kind, so a watch
//! of one tree is told only that tree moved, and a page lists a kind
//! without the rest. Each record keeps the record's own fields and words,
//! with times in RFC 3339 as every answer says them, and names the records
//! it belongs to by their keys, so a view joins them as it needs.
//!
//! - `reading`, the one record of where it looked and how its sources
//!   stand, keyed by its kind alone. It leaves out when the disk was read,
//!   which moves every reading and would tell every watch nothing new.
//! - `repository:<key>`, each repository once.
//! - `checkout:<path>`, each checkout, naming its repository.
//! - `tree:<path>`, each tree, naming its checkout and its repository, and
//!   the change open at its branch where there is one.
//! - `change:<repository's key>#<number>`, each open change, by the number
//!   its source gives it, else `@<branch>`.

use daemon_serve::asks::Domain;
use serde_json::{Value, json};

use repos::model::{Change, Record, Repository, TreeKind};

use crate::views::{answered, change_at};

/// The daemon as its clients know it.
pub const DOMAIN: Domain = Domain {
    name: "repos",
    kinds: &["reading", "repository", "checkout", "tree", "change"],
    sole: &["reading"],
    named_only: &[],
};

/// Every record the daemon holds of `record`, each repository's after it.
pub fn records(record: &Record) -> Vec<Value> {
    let mut said = vec![reading(record)];
    for repository in &record.repositories {
        said.push(json!({
            "key": format!("repository:{}", repository.key),
            "kind": "repository",
            "name": repository.name,
            "owner": repository.owner,
            "remotes": repository.remotes,
            "trunk": repository.trunk,
            "written_at": repository.written_at,
            "read_through": repository.read_through,
        }));
        for checkout in &repository.checkouts {
            said.push(json!({
                "key": format!("checkout:{}", checkout.path),
                "kind": "checkout",
                "repository": repository.key,
                "path": checkout.path,
                "bare": checkout.bare,
                "head": checkout.head,
                "written_at": checkout.written_at,
                "weight": checkout.weight,
            }));
            for tree in &checkout.trees {
                said.push(json!({
                    "key": format!("tree:{}", tree.path),
                    "kind": "tree",
                    "repository": repository.key,
                    "checkout": checkout.path,
                    "path": tree.path,
                    "snapshot": tree.kind == TreeKind::Snapshot,
                    "present": tree.present,
                    "head": tree.head,
                    "written_at": tree.written_at,
                    "change": change_at(repository, tree).map(|change| change_key(repository, change)),
                }));
            }
        }
        for change in &repository.changes {
            let mut fields = json!(change);
            fields["key"] = json!(change_key(repository, change));
            fields["kind"] = json!("change");
            fields["repository"] = json!(repository.key);
            said.push(fields);
        }
    }
    said.into_iter().map(answered).collect()
}

/// How the sources stand, as health says them: git always read, since a
/// reading is the disk's; GitHub answering once a refresh has asked it,
/// with what it couldn't see as its reason, and not answering when what
/// the refresh kept couldn't be read, since GitHub's answer never came.
pub fn sources(record: &Record) -> Value {
    let github = record.sources.iter().find(|source| source.name == "github");
    let github = match github {
        None => json!({
            "state": "not answering",
            "reason": "no refresh has asked GitHub yet",
        }),
        Some(github) if github.read_at.is_none() => json!({
            "state": "not answering",
            "reason": github.unread.join("; "),
        }),
        Some(github) if github.unread.is_empty() => json!("answering"),
        Some(github) => json!({
            "state": "answering",
            "reason": github.unread.join("; "),
        }),
    };
    json!({"git": "answering", "github": github})
}

/// The one record of where it looked and how its sources stand.
fn reading(record: &Record) -> Value {
    json!({
        "key": "reading",
        "kind": "reading",
        "machine": record.machine,
        "roots": record.roots,
        "sources": record.sources,
    })
}

/// A change's key: its repository's, and the number its source gives it,
/// else the branch it holds.
fn change_key(repository: &Repository, change: &Change) -> String {
    match &change.github {
        Some(github) => format!("change:{}#{}", repository.key, github.number),
        None => format!("change:{}@{}", repository.key, change.branch),
    }
}

#[cfg(test)]
mod tests;

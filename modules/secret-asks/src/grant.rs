//! Standing grants: Joel's yes to one item, for one command, from one
//! repository, lets the same ask from any tree of that repository run
//! for a week without asking him again (ISXEG8's approvals audit, 15
//! November, after Joel asked why he approves everything). A grant never
//! covers another item, another command or another repository, and Asks
//! lists each with Forget It, which revokes it.
//!
//! A command is its words and the program they come to, as the releaser
//! resolves it, so the same words come to another program later are no
//! command he said yes to. And since a single yes is to the tree's code
//! as he saw it, a grant covers a tree only where its code is all on its
//! origin's default branch (ISXEG8, 15 November): an agent's later edit,
//! unseen and unmerged, waits for him as any ask does.
//!
//! Grants are held in the daemon alone, never written to a file: any
//! process of Joel's account could write one, and so grant itself what he
//! never said yes to. A restart forgets them, which costs him one yes
//! each, never a release he didn't give.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use ready_rows::time::{self, Form};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::ask::Ask;
use crate::row::PLACE;
use crate::tree::Checkout;
use crate::words::Words;

/// How long a grant lasts from the yes that made it, in milliseconds.
pub const LASTS_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// The repository a tree's history is kept in, by its git folder, which
/// every worktree of it shares; none where there's no history, or where
/// its `.git` is one git wouldn't vouch for, since a grant to that could
/// be claimed by any tree that names it.
pub fn repository(checkout: &Checkout) -> Option<PathBuf> {
    match checkout {
        Checkout::Own { dot_git } => std::fs::canonicalize(dot_git).ok(),
        Checkout::Worktree { common, .. } => Some(common.clone()),
        Checkout::None | Checkout::Unvouched { .. } => None,
    }
}

/// A repository's name as Joel calls it: the folder its `.git` is in,
/// `core.nix` for `~/src/core.nix/.git`, or its own name, less `.git`,
/// for a bare one.
pub fn named(repository: &Path) -> String {
    let folder = match repository.file_name() {
        Some(name) if name == ".git" => repository.parent().and_then(Path::file_name),
        name => name,
    };
    folder.map_or_else(
        || repository.to_string_lossy().into_owned(),
        |name| {
            let name = name.to_string_lossy();
            name.strip_suffix(".git").unwrap_or(&name).to_owned()
        },
    )
}

/// One grant: what it lets run, from where, and until when.
#[derive(Clone, Debug, PartialEq)]
pub struct Grant {
    /// Its id, the hash of what it covers, so a second yes to the same
    /// renews it rather than adding another.
    pub id: String,
    pub item: String,
    pub argv: Vec<String>,
    /// The program the command's first word came to.
    pub program: PathBuf,
    pub repository: PathBuf,
    pub granted_ms: i64,
}

impl Grant {
    /// When it ends.
    #[must_use]
    pub fn ends_ms(&self) -> i64 {
        self.granted_ms.saturating_add(LASTS_MS)
    }

    /// Whether it covers an ask from a tree of `repository` at `now`,
    /// its command come to `program`: the same item, every word of the
    /// same command, the same program, the same repository.
    fn covers(&self, ask: &Ask, repository: &Path, program: &Path, now: i64) -> bool {
        now < self.ends_ms()
            && self.item == ask.item
            && self.argv == ask.argv
            && self.program == program
            && self.repository == repository
    }
}

/// The id of a grant for what it covers.
fn id_of(item: &str, argv: &[String], program: &Path, repository: &Path) -> String {
    let said = json!([
        item,
        argv,
        program.to_string_lossy(),
        repository.to_string_lossy()
    ])
    .to_string();
    Sha256::digest(said.as_bytes())
        .iter()
        .take(16)
        .fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// The grants held, oldest first.
#[derive(Debug, Default)]
pub struct Grants(Vec<Grant>);

impl Grants {
    /// Grants what a yes to `ask` from a tree kept as `checkout`, its
    /// command come to `program`, covers, from `now`, renewing a grant
    /// for the same; nothing where the tree is in no repository a grant
    /// can name.
    pub fn grant(
        &mut self,
        ask: &Ask,
        checkout: &Checkout,
        program: &Path,
        now: i64,
    ) -> Option<Grant> {
        let repository = repository(checkout)?;
        let id = id_of(&ask.item, &ask.argv, program, &repository);
        self.0.retain(|one| one.id != id);
        let grant = Grant {
            id,
            item: ask.item.clone(),
            argv: ask.argv.clone(),
            program: program.to_owned(),
            repository,
            granted_ms: now,
        };
        self.0.push(grant.clone());
        Some(grant)
    }

    /// The grant covering an ask from a tree kept as `checkout` at `now`,
    /// its command come to `program`, if one does. Whether the tree's
    /// code is on main is the engine's to ask, since only git can say.
    #[must_use]
    pub fn covering(
        &self,
        ask: &Ask,
        checkout: &Checkout,
        program: &Path,
        now: i64,
    ) -> Option<&Grant> {
        let repository = repository(checkout)?;
        self.0
            .iter()
            .find(|one| one.covers(ask, &repository, program, now))
    }

    /// Ends the grant with this id; whether there was one.
    pub fn revoke(&mut self, id: &str) -> bool {
        let before = self.0.len();
        self.0.retain(|one| one.id != id);
        self.0.len() != before
    }

    /// Lets go of each grant ended by `now`; answers when the next ends.
    pub fn tidy(&mut self, now: i64) -> Option<i64> {
        self.0.retain(|one| now < one.ends_ms());
        self.0.iter().map(Grant::ends_ms).min()
    }

    /// Each grant held, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &Grant> {
        self.0.iter()
    }
}

/// A grant's row in Asks: what it lets run and from where, until when,
/// and its one act, Forget It, which Return does. Forgetting a standing
/// yes costs Joel no more than his next yes, so it's served as costing
/// little, as a notice's Forget It is, and the launcher doesn't ask first;
/// as an ending act, Return typed in insert never does it.
pub fn row(grant: &Grant, words: &Words) -> Value {
    let command = crate::row::command(&grant.argv);
    let name = named(&grant.repository);
    let thing = format!("grant:{}", grant.id);
    let note = words.fill("grantNote", &[("command", &command), ("repository", &name)]);
    json!({
        "key": format!("row:{PLACE}:grant:{}", grant.id),
        "thing": thing,
        "place": PLACE,
        "kind": "grant",
        "state": "granted",
        "name": grant.item,
        "glyph": "passwords",
        "hue": "machine",
        "note": note,
        "side": [time::at(grant.ends_ms(), Form::Day)],
        "acts": [{
            "key": "verb:grant:forget",
            "ask": {"ask": "revoke", "key": thing},
            "costly": false,
        }],
        "drawer": {
            "eyebrow": words.fill("granted", &[]),
            "title": grant.item,
            "body": words.fill("grantBody", &[]),
            "facts": [
                {"name": words.fill("fact:runs", &[]), "said": command, "identifies": true},
                {"name": words.fill("fact:from", &[]),
                 "said": words.fill("anyTree", &[("repository", &name)]), "identifies": true},
                {"name": words.fill("fact:granted", &[]),
                 "at": grant.granted_ms, "as": Form::Moment.name()},
                {"name": words.fill("fact:ends", &[]),
                 "at": grant.ends_ms(), "as": Form::Moment.name()},
            ],
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(item: &str, argv: &[&str]) -> Ask {
        Ask {
            id: "3f0c9e2a7b5d41e8a6c2f9d01b7e4a35".to_owned(),
            asked_ms: 1,
            session: "S1".to_owned(),
            quest: None,
            item: item.to_owned(),
            why: "To try the line.".to_owned(),
            argv: argv.iter().map(|word| (*word).to_owned()).collect(),
            tree: "/tree".to_owned(),
            given: "RELEASED".to_owned(),
            minutes: 1,
        }
    }

    /// A worktree of the repository at `common`, as tree.rs reads one.
    fn worktree_of(common: &str) -> Checkout {
        Checkout::Worktree {
            dot_git: PathBuf::from("/wt/.git"),
            common: PathBuf::from(common),
            own: PathBuf::from(format!("{common}/worktrees/wt")),
        }
    }

    const ITEM: &str = "op://Private/Token/credential";
    const PUBLISH: &[&str] = &["npm", "publish"];
    /// The program `npm` comes to, as the releaser resolves it.
    const NPM: &str = "/nix/store/aaaa-nodejs/bin/npm";

    fn npm() -> &'static Path {
        Path::new(NPM)
    }

    fn granted() -> Grants {
        let mut grants = Grants::default();
        grants
            .grant(&ask(ITEM, PUBLISH), &worktree_of("/src/a/.git"), npm(), 0)
            .expect("a worktree's repository is granted");
        grants
    }

    #[test]
    fn a_grant_covers_the_same_ask_from_any_tree_of_its_repository_for_a_week() {
        let grants = granted();
        let same = ask(ITEM, PUBLISH);
        assert!(
            grants
                .covering(&same, &worktree_of("/src/a/.git"), npm(), LASTS_MS - 1)
                .is_some(),
            "the same ask, a moment before the week ends, isn't covered"
        );
        assert!(
            grants
                .covering(&same, &worktree_of("/src/a/.git"), npm(), LASTS_MS)
                .is_none(),
            "a grant outlasted its week"
        );
    }

    #[test]
    fn a_grant_never_covers_another_item_command_or_repository() {
        let grants = granted();
        let here = worktree_of("/src/a/.git");
        for (other, kept) in [
            (ask("op://Private/Other/credential", PUBLISH), here.clone()),
            (
                ask(ITEM, &["npm", "publish", "--tag", "next"]),
                here.clone(),
            ),
            (ask(ITEM, &["npm"]), here.clone()),
            (ask(ITEM, PUBLISH), worktree_of("/src/b/.git")),
        ] {
            assert!(
                grants.covering(&other, &kept, npm(), 0).is_none(),
                "a grant covered {other:?} from {kept:?}"
            );
        }
    }

    #[test]
    fn a_grant_never_covers_the_same_words_come_to_another_program() {
        let grants = granted();
        assert!(
            grants
                .covering(
                    &ask(ITEM, PUBLISH),
                    &worktree_of("/src/a/.git"),
                    Path::new("/nix/store/bbbb-nodejs/bin/npm"),
                    0,
                )
                .is_none(),
            "a grant covered npm come to another program than the yes was to"
        );
    }

    #[test]
    fn a_tree_in_no_repository_or_an_unvouched_one_is_never_granted() {
        let mut grants = Grants::default();
        let one = ask(ITEM, PUBLISH);
        for kept in [
            Checkout::None,
            Checkout::Unvouched {
                dot_git: PathBuf::from("/x/.git"),
            },
        ] {
            assert!(
                grants.grant(&one, &kept, npm(), 0).is_none(),
                "{kept:?} was granted"
            );
            assert!(
                grants.covering(&one, &kept, npm(), 0).is_none(),
                "{kept:?} was covered"
            );
        }
        assert_eq!(grants.iter().count(), 0, "a grant was held");
    }

    #[test]
    fn a_second_yes_renews_its_grant_rather_than_adding_one() {
        let mut grants = granted();
        grants.grant(&ask(ITEM, PUBLISH), &worktree_of("/src/a/.git"), npm(), 5);
        let held: Vec<&Grant> = grants.iter().collect();
        assert_eq!(held.len(), 1, "the same grant was held twice: {held:?}");
        assert_eq!(held[0].granted_ms, 5, "the grant wasn't renewed");
    }

    #[test]
    fn revoking_ends_one_grant_alone() {
        let mut grants = granted();
        grants.grant(&ask(ITEM, PUBLISH), &worktree_of("/src/b/.git"), npm(), 0);
        let id = grants.iter().next().map(|one| one.id.clone());
        assert!(
            grants.revoke(&id.expect("a grant is held")),
            "the revoke found nothing"
        );
        assert!(!grants.revoke("nothing"), "a revoke of nothing found one");
        let left: Vec<PathBuf> = grants.iter().map(|one| one.repository.clone()).collect();
        assert_eq!(left, [PathBuf::from("/src/b/.git")], "the wrong grant went");
    }

    #[test]
    fn tidying_lets_go_of_ended_grants_and_says_when_the_next_ends() {
        let mut grants = granted();
        grants.grant(&ask(ITEM, PUBLISH), &worktree_of("/src/b/.git"), npm(), 10);
        assert_eq!(grants.tidy(0), Some(LASTS_MS), "the next end is wrong");
        assert_eq!(
            grants.tidy(LASTS_MS),
            Some(LASTS_MS + 10),
            "an ended grant stayed"
        );
        assert_eq!(
            grants.tidy(LASTS_MS + 10),
            None,
            "a grant outlived its week"
        );
    }

    #[test]
    fn a_repository_is_named_as_joel_calls_it() {
        assert_eq!(named(Path::new("/home/joel/src/core.nix/.git")), "core.nix");
        assert_eq!(named(Path::new("/srv/git/tools.git")), "tools");
    }
}

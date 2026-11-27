//! A rider's pull request as GitHub lists it: its number, title and head,
//! and whether it's a draft. At 02:58 on 19 November #5645, a draft, rode
//! a candidate, and GitHub refused to merge it only after the two riders
//! before it had merged, so a draft neither boards nor rides.
//!
//! A rider GitHub can't be asked about waits for the next candidate rather
//! than step off: on 26 November a failed `gh` call read as a closed pull
//! request dropped #6748 at 15:19 and #6821 at 16:35, both open and
//! unmoved, each costing its owner another ready and a later landing. It
//! waits at most `WAITS_FOR`, as a GitHub that never answers would keep
//! the train running for it for ever; a commit GitHub says it doesn't
//! hold heads no pull request, and steps off at once.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use super::{Left, Merging, Rider, hub, leave, slug_of, subject};
use crate::state::short;

/// How long a rider waits on a GitHub that can't be asked about it before
/// it steps off, as long as a candidate no builder takes keeps its riders.
pub(super) const WAITS_FOR: u64 = super::UNTAKEN_FOR;

/// An open pull request into main from one of the repository's own
/// branches, as the train reads it.
#[derive(Debug)]
pub struct Pull {
    pub number: u64,
    pub title: String,
    pub head: String,
    /// The branch it merges from, which a merge's subject names.
    pub branch: String,
    /// Whether it's a draft, which GitHub won't merge.
    pub draft: bool,
}

/// What GitHub says of a rider's open pull request: the one it found, or
/// none open, or why it couldn't be asked, which says nothing either way.
pub type Asked = Result<Option<Pull>, String>;

/// The open pull request from `branch` of the repository `slug`.
pub fn pull_of(slug: &str, branch: &str) -> Asked {
    let out = hub::gh(&[
        "pr",
        "list",
        "--repo",
        slug,
        "--head",
        branch,
        "--state",
        "open",
        "--json",
        "number,title,headRefOid,headRepositoryOwner,baseRefName,isDraft",
    ])?;
    pull_in(&out, slug.split('/').next().unwrap_or_default(), branch)
}

/// The pull request of `owner`'s own from `branch` in GitHub's list
/// `listed`. A fork's pull request may name a branch of the same name;
/// only the repository's own are the rider's, and only one into main, as
/// GitHub merges a pull request into its own base.
fn pull_in(listed: &str, owner: &str, branch: &str) -> Asked {
    let pulls = listed_of(listed)?;
    Ok(pulls
        .iter()
        .find(|pull| {
            pull["headRepositoryOwner"]["login"].as_str() == Some(owner)
                && pull["baseRefName"].as_str() == Some("main")
        })
        .and_then(|pull| {
            Some(Pull {
                number: pull["number"].as_u64()?,
                title: pull["title"].as_str()?.to_owned(),
                head: pull["headRefOid"].as_str()?.to_owned(),
                branch: branch.to_owned(),
                draft: pull["isDraft"].as_bool().unwrap_or(false),
            })
        }))
}

/// The open pull request into main of the repository `slug` whose head is
/// `head`, found by its commit. A rider asked of a tree on a local branch
/// with no pull request is recorded under that branch, as #6757 was at
/// 16:05 on 26 November, though its commit still heads one.
pub fn pull_at(slug: &str, head: &str) -> Asked {
    match hub::gh(&["api", &format!("repos/{slug}/commits/{head}/pulls")]) {
        Ok(out) => pull_at_in(&out, slug.split('/').next().unwrap_or_default(), head),
        Err(why) if holds_no_commit(&why) => Ok(None),
        Err(why) => Err(why),
    }
}

/// Whether `gh`'s failure `why` is GitHub saying it holds no such commit,
/// which no pull request can head, rather than GitHub failing to answer.
fn holds_no_commit(why: &str) -> bool {
    why.contains("No commit found for SHA") || why.contains("(HTTP 422)")
}

/// The open pull request of `owner`'s own into main headed by `head`, in
/// GitHub's answer `listed` for that commit's pull requests, which also
/// names closed ones and those that only hold it.
fn pull_at_in(listed: &str, owner: &str, head: &str) -> Asked {
    let pulls = listed_of(listed)?;
    Ok(pulls
        .iter()
        .find(|pull| {
            pull["state"].as_str() == Some("open")
                && pull["head"]["sha"].as_str() == Some(head)
                && pull["head"]["repo"]["owner"]["login"].as_str() == Some(owner)
                && pull["base"]["ref"].as_str() == Some("main")
        })
        .and_then(|pull| {
            Some(Pull {
                number: pull["number"].as_u64()?,
                title: pull["title"].as_str()?.to_owned(),
                head: head.to_owned(),
                branch: pull["head"]["ref"].as_str()?.to_owned(),
                draft: pull["draft"].as_bool().unwrap_or(false),
            })
        }))
}

/// GitHub's answer `listed` as the list it is, or why it isn't one.
fn listed_of(listed: &str) -> Result<Vec<Value>, String> {
    match serde_json::from_str(listed) {
        Ok(Value::Array(pulls)) => Ok(pulls),
        Ok(_) => Err("GitHub's answer wasn't a list".to_owned()),
        Err(error) => Err(format!("GitHub's answer couldn't be read: {error}")),
    }
}

/// The open pull request of `rider`, by its branch and, where its branch
/// heads none, by its commit.
pub(super) fn asked_of(slug: &str, rider: &Rider) -> Asked {
    match pull_of(slug, &rider.branch)? {
        Some(pull) => Ok(Some(pull)),
        None => pull_at(slug, &rider.head),
    }
}

/// What boarding does with a rider at `head`, by its open pull request.
#[derive(Debug, PartialEq, Eq)]
pub enum Boarding {
    Rides,
    /// A draft, which steps off until it's marked ready, for this reason.
    Draft(String),
    /// One whose pull request moved on or closed, which steps off.
    StepsOff,
    /// One GitHub couldn't be asked about, which waits for the next
    /// candidate, for this reason.
    Waits(String),
}

/// What boarding does with the rider at `head`, of whose open pull request
/// GitHub said `asked`. GitHub merges no draft, and refuses one only once
/// the riders before it have merged, so a draft never rides a candidate.
pub fn boarding(asked: &Asked, head: &str) -> Boarding {
    match asked {
        Err(why) => Boarding::Waits(format!(
            "GitHub couldn't say whether its pull request is open ({why})"
        )),
        Ok(Some(pull)) if pull.head != head => Boarding::StepsOff,
        Ok(Some(pull)) if pull.draft => Boarding::Draft(refused_as_draft(pull)),
        Ok(Some(_)) => Boarding::Rides,
        Ok(None) => Boarding::StepsOff,
    }
}

/// The riders a candidate takes, and whether any waited on GitHub or a
/// fetch rather than board or step off, so the train pauses before it
/// asks again.
#[derive(Debug, Default)]
pub struct Aboard {
    pub mergings: Vec<Merging>,
    pub waited: bool,
}

/// Since when each rider has waited on GitHub or a fetch, by its head, in
/// seconds since the epoch, kept across candidates.
pub type Waits = HashMap<String, u64>;

/// How a ride asks of its riders: GitHub of each one's open pull request,
/// `git` in its checkout, and the time, in seconds since the epoch.
pub struct Asking<'a> {
    pub ask: &'a dyn Fn(&Rider) -> Asked,
    pub git: &'a dyn Fn(&[&str]) -> Result<String, String>,
    pub now: u64,
}

/// Whether the rider at `head`, which waits for `why` at `now`, has waited
/// since `waits` first saw it wait for `WAITS_FOR` or longer, saying so
/// as it steps off under `root`; otherwise it waits on.
fn waited_out(root: &Path, waits: &mut Waits, head: &str, why: &str, now: u64) -> bool {
    let since = *waits.entry(head.to_owned()).or_insert(now);
    if now.saturating_sub(since) < WAITS_FOR {
        println!(
            "merge-net: {} waits for the next candidate: {why}",
            short(head)
        );
        return false;
    }
    println!(
        "merge-net: {} steps off the train: it waited {} minutes, and still {why}",
        short(head),
        WAITS_FOR / 60
    );
    waits.remove(head);
    leave(root, head, Left::Off);
    true
}

/// The riders of `waiting` still their pull request's head, under `root`,
/// as the candidate merges them, fetched into the checkout; a draft, or
/// one whose pull request moved on or closed, steps off, and one GitHub or
/// the fetch failed for waits aboard, for `WAITS_FOR` at most, as `waits`
/// keeps. A draft keeps no fell record, which would bar its head from the
/// train once it was marked ready and cost its owner a gate run alone:
/// `ready` refuses it while it's a draft, and boards it again once it
/// isn't.
pub fn aboard(
    root: &Path,
    asking: &Asking,
    owner: &str,
    waiting: &[Rider],
    waits: &mut Waits,
) -> Aboard {
    let mut aboard = Aboard::default();
    for rider in waiting {
        let asked = (asking.ask)(rider);
        match (boarding(&asked, &rider.head), asked) {
            (Boarding::Draft(why), _) => {
                println!(
                    "merge-net: {} steps off the train: {why}",
                    short(&rider.head)
                );
                waits.remove(&rider.head);
                leave(root, &rider.head, Left::Off);
            }
            (Boarding::Waits(why), _) => {
                aboard.waited |= !waited_out(root, waits, &rider.head, &why, asking.now);
            }
            (Boarding::Rides, Ok(Some(pull))) => {
                match (asking.git)(&["fetch", "--quiet", "origin", &rider.head]) {
                    Ok(_) => {
                        waits.remove(&rider.head);
                        aboard.mergings.push(Merging {
                            number: pull.number,
                            head: rider.head.clone(),
                            subject: subject(pull.number, owner, &pull.branch),
                            title: pull.title,
                        });
                    }
                    Err(why) => {
                        let why = format!("its head couldn't be fetched ({why})");
                        aboard.waited |= !waited_out(root, waits, &rider.head, &why, asking.now);
                    }
                }
            }
            _ => {
                println!(
                    "merge-net: {} steps off the train: its pull request moved on or closed",
                    short(&rider.head)
                );
                waits.remove(&rider.head);
                leave(root, &rider.head, Left::Off);
            }
        }
    }
    aboard
}

/// Why the draft `pull` goes no further toward main.
#[must_use]
pub fn refused_as_draft(pull: &Pull) -> String {
    format!(
        "#{} is a draft, which GitHub won't merge, so merge-net doesn't take it to main; \
         mark it ready for review when its owner means it to land, then ask merge-net \
         ready again",
        pull.number
    )
}

/// Why the change checked out on `branch` may not merge as a draft, if its
/// pull request is one; none when it isn't, or GitHub can't say.
#[must_use]
pub fn drafted(branch: &str) -> Option<String> {
    let url = crate::git(&["remote", "get-url", "origin"]).ok()?;
    let pull = pull_of(&slug_of(&url), branch).ok()??;
    pull.draft.then(|| refused_as_draft(&pull))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_draft_and_a_moved_one_step_off_at_boarding() {
        let pull = |head: &str, draft: bool| -> Asked {
            Ok(Some(Pull {
                number: 5645,
                title: "Ours".to_owned(),
                head: head.to_owned(),
                branch: "ours".to_owned(),
                draft,
            }))
        };
        assert_eq!(
            boarding(&pull("abc", false), "abc"),
            Boarding::Rides,
            "an open pull request at the rider's head didn't ride"
        );
        assert!(
            matches!(
                boarding(&pull("abc", true), "abc"),
                Boarding::Draft(why) if why.contains("#5645 is a draft")
            ),
            "a draft rode"
        );
        assert_eq!(
            boarding(&pull("newer", true), "abc"),
            Boarding::StepsOff,
            "a draft that moved on fell for being a draft, not for moving"
        );
        assert_eq!(
            boarding(&Ok(None), "abc"),
            Boarding::StepsOff,
            "a rider with no open pull request stayed aboard"
        );
    }

    // #6821, open at its head, stepped off at 16:35 on 26 November when a
    // call to GitHub failed and its failure read as a closed pull request.
    #[test]
    fn a_rider_github_cant_be_asked_about_waits_rather_than_step_off() {
        let failed: Asked = Err("gh pr: unexpected EOF".to_owned());
        assert!(
            matches!(
                boarding(&failed, "abc"),
                Boarding::Waits(why) if why.contains("unexpected EOF")
            ),
            "a failed call to GitHub didn't keep its rider waiting with GitHub's words"
        );
        assert!(
            pull_in("<html>busy</html>", "owner", "ours").is_err(),
            "an answer that isn't GitHub's list read as no pull request"
        );
        assert!(
            pull_at_in("{\"message\": \"Server Error\"}", "owner", "abc").is_err(),
            "an error object read as no pull request"
        );
    }

    #[test]
    fn a_rider_is_found_by_its_commit_when_its_branch_heads_no_pull_request() {
        let listed = json!([
            {
                "number": 6700,
                "title": "closed",
                "state": "closed",
                "draft": false,
                "head": {"sha": "abc", "ref": "old", "repo": {"owner": {"login": "owner"}}},
                "base": {"ref": "main"},
            },
            {
                "number": 6701,
                "title": "holds it below its head",
                "state": "open",
                "draft": false,
                "head": {"sha": "later", "ref": "above", "repo": {"owner": {"login": "owner"}}},
                "base": {"ref": "main"},
            },
            {
                "number": 6757,
                "title": "Ours",
                "state": "open",
                "draft": false,
                "head": {"sha": "abc", "ref": "theirs", "repo": {"owner": {"login": "owner"}}},
                "base": {"ref": "main"},
            },
        ])
        .to_string();
        let pull = pull_at_in(&listed, "owner", "abc")
            .expect("GitHub's list was read")
            .expect("the open pull request headed by the commit");
        assert_eq!(
            (pull.number, pull.branch.as_str()),
            (6757, "theirs"),
            "the commit's own open pull request wasn't the one found, under its own branch"
        );
        assert!(
            pull_at_in(&listed, "owner", "none").unwrap().is_none(),
            "a commit heading no open pull request found one"
        );
    }

    // An ssh origin read as a path named no repository, so ready's draft
    // check asked GitHub of none and let a draft through.
    #[test]
    fn an_origin_names_its_repository_however_its_reached() {
        for url in [
            "https://github.com/Jitsusama/core.nix.git",
            "https://github.com/Jitsusama/core.nix",
            "git@github.com:Jitsusama/core.nix.git",
            "ssh://git@github.com/Jitsusama/core.nix.git",
            "https://github.com/Jitsusama/core.nix.git/",
        ] {
            assert_eq!(slug_of(url), "Jitsusama/core.nix", "{url}");
        }
    }

    #[test]
    fn a_draft_is_read_as_one_from_githubs_list() {
        let listed = |draft: bool| {
            json!([
                {
                    "number": 9,
                    "title": "a fork's",
                    "headRefOid": "fork",
                    "headRepositoryOwner": {"login": "someone"},
                    "baseRefName": "main",
                    "isDraft": false,
                },
                {
                    "number": 5645,
                    "title": "Ours",
                    "headRefOid": "abc",
                    "headRepositoryOwner": {"login": "owner"},
                    "baseRefName": "main",
                    "isDraft": draft,
                },
            ])
            .to_string()
        };
        let pull = pull_in(&listed(true), "owner", "ours")
            .expect("GitHub's list was read")
            .expect("the repository's own pull request");
        assert_eq!(
            (pull.number, pull.head.as_str(), pull.draft),
            (5645, "abc", true),
            "the repository's own pull request wasn't read as a draft"
        );
        assert!(
            !pull_in(&listed(false), "owner", "ours")
                .unwrap()
                .unwrap()
                .draft,
            "a pull request ready for review was read as a draft"
        );
        assert!(
            refused_as_draft(&pull).contains("draft"),
            "a draft's refusal doesn't say it's a draft"
        );
    }

    #[test]
    fn a_commit_github_doesnt_hold_heads_no_pull_request_and_a_failure_says_nothing() {
        assert!(
            holds_no_commit("gh api: No commit found for SHA: abc (HTTP 422)"),
            "GitHub saying it holds no such commit read as a failure to answer"
        );
        assert!(
            !holds_no_commit("gh api: HTTP 502: Bad Gateway"),
            "a failure to answer read as GitHub holding no such commit"
        );
    }

    fn ours(head: &str) -> Rider {
        Rider {
            head: head.to_owned(),
            branch: format!("jitsusama/{head}"),
            at: 10,
        }
    }

    fn open(head: &str, draft: bool) -> Asked {
        Ok(Some(Pull {
            number: 6821,
            title: "Ours".to_owned(),
            head: head.to_owned(),
            branch: format!("jitsusama/{head}"),
            draft,
        }))
    }

    /// The heads still aboard under `root`.
    fn aboard_in(root: &Path) -> Vec<String> {
        super::super::riders(root)
            .into_iter()
            .map(|rider| rider.head)
            .collect()
    }

    // #6748 and #6821, open and unmoved, stepped off on 26 November when
    // a call to GitHub failed; one GitHub never answers for must not keep
    // the train running for ever either.
    #[test]
    fn a_rider_github_cant_answer_for_waits_until_it_has_waited_out() {
        let root = super::super::tests::root("waits");
        let heads = ["rides", "unasked", "unfetched", "drafted", "closed"];
        for head in heads {
            super::super::board(&root, &ours(head)).unwrap();
        }
        let waiting: Vec<Rider> = heads.iter().map(|&head| ours(head)).collect();
        let ask = |rider: &Rider| match rider.head.as_str() {
            "unasked" => Err("gh pr: unexpected EOF".to_owned()),
            "drafted" => open("drafted", true),
            "closed" => Ok(None),
            head => open(head, false),
        };
        let git = |arguments: &[&str]| {
            if arguments.contains(&"unfetched") {
                Err("fatal: the remote hung up".to_owned())
            } else {
                Ok(String::new())
            }
        };
        let mut waits = Waits::new();
        let at = |now: u64| Asking {
            ask: &ask,
            git: &git,
            now,
        };
        let first = aboard(&root, &at(1000), "jitsusama", &waiting, &mut waits);
        assert_eq!(
            (
                first
                    .mergings
                    .iter()
                    .map(|m| m.head.as_str())
                    .collect::<Vec<_>>(),
                first.waited,
            ),
            (vec!["rides"], true),
            "only the open rider at its head didn't ride, or the train wouldn't pause"
        );
        assert_eq!(
            aboard_in(&root),
            ["rides", "unasked", "unfetched"],
            "a rider GitHub or a fetch failed for stepped off, or a draft or a closed one stayed"
        );
        let still = aboard(
            &root,
            &at(1000 + WAITS_FOR - 1),
            "jitsusama",
            &waiting[..3],
            &mut waits,
        );
        assert!(
            still.waited && aboard_in(&root).len() == 3,
            "a rider stepped off before it had waited out"
        );
        let out = aboard(
            &root,
            &at(1000 + WAITS_FOR),
            "jitsusama",
            &waiting[..3],
            &mut waits,
        );
        assert_eq!(
            (aboard_in(&root), out.waited),
            (vec!["rides".to_owned()], false),
            "a rider that waited out stayed aboard, or the train still paused for it"
        );
        assert!(waits.is_empty(), "a rider that left kept its wait");
        fs::remove_dir_all(&root).unwrap();
    }
}

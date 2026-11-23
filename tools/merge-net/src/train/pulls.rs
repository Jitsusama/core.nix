//! A rider's pull request as GitHub lists it: its number, title and head,
//! and whether it's a draft. At 02:58 on 19 November #5645, a draft, rode
//! a candidate, and GitHub refused to merge it only after the two riders
//! before it had merged, so a draft neither boards nor rides.

use std::path::Path;

use serde_json::Value;

use super::{Left, Merging, Rider, hub, leave, slug_of, subject};
use crate::state::short;

/// An open pull request into main from one of the repository's own
/// branches, as the train reads it.
#[derive(Debug)]
pub struct Pull {
    pub number: u64,
    pub title: String,
    pub head: String,
    /// Whether it's a draft, which GitHub won't merge.
    pub draft: bool,
}

/// The open pull request from `branch` of the repository `slug`.
pub fn pull_of(slug: &str, branch: &str) -> Option<Pull> {
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
    ])
    .ok()?;
    pull_in(&out, slug.split('/').next().unwrap_or_default())
}

/// The pull request of `owner`'s own in GitHub's list `listed`. A fork's
/// pull request may name a branch of the same name; only the repository's
/// own are the rider's, and only one into main, as GitHub merges a pull
/// request into its own base.
fn pull_in(listed: &str, owner: &str) -> Option<Pull> {
    let pulls: Value = serde_json::from_str(listed).ok()?;
    let pull = pulls.as_array()?.iter().find(|pull| {
        pull["headRepositoryOwner"]["login"].as_str() == Some(owner)
            && pull["baseRefName"].as_str() == Some("main")
    })?;
    Some(Pull {
        number: pull["number"].as_u64()?,
        title: pull["title"].as_str()?.to_owned(),
        head: pull["headRefOid"].as_str()?.to_owned(),
        draft: pull["isDraft"].as_bool().unwrap_or(false),
    })
}

/// What boarding does with a rider at `head`, by its open pull request.
#[derive(Debug, PartialEq, Eq)]
pub enum Boarding {
    Rides,
    /// A draft, which steps off until it's marked ready, for this reason.
    Draft(String),
    /// One whose pull request moved on or closed, which steps off.
    StepsOff,
}

/// What boarding does with the rider at `head`, whose open pull request
/// is `open`. GitHub merges no draft, and refuses one only once the
/// riders before it have merged, so a draft never rides a candidate.
pub fn boarding(open: Option<&Pull>, head: &str) -> Boarding {
    match open {
        Some(pull) if pull.head != head => Boarding::StepsOff,
        Some(pull) if pull.draft => Boarding::Draft(refused_as_draft(pull)),
        Some(_) => Boarding::Rides,
        None => Boarding::StepsOff,
    }
}

/// The riders of `waiting` still their pull request's head, under `root`,
/// as the candidate merges them, fetched by `git` into the checkout; a
/// draft, or one whose pull request moved on or closed, steps off. A draft
/// keeps no fell record, which would bar its head from the train once it
/// was marked ready and cost its owner a gate run alone: `ready` refuses
/// it while it's a draft, and boards it again once it isn't.
pub fn aboard(
    root: &Path,
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    slug: &str,
    owner: &str,
    waiting: &[Rider],
) -> Vec<Merging> {
    let mut mergings = Vec::new();
    for rider in waiting {
        let open = pull_of(slug, &rider.branch);
        let fetched = || git(&["fetch", "--quiet", "origin", &rider.head]).is_ok();
        match (boarding(open.as_ref(), &rider.head), open) {
            (Boarding::Draft(why), _) => {
                println!(
                    "merge-net: {} steps off the train: {why}",
                    short(&rider.head)
                );
                leave(root, &rider.head, Left::Off);
            }
            (Boarding::Rides, Some(pull)) if fetched() => mergings.push(Merging {
                number: pull.number,
                head: rider.head.clone(),
                subject: subject(pull.number, owner, &rider.branch),
                title: pull.title,
            }),
            _ => {
                println!(
                    "merge-net: {} steps off the train: its pull request moved on or closed",
                    short(&rider.head)
                );
                leave(root, &rider.head, Left::Off);
            }
        }
    }
    mergings
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
    let pull = pull_of(&slug_of(&url), branch)?;
    pull.draft.then(|| refused_as_draft(&pull))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_draft_and_a_moved_one_step_off_at_boarding() {
        let pull = |head: &str, draft: bool| Pull {
            number: 5645,
            title: "Ours".to_owned(),
            head: head.to_owned(),
            draft,
        };
        assert_eq!(boarding(Some(&pull("abc", false)), "abc"), Boarding::Rides);
        assert!(
            matches!(
                boarding(Some(&pull("abc", true)), "abc"),
                Boarding::Draft(why) if why.contains("#5645 is a draft")
            ),
            "a draft rode"
        );
        assert_eq!(
            boarding(Some(&pull("newer", true)), "abc"),
            Boarding::StepsOff,
            "a draft that moved on fell for being a draft, not for moving"
        );
        assert_eq!(boarding(None, "abc"), Boarding::StepsOff);
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
        let pull = pull_in(&listed(true), "owner").expect("the repository's own pull request");
        assert_eq!(
            (pull.number, pull.head.as_str(), pull.draft),
            (5645, "abc", true),
            "the repository's own pull request wasn't read as a draft"
        );
        assert!(
            !pull_in(&listed(false), "owner").unwrap().draft,
            "a pull request ready for review was read as a draft"
        );
        assert!(
            refused_as_draft(&pull).contains("draft"),
            "a draft's refusal doesn't say it's a draft"
        );
    }
}

//! A rider's pull request as GitHub lists it: its number, title and head,
//! and whether it's a draft. At 02:58 on 19 November #5645, a draft, rode
//! a candidate, and GitHub refused to merge it only after the two riders
//! before it had merged, so a draft neither boards nor rides.

use serde_json::Value;

use super::{hub, slug_of};

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

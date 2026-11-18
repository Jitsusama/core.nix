//! Where the train's passed riders merge: GitHub, through `gh` and its
//! own login, whose merges are signed by the key machines trust main by,
//! or a test's stand-in.

use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use super::{Merging, subject};

/// Where a passed candidate's riders merge: GitHub, whose merges main
/// trusts by its own key, or a test's stand-in.
pub trait Hub {
    /// Merge `merging`'s pull request at its head, worded as it says,
    /// giving the merge's commit.
    ///
    /// # Errors
    ///
    /// Why the merge was refused.
    fn merge(&self, merging: &Merging) -> Result<String, String>;

    /// The merge's commit of pull request `number`, once it reads merged,
    /// as a merge whose answer was lost may have landed all the same;
    /// none while it reads unmerged.
    ///
    /// # Errors
    ///
    /// Why GitHub couldn't be asked, which says nothing either way.
    fn merged(&self, number: u64) -> Result<Option<String>, String>;

    /// The tree GitHub holds at `commit`, so a landing is judged by what
    /// GitHub merged rather than by a fetch that may lag or fail.
    ///
    /// # Errors
    ///
    /// Why GitHub couldn't say.
    fn tree(&self, commit: &str) -> Result<String, String>;

    /// Revert `merging`'s merge on main through a pull request of its
    /// own, merged, giving that pull request's number.
    ///
    /// # Errors
    ///
    /// Why the revert didn't land, naming its pull request if one was
    /// opened and left open.
    fn revert(&self, merging: &Merging) -> Result<u64, String>;
}

/// GitHub, for the repository `slug`, through `gh` and its own login.
pub struct GitHub {
    pub slug: String,
}

impl Hub for GitHub {
    fn merge(&self, merging: &Merging) -> Result<String, String> {
        merge_at(
            &self.slug,
            merging.number,
            &merging.head,
            &merging.subject,
            &merging.title,
        )
    }

    fn merged(&self, number: u64) -> Result<Option<String>, String> {
        let read = gh(&[
            "api",
            &format!("repos/{}/pulls/{number}", self.slug),
            "--jq",
            "select(.merged) | .merge_commit_sha",
        ])?;
        Ok((!read.is_empty()).then_some(read))
    }

    fn tree(&self, commit: &str) -> Result<String, String> {
        gh(&[
            "api",
            &format!("repos/{}/git/commits/{commit}", self.slug),
            "--jq",
            ".tree.sha",
        ])
        .and_then(|tree| {
            if tree.is_empty() {
                Err(format!("GitHub knows no commit {commit}"))
            } else {
                Ok(tree)
            }
        })
    }

    fn revert(&self, merging: &Merging) -> Result<u64, String> {
        let number = merging.number;
        let node = gh(&[
            "api",
            &format!("repos/{}/pulls/{number}", self.slug),
            "--jq",
            ".node_id",
        ])?;
        let opened = gh(&[
            "api",
            "graphql",
            "-f",
            "query=mutation($id: ID!) { revertPullRequest(input: {pullRequestId: $id}) \
             { revertPullRequest { number headRefOid headRefName title } } }",
            "-f",
            &format!("id={node}"),
            "--jq",
            ".data.revertPullRequest.revertPullRequest",
        ])?;
        let opened: Value = serde_json::from_str(&opened)
            .map_err(|error| format!("GitHub's revert of #{number} read as {error}"))?;
        let (Some(reverting), Some(head), Some(branch), Some(title)) = (
            opened["number"].as_u64(),
            opened["headRefOid"].as_str(),
            opened["headRefName"].as_str(),
            opened["title"].as_str(),
        ) else {
            return Err(format!("GitHub opened no revert of #{number}: {opened}"));
        };
        let owner = self.slug.split('/').next().unwrap_or_default();
        let worded = subject(reverting, owner, branch);
        let mut tried = 0;
        loop {
            tried += 1;
            match merge_at(&self.slug, reverting, head, &worded, title) {
                Ok(_) => return Ok(reverting),
                Err(error) if tried >= REVERT_TRIES => {
                    return Err(format!(
                        "GitHub's revert, pull request #{reverting}, stays open: {error}"
                    ));
                }
                // GitHub works out whether a pull request just opened can
                // merge a moment after it opens, refusing it until then.
                Err(_) => std::thread::sleep(Duration::from_secs(2)),
            }
        }
    }
}

/// How many times a revert's pull request is asked to merge.
const REVERT_TRIES: u32 = 6;

/// Merge pull request `number` of `slug` at `head` through GitHub's merge,
/// as `subject` and `title` word it, giving the merge's commit.
fn merge_at(
    slug: &str,
    number: u64,
    head: &str,
    subject: &str,
    title: &str,
) -> Result<String, String> {
    gh(&[
        "api",
        "-X",
        "PUT",
        &format!("repos/{slug}/pulls/{number}/merge"),
        "-f",
        &format!("sha={head}"),
        "-f",
        "merge_method=merge",
        "-f",
        &format!("commit_title={subject}"),
        "-f",
        &format!("commit_message={title}"),
        "--jq",
        ".sha",
    ])
}

/// What `gh` says, or why it failed, given two minutes, as a stalled
/// call would otherwise hold the train between merges.
fn gh(arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new("timeout")
        .arg("120")
        .arg("gh")
        .args(arguments)
        .output()
        .map_err(|error| format!("running gh: {error}"))?;
    if ran.status.success() {
        Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
    } else {
        Err(format!(
            "gh {}: {}",
            arguments.first().copied().unwrap_or_default(),
            String::from_utf8_lossy(&ran.stderr).trim()
        ))
    }
}

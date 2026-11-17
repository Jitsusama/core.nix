//! Where the train's passed riders merge: GitHub, through `gh` and its
//! own login, whose merges are signed by the key machines trust main by,
//! or a test's stand-in.

use std::process::Command;

use serde_json::Value;

use super::Merging;

/// Where a passed candidate's riders merge: GitHub, whose merges main
/// trusts by its own key, or a test's stand-in.
pub trait Hub {
    /// Merge `merging`'s pull request at its head, worded as it says.
    ///
    /// # Errors
    ///
    /// Why the merge was refused.
    fn merge(&self, merging: &Merging) -> Result<(), String>;

    /// Revert pull request `number`'s merge on main.
    ///
    /// # Errors
    ///
    /// Why the revert didn't land.
    fn revert(&self, number: u64) -> Result<(), String>;
}

/// GitHub, for the repository `slug`, through `gh` and its own login.
pub struct GitHub {
    pub slug: String,
}

impl Hub for GitHub {
    fn merge(&self, merging: &Merging) -> Result<(), String> {
        merge_at(
            &self.slug,
            merging.number,
            &merging.head,
            &merging.subject,
            &merging.title,
        )
    }

    fn revert(&self, number: u64) -> Result<(), String> {
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
             { revertPullRequest { number headRefOid title } } }",
            "-f",
            &format!("id={node}"),
            "--jq",
            ".data.revertPullRequest.revertPullRequest",
        ])?;
        let opened: Value = serde_json::from_str(&opened)
            .map_err(|error| format!("GitHub's revert of #{number} read as {error}"))?;
        let (Some(reverting), Some(head), Some(title)) = (
            opened["number"].as_u64(),
            opened["headRefOid"].as_str(),
            opened["title"].as_str(),
        ) else {
            return Err(format!("GitHub opened no revert of #{number}: {opened}"));
        };
        merge_at(
            &self.slug,
            reverting,
            head,
            &format!("Revert #{number}, whose merge the train didn't test"),
            title,
        )
    }
}

/// Merge pull request `number` of `slug` at `head` through GitHub's merge,
/// as `subject` and `title` word it.
fn merge_at(slug: &str, number: u64, head: &str, subject: &str, title: &str) -> Result<(), String> {
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
    ])
    .map(|_| ())
}

/// What `gh` says, or why it failed.
fn gh(arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new("gh")
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

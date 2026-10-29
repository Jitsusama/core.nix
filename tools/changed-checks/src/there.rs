//! Building the checks on the builder nix-offload lends, by sending it a
//! command rather than paths.
//!
//! The build hook builds a check's inputs here and copies each up, and a
//! VM test took 1025 s that way against 60 s here, nearly all of it the
//! upload. So the builder is given a command naming a commit origin has,
//! and it fetches the commit and every input, evaluates and builds there.
//! Only the command goes up, and the log comes down; evaluation, the
//! heaviest part of a check here, leaves this machine too.
//!
//! `nix-offload there` sends it, since nix-offload owns how its builder is
//! reached; a machine that lends a builder has it, since the module that
//! writes the store file installs it.

use std::fs;
use std::path::Path;
use std::process::Command;

/// How a build there ended, and what it sent the builder.
pub struct Sent {
    pub passed: bool,
    /// Whether it ran on the builder, rather than here when none answered.
    pub there: bool,
    /// What ssh sent, as nix-offload journalled it.
    pub bytes: Option<u64>,
}

/// Whether a builder is lent: the first line of `store` in nix-offload's
/// folder names an ssh store while one stands.
pub fn lent(offload: &Path) -> bool {
    fs::read_to_string(offload.join("store")).is_ok_and(|named| {
        named
            .lines()
            .next()
            .is_some_and(|line| line.starts_with("ssh-ng://") || line.starts_with("ssh://"))
    })
}

/// The flake a builder fetches a commit from origin's URL by: GitHub's
/// archive where origin is there, since it needs no git and no API, and
/// otherwise git over the URL.
pub fn fetched(url: &str, commit: &str) -> String {
    let on_github = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"));
    if let Some(path) = on_github {
        let path = path.trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        return format!("github:{path}/{commit}");
    }
    if url.contains("://") {
        format!("git+{url}?rev={commit}")
    } else {
        // git's short form, `user@host:path`, is ssh.
        let (host, path) = url.split_once(':').unwrap_or((url, ""));
        format!("git+ssh://{host}/{path}?rev={commit}")
    }
}

/// Build `checks` of `flake` for `system` with `nix-offload there`, its
/// log coming here as it goes, and read what it journalled in `journal`.
pub fn build(flake: &str, system: &str, checks: &[&str], journal: &Path) -> Result<Sent, String> {
    eprintln!("changed-checks: building on the lent builder, {flake}");
    let status = Command::new("nix-offload")
        .args(["there", "-L", "--keep-going", flake])
        .args(
            checks
                .iter()
                .map(|check| format!("checks.{system}.{check}")),
        )
        .status()
        .map_err(|error| format!("running nix-offload: {error}"))?;
    // It refuses what it can't read as 64, its usage's status, which no
    // build gives; one older than `there` refuses it so.
    if status.code() == Some(USAGE) {
        return Err("nix-offload couldn't take the build there".to_owned());
    }
    let noted = fs::read_to_string(journal)
        .ok()
        .and_then(|journal| last_of(&journal, flake));
    Ok(Sent {
        passed: status.success(),
        there: noted.as_ref().is_some_and(|line| line["on"].is_string()),
        bytes: noted.and_then(|line| line["sent"].as_u64()),
    })
}

/// The status nix-offload refuses its arguments with.
const USAGE: i32 = 64;

/// The journal's last line for a build of `flake`.
fn last_of(journal: &str, flake: &str) -> Option<serde_json::Value> {
    journal.lines().rev().find_map(|line| {
        serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .filter(|line| line["flake"] == flake)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_builder_is_lent_while_the_store_names_an_ssh_store() {
        let folder = std::env::temp_dir().join(format!("there-lent-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let _ = fs::remove_file(folder.join("store"));
        assert!(!lent(&folder));
        fs::write(folder.join("store"), "").expect("written");
        assert!(!lent(&folder));
        fs::write(folder.join("store"), "ssh-ng://far?ssh-key=/k\n").expect("written");
        assert!(lent(&folder));
        fs::remove_dir_all(&folder).expect("removed");
    }

    #[test]
    fn a_commit_is_fetched_from_githubs_archive_or_else_by_git() {
        let commit = "fedcba98";
        for url in [
            "https://github.com/someone/thing.git",
            "https://github.com/someone/thing",
            "git@github.com:someone/thing.git",
            "ssh://git@github.com/someone/thing.git",
        ] {
            assert_eq!(
                fetched(url, commit),
                "github:someone/thing/fedcba98",
                "{url}"
            );
        }
        assert_eq!(
            fetched("https://git.example/thing.git", commit),
            "git+https://git.example/thing.git?rev=fedcba98"
        );
        assert_eq!(
            fetched("me@git.example:thing.git", commit),
            "git+ssh://me@git.example/thing.git?rev=fedcba98"
        );
    }

    #[test]
    fn what_was_sent_is_read_from_the_last_build_of_the_flake() {
        let journal = [
            r#"{"flake":"github:o/r/a","on":"far","sent":1}"#,
            r#"{"flake":"github:o/r/b","on":"far","sent":2}"#,
            r#"{"flake":"github:o/r/a","on":"far","sent":3}"#,
        ]
        .join("\n");
        assert_eq!(last_of(&journal, "github:o/r/a").expect("found")["sent"], 3);
        assert!(last_of(&journal, "github:o/r/c").is_none());
    }
}

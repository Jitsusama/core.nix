//! The inputs a flake's lock takes from this machine alone, and how a
//! flake holding some still builds on a lent builder.
//!
//! Such an input is a store path or a repository here, which a builder
//! can't fetch. Until 10 November a flake holding one was never sent, so
//! the home zone's machine, whose lock names mint as a store path and the
//! machine's secrets as a `git+file:` repository, took about 25 minutes
//! and several GB to evaluate here for each change, at a load of 100.
//! Nix fetches an input only once an evaluation reads it, though, and the
//! secrets are read by another machine's outputs alone. So a store path
//! that's no local checkout goes to the builder ahead of the build, a
//! repository or folder here never goes, and an evaluation there that
//! fails for want of one comes back as [`KEPT`], to be made here.

/// What a build sent far exits with when it failed naming an input kept
/// on this machine, sysexits.h's data error, so the caller makes it here
/// rather than taking the failure as the change's own.
pub const KEPT: i32 = 65;

/// The inputs a lock takes from this machine alone, sorted: the store
/// paths that go to the builder ahead of the build, and the inputs kept
/// here, each named with what an error reading it would say, its path.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Inputs {
    pub sent: Vec<String>,
    pub kept: Vec<(String, String)>,
}

impl Inputs {
    /// The names of those kept here, for saying.
    pub fn kept_names(&self) -> String {
        self.kept
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Sorts what `lock` takes from this machine alone: a store path or a
/// repository at a `file:` URL, as the lock pins it. What decides is where
/// it came from, its `original`, since a local checkout locks as the store
/// path it was copied to: one is sent only when that original is itself a
/// store path or is fetched from elsewhere. Anything naming `.secrets`,
/// in either, or whose own name holds `secrets`, stays, whatever its form,
/// as do inputs with no original.
pub fn inputs_of(lock: &str) -> Inputs {
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(lock) else {
        return Inputs::default();
    };
    let mut inputs = Inputs::default();
    let nodes = lock["nodes"].as_object().into_iter().flatten();
    for (name, node) in nodes {
        let locked = &node["locked"];
        let locked_path = locked["path"].as_str().unwrap_or_default();
        let here = locked["type"] == "path" || file_url(&locked["url"]);
        if !here {
            continue;
        }
        let original = &node["original"];
        // Named so, a store copy of secrets can say nothing else of it: its
        // path and original are a bare `/nix/store/<hash>-source`.
        let secret = name.contains("secrets")
            || [locked, original]
                .iter()
                .flat_map(|said| [&said["path"], &said["url"]])
                .filter_map(serde_json::Value::as_str)
                .any(|text| text.contains(".secrets"));
        let from_store =
            original["type"] == "path" && is_store(original["path"].as_str().unwrap_or_default());
        // Only a forge's own kind names nowhere here: a git, tarball or
        // file original may be a URL on this machine.
        let from_elsewhere = original["type"]
            .as_str()
            .is_some_and(|kind| FORGES.contains(&kind));
        if !secret && is_store(locked_path) && (from_store || from_elsewhere) {
            inputs.sent.push(locked_path.to_owned());
        } else {
            inputs.kept.push((name.clone(), where_of(original, locked)));
        }
    }
    inputs.sent.sort();
    inputs.sent.dedup();
    inputs.kept.sort();
    inputs
}

/// The kinds of flake reference that always name a forge elsewhere.
const FORGES: [&str; 3] = ["github", "gitlab", "sourcehut"];

fn is_store(path: &str) -> bool {
    path.starts_with("/nix/store/")
}

fn file_url(url: &serde_json::Value) -> bool {
    url.as_str().is_some_and(|url| url.starts_with("file:"))
}

/// The path an error reading an input names: its original's folder or
/// repository, as Nix would look for it, else what it locks to.
fn where_of(original: &serde_json::Value, locked: &serde_json::Value) -> String {
    [original, locked]
        .iter()
        .flat_map(|said| [&said["path"], &said["url"]])
        .filter_map(serde_json::Value::as_str)
        .map(|text| {
            let text = text.split('?').next().unwrap_or(text);
            text.trim_start_matches("git+")
                .trim_start_matches("file://")
                .to_owned()
        })
        .find(|text| !text.is_empty())
        .unwrap_or_default()
}

/// `command` as a builder runs it for a flake holding inputs `kept` here:
/// what it says on its error is passed on as it comes and read once it
/// fails, and a failure naming one of them ends as [`KEPT`]. Its standard
/// output passes untouched, as `--out`'s tarball goes.
pub fn wrapped(command: &str, kept: &[(String, String)]) -> String {
    let needles: String = kept.iter().filter(|(_, path)| !path.is_empty()).fold(
        String::new(),
        |mut needles, (_, path)| {
            needles.push_str(" -e ");
            needles.push_str(&quoted(path));
            needles
        },
    );
    if needles.is_empty() {
        return command.to_owned();
    }
    format!(
        "kept=\"${{TMPDIR:-/tmp}}/nix-offload-kept-$$\"; exec 3>&1; \
         {{ ( {command} ); echo $? > \"$kept.status\"; }} 2>&1 1>&3 3>&- \
         | tee \"$kept.said\" >&2; \
         ended=$(cat \"$kept.status\" 2>/dev/null || echo 1); \
         if [ \"$ended\" != 0 ] && grep -qF{needles} \"$kept.said\"; then ended={KEPT}; fi; \
         rm -f \"$kept.status\" \"$kept.said\"; (exit \"$ended\")"
    )
}

/// A word as a POSIX shell reads it, quoted.
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(lock: &str) -> Vec<String> {
        inputs_of(lock).sent
    }

    fn kept(lock: &str) -> Vec<String> {
        inputs_of(lock)
            .kept
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    #[test]
    fn a_store_path_named_as_one_goes_to_the_builder() {
        let lock = r#"{"nodes": {
            "mint": {"locked": {"type": "path", "path": "/nix/store/abc-mint"},
                     "original": {"type": "path", "path": "/nix/store/abc-mint"}},
            "core": {"locked": {"type": "github", "owner": "o"},
                     "original": {"type": "github", "owner": "o"}},
            "root": {"inputs": {}}
        }}"#;
        assert_eq!(sent(lock), ["/nix/store/abc-mint"]);
        assert_eq!(kept(lock), Vec::<String>::new());
    }

    #[test]
    fn secrets_as_a_repository_or_as_its_store_copy_never_go() {
        let lock = r#"{"nodes": {
            "secrets": {"locked": {"type": "git", "url": "file:///Users/me/.config/nix/.secrets",
                                   "rev": "f42c"},
                        "original": {"type": "git", "url": "file:///Users/me/.config/nix/.secrets"}},
            "copied": {"locked": {"type": "path", "path": "/nix/store/def-source"},
                       "original": {"type": "git", "url": "file:///home/me/.secrets"}},
            "named": {"locked": {"type": "path", "path": "/nix/store/ghi-.secrets"},
                      "original": {"type": "path", "path": "/nix/store/ghi-.secrets"}},
            "home-secrets": {"locked": {"type": "path", "path": "/nix/store/jkl-source"},
                             "original": {"type": "path", "path": "/nix/store/jkl-source"}}
        }}"#;
        assert_eq!(sent(lock), Vec::<String>::new());
        assert_eq!(kept(lock), ["copied", "home-secrets", "named", "secrets"]);
    }

    #[test]
    fn a_local_checkout_locked_to_the_store_stays() {
        let lock = r#"{"nodes": {
            "near": {"locked": {"type": "path", "path": "/nix/store/jkl-near"},
                     "original": {"type": "path", "path": "/home/me/near"}},
            "bare": {"locked": {"type": "path", "path": "/nix/store/mno-bare"}},
            "fetched": {"locked": {"type": "path", "path": "/nix/store/pqr-fetched"},
                        "original": {"type": "github", "owner": "o", "repo": "r"}}
        }}"#;
        assert_eq!(sent(lock), ["/nix/store/pqr-fetched"]);
        assert_eq!(kept(lock), ["bare", "near"]);
    }

    #[test]
    fn a_kept_input_is_named_by_where_nix_would_look_for_it() {
        let lock = r#"{"nodes": {
            "secrets": {"locked": {"type": "git", "url": "file:///Users/me/.secrets?ref=main"},
                        "original": {"type": "git", "url": "git+file:///Users/me/.secrets"}}
        }}"#;
        assert_eq!(
            inputs_of(lock).kept,
            [("secrets".to_owned(), "/Users/me/.secrets".to_owned())]
        );
        assert_eq!(inputs_of("not a lock"), Inputs::default());
    }

    fn ran(command: &str) -> (i32, String, String) {
        let kept = [("secrets".to_owned(), "/Users/me/.secrets".to_owned())];
        let scratch = tempfile::tempdir().unwrap();
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(wrapped(command, &kept))
            .env("TMPDIR", scratch.path())
            .output()
            .unwrap();
        let left = std::fs::read_dir(scratch.path()).unwrap().count();
        assert_eq!(left, 0, "the wrapper's files were left behind");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    #[test]
    fn a_failure_naming_a_kept_input_ends_as_kept() {
        let (status, _, said) = ran(
            "set -e; echo 'error: Git repository \"/Users/me/.secrets\" does not exist.' >&2; \
             exit 1",
        );
        assert_eq!(status, KEPT);
        assert!(said.contains("does not exist"), "{said}");
    }

    #[test]
    fn any_other_failure_or_success_ends_as_it_did_with_its_output_untouched() {
        assert_eq!(
            ran("echo out; echo 'error: a type error' >&2; exit 3"),
            (3, "out\n".to_owned(), "error: a type error\n".to_owned())
        );
        assert_eq!(
            ran("echo /Users/me/.secrets; true"),
            (0, "/Users/me/.secrets\n".to_owned(), String::new())
        );
    }

    #[test]
    fn a_command_kept_nothing_by_is_sent_as_it_is() {
        assert_eq!(wrapped("nix build x", &[]), "nix build x");
    }
}

//! The flake's inputs that follow their own repository's main rather than
//! waiting for the flake's lock to move: each fetched into a copy of root's
//! own, built only when a key its config names signed it and it descends
//! from both what the lock pins and what the machine last followed, then
//! given to the build in place of the lock's.

use crate::config::Config;
use crate::git::{self, Repo};
use crate::hosted;
use crate::run::Failure;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

/// Each followed input's commit, by its name in the flake.
pub type Followed = BTreeMap<String, String>;

/// Fetches each followed input's main and gives its head, or why one is
/// refused. `head` is main's, whose lock says where each input was pinned.
pub fn heads(
    config: &Config,
    head: &str,
    last: &Followed,
) -> Result<Result<Followed, String>, Failure> {
    let mut followed = Followed::new();
    if config.follows.is_empty() {
        return Ok(Ok(followed));
    }
    let lock = lock(config, head)?;
    for (name, follow) in &config.follows {
        let repo = Repo::followed(config, name, follow);
        git::keyring(&repo)?;
        let commit = git::fetch(&repo)?;
        if let Err(why) = git::signed(&repo, &commit)? {
            return Ok(Err(format!("{}: {why}", repo.called)));
        }
        let pinned = hosted::locked(&lock, name)?["rev"]
            .as_str()
            .ok_or_else(|| Failure::new(format!("the flake's lock pins {name} to no commit")))?;
        let behind = [
            (Some(pinned), "the flake's lock pins"),
            (
                last.get(name).map(String::as_str),
                "this machine last followed",
            ),
        ];
        for (older, what) in behind.into_iter().filter_map(|(o, w)| Some((o?, w))) {
            if !git::descends(&repo, older, &commit)? {
                return Ok(Err(format!(
                    "{} at {} doesn't descend from {}, which {what}",
                    repo.called,
                    git::short(&commit),
                    git::short(older)
                )));
            }
        }
        followed.insert(name.clone(), commit);
    }
    Ok(Ok(followed))
}

/// What the build is given so each followed input is its followed commit.
pub fn overrides(config: &Config, followed: &Followed) -> Vec<String> {
    followed
        .iter()
        .filter_map(|(name, commit)| {
            let follow = config.follows.get(name)?;
            let repo = Repo::followed(config, name, follow);
            Some([
                "--override-input".to_owned(),
                name.clone(),
                git::reference(&repo, commit),
            ])
        })
        .flatten()
        .collect()
}

/// What the machine last followed, none when it followed nothing.
pub fn last(config: &Config) -> Followed {
    fs::read_to_string(config.followed_file())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Keeps what the machine now follows, for the next update to compare.
pub fn remember(config: &Config, followed: &Followed) -> Result<(), Failure> {
    let file = config.followed_file();
    let new = file.with_extension("new");
    let text = serde_json::to_string(followed).map_err(|e| Failure::new(e.to_string()))?;
    fs::write(&new, text)
        .and_then(|()| fs::rename(&new, &file))
        .map_err(|e| Failure::new(format!("{}: {e}", file.display())))
}

/// The flake's lock at main's `head`, read from root's copy.
fn lock(config: &Config, head: &str) -> Result<Value, Failure> {
    let path = match &config.hosted {
        Some(hosted) if !hosted.directory.is_empty() => {
            format!("{}/flake.lock", hosted.directory.trim_end_matches('/'))
        }
        _ => "flake.lock".to_owned(),
    };
    let text = git::show(&Repo::main(config), head, &path).ok_or_else(|| {
        Failure::new(format!(
            "main at {} has no {path}, so nothing says where its inputs were",
            git::short(head)
        ))
    })?;
    serde_json::from_str(&text).map_err(|e| Failure::new(format!("reading {path}: {e}")))
}

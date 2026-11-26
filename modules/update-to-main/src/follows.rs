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

/// Fetches each followed input's main and gives its head, or the commit
/// `at` names for it, or why one is refused. `head` is main's, whose lock
/// says where each input was pinned.
///
/// A commit `at` names must be on its main, so an ask can step back from
/// a head that holds a change only while that change's revert is still
/// on its way: on 12 November a hold on one merge kept ten others off the
/// machine for half an hour, though the main just before it was fine. It
/// is held to the same signature and the same history as a head, so it
/// never takes the machine back past what it last followed.
///
/// An input followed only when asked (`asked`) stays where its lock pins
/// it until an ask names a commit of it, so a person chooses when it moves.
/// Once asked, it stays at that commit while it is ahead of what its lock
/// pins, since falling back to an older pin would undo what was asked
/// for. Once the lock pins it, past it or off its line, the lock rules
/// again: holding an ask against a lock gone another way would refuse
/// every update, and no ask could clear it.
pub fn heads(
    config: &Config,
    head: &str,
    last: &Followed,
    at: &Followed,
) -> Result<Result<Followed, String>, Failure> {
    let mut followed = Followed::new();
    if let Some(name) = at.keys().find(|name| !config.follows.contains_key(*name)) {
        return Ok(Err(format!(
            "{name} isn't an input this machine follows, so no ask may name a commit of it"
        )));
    }
    if config.follows.is_empty() {
        return Ok(Ok(followed));
    }
    let lock = lock(config, head)?;
    // A name sorts before its own inputs' (core before core/mycelium), so
    // an input's parent is settled before its pin is read from it.
    for (name, follow) in &config.follows {
        let kept = last.get(name);
        if follow.asked && kept.is_none() && !at.contains_key(name) {
            continue;
        }
        let repo = Repo::followed(config, name, follow);
        git::keyring(&repo)?;
        let main = git::fetch(&repo)?;
        let (pinned, pins) = pinned(config, &lock, name, &followed)?;
        let commit = match at.get(name) {
            None if follow.asked => match kept {
                Some(kept)
                    if git::descends(&repo, &pinned, kept)?
                        && !git::descends(&repo, kept, &pinned)? =>
                {
                    kept.clone()
                }
                _ => continue,
            },
            None => main,
            Some(asked) if git::descends(&repo, asked, &main)? => asked.clone(),
            Some(asked) => {
                return Ok(Err(format!(
                    "{name} at {} isn't on its main, at {}, so it isn't taken",
                    git::short(asked),
                    git::short(&main)
                )));
            }
        };
        if let Err(why) = git::signed(&repo, &commit)? {
            return Ok(Err(format!("{}: {why}", repo.called)));
        }
        let behind = [
            (Some(pinned.as_str()), pins.as_str()),
            (kept.map(String::as_str), "this machine last followed"),
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

/// The commit `name` is pinned at, and what pins it: main's lock, or, for
/// an input of an input followed, that input's own lock at the commit
/// followed, since that's the lock the build reads it from.
fn pinned(
    config: &Config,
    lock: &Value,
    name: &str,
    followed: &Followed,
) -> Result<(String, String), Failure> {
    let rev = |locked: &Value| {
        locked["rev"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| Failure::new(format!("the flake's lock pins {name} to no commit")))
    };
    if let Some((parent, own)) = name.rsplit_once('/')
        && let Some(commit) = followed.get(parent)
        && let Some(follow) = config.follows.get(parent)
    {
        let repo = Repo::followed(config, parent, follow);
        let at = git::short(commit);
        let text = git::show(&repo, commit, "flake.lock").ok_or_else(|| {
            Failure::new(format!(
                "{parent} at {at} has no flake.lock, so nothing pins {name}"
            ))
        })?;
        let theirs: Value = serde_json::from_str(&text)
            .map_err(|e| Failure::new(format!("reading {parent}'s flake.lock at {at}: {e}")))?;
        return Ok((
            rev(hosted::locked(&theirs, own)?)?,
            format!("{parent}'s lock at {at} pins"),
        ));
    }
    Ok((
        rev(hosted::locked(lock, name)?)?,
        "the flake's lock pins".to_owned(),
    ))
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
    let text = match &config.hosted {
        Some(hosted) => hosted::show(config, hosted, head, "flake.lock"),
        None => git::show(&Repo::main(config), head, &path),
    };
    let text = text.ok_or_else(|| {
        Failure::new(format!(
            "main at {} has no {path}, so nothing says where its inputs were",
            git::short(head)
        ))
    })?;
    serde_json::from_str(&text).map_err(|e| Failure::new(format!("reading {path}: {e}")))
}

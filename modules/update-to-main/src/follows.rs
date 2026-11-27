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
/// it until an ask names a commit of it, so its own main never moves it;
/// a lock that moves, as a followed parent's merge can, still does.
/// Once asked, it stays at that commit while it is ahead of what its lock
/// pins, since falling back to an older pin would undo what was asked
/// for. Once the lock pins it, past it or off its line, the lock rules
/// again: holding an ask against a lock gone another way would refuse
/// every update, and no ask could clear it. A follow whose lock no longer
/// has its input is left out, asked or not, since the build no longer
/// reads it. An ask in `waited`, built and waiting for a hold, is kept as
/// one followed is, but only what `last` followed is a floor no ask may
/// step behind, since a commit that only waited never ran.
pub fn heads(
    config: &Config,
    head: &str,
    last: &Followed,
    waited: &Followed,
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
        let kept = waited.get(name).or_else(|| last.get(name));
        if follow.asked && kept.is_none() && !at.contains_key(name) {
            continue;
        }
        // The lock is read before anything is fetched, so an input it
        // dropped, whose repository may have gone with it, can't fail the
        // update.
        let Some((pinned, pins)) = pinned(config, &lock, name, &followed)? else {
            // A lock that no longer has the input builds without it, so
            // there is nothing to override, nor for a kept ask to stand
            // ahead of. Failing instead would fail every update, and the
            // config that drops the follow comes only with a switch. An
            // ask naming it is refused, since nothing would build it.
            if !at.contains_key(name) {
                continue;
            }
            return Ok(Err(format!(
                "{name} isn't in the lock the build reads, so no ask may name a commit of it"
            )));
        };
        let repo = Repo::followed(config, name, follow);
        git::keyring(&repo)?;
        let commit = match (at.get(name), kept) {
            // A kept ask needs its main only when the copy lacks what it
            // compares, so a repository out of reach fails no update that
            // the copy alone can settle.
            (None, Some(kept)) if follow.asked => {
                if !git::has(&repo, kept) || !git::has(&repo, &pinned) {
                    git::fetch(&repo)?;
                }
                if !git::descends(&repo, &pinned, kept)? || git::descends(&repo, kept, &pinned)? {
                    continue;
                }
                kept.clone()
            }
            (None, _) => git::fetch(&repo)?,
            (Some(asked), _) => {
                let main = git::fetch(&repo)?;
                if !git::descends(&repo, asked, &main)? {
                    return Ok(Err(format!(
                        "{name} at {} isn't on its main, at {}, so it isn't taken",
                        git::short(asked),
                        git::short(&main)
                    )));
                }
                asked.clone()
            }
        };
        if let Err(why) = git::signed(&repo, &commit)? {
            return Ok(Err(format!("{}: {why}", repo.called)));
        }
        let ran = last.get(name).map(String::as_str);
        let behind = [
            (Some(pinned.as_str()), pins.as_str()),
            (ran, "this machine last followed"),
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

/// Each followed input's commit as main at `head` builds it with
/// `followed`: its followed commit, or where its lock pins it. An input
/// the lock no longer has isn't built, so it has none. Only locks are
/// read, main's and followed parents' already fetched, so nothing is
/// fetched.
pub fn built(config: &Config, head: &str, followed: &Followed) -> Result<Followed, Failure> {
    let mut built = followed.clone();
    if config.follows.is_empty() {
        return Ok(built);
    }
    let lock = lock(config, head)?;
    for name in config.follows.keys() {
        if built.contains_key(name) {
            continue;
        }
        if let Some((pinned, _)) = pinned(config, &lock, name, followed)? {
            built.insert(name.clone(), pinned);
        }
    }
    Ok(built)
}

/// The commit `name` is pinned at, and what pins it: main's lock, or, under
/// an input followed, the nearest such input's own lock at the commit
/// followed, since that's the lock the build reads it from; core's, for
/// core/mycelium/kit with only core followed. None where that lock has no
/// such input.
fn pinned(
    config: &Config,
    lock: &Value,
    name: &str,
    followed: &Followed,
) -> Result<Option<(String, String)>, Failure> {
    let rev = |locked: &Value| {
        locked["rev"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| Failure::new(format!("the flake's lock pins {name} to no commit")))
    };
    let nearest = name.rmatch_indices('/').find_map(|(cut, _)| {
        let parent = &name[..cut];
        Some((
            parent,
            &name[cut + 1..],
            followed.get(parent)?,
            config.follows.get(parent)?,
        ))
    });
    if let Some((parent, own, commit, follow)) = nearest {
        let repo = Repo::followed(config, parent, follow);
        let at = git::short(commit);
        // Only a lock read and found without the input lets go of a kept
        // ask: a parent git can't read would otherwise drop it unasked.
        if !git::has(&repo, commit) {
            return Err(Failure::new(format!("{parent} at {at} can't be read")));
        }
        if !git::holds_file(&repo, commit, "flake.lock") {
            return Ok(None);
        }
        let Some(text) = git::show(&repo, commit, "flake.lock") else {
            return Err(Failure::new(format!(
                "{parent}'s flake.lock at {at} can't be read"
            )));
        };
        let theirs: Value = serde_json::from_str(&text)
            .map_err(|e| Failure::new(format!("reading {parent}'s flake.lock at {at}: {e}")))?;
        return hosted::found(&theirs, own)
            .map(|locked| Ok((rev(locked)?, format!("{parent}'s lock at {at} pins"))))
            .transpose();
    }
    hosted::found(lock, name)
        .map(|locked| Ok((rev(locked)?, "the flake's lock pins".to_owned())))
        .transpose()
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

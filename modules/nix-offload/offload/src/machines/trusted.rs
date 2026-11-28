//! How each builder beside stands, so it's trusted with builds only while
//! its proofs say: as root's prover writes, or as proven here while Nix
//! trusts this account.

use super::seconds;
use crate::proof::{self, Health, Keeper, State};
use std::path::Path;
use std::time::{Duration, Instant};

/// How the builder beside `line` names stands, keeping it in `kept`,
/// which goes on from how `earlier` said it stood: whether it's left out,
/// having failed its proofs, with its store full or with no proof yet,
/// saying so as that changes. Where root's prover writes `root`, that
/// says, since only an account the daemon trusts can prove a builder
/// beside; elsewhere it's proven here when due, with `nix`, unless `trust`
/// hears that Nix doesn't trust this account. One unproven waits for its
/// first proof, a few seconds, since its store may be full.
pub(super) fn kept_beside(
    (kept, earlier): (&mut Vec<(String, Keeper)>, &[Health]),
    line: &str,
    (nix, root, trust): (&str, &Path, &mut Trust),
    now: Instant,
) -> bool {
    let index = kept
        .iter()
        .position(|(kept, _)| kept == line)
        .unwrap_or_else(|| {
            let named = proof::named(line);
            let health = earlier
                .iter()
                .find(|health| health.builder == named)
                .map_or_else(|| Health::of(&named), Clone::clone);
            kept.push((line.to_owned(), Keeper::going_on(health)));
            kept.len() - 1
        });
    let keeper = &mut kept[index].1;
    let was = keeper.health.state();
    let uri = line.split_whitespace().next().unwrap_or_default();
    if let Some(health) = proof::standing_health(root, uri, &keeper.health.builder, seconds()) {
        keeper.health = health;
    } else if !trust.untrusted(nix, now) {
        let (proving, nix) = (line.to_owned(), nix.to_owned());
        keeper.keep(now, seconds(), move || {
            proof::through_daemon(&proving, &nix, proof::WITHIN)
        });
    }
    let state = keeper.health.state();
    let name = &keeper.health.builder;
    match (was, state) {
        (was, is) if was == is => {}
        (_, State::Dropped) => eprintln!(
            "nix-offload: {name} failed {} proofs in a row, so no build goes there until one \
             passes",
            proof::FAILS
        ),
        (_, State::Full) => eprintln!(
            "nix-offload: {name} has {} GB free, under the {} GB floor, so no build goes \
             there until it has room",
            keeper.health.free.unwrap_or_default() / 1_000_000_000,
            proof::FLOOR / 1_000_000_000
        ),
        (State::Dropped, _) => {
            eprintln!("nix-offload: {name} passed its proof again, so builds go there again");
        }
        (State::Full, State::Passing) => {
            eprintln!("nix-offload: {name} has room again, so builds go there again");
        }
        _ => {}
    }
    keeper.health.left_out() || state == State::Unproven
}

/// Where root's prover says how the builders beside stand. A unit test
/// reads none unless it names its own: on 12 November six read this
/// machine's, which named none of their builders, and failed outside the
/// build's sandbox.
/// Whether Nix trusts this account, as it last said: asked again at most
/// every [`TRUST_AGAIN`], since only a switch changes it.
#[derive(Default)]
pub struct Trust {
    said: Option<(bool, Instant)>,
}

/// How long what Nix said of this account's trust is taken as still so.
const TRUST_AGAIN: Duration = Duration::from_secs(300);

impl Trust {
    /// Whether `nix`'s daemon says it doesn't trust this account, saying
    /// so once it starts to. A builder beside that root's prover hasn't
    /// proven then keeps how it last stood, rather than a proof here
    /// failing it with builders Nix ignores, as it would once the person's
    /// account left Nix's trusted users (ISXEG8, 14 November).
    fn untrusted(&mut self, nix: &str, now: Instant) -> bool {
        if let Some((untrusted, at)) = self.said
            && now.saturating_duration_since(at) < TRUST_AGAIN
        {
            return untrusted;
        }
        let untrusted = proof::untrusted(nix);
        if untrusted && self.said.is_none_or(|(was, _)| !was) {
            eprintln!(
                "nix-offload: Nix doesn't trust this account, so a builder beside that root's \
                 prover hasn't proven waits for it, as it last stood, rather than being proven \
                 here with builders Nix would ignore"
            );
        }
        self.said = Some((untrusted, now));
        untrusted
    }
}

#[cfg(test)]
#[path = "trusted_tests.rs"]
mod tests;

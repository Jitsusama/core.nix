//! Keeping every ssh nix-offload starts, its own and the ones under the
//! Nix it runs, from asking anyone anything. A copy up's Nix pins the
//! builder's host key in a file under its TMPDIR, and its ssh reads it
//! there. At 22:37 on 21 November something emptied /tmp under the copies
//! running then, the files went, and each copy's ssh, with no tty but the
//! user manager's SSH_ASKPASS and DISPLAY, asked Joel on his screen
//! whether to trust the builder: about 22 dialogs in six minutes. Told
//! never to ask and to trust only a key it was given, ssh refuses the
//! builder instead, which the run reads as a lost one and sends again.

use std::env;

/// What every ssh under Nix is told ahead of anything else: never ask,
/// and connect only to a host whose key it already holds.
pub const NEVER_ASKS: &str = "-o BatchMode=yes -o StrictHostKeyChecking=yes";

/// `NIX_SSHOPTS` as `given`, with [`NEVER_ASKS`] ahead of it, since ssh
/// keeps the first value it reads for each option, so nothing given can
/// loosen it. One that already starts with it is kept as it is, as one
/// nix-offload run starting another gives it.
pub fn sshopts(given: Option<&str>) -> String {
    match given.map(str::trim) {
        Some(given) if given.starts_with(NEVER_ASKS) => given.to_owned(),
        Some(given) if !given.is_empty() => format!("{NEVER_ASKS} {given}"),
        _ => NEVER_ASKS.to_owned(),
    }
}

/// Sets this process's environment so that nothing it starts asks anyone:
/// no askpass for any ssh, and [`sshopts`] for every ssh under Nix. Called
/// first in `main`, before any thread starts.
pub fn never_ask() {
    let sshopts = sshopts(env::var("NIX_SSHOPTS").ok().as_deref());
    // SAFETY: `main` calls this before it starts any thread, so nothing
    // reads the environment while it changes.
    unsafe {
        env::remove_var("SSH_ASKPASS");
        env::set_var("SSH_ASKPASS_REQUIRE", "never");
        env::set_var("NIX_SSHOPTS", sshopts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_given_is_told_never_to_ask() {
        assert_eq!(sshopts(None), NEVER_ASKS);
        assert_eq!(sshopts(Some("  ")), NEVER_ASKS);
    }

    #[test]
    fn what_is_given_comes_after_never_asking_so_it_cant_loosen_it() {
        let given = "-o StrictHostKeyChecking=no -o ServerAliveCountMax=40";
        assert_eq!(sshopts(Some(given)), format!("{NEVER_ASKS} {given}"));
    }

    #[test]
    fn options_already_told_never_to_ask_are_kept_as_they_are() {
        let given = format!("{NEVER_ASKS} -o ServerAliveCountMax=40");
        assert_eq!(sshopts(Some(&given)), given);
    }
}

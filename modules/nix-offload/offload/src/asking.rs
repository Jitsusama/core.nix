//! Keeping every ssh nix-offload starts, its own and the ones under the
//! Nix it runs, from asking anyone anything. A copy up's Nix pins the
//! builder's host key in a file under its `TMPDIR`, and its ssh reads it
//! there. At 22:37 on 21 November something emptied /tmp under the copies
//! running then, the files went, and each copy's ssh, with no tty but the
//! user manager's `SSH_ASKPASS` and `DISPLAY`, asked Joel on his screen
//! whether to trust the builder: about 22 dialogs in six minutes. Told
//! never to ask and to trust only a key it was given, ssh refuses the
//! builder instead, which the run reads as a lost one and sends again.
//!
//! Where it stops: this governs ssh's own questions, about host keys and
//! passphrases. An ssh agent asks through its own environment, so one
//! that wants a PIN still can; a builder's own key keeps the agent out
//! where one is lent ([`crate::there::reach`]). Strict checking reaches
//! every Nix copy, nearby machines' too, whose stores pin no key, so a
//! nearby machine is reached only while its key is in known_hosts under
//! the name it's given.

use std::env;
use std::ffi::OsString;

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
/// no askpass for any ssh, and [`sshopts`] for every ssh under Nix. One
/// given that isn't text keeps its bytes after [`NEVER_ASKS`].
///
/// # Safety
///
/// It changes the environment, so it must be called before any thread
/// starts, as `main` calls it first.
pub unsafe fn never_ask() {
    let sshopts = match env::var_os("NIX_SSHOPTS") {
        Some(given) => match given.to_str() {
            Some(text) => OsString::from(sshopts(Some(text))),
            None => {
                let mut joined = OsString::from(format!("{NEVER_ASKS} "));
                joined.push(given);
                joined
            }
        },
        None => OsString::from(NEVER_ASKS),
    };
    // SAFETY: the caller calls this before any thread starts, so nothing
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

//! How a machine's going down or coming back reaches Joel, as a desktop
//! notice, and the orchestrator and its owner, as a finding, each once.

use std::process::{Command, Stdio};

use super::{Tell, Told};

/// Tells each of `tells`: a going down by `notify` to Joel once a spell and
/// by `send` as a finding to the orchestrator and the owner `owner_of`
/// names for `down:<machine>`, each who hasn't had it; a coming back to
/// each who had its going down, replacing Joel's notice. A going down not
/// told to all is left untold in `told`, so the next tick tells the rest.
///
/// # Errors
///
/// The last error sending one, once each is tried.
pub fn tell<E>(
    tells: &[Tell],
    told: &mut Told,
    orchestrator: &str,
    owner_of: impl Fn(&str) -> Option<String>,
    notify: impl Fn(&Tell) -> Option<u32>,
    send: impl Fn(&str, String) -> Result<(), E>,
) -> Result<(), E> {
    let mut failed = None;
    for one in tells {
        match one {
            Tell::Down { machine, .. } => {
                let mut to = vec![orchestrator.to_owned()];
                to.extend(
                    owner_of(&format!("down:{machine}"))
                        .filter(|owner| !owner.eq_ignore_ascii_case(orchestrator)),
                );
                let spell = told.machines.entry(machine.clone()).or_default();
                if spell.notice.is_none() {
                    spell.notice = notify(one);
                }
                for address in &to {
                    if spell.sent.contains(address) {
                        continue;
                    }
                    match send(address, one.finding(spell.notice.is_some())) {
                        Ok(()) => {
                            spell.sent.insert(address.clone());
                        }
                        Err(err) => failed = Some(err),
                    }
                }
                spell.told = spell.notice.is_some() && to.iter().all(|a| spell.sent.contains(a));
            }
            Tell::Back { to, notice, .. } => {
                if notice.is_some() {
                    notify(one);
                }
                // A coming back not sent is let go, as the machine is back.
                for address in to {
                    if let Err(err) = send(address, one.finding(notice.is_some())) {
                        failed = Some(err);
                    }
                }
            }
        }
    }
    failed.map_or(Ok(()), Err)
}

/// The notice's body as markup reads it, so nothing in it is taken for a
/// tag or a link.
pub(super) fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// What `notify-send` is given to show `one`: printing the id it's shown
/// by, and a coming back replacing its going down. An id kept for hours
/// stays its notice's across a restart of the shell, since the notices
/// daemon keeps its next id with its store and replaces only a notice
/// still open, giving a new id otherwise; only a store it can't read,
/// which it sets aside, numbers from 1 again.
pub(super) fn notice_args(one: &Tell) -> Vec<String> {
    let mut args: Vec<String> = ["-p", "-a", "Machines", "-u", one.urgency()]
        .map(str::to_owned)
        .into();
    if let Tell::Back {
        notice: Some(id), ..
    } = one
    {
        args.extend(["-r".to_owned(), id.to_string()]);
    }
    args.extend([
        "--".to_owned(),
        one.machine().to_owned(),
        escaped(&one.notice()),
    ]);
    args
}

/// Shows Joel `one`'s notice, with its id when it was shown.
pub(super) fn notify(one: &Tell) -> Option<u32> {
    let out = Command::new("notify-send")
        .args(notice_args(one))
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|out| out.status.success())?;
    Some(
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or(0),
    )
}

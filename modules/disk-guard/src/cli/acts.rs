//! The commands that change what the account's guard does: keeping a
//! folder for a reason and letting it go, clearing one now, thawing what
//! it froze, and asking the machine's guard what files take on the disk.
//! The launcher's disk piece runs the first four through the machine
//! daemon with `--format json`, so each answers its facts as JSON and its
//! sentence as text, and the machine daemon words the sentence itself.

use super::{Env, failed, said_path};
use crate::units::shown as size;
use serde_json::{Value, json};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use terminal_kit::{Context, Exit, Failure};

fn now() -> f64 {
    super::wall()
}

/// A folder named that isn't there.
fn not_there(path: &Path, to: &str) -> Failure {
    Failure::new(
        Exit::NotFound,
        format!("{} isn't there to {to}", path.display()),
    )
}

/// Keeps a folder for a reason, and says so in the status file before it
/// ends, so whatever asked sees the holder change as it hears the answer.
pub(super) fn keep(
    env: &Env,
    path: &Path,
    reason: &str,
    context: &Context,
    out: &mut dyn Write,
) -> Result<(), Failure> {
    let path = fs::canonicalize(path).map_err(|_| not_there(path, "keep"))?;
    fs::create_dir_all(&env.state).map_err(failed)?;
    crate::keeps::keep(&env.state, &path, reason, now()).map_err(|error| {
        Failure::new(Exit::Failed, format!("{} couldn't be kept", path.display()))
            .because(error.to_string())
    })?;
    marked(env)?;
    if context.format().is_json() {
        let answer = json!({
            "schema": "disk-guard.keep/1",
            "path": said_path(&path),
            "reason": reason,
        });
        return context.answer_to(out, &answer).map_err(failed);
    }
    writeln!(out, "Kept {}: {reason}.", path.display()).map_err(failed)
}

/// Lets go of a keep, saying whether there was one.
pub(super) fn unkeep(
    env: &Env,
    path: &Path,
    context: &Context,
    out: &mut dyn Write,
) -> Result<(), Failure> {
    // A kept folder may have gone since, and its keep with it.
    let path = fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .map_err(failed)?;
    let was = crate::keeps::unkeep(&env.state, &path).map_err(|error| {
        Failure::new(
            Exit::Failed,
            format!("{} couldn't be let go", path.display()),
        )
        .because(error.to_string())
    })?;
    marked(env)?;
    if context.format().is_json() {
        let answer = json!({
            "schema": "disk-guard.unkeep/1",
            "path": said_path(&path),
            "was_kept": was,
        });
        return context.answer_to(out, &answer).map_err(failed);
    }
    if was {
        writeln!(out, "No longer kept: {}.", path.display()).map_err(failed)
    } else {
        writeln!(out, "{} wasn't kept.", path.display()).map_err(failed)
    }
}

/// Asks the account's guard to clear what it would take of a folder now,
/// and says so in the status file before it ends. A kept folder is
/// refused here, though the guard would take nothing kept anyway, since
/// asking to clear what Joel keeps is a slip worth saying.
pub(super) fn clear(
    env: &Env,
    path: &Path,
    context: &Context,
    out: &mut dyn Write,
) -> Result<(), Failure> {
    let path = fs::canonicalize(path).map_err(|_| not_there(path, "clear"))?;
    let keeps = crate::keeps::load(&env.state).map_err(|error| {
        Failure::new(Exit::Failed, "the keeps can't be read").because(error.to_string())
    })?;
    // The folder kept may hold this one, and it's that keep to let go.
    if let Some((kept, keep)) = crate::keeps::kept_over(&keeps, &path) {
        return Err(
            Failure::new(Exit::Failed, format!("{} stays", path.display()))
                .because(format!("you keep it: {}", keep.reason))
                .try_this(format!("disk-guard keeps {} remove", kept.display())),
        );
    }
    crate::clears::ask(&env.state, &path, now()).map_err(|error| {
        Failure::new(
            Exit::Failed,
            format!("clearing {} couldn't be asked", path.display()),
        )
        .because(error.to_string())
    })?;
    marked(env)?;
    if context.format().is_json() {
        let answer = json!({"schema": "disk-guard.clear/1", "path": said_path(&path)});
        return context.answer_to(out, &answer).map_err(failed);
    }
    writeln!(
        out,
        "Clearing {}; the guard starts on its next step.",
        path.display()
    )
    .map_err(failed)
}

/// How long a footprint may take the machine's guard to walk.
const FOOTPRINT_TIMEOUT: Duration = Duration::from_mins(5);

/// Asks the machine's guard for a footprint, asking again while it says
/// another of the account's asks came a moment ago, or another footprint
/// is being walked, until the walk's time is up.
fn asked_again(
    socket: &Path,
    request: &crate::socket::Request,
) -> io::Result<crate::socket::Reply> {
    let until = std::time::Instant::now() + FOOTPRINT_TIMEOUT;
    loop {
        match crate::socket::ask(socket, request, FOOTPRINT_TIMEOUT) {
            Err(error) if std::time::Instant::now() < until => {
                let why = error.to_string();
                if why == crate::socket::TOO_SOON {
                    std::thread::sleep(crate::socket::ASK_EVERY);
                } else if why == crate::socket::WALKING {
                    std::thread::sleep(Duration::from_secs(1));
                } else {
                    return Err(error);
                }
            }
            answered => return answered,
        }
    }
}

/// Says what the files under each of `paths` take on the disk, as the
/// machine's guard reads it, beside what they hold once read. Every path
/// is said, and the run fails after if one's footprint couldn't be read.
pub(super) fn footprint(
    env: &Env,
    paths: &[PathBuf],
    context: &Context,
    out: &mut dyn Write,
) -> Result<(), Failure> {
    let paths: Vec<PathBuf> = paths
        .iter()
        .map(|path| fs::canonicalize(path).unwrap_or_else(|_| path.clone()))
        .collect();
    let request = crate::socket::Request::Footprint {
        paths: paths.clone(),
    };
    let asked = |error: io::Error| {
        let exit = if matches!(
            error.kind(),
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
        ) {
            Exit::Unavailable
        } else {
            Exit::Failed
        };
        Failure::new(exit, "the machine's guard couldn't be asked")
            .because(error.to_string())
            .try_this("systemctl status disk-guard")
    };
    let found = match asked_again(&env.system_socket, &request).map_err(asked)? {
        crate::socket::Reply::Footprints(found) if found.len() == paths.len() => found,
        other => {
            return Err(
                Failure::new(Exit::Failed, "the machine's guard answered something else")
                    .because(format!("{other:?}")),
            );
        }
    };
    let mut unread = 0;
    let mut said = Vec::new();
    for (path, disk) in paths.iter().zip(found) {
        let files = crate::footprint::files(path);
        if disk.is_none() {
            unread += 1;
        }
        said.push(json!({
            "path": said_path(path),
            "disk_bytes": disk,
            "files_bytes": files,
        }));
        if context.format().is_json() {
            continue;
        }
        let files = size(files);
        if let Some(disk) = disk {
            writeln!(
                out,
                "{}: {} on the disk, for {files} of files.",
                path.display(),
                size(disk)
            )
        } else {
            writeln!(
                out,
                "{}: {files} of files; what they take on the disk can't be read \
                 (not yours, not on Btrfs, or gone).",
                path.display()
            )
        }
        .map_err(failed)?;
    }
    if context.format().is_json() {
        let answer = json!({"schema": "disk-guard.footprint/1", "paths": said});
        context.answer_to(out, &answer).map_err(failed)?;
    }
    if unread > 0 {
        return Err(Failure::new(
            Exit::Failed,
            format!("{unread} of the footprints couldn't be read"),
        ));
    }
    Ok(())
}

/// Marks the status file's holders with the keeps and clears as they are
/// now. The daemon marks each status it writes the same way, so whichever
/// writes last, the file says what's kept and what's clearing.
fn marked(env: &Env) -> Result<(), Failure> {
    let refused = |error: io::Error| {
        Failure::new(Exit::Failed, "the status couldn't be marked").because(error.to_string())
    };
    let Some(mut status) = crate::state::load::<Value>(&env.status).map_err(refused)? else {
        return Ok(());
    };
    let Ok(mut holders) =
        serde_json::from_value::<Vec<crate::holders::Holder>>(status["holders"].clone())
    else {
        return Ok(());
    };
    let keeps = crate::keeps::load(&env.state).map_err(refused)?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let clearing = crate::clears::clearing(&env.state);
    crate::holders::mark(&mut holders, &keeps, &clearing, home.as_deref());
    status["holders"] =
        serde_json::to_value(holders).map_err(|error| refused(io::Error::other(error)))?;
    crate::state::share(&env.status, &status).map_err(refused)
}

/// What a thaw says: each unit thawed, or that nothing was frozen.
fn thawed(thawed: &[String], context: &Context, out: &mut dyn Write) -> Result<(), Failure> {
    if context.format().is_json() {
        let answer = json!({"schema": "disk-guard.thaw/1", "thawed": thawed});
        return context.answer_to(out, &answer).map_err(failed);
    }
    if thawed.is_empty() {
        writeln!(out, "Nothing is frozen.").map_err(failed)?;
    }
    for unit in thawed {
        writeln!(out, "Thawed {unit}.").map_err(failed)?;
    }
    Ok(())
}

/// Thaws whatever the account's guard froze, through the account's own
/// service manager. The guard sees it thawed and never freezes it again.
#[cfg(target_os = "linux")]
pub(super) fn thaw(context: &Context, out: &mut dyn Write) -> Result<(), Failure> {
    let failed = |error: zbus::Error| {
        Failure::new(Exit::Unavailable, "the service manager couldn't be asked")
            .because(error.to_string())
    };
    let builder = zbus::blocking::connection::Builder::session().map_err(failed)?;
    let freezer = crate::freezer::Freezer::new(crate::desktop::patient(builder).map_err(failed)?);
    let units = crate::freezer::thaw_all(&freezer).map_err(failed)?;
    thawed(&units, context, out)
}

/// A Mac's guard freezes nothing, so nothing is frozen.
#[cfg(not(target_os = "linux"))]
pub(super) fn thaw(context: &Context, out: &mut dyn Write) -> Result<(), Failure> {
    thawed(&[], context, out)
}

//! What's read as it changes rather than on the second: the files the
//! backlight program, the cameras' holders, the keyboards' watchers and
//! Keep Awake write, the kernel's links as it says one changed, and the
//! light sensors on a thread of their own.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::bus::unheard;
use crate::making::watched;
use crate::reading::{due, due_while_watched, read};
use crate::{Daemon, EVERY, KEPT, Shared, lock};
use machine::{Root, kept, light, links, written::Written};

/// How often a watch on files heard looks to see whether it's still
/// wanted, once every folder is watched.
const RESTING: Duration = Duration::from_secs(60);
/// How long `kept_loop` waits on its files before looking again for a
/// folder that isn't there, as `/run/power-profile` isn't while its
/// service starts again.
const KEPT_LOOKS_AGAIN: Duration = Duration::from_secs(10);
/// Reads the light sensors each second while they're watched, on a
/// thread of their own: a sensor's raw count takes about a tenth of a
/// second to read, and read with the rest it would hold every other kind
/// back, a step's first answer among them. What it reads is told at once.
/// With the last watch its readings are let go, so the next watch's first
/// answer says the sensor unread rather than an old count as now.
pub(crate) fn light_loop(shared: &Shared, root: &Root) {
    let mut last = None;
    let kinds = HashSet::from(["light".to_owned()]);
    loop {
        if !due_while_watched(shared, "light", EVERY, &mut last) {
            unheard(&mut lock(&shared.daemon));
            due(shared, "light", EVERY, &mut last);
        }
        let said = light::sensors(root);
        lock(&shared.daemon).light = said;
        read(shared, &kinds, &[]);
    }
}

/// Hears the files of these kinds in these folders as they're written
/// while the kinds are watched, and tells what changed at once: the
/// backlight program's, the cameras' holders', and the keyboards'
/// watchers'. They're read at no
/// other time but a watch's first answer and a get. They're read once more
/// as the watch on them begins, so one written since the first answer is
/// said all the same. With the last watch the files are let be, so an
/// idle machine is woken by none of their writes.
pub(crate) fn heard_loop(
    shared: &Shared,
    kinds: &[&str],
    folders: &[PathBuf],
    ours: impl Fn(&str) -> bool,
) {
    heard_until(shared, kinds, folders, ours, |_| None);
}

/// As `heard_loop`, and read again too when `due` says, as a watcher's
/// status is when its heartbeat is late, though nothing was written.
pub(crate) fn heard_until(
    shared: &Shared,
    kinds: &[&str],
    folders: &[PathBuf],
    ours: impl Fn(&str) -> bool,
    due: impl Fn(&Shared) -> Option<Instant>,
) {
    let kinds: HashSet<String> = kinds.iter().map(|kind| (*kind).to_owned()).collect();
    let watching = |daemon: &Daemon| watched(daemon).iter().any(|kind| kinds.contains(kind));
    loop {
        {
            let mut daemon = lock(&shared.daemon);
            while !watching(&daemon) {
                daemon = shared
                    .woken
                    .wait(daemon)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        let Some(mut written) = Written::on(folders.iter().cloned()) else {
            // With no watch the kernel will give, they're read on the
            // second instead, as every other kind is.
            while watching(&lock(&shared.daemon)) {
                read(shared, &kinds, &[]);
                std::thread::sleep(EVERY);
            }
            continue;
        };
        read(shared, &kinds, &[]);
        while watching(&lock(&shared.daemon)) {
            // A folder not there yet is looked for each second; with all
            // of them watched, only to see whether the watch has ended,
            // which can wait, since a file heard while nothing watches
            // tells nobody.
            let resting = if written.looking() { EVERY } else { RESTING };
            let wait = due(shared).map_or(resting, |at| {
                resting.min(at.saturating_duration_since(Instant::now()))
            });
            let heard = written.wait(wait);
            let late = due(shared).is_some_and(|at| at <= Instant::now());
            if late || heard.iter().any(|name| name.is_empty() || ours(name)) {
                read(shared, &kinds, &[]);
            }
        }
    }
}

/// Hears the kernel say a link or an address came, changed or went while
/// `link` is watched, and tells what changed at once: the links are
/// dumped as the watch begins and again only as the kernel speaks, a
/// burst of its messages read once, so nothing polls. Within a minute of
/// the last watch ending the socket is let go, so an idle machine hears
/// none of it.
pub(crate) fn links_loop(shared: &Shared) {
    let kinds = HashSet::from(["link".to_owned()]);
    let watching = |daemon: &Daemon| watched(daemon).contains("link");
    loop {
        {
            let mut daemon = lock(&shared.daemon);
            while !watching(&daemon) {
                daemon = shared
                    .woken
                    .wait(daemon)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        let Some(mut heard) = links::Heard::on() else {
            // With no socket the kernel will give, they're read on the
            // second instead, as every other kind is.
            while watching(&lock(&shared.daemon)) {
                read(shared, &kinds, &[]);
                std::thread::sleep(EVERY);
            }
            continue;
        };
        read(shared, &kinds, &[]);
        while watching(&lock(&shared.daemon)) {
            // Only whether the watch has ended waits on the clock, which
            // can, since a change heard while nothing watches tells nobody.
            if heard.wait(RESTING) {
                read(shared, &kinds, &[]);
            }
        }
    }
}

/// Reads these of the kinds heard by a thread of their own, those of them
/// watched, and tells what changed; unwatched, they're read afresh by the
/// next watch's first answer.
pub(crate) fn heard(shared: &Shared, kinds: &[&str]) {
    let watching = watched(&lock(&shared.daemon));
    let kinds: HashSet<String> = kinds
        .iter()
        .map(|kind| (*kind).to_owned())
        .filter(|kind| watching.contains(kind))
        .collect();
    if !kinds.is_empty() {
        read(shared, &kinds, &[]);
    }
}

/// Hears Keep Awake's files come and go in the runtime folder, and the
/// machine's chosen profile written, from the daemon's start, and tells
/// what changed at once. The shell watches both while the launcher's
/// shut, so they're never read on a clock: only a write wakes it, and a
/// folder not there, or taken away as systemd does a service's runtime
/// folder, is looked for again every few seconds.
pub(crate) fn kept_loop(shared: &Shared, root: &Root) {
    let runtime = lock(&shared.reader).runtime.clone();
    let runtime = runtime.map(|folder| root.path(&folder));
    let folders = runtime.into_iter().chain([root.path(kept::CHOSEN)]);
    let Some(mut written) = Written::on(folders) else {
        // With no watch the kernel will give, they're read by each watch's
        // first answer and each get alone.
        return;
    };
    // A file written after a watch was answered but before the kernel's
    // watch began would be heard by neither, so what's there is read once
    // now, telling only the kinds watched: a test that keeps itself awake
    // just after watching waited the whole of its patience for it.
    heard(shared, &KEPT);
    loop {
        let names = written.wait(KEPT_LOOKS_AGAIN);
        let locks = names
            .iter()
            .any(|name| name.is_empty() || kept::names_a_lock(name));
        let choice = names
            .iter()
            .any(|name| name.is_empty() || kept::names_the_choice(name));
        match (locks, choice) {
            (true, true) => heard(shared, &KEPT),
            (true, false) => heard(shared, &["kept"]),
            (false, true) => heard(shared, &["profile"]),
            (false, false) => {}
        }
    }
}

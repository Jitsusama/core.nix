//! What every kind of source but a cargo or installs one offers: a
//! producer's or podman's run, caches, Nix's store roots and kept builds,
//! scratch, snapshots, what git staged, quest workspaces, temporary files,
//! trashes, core dumps and logs.

use super::{Action, Offer, World, changed, device, named, resolved, since_ran, size, sized};
use crate::activity;
use crate::band::Band;
use crate::config::{CacheProtocol, Nix, Source};
use crate::git;
use crate::plan::{Candidate, Class};
use crate::units::Span;
use crate::{
    age, caches, coredumps, logs, nix_builds, scratch, snapshots, store_roots, temp, trash,
    workspaces,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How deep and how widely a workspace's writes are read.
const WRITE_DEPTH: usize = 4;
const WRITE_BUDGET: usize = 50_000;

/// A producer's run, offered whatever it would free, since that is measured once it has run.
pub(super) fn offer_producer(name: &str, source: &Source, world: &World, offers: &mut Vec<Offer>) {
    let Source::Producer {
        command,
        path,
        every,
        from,
        watch,
        timeout,
    } = source
    else {
        return;
    };
    let Some(device) = device(path) else { return };
    let idle = if changed(
        watch,
        world.ran.get(name).copied(),
        world.said.get(name).copied(),
    ) {
        f64::INFINITY
    } else {
        since_ran(world, name)
    };
    offers.push(Offer {
        candidate: Candidate {
            path: named(path, name),
            class: Class::Producer {
                every: every.map(|span| span.0.as_secs_f64()),
                from: *from,
            },
            idle,
            // What it frees is measured once it has run.
            size: 0,
            occupied: false,
            spent: false,
            checkout: None,
        },
        action: Action::Run {
            root: path.clone(),
            source: name.to_owned(),
            argv: command.clone(),
            timeout: timeout.0,
        },
        device,
    });
}

/// A cache's offer: each entry of a Go build cache, or else the whole cache.
pub(super) fn offer_cache(
    path: &Path,
    from: Band,
    protocol: Option<CacheProtocol>,
    rank: u8,
    world: &World,
    offers: &mut Vec<Offer>,
) {
    let Some(device) = device(path) else { return };
    match protocol {
        Some(CacheProtocol::GoBuild) => {
            for entry in caches::go_build(path) {
                let candidate = caches::go_entry(&entry, rank, from, world.awake, world.now);
                offers.push(Offer {
                    candidate,
                    action: Action::CacheEntry {
                        root: path.to_owned(),
                    },
                    device,
                });
            }
        }
        None => {
            let Some(parent) = path.parent() else {
                return;
            };
            // Idle since anything in it was last written, or not at all
            // where that can't be told, so only a band that takes it at
            // once does.
            let recent = world.now - crate::plan::CACHE_UNWRITTEN;
            let idle = caches::last_written(path, caches::WALKED, recent).map_or(0.0, |written| {
                world
                    .awake
                    .since(written.min(world.now), world.now)
                    .max(0.0)
            });
            let candidate = caches::whole(path, rank, from, size(path, world), idle);
            offers.push(Offer {
                candidate,
                action: Action::Entry {
                    root: parent.to_owned(),
                },
                device,
            });
        }
    }
}

/// The links under `roots` Nix's indirect roots name and `pattern` matches,
/// but for any in a tree a process works in. Where processes work is read
/// once, as there can be hundreds of links to ask of.
pub(super) fn offer_store_roots(
    roots: &[PathBuf],
    pattern: &str,
    unused: Span,
    nix: &Nix,
    world: &World,
    offers: &mut Vec<Offer>,
) {
    let Ok(pattern) = regex_lite::Regex::new(pattern) else {
        return;
    };
    let places = activity::Workplaces::new(world.processes, world.me);
    let auto = nix.roots.join("auto");
    let unused = unused.0.as_secs_f64();
    for root in roots {
        let Some(device) = device(root) else { continue };
        let root = resolved(root);
        for link in store_roots::found(&auto, &root, &pattern, &nix.store) {
            if places.work_in(&link.tree) {
                continue;
            }
            offers.push(Offer {
                candidate: store_roots::candidate(&link, unused, world.awake, world.now),
                action: Action::Entry { root: root.clone() },
                device,
            });
        }
    }
}

/// A scratch directory's entries.
pub(super) fn offer_scratch(
    path: &Path,
    healthy: Option<Span>,
    world: &World,
    offers: &mut Vec<Offer>,
) {
    let Some(device) = device(path) else { return };
    let healthy = healthy.map(|span| span.0.as_secs_f64());
    for entry in scratch::entries(path) {
        let size = size(&entry.path, world);
        let candidate = scratch::candidate(&entry, healthy, size, world.awake, world.now);
        offers.push(Offer {
            candidate,
            action: Action::Entry {
                root: path.to_owned(),
            },
            device,
        });
    }
}

/// The snapshots matching `pattern` that no process running `command` keeps.
pub(super) fn offer_snapshots(
    path: &Path,
    pattern: &str,
    command: &str,
    world: &World,
    offers: &mut Vec<Offer>,
) {
    let (Some(device), Ok(pattern)) = (device(path), regex_lite::Regex::new(pattern)) else {
        return;
    };
    for snapshot in snapshots::entries(path, &pattern) {
        if snapshots::orphaned(&snapshot, world.processes, &[command], world.boot) {
            let candidate = snapshots::candidate(
                &snapshot,
                size(&snapshot.path, world),
                world.awake,
                world.now,
            );
            offers.push(Offer {
                candidate,
                action: Action::Entry {
                    root: path.to_owned(),
                },
                device,
            });
        }
    }
}

/// What git staged in the repository whose git directory is `repo`, once
/// no git works on it. Each goes whole, on the next pass, however young:
/// git removes them as it exits, so one still there with no git at work
/// is one a killed git left, and waiting on its age only keeps it.
pub(super) fn offer_git(repo: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(repo) else { return };
    if git::busy(&git::places(repo), world.processes, world.uid) {
        return;
    }
    let root = repo.join("objects");
    for path in git::temporaries(repo) {
        let size = size(&path, world);
        offers.push(Offer {
            candidate: git::candidate(path, size),
            action: Action::Entry { root: root.clone() },
            device,
        });
    }
}

/// Which quests live sessions work on, as a survey of workspaces knows it.
pub(super) enum Worked {
    /// No session records are named, so each entry goes by its own age.
    Unknown,
    /// They're named but can't be read, so any quest may be.
    Any,
    These(HashSet<String>),
}

impl Worked {
    pub(super) fn read(sessions: Option<&Path>, rosters: Option<&Path>, world: &World) -> Self {
        let Some(sessions) = sessions else {
            return Worked::Unknown;
        };
        workspaces::worked_on(sessions, rosters, world.processes, &world.boot)
            .map_or(Worked::Any, Worked::These)
    }
}

/// The quest workspaces in `path`, judged by their quests' READMEs in
/// `quests`, and in each one still at work the browser profiles its `tmp/`
/// holds at rest and what else it left there unwritten, but for what a
/// process works in. Use is judged only of an entry old enough to offer,
/// and before it's sized, which walks all of it; where processes work is
/// read once, as there can be tens of thousands of entries to ask of.
/// A quest a live session works on, as `worked` says, or every quest when
/// that can't be told, keeps all it left beside `tmp/`, and in `tmp/` what
/// was written within `scratch::LIVE_QUEST_UNWRITTEN` anywhere in it or
/// holds work, as `scratch::live_reading` reads it; what goes is read
/// again just before it does.
pub(super) fn offer_workspaces(
    path: &Path,
    quests: &Path,
    worked: &Worked,
    world: &World,
    offers: &mut Vec<Offer>,
) {
    let Some(device) = device(path) else { return };
    let places = activity::Workplaces::new(world.processes, world.me);
    let worked_in = |path: &Path| places.work_in(path);
    let (over, working): (Vec<_>, Vec<_>) = workspaces::find(path, quests)
        .into_iter()
        .partition(workspaces::over);
    let mut left = scratch::LIVE_SURVEY_BUDGET;
    for workspace in working {
        let tmp = workspace.path.join("tmp");
        let profiles = temp::profiles(&tmp, world.uid, world.now);
        for entry in &profiles {
            offers.push(Offer {
                candidate: temp::profile_candidate(entry, world.awake, world.now),
                action: Action::Entry {
                    root: resolved(&tmp),
                },
                device,
            });
        }
        let id = workspace.path.file_name().and_then(|name| name.to_str());
        let live = match worked {
            Worked::Unknown => false,
            Worked::Any => true,
            Worked::These(quests) => id.is_some_and(|id| quests.contains(id)),
        };
        // A quest a live session works on keeps what it wrote lately, what
        // may be found nowhere else, its build roots and all beside tmp/:
        // only what it left two days unwritten in tmp/ goes.
        let unwritten = if live {
            scratch::LIVE_QUEST_UNWRITTEN
        } else {
            scratch::QUEST_UNWRITTEN
        };
        let a_profile = |path: &Path| profiles.iter().any(|profile| profile.path == path);
        for mut entry in scratch::entries(&tmp) {
            let young = scratch::idle(&entry, world.awake, world.now) < unwritten;
            if young || a_profile(&entry.path) || worked_in(&entry.path) {
                continue;
            }
            // The shallow date can miss a live session's edit far down, so
            // what passes it is read whole, and dated again by that.
            let whole = live && !(world.stop)() && scratch::dated_whole(&mut entry, &mut left);
            if live && (!whole || scratch::idle(&entry, world.awake, world.now) < unwritten) {
                continue;
            }
            let size = || sized(&entry.path, entry.written, world);
            let candidate = scratch::quest_candidate(&entry, size, world.awake, world.now);
            if let Some(candidate) =
                candidate.filter(|found| !live || found.class == Class::QuestTemp)
            {
                let root = resolved(&tmp);
                let action = if live {
                    Action::LiveEntry {
                        root,
                        written: entry.written,
                    }
                } else {
                    Action::Entry { root }
                };
                offers.push(Offer {
                    candidate,
                    action,
                    device,
                });
            }
        }
        if live {
            continue;
        }
        for entry in scratch::beside_tmp(&workspace.path) {
            let idle = scratch::idle(&entry, world.awake, world.now);
            if idle < scratch::QUEST_SCRATCH_UNWRITTEN || worked_in(&entry.path) {
                continue;
            }
            let size = || sized(&entry.path, entry.written, world);
            if let Some(candidate) =
                scratch::quest_scratch_candidate(&entry, size, world.awake, world.now)
            {
                offers.push(Offer {
                    candidate,
                    action: Action::Entry {
                        root: resolved(&workspace.path),
                    },
                    device,
                });
            }
        }
    }
    for workspace in over {
        let written = age::last_write(&workspace.path, WRITE_DEPTH, WRITE_BUDGET);
        let size = sized(&workspace.path, written, world);
        if let Some(candidate) =
            workspaces::candidate(&workspace, written, size, world.awake, world.now)
        {
            offers.push(Offer {
                candidate,
                action: Action::Workspace {
                    root: path.to_owned(),
                },
                device,
            });
        }
    }
}

/// What's left in a temporary directory, but for any a process names as its
/// `TMPDIR`.
pub(super) fn offer_temp(path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let path = &resolved(path);
    let Some(device) = device(path) else { return };
    let tmpdirs: Vec<PathBuf> = world
        .processes
        .iter()
        .flat_map(|process| &process.env)
        .filter(|(name, _)| name == "TMPDIR")
        .map(|(_, value)| PathBuf::from(value))
        .collect();
    let boot = world.boot.wall;
    for entry in temp::leftovers(temp::survey(path), world.uid, &tmpdirs, boot) {
        let candidate = temp::candidate(&entry, world.awake, world.now, boot);
        offers.push(Offer {
            candidate,
            action: Action::Entry {
                root: path.to_owned(),
            },
            device,
        });
    }
}

/// A desktop trash's items.
pub(super) fn offer_trash(path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    for item in trash::items(path) {
        let candidate = trash::candidate(&item, world.awake, world.now);
        offers.push(Offer {
            candidate,
            action: Action::Trash {
                trash: path.to_owned(),
            },
            device,
        });
    }
}

/// The core dumps kept in `path`.
pub(super) fn offer_coredumps(path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    for dump in coredumps::find(path) {
        let candidate = coredumps::candidate(&dump, world.awake, world.now);
        offers.push(Offer {
            candidate,
            action: Action::Entry {
                root: path.to_owned(),
            },
            device,
        });
    }
}

/// The build directories Nix kept in `path` whose builder is gone.
pub(super) fn offer_nix_builds(path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    for build in nix_builds::find(path) {
        if nix_builds::orphaned(&build, world.processes, world.boot) {
            let size = size(&build.snapshot.path, world);
            let candidate = nix_builds::candidate(&build, size, world.awake, world.now);
            offers.push(Offer {
                candidate,
                action: Action::Entry {
                    root: path.to_owned(),
                },
                device,
            });
        }
    }
}

/// The logs beneath `path` past their limit, each offering what trimming it
/// drops.
pub(super) fn offer_logs(path: &Path, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    for log in logs::oversized(path, logs::LIMIT) {
        let Ok(meta) = fs::symlink_metadata(&log) else {
            continue;
        };
        let candidate = Candidate {
            path: log,
            class: Class::Log,
            idle: 0.0,
            size: meta.len().saturating_sub(logs::KEEP),
            occupied: false,
            spent: false,
            checkout: None,
        };
        offers.push(Offer {
            candidate,
            action: Action::Trim {
                root: path.to_owned(),
            },
            device,
        });
    }
}

/// Podman's two prunes: its build cache, and every image no container uses.
pub(super) fn offer_podman(
    name: &str,
    command: &Path,
    path: &Path,
    world: &World,
    offers: &mut Vec<Offer>,
) {
    let Some(device) = device(path) else { return };
    let program = command.to_string_lossy().into_owned();
    for (unused, source, how) in [
        (false, name.to_owned(), "--build-cache"),
        (true, format!("{name}:unused"), "--all"),
    ] {
        let argv = [&program, "image", "prune", "--force", how];
        offers.push(Offer {
            candidate: Candidate {
                path: named(path, &source),
                class: Class::Podman { unused },
                idle: since_ran(world, &source),
                size: 0,
                occupied: false,
                spent: false,
                checkout: None,
            },
            action: Action::Run {
                root: path.to_owned(),
                source,
                argv: argv.map(str::to_owned).to_vec(),
                timeout: PODMAN_TIMEOUT,
            },
            device,
        });
    }
}

/// How long podman's prunes may take.
const PODMAN_TIMEOUT: Duration = Duration::from_secs(300);

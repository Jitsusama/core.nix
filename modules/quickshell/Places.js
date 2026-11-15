.pragma library
// Where Reopen Them All puts each session back (Sessions.qml), from the
// places kept for it: tests/places.qml holds it to the cases below. A kept
// place names its screen by what the screen is, its maker, model and
// serial as niri describes it, since a dock or a boot can give the same
// screen another connector: on 2026-11-01 the same monitor came up as DP-5
// after a boot as DP-4, and every session kept on DP-4 opened on the
// laptop's own screen.

// A screen as niri describes it, as its config names one: "HP Inc. HP
// Z27k G3 CN41282GC1". niri's answer to Outputs gives each by connector.
function described(output) {
    return [output?.make, output?.model, output?.serial]
        .filter(part => typeof part === "string" && part !== "")
        .join(" ");
}

// Each connector's description, from niri's answer to Outputs, but for a
// screen that's off, as the laptop's is while its lid is shut on the dock:
// niri still lists it, with no place in the layout (`logical` null), and it
// holds no workspace, so it's as gone as one unplugged. Counted as there,
// every workspace noted away from it went "home" to it at each change of
// the workspaces, putting them back in the order they had on it whenever
// Joel moved one (P9R1KI, 2026-11-03).
function screensOf(outputs) {
    const screens = {};
    for (const [name, output] of Object.entries(outputs ?? {})) {
        if (output?.logical !== null) {
            screens[name] = described(output);
        }
    }
    return screens;
}

// The connector a kept place's screen is on now, or undefined when it's
// gone: found by what it is, the one it was kept on when two screens are
// alike, as two monitors of one model without serials are, or by its
// connector when it was kept before screens were named so.
function connectorOf(at, screens) {
    if (at.screen) {
        const alike = Object.keys(screens).filter(name => screens[name] === at.screen);
        return alike.includes(at.output) ? at.output : alike[0];
    }
    return at.output in screens ? at.output : undefined;
}

// Where each kept place goes now, in the order given, null for none: on
// its own screen, wherever it's connected now. One whose screen is gone
// goes to the screen in use, its workspaces after every workspace the
// places there use, in their order, as niri moves a gone screen's
// workspaces, rather than mixed into the same numbers, and says where it
// belongs (`away`), so its workspace can go there once the screen comes.
// `screens` maps each connector there is to its description; with none
// known, as when niri didn't answer, or no screen in use, a gone one is
// left as it was kept.
function placed(places, screens, inUse) {
    const known = Object.keys(screens ?? {}).length > 0;
    const found = places.map(at => {
        if (!at) {
            return null;
        }
        const output = known ? connectorOf(at, screens) : at.output;
        return output === undefined ? { at, gone: at.screen || at.output } : { at, output };
    });
    let last = 0;
    for (const one of found) {
        if (one?.output !== undefined && one.output === inUse) {
            last = Math.max(last, one.at.workspace);
        }
    }
    const goneScreens = found
        .filter(one => one?.gone !== undefined)
        .map(one => one.gone)
        .filter((gone, at, all) => all.indexOf(gone) === at)
        .sort();
    const after = {};
    for (const gone of goneScreens) {
        after[gone] = last;
        for (const one of found) {
            if (one?.gone === gone) {
                last = Math.max(last, after[gone] + one.at.workspace);
            }
        }
    }
    return found.map(one => {
        if (one === null) {
            return null;
        }
        if (one.output !== undefined) {
            return Object.assign({}, one.at, { output: one.output });
        }
        if (inUse === null || inUse === undefined) {
            return Object.assign({}, one.at);
        }
        return Object.assign({}, one.at, {
            output: inUse,
            workspace: after[one.gone] + one.at.workspace,
            away: { screen: one.gone, workspace: one.at.workspace }
        });
    });
}

// The moves that take workspaces given in place of a gone screen to it once
// it's connected again. `away` holds, by each such screen's description (or
// its connector, for a place kept before screens were named so), the
// workspaces given in its place, by niri's id, each with the number it was
// kept at there. The moves go lowest kept number first; their order there
// is the windows daemon's to put back, as Joel last had it. One gone already
// isn't moved, and one on the screen already is marked (`there`) and left.
// `home` names the screens connected again, which are done with.
function homeward(away, screens, workspaces) {
    const moves = [];
    const home = [];
    for (const screen of Object.keys(away ?? {}).sort()) {
        const output = Object.keys(screens ?? {})
            .find(name => screens[name] === screen || name === screen);
        if (output !== undefined) {
            home.push(screen);
            const ranked = away[screen]
                .filter(one => workspaces.some(each => each.id === one.id))
                .sort((a, b) => a.workspace - b.workspace);
            for (const one of ranked) {
                const now = workspaces.find(each => each.id === one.id);
                moves.push(now.output === output ? { output, id: one.id, there: true }
                    : { output, id: one.id });
            }
        }
    }
    return { moves, home };
}

// Which window kept, of a boot's windows other than its sessions, a window
// opening now is: the first not used yet of the same program with the same
// title, else the first of that program, since a page's title can change
// before its window shows. -1 for none. `used` lists the indices taken.
function matchOther(kept, used, app, title) {
    const free = (one, at) => !used.includes(at) && one.app === app;
    const same = kept.findIndex((one, at) => free(one, at) && one.title === title);
    return same >= 0 ? same : kept.findIndex(free);
}

// Where a window put back goes, beside the windows already back from the
// same boot, rather than by its workspace's number: a workspace that held no
// session isn't made again as sessions are reopened, so every number after
// it is one less. `beside` holds each window back: its id, its kept place,
// the workspace it's on now and its column there. It joins the workspace
// holding one kept on its screen at its number (`join`): just after the
// nearest kept left of it (`after`), else just before the nearest kept
// right of it (`before`), by window rather than by column number, since a
// window Joel opened since can sit among them. One kept below another in its
// column goes under it (`under`). Else it's given a workspace
// of its own (`make`), just after the one holding those kept at the next
// number below on its screen (`output`, `index`), or first, on a screen
// unknown, when none is. One below that sits on another screen than this
// one's, as one that opened while every screen was gone sits on one
// standing in for its own, isn't followed there: its number says nothing
// of this screen's.
function whereBack(at, beside) {
    const screenOf = one => one.screen || one.output;
    const there = beside.filter(one => screenOf(one.at) === screenOf(at));
    const joined = there.find(one => one.at.workspace === at.workspace);
    if (joined !== undefined) {
        const into = joined.workspace;
        const kin = there
            .filter(one => one.workspace.id === into.id && one.at.workspace === at.workspace)
            .sort((a, b) => a.at.column - b.at.column || tileOf(a.at) - tileOf(b.at)
                || a.column - b.column);
        const left = kin.filter(one => one.at.column < at.column
            || one.at.column === at.column && tileOf(one.at) < tileOf(at));
        if (left.length > 0) {
            const after = left[left.length - 1];
            return after.at.column === at.column ? { join: into.id, after: after.id, under: true }
                : { join: into.id, after: after.id };
        }
        return { join: into.id, before: kin[0].id };
    }
    const below = there
        .filter(one => one.at.workspace < at.workspace)
        .filter(one => !at.output || one.workspace.output === at.output)
        .sort((a, b) => b.at.workspace - a.at.workspace)[0];
    if (below === undefined) {
        return { make: true, output: null, index: 1 };
    }
    return { make: true, output: below.workspace.output, index: below.workspace.idx + 1 };
}

// A kept place's tile in its column, the first when none was kept, as for
// the windows of other programs kept before tiles were.
function tileOf(at) {
    return at.tile ?? 1;
}

// Whether a window is where whereBack says already, so nothing need move:
// `now` is the workspace it's on and its column there, and `beside` the
// windows back before it, as whereBack had them. One given a workspace of
// its own holds while it's alone there among them, on the screen named
// (`output`, when one is), whatever number that workspace has now: a
// number is only where it stands, which a workspace emptied or made
// before it moves, and chasing one moved 17 windows on screen at the
// 08:06 boot on 2026-11-09. Checked for each window back, the lowest kept
// first, after Reopen Them All, since a window can open anywhere while
// Joel moves the focus or a screen comes and goes.
function holds(where, now, beside) {
    if (where.make) {
        return !beside.some(one => one.workspace.id === now.workspace.id)
            && (where.output === null || now.workspace.output === where.output);
    }
    if (now.workspace.id !== where.join) {
        return false;
    }
    const next = beside.find(one => one.id === (where.after ?? where.before));
    if (where.after !== undefined) {
        return now.column === next.column + (where.under ? 0 : 1);
    }
    return now.column === next.column - 1;
}

// The column, from 1, a window on the workspace whereBack joins goes to:
// just right of the one it goes after, or just left of the one it goes
// before, counted as niri counts once the window has left its own column.
function columnTo(where, column, beside) {
    const next = beside.find(one => one.id === (where.after ?? where.before));
    if (where.after !== undefined) {
        return column > next.column ? next.column + 1 : next.column;
    }
    return column < next.column ? next.column - 1 : next.column;
}

// The column, from 1, a window not open yet opens as on the workspace
// whereBack joins: just right of the one it goes after, or where the one it
// goes before is, which moves right. One going under another opens beside it
// and is taken into its column after. null for one given a workspace of its
// own, which isn't there to open on.
function opensAt(where, beside) {
    if (where.make) {
        return null;
    }
    const next = beside.find(one => one.id === (where.after ?? where.before));
    return where.after !== undefined ? next.column + 1 : next.column;
}

// How much of the disk's time may be stalled, as the share of the last ten
// seconds some task waited on it, before the next session waits too: past
// it git, rustc and nix sit in D state and journalctl stops answering.
const ioStalls = 50;
// How old the queue's heartbeat may be, in seconds, and still be heard.
const queueHeard = 120;
// How fast the uplink may already send, in Mbit/s, before the next
// session is told to carry on: the evening's rate on 2026-11-14, which
// the builders sharing the home line lived with, where the 22:24 boot's
// seventy-two told at once sent 6.4 of the 8.2 it had for 17 minutes.
const uplinkRoom = 4.5;
// While the queue says nothing of the uplink, this many told and still
// working at once, each counted until it goes idle or this many seconds
// pass since it was told.
const fewAtOnce = 4;
const fewPatience = 60;

// Where a session to reopen sat, by its id: this boot's place while it
// has one, else the restored boot's. An orchestrator bringing its own
// sessions back after a reboot hands them over before Reopen Them All
// reaches them, and without a kept place each went to its orchestrator's
// workspace wherever niri put it (40 of them after the 21:24 boot on
// 2026-11-08). One pi's host gave back this boot sat this boot, where its
// viewer was, not where the boot before left it. `kept` holds `now` and
// `restored`, each places by id.
function keptFor(id, kept) {
    return kept.now?.[id] ?? kept.restored?.[id] ?? null;
}

// Reopen Them All's queue, `{ one, kept }` each, with those whose kept
// place says it `leads` (it orchestrated others as it was kept) first,
// each part in the order it came, so the sessions an orchestrator leads
// have it back to answer to as they return. Where each goes doesn't hang on
// the order begun, since each is placed by the kept places of those back.
function leadersFirst(queue) {
    const leads = entry => entry.kept?.leads === true;
    return queue.filter(leads).concat(queue.filter(entry => !leads(entry)));
}

// The address of the workspace a session goes home to, its orchestrator's,
// as Sessions.qml's opensOn finds it (Joel, 2026-11-13 16:32: "agent
// sessions only land on the workspace of their orchestrator"): the
// `workspace` the harness hands it over with or kept it by, the top
// orchestrator over its quest, else the orchestrator it says, else the one
// its place kept last boot names it under (`kept.under`); "" for none. A
// word but an address's is no home.
const anAddress = /^[A-Z0-9]+$/u;
function homeOf(one, kept) {
    const address = word => typeof word === "string" && anAddress.test(word) ? word : "";
    return address(one?.workspace) || address(one?.orchestratedBy) || address(kept?.under);
}

// The name the workspace `into` takes as an orchestrator opens on it, by
// its id, as Sessions.qml's opensOn finds it: its `home`, when `one` is
// the orchestrator of that home and no workspace has the name yet, so a
// session of its own back before its pi says who it is joins it there
// (Joel, 2026-11-14 22:36: one made a workspace of its own, two seconds
// after its orchestrator came back, and stayed alone on it). "" for any
// other: a workspace already named, for this home or another's, keeps its
// name. `workspaces` are niri's, { id, name } each.
function homeToName(one, home, into, workspaces) {
    const leads = one?.address === home || one?.orchestrates === home;
    const there = workspaces.find(space => space.id === into);
    if (home === "" || !leads || !there || there.name) {
        return "";
    }
    return workspaces.some(space => space.name === home) ? "" : home;
}

// Where a session started under an orchestrator opens, { workspace, column }
// as opensOn gives it: on the orchestrator's workspace, the column after the
// last of the sessions it orchestrates there whose title comes before this
// one's, the orchestrator itself always first, so its sessions stand in title
// order after it (P9R1KI, 2026-11-08 [38061f98]: a session started for
// another orchestrator's tree opened on the starter's workspace, beside the
// focused column). One with no title goes after them all. `orchestrator` and
// each of `under` are { workspace, column, title }; with no orchestrator,
// null.
function underOrchestrator(title, orchestrator, under) {
    if (!orchestrator) {
        return null;
    }
    const there = under.filter(one => one.workspace === orchestrator.workspace);
    const before = title === "" ? there
        : there.filter(one => one.title.localeCompare(title) < 0);
    const column = Math.max(orchestrator.column, ...before.map(one => one.column)) + 1;
    return { workspace: orchestrator.workspace, column };
}

// Reopen Them All's queue with `more` joined at its end, but for each one
// it holds already or has `begun`, by id: an orchestrator handing over its
// own sessions as it comes back, while the queue still holds them, would
// otherwise begin each twice, two pis on one log, before either has said
// it runs here.
function queuedOnce(queue, more, begun) {
    const asked = new Set(queue.map(entry => entry.one.id).concat(begun));
    return queue.concat(more.filter(entry => !asked.has(entry.one.id)));
}

// Those of `pending` (each `{ id, wants, carried, leads }`, as reopened,
// `wants` when it waits on Joel) to tell to carry on now, in the order
// they're told: those waiting on Joel, then those carried in from another
// machine, then leaders, then
// the rest, each part in the order given. `told` is those told already,
// `{ id, at, idle }` each, `at` in seconds as `now` is, and none of them is
// told again. With the machine queue's `text` saying the uplink sends under
// uplinkRoom, one; past it, none. With a queue that says nothing of the
// uplink, as before the machine queue measured it, fewAtOnce working at
// once. A queue unread or silent past `queueHeard` says nothing of the
// line, so all are told, as mayBegin holds nothing back.
function carryOnsDue(pending, told, text, now) {
    const done = new Set(told.map(one => one.id));
    const left = pending.filter(one => !done.has(one.id));
    const first = [one => one.wants === true, one => Boolean(one.carried),
        one => one.leads === true];
    const rank = one => {
        const at = first.findIndex(is => is(one));
        return at === -1 ? first.length : at;
    };
    const ordered = left.map((one, at) => ({ one, at }))
        .sort((a, b) => rank(a.one) - rank(b.one) || a.at - b.at).map(entry => entry.one);
    let read = null;
    try {
        read = JSON.parse(text);
    } catch (_error) {
        return ordered;
    }
    const machine = read?.machine;
    if (typeof read?.heartbeat !== "number" || now - read.heartbeat > queueHeard || !machine) {
        return ordered;
    }
    const sent = machine.uplink?.sentMbit;
    if (typeof sent === "number") {
        return ordered.slice(0, sent < uplinkRoom ? 1 : 0);
    }
    const working = told.filter(one => !one.idle && now - one.at < fewPatience).length;
    return ordered.slice(0, Math.max(0, fewAtOnce - working));
}

// Whether Reopen Them All may begin the next session now, from the machine
// queue's own verdict (`text`, /run/user/$UID/jobs/queue.json): not while
// it says the memory or the processors are over, or while the disk stalls,
// as eighty sessions begun two seconds apart did eleven minutes after the
// 09:23 boot on 2026-11-08 (load 245, IO pressure 98%, 16 GB swapped). A
// queue unread, or silent past `queueHeard`, holds nothing back, so a
// reopening never waits on a queue that's gone. `now` is in seconds since
// the epoch, as the heartbeat is.
function mayBegin(text, now) {
    let read = null;
    try {
        read = JSON.parse(text);
    } catch (_error) {
        return true;
    }
    const machine = read?.machine;
    if (typeof read?.heartbeat !== "number" || now - read.heartbeat > queueHeard || !machine) {
        return true;
    }
    return (machine.over ?? []).length === 0 && (machine.pressure?.io ?? 0) < ioStalls;
}

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

// connectorOf, beside the places kept with `at` at once: an alike screen
// another was kept on is that one, so with only it connected `at`'s is gone.
function whereKept(at, beside, screens) {
    const taken = beside.filter(one => one && one !== at && at.screen && one.screen === at.screen
        && one.output !== at.output).map(one => one.output);
    const found = connectorOf(at, screens);
    if (found === undefined || found === at.output || !taken.includes(found)) {
        return found;
    }
    return Object.keys(screens).find(name => screens[name] === at.screen && !taken.includes(name));
}

// The other screens as `at` is found gone (othersAt): those connected, and
// those of the places kept with it (`beside`) gone too, as alike twins are.
function othersGone(at, beside, screens) {
    const gone = beside.filter(one => one && one !== at && one.output
        && whereKept(one, beside, screens) === undefined);
    const heard = Object.assign({}, ...gone.map(one => ({ [one.output]: one.screen || one.output })), screens);
    return othersAt(at.output, Object.keys(screens), gone.map(one => one.output), heard);
}

// Where each kept place goes now, in the order given, null for none: on
// its own screen, wherever it's connected now. One whose screen is gone
// goes to the screen in use, its workspaces after every workspace the
// places there use, in their order, as niri moves a gone screen's
// workspaces, rather than mixed into the same numbers, and says where it
// belongs (`away`), so its workspace can go there once the screen comes,
// with the other screens as it was found gone (`others`, of othersGone)
// beside those of the same boot (`boots`, each place's; none for one boot).
// `screens` maps each connector there is to its description; with none
// known, as when niri didn't answer, or no screen in use, a gone one is
// left as it was kept.
function placed(places, screens, inUse, boots) {
    const known = Object.keys(screens ?? {}).length > 0;
    const besideOf = index => places.filter((_, at) => (boots?.[at] ?? "") === (boots?.[index] ?? ""));
    const found = places.map((at, index) => {
        if (!at) {
            return null;
        }
        const output = known ? whereKept(at, besideOf(index), screens) : at.output;
        return output === undefined ? { at, gone: at.screen || at.output, index } : { at, output };
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
            away: {
                screen: one.gone,
                workspace: one.at.workspace,
                output: one.at.output,
                others: othersGone(one.at, besideOf(one.index), screens)
            }
        });
    });
}

// The moves that take workspaces given in place of a gone screen to it once
// it's connected again. `away` holds, by each screen's description (or its
// connector, for a place kept before screens were named so), the workspaces
// given in its place, by niri's id, each with the number it was kept at
// there, the connector it left (`output`), if known, and the other screens
// as it left (`others`, of othersAt). Each decides on its own, by wayBack,
// lowest kept number first, and the moves go so; their order there is the
// windows daemon's to put back. One gone already is dropped; one on its
// screen already is marked (`there`) and left. `waiting` is `away` with
// those still waiting, each knowing a connector a sibling that was another
// screen as it left (or the other way) took as another's, kept for later.
function homeward(away, screens, workspaces) {
    const moves = [];
    const waiting = {};
    for (const screen of Object.keys(away ?? {}).sort()) {
        const given = [];
        const held = [];
        const live = away[screen]
            .filter(one => workspaces.some(each => each.id === one.id))
            .sort((a, b) => a.workspace - b.workspace);
        for (const one of live) {
            const claimed = given.filter(was => one.others?.[was.from] === screen
                || was.one.others?.[one.output] === screen).map(was => was.to);
            const output = wayBack(one, screen, screens, claimed);
            if (output === null) {
                const others = Object.assign({}, one.others, ...claimed.map(name => ({ [name]: screen })));
                held.push(claimed.length > 0 ? Object.assign({}, one, { others }) : one);
                continue;
            }
            given.push({ to: output, from: one.output, one });
            const now = workspaces.find(each => each.id === one.id);
            moves.push(now.output === output ? { output, id: one.id, there: true } : { output, id: one.id });
        }
        if (held.length > 0) {
            waiting[screen] = held;
        }
    }
    return { moves, waiting };
}

// Where a workspace noted away from `screen` goes now, of `screens` by
// connector, or null while it waits. The connector it left, once that's the
// screen again, is home. Else any screen alike is, but one that was another
// screen alike it as it left, or one not heard yet then, which may have
// been: two monitors of one model without serials, or a VM's two, describe
// themselves the same, and taking the one it was sent to for home dropped
// its note before the one it left came back (vm-screens-keep-places,
// 2026-11-18). `claimed` are those taken just now by a sibling that was
// another screen as it left, so two alike back on new connectors take one
// each. Where it sits now doesn't count, so one Joel moves while it waits
// stays where he put it.
function wayBack(one, screen, screens, claimed) {
    const alike = Object.keys(screens ?? {})
        .filter(name => screens[name] === screen || name === screen)
        .sort();
    if (alike.includes(one.output)) {
        return one.output;
    }
    const others = one.others ?? {};
    const notIt = name => name in others && (others[name] === screen || others[name] === "");
    return alike.find(name => !notIt(name) && !claimed.includes(name)) ?? null;
}

// The connectors other screens as a workspace leaves `left`, by `screens`'
// descriptions of them, "" for one not heard yet: those connected
// (`outputs`) and those gone in the same moment (`lefts`), each once.
function othersAt(left, outputs, lefts, screens) {
    const others = {};
    for (const name of outputs.concat(lefts)) {
        if (name !== left && !(name in others)) {
            others[name] = screens?.[name] ?? "";
        }
    }
    return others;
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
// Which boot keptFor took a session's place from: "now", "restored" or "".
function keptFrom(id, kept) {
    return ["now", "restored"].find(boot => (kept[boot]?.[id] ?? null) !== null) ?? "";
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

// The file a pi host keeps its own list of the sessions it runs in, in the
// harness's hosts folder, for the host a hosted presence's `owner` names:
// `<pid>-<start>.json`, its start the process's start ticks where the
// owner says them, else its start in milliseconds, as the harness names
// it (its lib/core/session/registry.ts, hostRegistryPath). "" for an
// owner naming no host process.
function hostListOf(owner) {
    const pid = owner?.pid;
    const start = owner?.startTicks ?? owner?.startedAt;
    if (!Number.isInteger(pid) || pid <= 0 || !Number.isInteger(start)) {
        return "";
    }
    return `${pid}-${start}.json`;
}

// What a pi host's own list says of the session a hosted presence's
// `owner` names, as the harness judges it (hostedStanding there), from
// `text`, the list read from hostListOf's file: "running" when the list
// is that very host's and names the session, "gone" when it's that
// host's and doesn't, and "unknown" for no list, an unreadable one, or
// one of another host or another start of it, which can't say a session
// closed.
function hostSays(owner, text) {
    let read = null;
    try {
        read = JSON.parse(text);
    } catch (_error) {
        return "unknown";
    }
    const same = ["pid", "startedAt", "bootId", "startTicks"]
        .every(field => read?.owner?.[field] === owner?.[field]);
    if (!same || typeof read?.key !== "string" || read.key !== owner?.host
        || !Array.isArray(read?.sessions)) {
        return "unknown";
    }
    return read.sessions.some(one => one?.id === owner.session) ? "running" : "gone";
}

// How Reopen Them All finds the session pi knows as `id` here already,
// among `running` and `parked` as Sessions.qml lists them, as `{ as,
// session }`. "running" for one a pi runs, which is never begun again,
// since two pis on one log would each write over the other. "parked"
// for one parked here, which is woken rather than left asleep. "stale"
// for one whose presence says pi's host runs it while the host's own
// list, `listed(owner)` as hostSays reads it, says it doesn't, which is
// begun as any other; and "" for none. After the rollback drill of 15
// November, XUBK2E's stale hosted presence and 2RCBOJ's parked hold
// each kept it shut, said to run here (OSPHT4, [58d7fa8e]). A host's
// list unread is believed, as the harness believes it.
function foundHere(id, running, parked, listed) {
    const ofId = session => session.here && session.presence?.session === id;
    const asleep = parked.find(ofId);
    if (asleep !== undefined) {
        return { as: "parked", session: asleep };
    }
    const awake = running.find(ofId);
    if (awake === undefined) {
        return { as: "", session: null };
    }
    const owner = awake.presence.owner;
    if (awake.presence.hosted && owner?.host
        && listed(Object.assign({ session: id }, owner)) === "gone") {
        return { as: "stale", session: awake };
    }
    return { as: "running", session: awake };
}

// The folder a new session on `quest` starts in, as `text`, the harness's
// quest index, says its quest spawn chooses it, or "" for no index, no
// such quest, or a start that isn't a whole path.
function questStart(text, quest) {
    try {
        const start = (JSON.parse(text)?.quests ?? []).find(one => one?.id === quest)?.start;
        return typeof start === "string" && start.startsWith("/") ? start : "";
    } catch (_error) {
        return "";
    }
}

// A new session's id for pi, as the harness mints one: a UUID version 7,
// of `now` in milliseconds, then `random`'s answers, each in [0, 1).
function freshId(now, random) {
    const hex = (count) => Array.from({ length: count },
        () => Math.floor(random() * 16).toString(16)).join("");
    const time = Math.floor(now).toString(16).padStart(12, "0").slice(-12);
    const variant = (8 + Math.floor(random() * 4)).toString(16);
    return `${time.slice(0, 8)}-${time.slice(8)}-7${hex(3)}-${variant}${hex(3)}-${hex(12)}`;
}

// What Reopen Them All begins as `id` in `start` for `one`, whose folder
// is gone: a session afresh on its quest, as the harness's own reopen
// does (sessionToReopen), since pi resumes one only in its own folder;
// `afreshFor` names the session it stands in for.
function afresh(one, start, id) {
    return Object.assign({}, one, { id, cwd: start, fresh: true, afreshFor: one.id });
}

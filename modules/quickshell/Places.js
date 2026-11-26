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

// The connector a kept place's screen is on now, or undefined when it's gone: found by what
// it is, the one it was kept on when two screens are alike, as two monitors of one model
// without serials are, or by its connector when kept before screens were named so or, with
// none alike, when it's `inUse`, not heard yet (""), which holds a gone one's places anyway.
function connectorOf(at, screens, inUse) {
    const said = Object.keys(screens);
    const alike = said.filter(name => at.screen && screens[name] === at.screen);
    const own = at.screen ? at.output === inUse && screens[inUse] === "" : said.includes(at.output);
    return alike.includes(at.output) || (alike.length === 0 && own) ? at.output : alike[0];
}

// The connectors but `at`'s that one moment's places (`moment`, the other
// windows a boot kept at once) show `at`'s screen on beside `at`'s own: an
// alike twin for certain. A boot's sessions' places can't say so, since
// one ended keeps its place, which may be one monitor's old connector.
function twinsOf(at, moment) {
    const on = [...new Set((moment ?? []).filter(one => at.screen && one?.screen === at.screen)
        .map(one => one.output))];
    return on.includes(at.output) ? on.filter(output => output !== at.output) : [];
}

// Where a place kept at `at` is now, `output`, connectorOf's (given `inUse`) but not a twin's
// (twinsOf) nor one `given` a twin this pass (`{ at, output }` each), so with only those connected
// `at`'s is gone, or as kept with no screen said; and the note it carries (`away`, of awayOf).
function whereKept(at, moment, screens, { given, inUse } = {}) {
    const twins = twinsOf(at, moment);
    const taken = twins.concat((given ?? []).filter(was => twins.includes(was.at.output)).map(was => was.output));
    const found = Object.keys(screens).length > 0 ? connectorOf(at, screens, inUse) : at.output;
    const output = found === undefined || found === at.output || !taken.includes(found) ? found
        : Object.keys(screens).find(name => screens[name] === at.screen && !taken.includes(name));
    return { output, away: awayOf(at, output, moment, screens) };
}

// The note (homeward's) a place kept at `at` and found on `output` (whereKept) carries, or null:
// one gone, or on a screen not heard yet (""), may be another's, so it goes home when its own
// comes. The others (othersAt) are those heard, and its twins (twinsOf) gone or not heard yet;
// another not heard is left out, as it may be `at`'s own, renumbered, which wayBack refuses.
function awayOf(at, output, moment, screens) {
    if (output !== undefined && !(at.screen && screens[output] === "")) {
        return null;
    }
    const lefts = twinsOf(at, moment).filter(one => !screens[one]);
    const heard = Object.assign({}, screens, ...lefts.map(one => ({ [one]: at.screen })));
    const said = Object.keys(screens).filter(name => screens[name]);
    const others = othersAt(at.output, said, lefts, heard);
    return { screen: at.screen || at.output, workspace: at.workspace, output: at.output, others };
}

// Where each kept place goes now, in the order given, null for none: on its own screen, wherever
// it's connected now. One whose screen is gone goes to the screen in use, its workspaces after
// every workspace the places there use, in their order, as niri moves a gone screen's workspaces,
// rather than mixed into the same numbers, and says where it belongs (`away`, of awayOf, as one
// on a screen not heard yet does), so its workspace can go there once the screen comes, each twin
// (of `moments`, each place's boot's) given a connector of its own. `screens` is connected's;
// with none, as before niri says any, or no screen in use, a gone one is left as it was kept.
function placed(places, screens, inUse, moments) {
    const given = [];
    const pass = { given, inUse };
    const found = places.map((at, index) => {
        if (!at) {
            return null;
        }
        const { output, away } = whereKept(at, moments?.[index], screens ?? {}, pass);
        given.push({ at, output });
        const gone = output === undefined ? at.screen || at.output : undefined;
        return { at, output, gone, away };
    });
    let last = 0;
    for (const one of found) {
        if (one?.output !== undefined && one.output === inUse) {
            last = Math.max(last, one.at.workspace);
        }
    }
    const goneScreens = [...new Set(found.map(one => one?.gone))]
        .filter(gone => gone !== undefined).sort();
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
        const { at, output, gone, away } = one;
        if (output !== undefined) {
            return Object.assign({}, at, { output }, away ? { away } : {});
        }
        if (inUse === null || inUse === undefined) {
            return Object.assign({}, at);
        }
        const workspace = after[gone] + at.workspace;
        return Object.assign({}, at, { output: inUse, workspace, away });
    });
}

// Of `screens` (screensOf), those niri lists a workspace on, all while it lists none, then, once
// any is said, each other it lists as "", not heard: heard apart, either can be stale
// (vm-screens-keep-places, 21 and 22 Nov). One screensOf left out as off is one so too: niri
// moves an off screen's workspaces off it, so only a stale list still puts one there.
function connected(screens, workspaces) {
    const listed = new Set((workspaces ?? []).map(each => each.output));
    const said = Object.keys(screens ?? {});
    const heard = said.filter(name => listed.size === 0 || listed.has(name));
    const unheard = said.length === 0 ? [] : [...listed]
        .filter(name => typeof name === "string" && !heard.includes(name));
    return heard.concat(unheard)
        .reduce((there, name) => Object.assign(there, { [name]: screens[name] ?? "" }), {});
}

// The moves that take workspaces given in place of a gone screen to it once
// it's connected again. `away` holds, by each screen's description (or its
// connector, for a place kept before screens were named so), the workspaces
// given in its place, by niri's id, each with the number it was kept at
// there, the connector it left (`output`), if known, and the other screens
// as it left (`others`, of othersAt). Each decides on its own, by wayBack,
// lowest kept number first, and the moves go so; their order there is the
// windows daemon's to put back. One gone already is dropped; one on its
// screen already is marked (`there`) and left. A twin, a sibling that was
// another screen as one left or the other way, never takes the other's
// connector or one given it, and `waiting` is `away` with those still
// waiting, each knowing those as another screen's for later calls.
function homeward(away, screens, workspaces) {
    const moves = [];
    const waiting = {};
    const there = connected(screens, workspaces);
    for (const screen of Object.keys(away ?? {}).sort()) {
        const given = [];
        const held = [];
        const live = away[screen]
            .filter(one => workspaces.some(each => each.id === one.id))
            .sort((a, b) => a.workspace - b.workspace);
        const twins = (one, was) => one.others?.[was.output] === screen || was.others?.[one.output] === screen;
        for (const one of live) {
            const claimed = [...new Set(given.filter(was => twins(one, was.one)).map(was => was.to)
                .concat(live.filter(was => was !== one && was.output && twins(one, was)).map(was => was.output)))];
            const output = wayBack(one, screen, there, claimed);
            if (output === null) {
                const others = Object.assign({}, one.others, ...claimed.map(name => ({ [name]: screen })));
                held.push(claimed.length > 0 ? Object.assign({}, one, { others }) : one);
                continue;
            }
            given.push({ to: output, one });
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
// stays where he put it. One noted for no screen ("") has none to go to.
function wayBack(one, screen, screens, claimed) {
    const alike = Object.keys(screens ?? {})
        .filter(name => screen !== "" && (screens[name] === screen || name === screen))
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
// of this screen's. Nor is one kept on a twin's connector (`twins`, twinsOf's): another monitor's.
function whereBack(at, beside, twins) {
    const screenOf = one => one.screen || one.output;
    const there = beside.filter(one => screenOf(one.at) === screenOf(at)
        && !(twins ?? []).includes(one.at.output));
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

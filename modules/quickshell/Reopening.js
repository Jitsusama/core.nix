.pragma library
// In what order and how fast Reopen Them All may go (Sessions.qml): leaders
// first, each once, when it may begin the next session, which of those
// begun to tell to carry on, from the machine queue's own verdict, and
// what each walk found gone (tallyOf). Where
// each goes back is Places.js's, but for a named workspace's (namedBack);
// tests/places.qml holds both to their cases.

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

// How Reopen Them All brings back a session the boot took (Joel,
// 2026-11-20 13:24, after the machine reset at 13:15: crash-taken windows
// come back where they were, unfocused, and windowless sessions come back
// windowless). One kept somewhere had a window in the boot, so it opens
// there (`window`), at its kept place rather than its orchestrator's
// workspace (`home` false), which moved one kept on the second workspace
// to its orchestrator's third. One kept nowhere had no window, as one an
// orchestrator started in the background has none, so it comes back held
// without one, for Go To to join; should it open a window all the same,
// as a parked one woken does, it goes home as before. That holds only
// within the walk (`walk`), whose places are the boot's own; only for one
// that sat here in that boot (`sat`, as satHere says), since one carried
// from another machine or given back by a host was never kept here; and
// only while
// the boot's places are known (`arranged`): with none, as when the agents
// daemon kept nothing, every session reads as kept nowhere, so each opens
// as before. Anything else, handed over or started alone, opens as it
// always did, going home first.
//
// A window the daemon saw but couldn't place, floating or drawn by a kitty
// beside another, is kept `unplaced`: it comes back in a window, going home
// first, as one with no place would have.
//
// Its limits, as the daemon keeps places: a place outlives its window for
// three minutes, a boot's end's time, and a window closed while no other
// its terminal draws still shows reads as that terminal gone, as a crash
// takes them; so one whose window Joel closed just before a restart, or the
// last of a terminal's he closed, comes back in a window. One opened in the
// last still moment before the crash was never kept, so its session comes
// back without one.
function broughtBack(walk, kept, arranged, sat) {
    if (!walk) {
        return { window: true, home: true };
    }
    if (kept !== null && kept !== undefined) {
        return { window: true, home: kept.unplaced === true };
    }
    return { window: !(arranged && sat), home: true };
}

// The place a kept one opens at: none for one kept `unplaced`, which says
// only that it had a window (broughtBack).
function placeOf(kept) {
    return kept?.unplaced === true ? null : kept;
}

// Whether a session to reopen sat here in the boot being restored, so one
// kept nowhere had no window (broughtBack): the way the daemon's offered
// record says it came, kept on each by Sessions.qml's wayCame.
function satHere(one) {
    return one?.way === "boot";
}

// Where a session's place says it opens, as each of the walk's ways opens
// one (a kitty, a viewer, a wake), its workspace's name kept with it
// (namedBack); null for none.
function atOf(at) {
    return at ? { output: at.output, workspace: at.workspace,
        workspaceName: at.workspaceName, column: at.column, tile: at.tile ?? 1 } : null;
}

// The name the workspace a place went back to takes: the kept one, `back`
// as namedBack gives it, only on the screen's empty workspace it found
// (`emptied`), and only while that's unnamed, as homeToName names; the
// last with windows may be one Joel named. "" for none.
function keptName(back, there, emptied) {
    return back?.name && there === emptied && !there?.name ? back.name : "";
}

// Where a place kept on a named workspace, an orchestrator's, goes back to,
// by that name rather than its number, since named workspaces come back
// from a boot in another order (2026-11-21 10:08: WM4KSZ's eleven sessions
// came back on workspace 3 by number, the name WM4KSZ going to an empty
// fifth). { join } the workspace named so now, by id; { name } to give the
// workspace it lands on when none has it; null for a place kept on a
// workspace named nothing, which goes by its number. `workspaces` are
// niri's, { id, name } each.
function namedBack(at, workspaces) {
    const name = typeof at?.workspaceName === "string" ? at.workspaceName : "";
    if (name === "") {
        return null;
    }
    const named = workspaces.find(space => space.name === name);
    return named ? { join: named.id } : { name };
}

// Whether a window on `workspace`, niri's, sits where its place `at` sent
// it by name (namedBack), on whatever screen that workspace is now, so
// placing it moves it no further.
function onNamed(at, workspace) {
    return Boolean(at?.workspaceName) && workspace?.name === at.workspaceName;
}

// How long a session started without a window is taken as begun, though
// its pi hasn't said so yet, in milliseconds: nothing on screen says it
// started, so Reopen Them All pressed again meanwhile would begin a second
// pi on its log, each writing over the other. Five minutes covers a pi
// starting on a machine just back from a crash.
const startedHeldFor = 5 * 60 * 1000;

// Whether `id` was started without a window in the last startedHeldFor,
// as `started` (ids to when, in milliseconds) records it, at `now`.
function startedHeldLately(started, id, now) {
    const at = started?.[id];
    return typeof at === "number" && now - at < startedHeldFor;
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

// The screen in use to put a window back by (Sessions.qml's putBack): the
// one in use as Reopen Them All placed its sessions (`placed`), so a focus
// moved since doesn't set a window apart from them, while that's still
// connected (`screens`, Places.js's connected), else the one in use `now`,
// as with none in use then: a window needs a screen to go to.
function inUseFor(placed, now, screens) {
    return typeof placed === "string" && Object.keys(screens ?? {}).includes(placed) ? placed : now;
}

// The connector a place found now (`place`, of one kept on `screen`) was given, or undefined.
// It must be connected now (`screens`, Places.js's connected) and show that screen, and the place
// must not be away. A place left as kept, with no screen said or none in use, keeps last boot's
// connector, which may be gone or another screen's now, so it was given none, as with no
// `screens` at all, which fails closed rather than stopping Reopen Them All's walk.
function givenOn(screen, place, screens) {
    const output = place?.output;
    // Own keys alone, so a prototype's name is never connected, as Sprouting.js's untold.
    const shows = Object.keys(screens ?? {}).includes(output)
        && (!screen || screens[output] === screen);
    return place?.away || !shows ? undefined : output;
}

// The connectors Reopen Them All's sessions were given (`places`, Places.js's placed, of the
// places `kept`), as whereKept's `given`, so its windows put back after take the same twin
// connectors as their sessions, whatever order they come back in. A session counts as given a
// connector only by givenOn, with `screens` connected now. An earlier window's entry (`before`)
// stays unless a session kept on the same screen and connector was given a connector, or a
// session was given the connector it took. So those windows never contradict the sessions, nor
// leave the other twin's windows none to go to.
function givenBy(kept, places, before, screens) {
    const given = kept
        .map((at, index) => ({ at, output: givenOn(at?.screen, places[index], screens) }))
        .filter(one => one.at);
    const keptOn = one => JSON.stringify([one.at.screen, one.at.output]);
    const placed = given.filter(one => one.output !== undefined);
    const said = new Set(placed.map(keptOn));
    const now = new Set(placed.map(one => one.output));
    return given.concat((before ?? [])
        .filter(was => !said.has(keptOn(was)) && !now.has(was.output)));
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

// The tally a Reopen Them All list counts what's found gone in, decided
// where Sessions joins the list to `walk` or begins one (reopenHere), since
// a press waits on niri's answer for the screens first (lookAtScreens), and
// one decided at the press could count in a walk the list never joins
// (core.nix#6126's round 3). The walk's own while it may still find one of
// that tally's folders gone, one of its sessions still queued or its folder
// still looked for (`looking`); else one of its own, as once every session
// it asked for is begun, though a window given up on keeps the walk, or
// the harness's sessions keep it busy, or for a walk the harness began.
// Only the tally's own sessions count, so the order against the list's own
// join (queuedOnce) is no matter. Each of `ids` counts once, however many
// lists ask for it, and `tell` hears each reading, the last list's kept. A
// list joining a tally already told of one gone tells it again with those
// it adds, or the reading Joel is told would stay short of them. No tally,
// fresh or joined, counts one the walk has `begun` and is done with, which
// it won't look for again (queuedOnce) and so can never find gone: a
// folder found gone among them would otherwise be said reopened. One begun
// and still findable, its folder looked for now or one queued afresh for
// it, counts, so its folder found gone is told. So a list given just after
// one the walk used up within the same turn, as one whose every session
// another machine holds, counts afresh, none that walk began among them,
// though the two were pressed as one.
function tallyOf(walk, ids, tell) {
    const own = walk?.tally ?? null;
    const idOf = entry => entry.one.afreshFor ?? entry.one.id;
    const asked = entry => own.asked[idOf(entry)] === true;
    const counting = own !== null
        && (walk.queue.some(asked) || (walk.looking !== null && asked(walk.looking)));
    const tally = counting ? own : { asked: {}, gone: {}, tell };
    const findable = new Set((walk?.queue ?? []).concat(walk?.looking ? [walk.looking] : [])
        .map(idOf));
    const done = new Set((walk?.begun ?? []).filter(id => !findable.has(id)));
    const added = ids.filter(id => tally.asked[id] !== true && !done.has(id));
    for (const id of added) {
        tally.asked[id] = true;
    }
    tally.tell = tell;
    if (counting && added.length > 0 && Object.keys(tally.gone).length > 0) {
        tell(reading(tally));
    }
    return tally;
}

// `one` found gone, counted in `tally` by the id a list asked for it by,
// one started afresh by the one it stands in for (Places.afresh), whichever
// list queued it, and the reading told. One no list counting asked for
// counts nowhere.
function foundGone(tally, one) {
    const id = one.afreshFor ?? one.id;
    if (tally === null || tally === undefined || tally.asked[id] !== true) {
        return;
    }
    tally.gone[id] = true;
    tally.tell(reading(tally));
}

// How many of a tally's sessions were found gone, and how many of the rest
// were asked to reopen.
function reading(tally) {
    const gone = Object.keys(tally.gone).length;
    return { gone, reopened: Object.keys(tally.asked).length - gone };
}

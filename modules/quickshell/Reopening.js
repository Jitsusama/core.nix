.pragma library
// In what order and how fast Reopen Them All may go (Sessions.qml): leaders
// first, each once, when it may begin the next session, and which of those
// begun to tell to carry on, from the machine queue's own verdict. Where each goes back is Places.js's; tests/places.qml
// holds both to their cases.

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
// windowless). One kept somewhere had a window as the boot ended, so it
// opens there (`window`), at its kept place rather than its
// orchestrator's workspace (`home` false), which moved one kept on the
// second workspace to its orchestrator's third. One kept nowhere had no
// window, as one an orchestrator started in the background has none, so
// it comes back held without one, for Go To to join. That holds only
// within the walk (`walk`), whose places are the boot's own, and only
// while the boot's places are known (`arranged`): with none, as when the
// agents daemon kept nothing, every session reads as kept nowhere, so
// each opens as before. Anything else, handed over or started alone,
// opens as it always did, going home first.
function broughtBack(walk, kept, arranged) {
    if (!walk) {
        return { window: true, home: true };
    }
    if (kept !== null && kept !== undefined) {
        return { window: true, home: false };
    }
    return arranged ? { window: false, home: false } : { window: true, home: true };
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

// Reopen Them All's queue with `more` joined at its end, but for each one
// it holds already or has `begun`, by id: an orchestrator handing over its
// own sessions as it comes back, while the queue still holds them, would
// otherwise begin each twice, two pis on one log, before either has said
// it runs here.
function queuedOnce(queue, more, begun) {
    const asked = new Set(queue.map(entry => entry.one.id).concat(begun));
    return queue.concat(more.filter(entry => !asked.has(entry.one.id)));
}

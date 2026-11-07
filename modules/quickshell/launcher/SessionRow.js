.pragma library
// A pi session as the launcher shows it, said once, and named apart from
// Session.qml, which is the desktop session: how it stands, the acts it
// offers in their order, and what its drawer says, wherever the launcher
// shows it (Agents, an orchestrator's row, a quest's Go To Its Session).
// Joel, 2026-10-28: "we need to not have different versions of these sort
// of elements. they need to be treated the same everywhere." The providers
// turn what this says into rows, each through its own `thing`.

// What a session is, as the launcher's components name their kinds.
const KIND = "session";

// How a running session can stand, in the one order every list of
// sessions keeps: what only Joel can move first, then what's under way,
// then what waits for work, then what he put down, then what gave its
// memory back until it's wanted (the harness's park-workflow). One idle
// is listening, never waiting, which is only ever said of something
// waiting on Joel.
const STATES = ["needs", "working", "inLine", "listening", "paused", "parked"];

// How a session stands, from its presence, whether its want is truly
// Joel's (`asks`, Sessions.asks) and whether it waits its turn in the
// machine's queue (`queued`): "off" with no presence at all. A want that
// isn't his reads as what the session does meanwhile.
function state(presence, asks, queued) {
    if (!presence) {
        return "off";
    }
    if (asks) {
        return "needs";
    }
    if (presence.state === "parked") {
        return "parked";
    }
    if (presence.state === "paused") {
        return "paused";
    }
    if (queued) {
        return "inLine";
    }
    return presence.busy || presence.state === "busy" ? "working" : "listening";
}

// Where a state stands in that order; one not running after them all.
function rank(standing) {
    const at = STATES.indexOf(standing);
    return at < 0 ? STATES.length : at;
}

// The deepest a row sits under another (docs/design.md keeps no layer
// more than two deep).
const DEEPEST = 2;

// Sessions laid out as Agents shows them, from [{ address, under, state }]
// in the order Sessions keeps: each under the session its `under` names,
// when that one's listed, as [{ at, depth, under, orchestrates }], `at` its
// place in what was given, `under` the address of the row it's drawn under
// or "", and `orchestrates` whether any row is under it. Siblings stand in
// the order of how they stand, then the order given; one with rows under
// it stands where its most urgent row would, so an orchestration moves as
// one. Orchestrators naming each other are cut where the first is met.
function tree(sessions) {
    const byAddress = {};
    for (const [at, one] of sessions.entries()) {
        if (byAddress[one.address] === undefined) {
            byAddress[one.address] = at;
        }
    }
    const parent = sessions.map((one, at) => {
        const found = one.under ? byAddress[one.under] : undefined;
        return found === undefined || found === at ? -1 : found;
    });
    for (const at of parent.keys()) {
        const seen = new Set([at]);
        for (let up = parent[at]; up >= 0; up = parent[up]) {
            if (seen.has(up)) {
                parent[at] = -1;
                break;
            }
            seen.add(up);
        }
    }
    const children = sessions.map(() => []);
    for (const [at, up] of parent.entries()) {
        if (up >= 0) {
            children[up].push(at);
        }
    }
    // Whether one [rank, place] is more urgent than another.
    const before = (one, other) => one[0] < other[0] || one[0] === other[0] && one[1] < other[1];
    const compared = (one, other) => {
        if (before(one, other)) {
            return -1;
        }
        return before(other, one) ? 1 : 0;
    };
    // The most urgent of a row and those under it, as [rank, place].
    const best = [];
    const bestOf = at => {
        if (best[at] === undefined) {
            best[at] = children[at].map(bestOf).reduce((most, other) =>
                before(other, most) ? other : most, [rank(sessions[at].state), at]);
        }
        return best[at];
    };
    const inOrder = list => list.slice().sort((a, b) => compared(bestOf(a), bestOf(b)));
    const laid = [];
    const lay = (at, depth) => {
        const up = parent[at];
        laid.push({ at, depth: Math.min(depth, DEEPEST),
            under: up >= 0 ? sessions[up].address : "", orchestrates: children[at].length > 0 });
        for (const child of inOrder(children[at])) {
            lay(child, depth + 1);
        }
    };
    for (const at of inOrder([...parent.keys()].filter(root => parent[root] < 0))) {
        lay(at, 0);
    }
    return laid;
}

// The acts a running session offers, by their vocabulary's ids, in the
// order every kind keeps (docs/design.md, the order of actions): what
// leaves it as it is before what changes it, and what ends it last. Every
// session on the mesh reads its steer inbox, so each can be talked to,
// paused, resumed and stopped; one with no window offers no Go To, and
// only one under way can be stopped. One parked has no process to pause
// or stop and no window known to go to: it can be woken, or talked to,
// which wakes it too.
function acts(standing, windowed) {
    if (standing === "parked") {
        return ["wake", "talk"];
    }
    const ids = windowed ? ["switch"] : [];
    ids.push("talk");
    ids.push(standing === "paused" ? "resume" : "pause");
    if (standing === "working" || standing === "inLine") {
        ids.push("stop");
    }
    return ids;
}

// The act Return does, one of `acts`: the one its state makes harmless
// (VKCF3Z, 2026-11-06). Go To one at work or waiting on Joel, Talk to one
// listening, Resume one paused and Wake one parked; one with no window
// to go to is talked to. Stop and Close end it, so they are never
// Return's.
function returns(standing, windowed) {
    if (standing === "parked") {
        return "wake";
    }
    if (standing === "paused") {
        return "resume";
    }
    return windowed && standing !== "listening" ? "switch" : "talk";
}

// What a session's row says at its right, each item with how much it's
// needed, so where room runs short they give way whole (Choice.qml): how
// it stands, then what it spent today, last and kept longest, and nothing
// more (VKCF3Z, 2026-11-06: "working \u00b7 $4.20"). Its address and
// folder are its drawer's eyebrow, and typing its address still finds it.
function side(doing, spent) {
    return [{ said: doing, keep: 3 }, { said: spent, keep: 4 }]
        .filter(item => item.said !== "");
}

// What its drawer says: its address, where it works and since when it's
// run as the eyebrow over its title, "8IDU4E \u00b7 core.nix \u00b7 since
// 21:52" (VKCF3Z, 2026-11-06); then what it works on, its step, what it
// waits on Joel for, what it's spent today (an orchestrator's whole
// orchestration, then its own and its sessions') and the machine it runs
// on, each as a name and a figure. Nothing it hasn't said is guessed, and
// what the eyebrow says isn't said again.
function drawer(title, presence, said) {
    const known = {
        works: presence?.worksOn,
        step: presence?.step,
        wants: presence?.wants?.said,
        spent: said.spent,
        own: said.own,
        under: said.under,
        machine: said.machine
    };
    const facts = Object.keys(known).filter(id => (known[id] ?? "") !== "")
        .map(id => ({ label: said.named(id), value: known[id] }));
    const eyebrow = [said.address, said.place, said.since]
        .filter(part => (part ?? "") !== "").join(" \u00b7 ");
    return { eyebrow, title, facts, body: "" };
}

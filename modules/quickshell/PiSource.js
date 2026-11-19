// pi as an agent source (KP0L6A's agent-source-contract.md): what the
// realm's mycelium node holds of pi's sessions, sorted into those running,
// parked and closed this boot, and each said as the contract's session
// record. Said in one place, out of PiSource.qml, so tests/pi-source.qml
// holds it in Qt's own engine. pi publishes under its own names, which
// this maps rather than renaming what pi writes (85PKB2, 2026-11-07
// [ecb89be5]); a fact pi doesn't publish is left out, never guessed.
.pragma library

// What a session's key begins with, before its id.
const SESSION_KEY = /^session:/;

// Where one of the node's records goes, as `sorted` says, and what it is
// there: `{ into, entry }`, `into` one of live, resting or ending, or
// null for one passed over. The node says how each stands (its
// `standing`), from the park markers and the boot it watches itself, so
// the shell no longer reads either: running, parked or closed this boot,
// and nothing for one passed over.
function placed(key, session) {
    const id = String(session.id ?? key.replace(SESSION_KEY, ""));
    const { presence } = session;
    if (!presence || (presence.addresses ?? []).length === 0) {
        return { into: null, entry: null };
    }
    const changed = Date.parse(session.at ?? "") || 0;
    // One parked keeps the window its holder keeps, where the node knows
    // it, so the cutover can keep its place before it wakes it, and the
    // words to view it, since one asleep in pi's host wakes only as it's
    // viewed. One parked on another machine says so, as one running there
    // does, so Wake never reopens it here (Sessions' reopensUnwoken).
    if (session.standing === "parked") {
        return { into: "resting", entry: Object.assign({
            file: id, presence, window: session.window ?? null, held: session.held ?? null,
            attach: session.attach ?? null
        }, session.here === false ? { machine: String(session.machine), here: false } : {}) };
    }
    if (session.standing === "closed") {
        return { into: "ending", entry: { file: id, presence, window: null, held: null, changed } };
    }
    if (session.standing !== "running") {
        return { into: null, entry: null };
    }
    const window = session.window ?? null;
    // One here keeps the words the node gives to view it, which for one pi's
    // host holds are its presence's own, so Go To and Reopen Them All open
    // its viewer; without them a hosted one here had none to open it by.
    if (session.here !== false) {
        return { into: "live", entry: { file: id, presence, window, held: session.held ?? null,
            attach: session.attach ?? null } };
    }
    const out = String(session.state ?? "").toLowerCase() === "unreachable";
    return { into: "live", entry: {
        file: id, presence, window, held: null,
        machine: String(session.machine), here: false,
        attach: out ? null : session.attach ?? null,
        reachable: !out, changed
    } };
}

// The node's records of pi's sessions, by key, sorted: `live`, those
// running, here or on another machine heard of this boot; `resting`, those
// parked here; `ending`, those here that ended this boot, unparked. Each
// is `{ file, presence, window, held }`, with where it runs, whether it
// can be reached and when it changed for one elsewhere, and when it
// changed for one ended. A stopgap: Agents takes its rows ready from the
// agents daemon, and the daemon's ready rows take over this sorting for
// Now and the windows' readers too, with the session acts, Thu 12 Nov.
function sorted(held) {
    return resorted(held, {}, Object.keys(held));
}

// The node's records sorted again as `sorted` sorts them, placing anew
// only the keys `heard` names, as Daemon.qml's `heard` names those a
// change moved or let go. `kept` holds each key's place as last sorted,
// `{ into, entry }`, and is mended in place, so a session the change
// didn't name keeps its entry, the same object, and whatever compares it
// stops at once. Joel's 126 sessions were placed again at each of the
// node's changes, about one a second (2026-11-14).
function resorted(held, kept, heard) {
    for (let at = 0; at < (heard?.length ?? 0); at += 1) {
        const key = heard[at];
        if (key in held) {
            kept[key] = placed(key, held[key]);
        } else {
            delete kept[key];
        }
    }
    const sorting = { live: [], resting: [], ending: [] };
    for (const key of Object.keys(kept).sort()) {
        const { into, entry } = kept[key];
        if (into !== null) {
            sorting[into].push(entry);
        }
    }
    return sorting;
}

// Whether a change the node told (Daemon.qml's `told`) moves anything a
// shut launcher reads: a session new, gone, or carrying a new digest of
// what's read shut (the node's `told`, which leaves out a session's step,
// usage, times, and working against listening). A whole answer always
// does. With the launcher shut, each change of a session's step had cost
// the shell's main thread 20 to 370 ms of sorting and binding that
// nothing on screen read; of Joel's 113 sessions' changes in 30 s on
// 2026-11-09, 21 of 26 moved nothing else.
// Its lists are read by length and index, since one passed through a QML
// signal, as Daemon.qml's `told` passes it, is a list Array.isArray
// doesn't take for an array.
function movesShut(answer) {
    const changed = answer?.changed;
    if ((answer?.gone?.length ?? 0) > 0 || answer?.records !== undefined) {
        return true;
    }
    for (let at = 0; at < (changed?.length ?? 0); at += 1) {
        if (changed[at]?.told !== undefined) {
            return true;
        }
    }
    return false;
}

// Whether two values say the same, as JSON would: a field left undefined
// is one not said, and the order fields come in tells nothing. Compared
// in place rather than as text: Joel's 117 sessions are about 280,000
// characters as JSON, and a text that size made each of the node's
// changes, about one a second, start a garbage collection of its own (303
// in 300 changes, against 12 compared in place, in Qt's engine on
// 2026-11-10).
//
// Stepped through by index, with no function made inside it: Qt's engine
// keeps each call's arguments on the heap for any function holding one
// that reads them, whether or not that call makes it, so comparing Joel's
// sessions and runs at each change made 188,592 of them between two of
// the collector's runs on 2026-11-17, the most of anything his shell made
// then.
function alike(before, after) {
    if (before === after) {
        return true;
    }
    if (typeof before !== "object" || typeof after !== "object" || before === null
        || after === null || Array.isArray(before) !== Array.isArray(after)) {
        return false;
    }
    if (Array.isArray(before)) {
        if (before.length !== after.length) {
            return false;
        }
        for (let at = 0; at < before.length; at += 1) {
            if (!alike(before[at], after[at])) {
                return false;
            }
        }
        return true;
    }
    return fieldsSaid(before, after) && fieldsSaid(after, before);
}

// Whether each field `one` has says in `other` what it says in `one`, a
// field one leaves undefined being one not said. Asked both ways, since
// alike is the same either way round.
function fieldsSaid(one, other) {
    const names = Object.keys(one);
    for (let at = 0; at < names.length; at += 1) {
        if (!alike(one[names[at]], other[names[at]])) {
            return false;
        }
    }
    return true;
}

// Copies the fields named from `from` that are said, so one unsaid is
// left out rather than set to nothing.
function said(from, names) {
    const kept = {};
    for (const name of names) {
        if (from && from[name] !== undefined && from[name] !== null) {
            kept[name] = from[name];
        }
    }
    return kept;
}

// The OpenTelemetry gen_ai names pi's usage carries, kept as pi says them.
const USAGE = ["gen_ai.request.model", "gen_ai.response.model", "gen_ai.provider.name", "last",
    "today", "context", "turns"];

// How a session stands, in the contract's words: working, listening or
// needs while it runs, unreachable while its machine can't be, parked or
// closed.
function standing(one, how) {
    if (how !== "live") {
        return how;
    }
    if (one.reachable === false) {
        return "unreachable";
    }
    if ((one.presence.wants ?? []).length > 0) {
        return "needs";
    }
    return one.presence.busy === true ? "working" : "listening";
}

// One session as the contract's record. `how` is live, parked or closed,
// and `machine` this machine's name.
function record(one, how, machine) {
    const { presence } = one;
    const id = String(presence.session ?? one.file);
    const usage = presence.usage ?? null;
    const cost = usage?.["pi.cost.usd"] ?? null;
    const quest = presence.questInfo ?? null;
    const kept = {
        key: `session:${id}`,
        "gen_ai.conversation.id": id,
        "gen_ai.agent.name": "pi",
        address: (presence.addresses ?? [])[0],
        machine: one.machine ?? machine,
        here: one.here ?? true,
        state: standing(one, how)
    };
    const step = presence.stepDetail ?? null;
    return Object.assign(kept,
        said({ title: quest?.title, folder: presence.cwd, repo: presence.repo,
            since: presence.since, wants: presence.wants }, ["title", "folder", "repo", "since",
            "wants"]),
        step ? { step: said(step, ["kind", "name", "detail", "since"]) } : {},
        usage ? { usage: said(usage, USAGE) } : {},
        cost ? { "cost.usd": cost } : {},
        quest ? { "pi.quest": said(quest, ["id", "title", "short", "status", "kind"]) } : {},
        said({ "pi.orchestratedBy": presence.orchestratedBy, "pi.held": one.held },
            ["pi.orchestratedBy", "pi.held"]));
}

// What `make` makes of each entry, kept in `kept` by its session's id
// beside the entry it was made from, and made again only for an entry
// that's new, as `resorted` makes one only for a session a change named;
// one no longer listed is let go. So `make` must read nothing but its
// entry. Sessions.qml copied each of Joel's sessions at every change and
// compared the copies field by field, about 280,000 characters of them
// (2026-11-09); kept, unchanged ones compare as the same object.
function keptBy(entries, kept, make) {
    const listed = new Set();
    const made = entries.map(one => {
        listed.add(one.file);
        const was = kept[one.file];
        if (was && was.one === one) {
            return was.made;
        }
        const now = make(one);
        kept[one.file] = { one, made: now };
        return now;
    });
    for (const file of Object.keys(kept)) {
        if (!listed.has(file)) {
            delete kept[file];
        }
    }
    return made;
}

// Every session pi tells of, as the contract's records: those running,
// then those parked, then those closed this boot. `made`, where given,
// keeps each session's record by its id beside the entry it was made
// from, mended in place: a record is made again only where its entry is
// a new one, as `resorted` makes only for what a change named. Making
// every record again took 2 of the 2.1 ms of each change at 126
// sessions, in Qt's engine (2026-11-15).
function sessions(sorting, machine, made) {
    const kept = made ?? {};
    const seen = new Set();
    const recordOf = (one, how) => {
        const was = kept[one.file];
        seen.add(one.file);
        if (was && was.one === one && was.how === how && was.machine === machine) {
            return was.record;
        }
        const now = record(one, how, machine);
        kept[one.file] = { one, how, machine, record: now };
        return now;
    };
    const records = sorting.live.map(one => recordOf(one, "live"))
        .concat(sorting.resting.map(one => recordOf(one, "parked")))
        .concat(sorting.ending.map(one => recordOf(one, "closed")));
    for (const file of Object.keys(kept)) {
        if (!seen.has(file)) {
            delete kept[file];
        }
    }
    return records;
}

// How a source stands, in the contract's words, from its node's link
// (Daemon.qml's `state`): quiet while it starts or where this machine
// runs no node, lost while it isn't answering, and otherwise answering,
// whatever word the node itself says for it.
function state(link) {
    if (!link || link.state === "starting") {
        return "quiet";
    }
    return link.state === "not answering" ? "lost" : "answering";
}

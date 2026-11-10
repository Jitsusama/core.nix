// pi as an agent source (KP0L6A's agent-source-contract.md): what the
// realm's mycelium node holds of pi's sessions, sorted into those running,
// parked and closed this boot, and each said as the contract's session
// record. Said in one place, out of PiSource.qml, so tests/pi-source.qml
// holds it in Qt's own engine. pi publishes under its own names, which
// this maps rather than renaming what pi writes (85PKB2, 2026-11-07
// [ecb89be5]); a fact pi doesn't publish is left out, never guessed.
.pragma library

// A park lasts only while its holder does, and no holder outlives the
// boot it began in, so one parked before this boot has nothing left to
// wake it, whatever its marker says: it's closed, and reopens as one.
// Taken for parked, it was passed over as running by Reopen (11 of them
// after the reboot at 19:15 on 2026-11-07). A record naming no boot is
// taken as this one's.
function parks(presence, markers, bootId) {
    return Boolean(presence) && presence.state === "parked" && (presence.addresses ?? []).length > 0
        && typeof presence.session === "string" && markers[presence.session] === true
        && (presence.owner?.bootId ?? bootId) === bootId;
}

// Whether a record has a session to show: a presence with an address.
function addressed(presence) {
    return Boolean(presence) && (presence.addresses ?? []).length > 0;
}

// What a session's key begins with, before its id.
const SESSION_KEY = /^session:/;

// Where one of the node's records goes, as `sorted` says, and what it is
// there: `{ into, entry }`, `into` one of live, resting or ending, or
// null for one passed over.
function placed(key, session, place) {
    const id = String(session.id ?? key.replace(SESSION_KEY, ""));
    const { presence } = session;
    const told = String(session.state ?? "").toLowerCase();
    const out = told === "unreachable";
    const running = (told === "busy" || told === "idle" || out) && addressed(presence);
    const here = typeof session.here === "boolean" ? session.here
        : (session.machine ?? place.machine) === place.machine;
    const changed = Date.parse(session.at ?? "") || 0;
    const ended = told === "ended" && here;
    // The node says one parked has ended, which its process has; its
    // record says it's parked.
    if (ended && parks(presence, place.markers, place.bootId)) {
        return { into: "resting", entry: { file: id, presence, window: null, held: null } };
    }
    if (ended && addressed(presence) && changed >= place.bootedAt) {
        return { into: "ending", entry: { file: id, presence, window: null, held: null, changed } };
    }
    const window = session.window ?? null;
    if (running && here) {
        return { into: "live", entry: { file: id, presence, window, held: session.held ?? null } };
    }
    // One elsewhere the node last heard of before this boot is long gone,
    // so a machine away for days leaves nothing.
    if (running && changed >= place.bootedAt) {
        return { into: "live", entry: {
            file: id, presence, window, held: null,
            machine: String(session.machine), here: false,
            attach: out ? null : session.attach ?? null,
            reachable: !out, changed
        } };
    }
    return { into: null, entry: null };
}

// The node's records of pi's sessions, by key, sorted: `live`, those
// running, here or on another machine heard of this boot; `resting`, those
// parked here; `ending`, those here that ended this boot, unparked. Each
// is `{ file, presence, window, held }`, with where it runs, whether it
// can be reached and when it changed for one elsewhere, and when it
// changed for one ended. `place` is `{ machine, bootId, bootedAt,
// markers }`, the markers the parked sessions' ids.
function sorted(held, place) {
    const sorting = { live: [], resting: [], ending: [] };
    for (const key of Object.keys(held).sort()) {
        const { into, entry } = placed(key, held[key], place);
        if (into !== null) {
            sorting[into].push(entry);
        }
    }
    return sorting;
}

// What a session's presence says that only the launcher shows, and that
// changes as the session works: its step, what it has used, when it was
// last heard, and whether it's working or listening and since when. Joel's
// 113 sessions changed 26 times in 30 s on 2026-11-09, and 21 of those
// changed nothing else. Of the rest, 14 in 61 on 2026-11-10 moved only
// between working and listening, which the launcher's foot and its
// Agents say and nothing outside it reads: For You, Super+Tab and the
// margin read what a session wants and where its window is. A park moves
// the session to `resting` as well, so its state going is never all that
// tells it.
const SHOWN_ONLY = ["at", "busy", "heard", "since", "state", "step", "stepDetail", "usage"];

// Whether two values say the same, as JSON would: a field left undefined
// is one not said, and the order fields come in tells nothing.
function alike(before, after) {
    if (before === after) {
        return true;
    }
    if (typeof before !== "object" || typeof after !== "object" || before === null
        || after === null || Array.isArray(before) !== Array.isArray(after)) {
        return false;
    }
    if (Array.isArray(before)) {
        return before.length === after.length && before.every((one, at) => alike(one, after[at]));
    }
    return fieldsAlike(before, after, []);
}

// Whether two objects say the same in every field but those passed over.
function fieldsAlike(before, after, passed) {
    const told = name => passed.includes(name) || alike(before[name], after[name]);
    return Object.keys(before).every(told) && Object.keys(after).every(told);
}

// What a sorted session (`sorted`) says that a shut launcher passes over
// besides its presence's own: a running session elsewhere carries when it
// changed from its record's own time, which moves with each step and only
// orders the launcher's lists.
const PRESENCE = ["presence"];
const ELSEWHERE = ["presence", "changed"];

// Whether two sortings (`sorted`) tell the same to what reads them: all
// of them while the launcher shows them, and otherwise all but what only
// the launcher shows. With the launcher shut, each change of a session's
// step had cost the shell's main thread 20 to 370 ms of sorting and
// binding that nothing on screen read. Compared field by field rather
// than as text: Joel's 117 sessions are about 280,000 characters as JSON,
// and a text that size made each of the node's changes, about one a
// second, start a garbage collection of its own (303 in 300 changes,
// against 12 compared in place, in Qt's engine on 2026-11-10).
function tellsAlike(before, after, shown) {
    for (const how of ["live", "resting", "ending"]) {
        const was = before[how];
        const is = after[how];
        if (was.length !== is.length) {
            return false;
        }
        const running = how === "live";
        if (!was.every((one, at) => entryAlike(one, is[at], running, shown))) {
            return false;
        }
    }
    return true;
}

// Whether one sorted session tells the same as another (`tellsAlike`).
function entryAlike(before, after, running, shown) {
    if (shown) {
        return alike(before, after);
    }
    const passed = running && before.here === false ? ELSEWHERE : PRESENCE;
    return fieldsAlike(before, after, passed)
        && fieldsAlike(before.presence ?? {}, after.presence ?? {}, SHOWN_ONLY);
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

// Every session pi tells of, as the contract's records: those running,
// then those parked, then those closed this boot.
function sessions(sorting, machine) {
    return sorting.live.map(one => record(one, "live", machine))
        .concat(sorting.resting.map(one => record(one, "parked", machine)))
        .concat(sorting.ending.map(one => record(one, "closed", machine)));
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

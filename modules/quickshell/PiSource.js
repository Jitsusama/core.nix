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

// What only Joel can do, for WaitingOnYou.qml and Sessions.qml (For You,
// VKCF3Z's, 2026-10-30), out of what the mycelium node serves of the files
// the agents keep: each quest's waiting/<address>.json, written by the
// session that needs him through mesh_wait, as `waiting` records; what the
// to do daemon found of each (verdicts.json, its alone), as `verdict`
// records; the quests still open, as `quest` records; and what Joel has
// said of each (the shell's own settled.json). Each item is written once,
// by its owner, and says how it knows it's done, so this decides which
// stand, now or later:
// one goes once its check passes or its signal comes, its quest ends, its
// owner takes it out or Joel says it's done; one is ready once its owner or
// a verdict says so. Nothing goes by its age. Kept apart from Quickshell's
// types, so the `needs-you-is-true` check runs it in Qt's own engine
// (tests/waiting-on-you.qml).
.pragma library
.import "PiSource.js" as Pi

// A quest's status that means it's over, so nothing of it waits.
const ENDED = ["concluded", "retired"];

// A quest's id, which the harness's index of quests stands behind; an
// item of anything else, as Slack's sign-in is, has no quest to end.
const QUEST = /^QEST-/;
// The front of each kind's keys, as the node gives them (mycelium's
// PROTOCOL.md, The Daemon Interface): `waiting:<quest>:<id>`, a folder's
// file keying its records by its `quest`; `verdict:<quest>:<id>`; and
// `quest:<id>`.
const WAITING = "waiting:";
const VERDICT = "verdict:";
const QUEST_KEY = "quest:";

// A quest's short address, its id's last part, as the harness names its
// file and its sessions by it: `from`, who keeps an item.
function addressOf(quest) {
    return (quest.split("-").pop() ?? quest).toUpperCase();
}

// A record as For You reads it: without when the node last heard it,
// which moves for every record each time a file is written again, and
// which nothing here says. So a list told again unchanged reads the same
// (kept), and no row can show a time that a kept record left behind.
function unheard(record) {
    const copy = {};
    for (const field of Object.keys(record)) {
        if (field !== "heard") {
            copy[field] = record[field];
        }
    }
    return copy;
}

// What `now` says, or what `was` keeps under `name` where that says the
// same, kept in its place. The node tells each kind again whole whenever
// a file of it is written, every record's `heard` moved and nothing else,
// and each time every binding below For You weighed and made its rows
// again, about 600 ms of the shell's main thread with the launcher shut
// (2026-11-10). A binding that gives back the same value notifies
// nothing, so what reads it is left as it was. `was` is a plain object,
// so keeping in it notifies nothing either.
function kept(was, name, now) {
    if (!Pi.alike(was[name], now)) {
        was[name] = now;
    }
    return was[name];
}

// Each item the node serves, from its `waiting` records by key, with its
// quest, the file's unless the item names its own, and who keeps it; one
// with no title says nothing. They're in the order their owners raised
// them within each quest, which is how its file lists them, since the
// node gives a file's records by id.
function served(held) {
    const listed = [];
    for (const key of Object.keys(held ?? {}).sort()) {
        const raw = held[key];
        if (!key.startsWith(WAITING) || typeof raw?.title !== "string" || raw.title === "") {
            continue;
        }
        const rest = key.slice(WAITING.length);
        const file = rest.slice(0, rest.indexOf(":"));
        const quest = typeof raw.quest === "string" ? raw.quest : file;
        listed.push({ file, raised: typeof raw.raisedAt === "string" ? raw.raisedAt : "",
            at: listed.length,
            item: Object.assign(unheard(raw), { quest, from: addressOf(file) }) });
    }
    return listed.sort((one, other) => one.file.localeCompare(other.file)
        || one.raised.localeCompare(other.raised) || one.at - other.at).map(one => one.item);
}

// The to do daemon's verdicts, from its `verdict` records by key, as
// { items: { identity: { state, at, how } } }.
function verdictsOf(held) {
    const items = {};
    for (const key of Object.keys(held ?? {})) {
        if (key.startsWith(VERDICT)) {
            items[key.slice(VERDICT.length)] = unheard(held[key]);
        }
    }
    return { items };
}

// Each quest `named` that the index holds, by id, as its status and
// title; null for one it doesn't, since it holds only the quests still
// open, so one gone from it has ended. Before the node has answered,
// `index` is null, and nothing is known of any.
function owners(index, named) {
    const known = {};
    if (index === null || index === undefined) {
        return known;
    }
    for (const quest of named) {
        const record = index[QUEST_KEY + quest];
        known[quest] = record ? { status: String(record.status ?? ""),
            title: String(record.title ?? "") } : null;
    }
    return known;
}

// Whether Joel can do an item now: once the to do daemon found it ready,
// else as its owner says, `ready`, or, in a file older than that, its when
// saying now.
function readyOf(item, verdict) {
    if (verdict?.state === "ready") {
        return true;
    }
    if (typeof item.ready === "boolean") {
        return item.ready;
    }
    return typeof item.when === "string" && item.when.trim().toLowerCase() === "now";
}

// When an item is for, or Infinity for one for no time in particular.
function timeOf(item) {
    const at = Date.parse(item.at ?? "");
    return Number.isNaN(at) ? Number.POSITIVE_INFINITY : at;
}

// The order Joel does them in (VKCF3Z, 2026-10-30): anything for a time
// first, soonest first, then the quickest, so a few minutes free always
// finds something; never by age, and two alike stay as their files list
// them (`listed`, since Qt's engine may not keep a sort's ties in order).
function before(one, other) {
    const times = timeOf(one) - timeOf(other);
    if (times !== 0 && !Number.isNaN(times)) {
        return times;
    }
    const minutes = item => (typeof item.minutes === "number" ? item.minutes
        : Number.POSITIVE_INFINITY);
    const quicker = minutes(one) - minutes(other);
    return quicker !== 0 && !Number.isNaN(quicker) ? quicker : one.listed - other.listed;
}

// Who an item is, across every file: its quest and its id.
function identity(item) {
    return `${item.quest}:${item.id ?? item.title}`;
}

// What an item says, so one Joel put off comes back once its owner says
// something new of it, and not when its file is rewritten for another.
// The to-do daemon writes the same to tell a put-off one apart
// (modules/to-do/src/stream.rs, `SAID`), so a field added here goes there.
function said(item) {
    return JSON.stringify([item.title, item.why ?? null, item.detail ?? null, item.when ?? null,
        item.ready ?? null, item.minutes ?? null, item.window ?? null, item.steps ?? null,
        item.at ?? null, item.deadline ?? null]);
}

// What an item not ready yet waits for, from what its owner said makes it
// ready: a signal as { signal, value } ("switch", or "device" and "dock"),
// a check as { check }, or, in a file older than that, its when as
// { words }; null when it says nothing.
function waitsFor(item) {
    const ready = item.readyWhen;
    if (typeof ready?.signal === "string") {
        const colon = ready.signal.indexOf(":");
        return colon < 0 ? { signal: ready.signal, value: "" }
            : { signal: ready.signal.slice(0, colon), value: ready.signal.slice(colon + 1) };
    }
    if (typeof ready?.check === "string") {
        return { check: true };
    }
    return typeof item.when === "string" && item.when !== "" ? { words: item.when } : null;
}

// How an item will be known done, from what its owner said of it: a
// signal as { signal, value }, a check as { check }, only Joel's word as
// { person }, or null when it says nothing, as a file older than that.
function doneBy(item) {
    const done = item.doneWhen;
    if (typeof done?.signal === "string") {
        const colon = done.signal.indexOf(":");
        return colon < 0 ? { signal: done.signal, value: "" }
            : { signal: done.signal.slice(0, colon), value: done.signal.slice(colon + 1) };
    }
    if (typeof done?.check === "string") {
        return { check: true };
    }
    return done?.by === "person" ? { person: true } : null;
}

// Whether anything will see an item done but Joel: the to do daemon, by
// its check or its signal, or its owner's session, while one is live
// (`live`, the addresses of those running). One only Joel can say is done
// and whose session has gone sits as though tracked, so it's said to be
// watched by nobody (VKCF3Z, 2026-10-30).
function watched(item, live) {
    const done = item.doneWhen;
    if (typeof done?.check === "string" || typeof done?.signal === "string") {
        return true;
    }
    const owner = item.owner?.address ?? item.from;
    return live[owner] === true;
}

// Whether a quest has ended, from what the index of quests said: unknown
// until the node answers, so a shell just started drops nothing; ended
// once the index no longer holds it, or says it's concluded or retired.
function ended(known, quest) {
    const owner = known[quest];
    return owner === null || (typeof owner?.status === "string" && ENDED.includes(owner.status));
}

// Everything that waits, in two lists: `now`, what Joel can do now; and
// `later`, what he can't yet or has put off; each in the order he does
// them (`before`). Joel's `settled` is { done: { identity: when }, notNow:
// { identity: { said, when } } }: one done stays gone, and one put off
// waits under Later until its owner says something new of it. The to do
// daemon's `verdicts` are { items: { identity: { state, at, how } } }: one
// done is gone, one ready is for now.
function sorted(items, known, settled, verdicts) {
    const now = [];
    const later = [];
    const done = settled?.done ?? {};
    const notNow = settled?.notNow ?? {};
    const found = verdicts?.items ?? {};
    items.forEach((one, listed) => {
        const verdict = found[identity(one)] ?? null;
        const gone = (QUEST.test(one.quest) && ended(known, one.quest))
            || done[identity(one)] !== undefined || verdict?.state === "done";
        if (gone) {
            return;
        }
        const item = Object.assign({}, one, { listed, ready: readyOf(one, verdict), verdict,
            putOff: notNow[identity(one)]?.said === said(one) });
        (item.ready && !item.putOff ? now : later).push(item);
    });
    return { now: now.sort(before), later: later.sort(before) };
}

// Joel's word on an item, added to what he's said: done, or not now.
function settle(settled, item, how, at) {
    const next = { done: Object.assign({}, settled?.done ?? {}),
        notNow: Object.assign({}, settled?.notNow ?? {}) };
    if (how === "done") {
        next.done[identity(item)] = at;
    } else {
        next.notNow[identity(item)] = { said: said(item), at };
    }
    return next;
}

// The quests the items name, whose place in the index says whether
// they've ended.
function quests(items) {
    const named = {};
    for (const item of items) {
        named[item.quest] = true;
    }
    return Object.keys(named).filter(quest => QUEST.test(quest)).sort();
}

// Who has something Joel can do now, by every name a session's presence
// might give them: the quest and the file's own address.
function standing(now) {
    const by = {};
    for (const item of now) {
        by[item.quest] = true;
        by[item.from] = true;
    }
    return by;
}

// Whether a session's want is truly on Joel. A yes it waits for, or a run
// stuck without him, only his keys answer. An action stands only while
// its quest has something he can do now, since a session's presence
// mirrors every item it keeps, ready or not. A question from a session
// another orchestrates is put to its orchestrator, who brings Joel what's
// his, so it isn't his to answer.
function asksJoel(presence, by) {
    const wants = presence?.wants;
    if (!wants) {
        return false;
    }
    if (wants.kind === "action") {
        const names = (presence.owns ?? []).concat(presence.addresses ?? []);
        return names.some(name => by[name] === true);
    }
    if (wants.kind === "question") {
        const orchestrator = presence.orchestratedBy;
        return typeof orchestrator !== "string" || orchestrator === ""
            || (presence.addresses ?? []).includes(orchestrator);
    }
    return true;
}

// Whether a want rings, as the bell says only that something needs
// Joel's keys now (WCJQSG's pivot, 2026-11-12, on his "my computer
// constantly dinging instead of being brought to what i need to interact
// with"): a session stopped on a question or a yes until he answers. Not
// a run that ended blocked, said only while he's away, nor an item of a
// session's own, which For You lists for him to do when he likes.
function rings(wants) {
    return wants?.kind === "question" || wants?.kind === "approval";
}

// Who a want is, for it to sound once: its session and the id the
// harness gives it, else its kind and when it began, which the harness
// keeps as a standing want is raised again. Never its words, which count
// up as a session holds more ("Holds 3 calls", "and 2 more"), and each
// count was once said as a new want, with its sound. Its words only when
// it gives no start.
function wantKey(address, wants) {
    if (typeof wants?.id === "string" && wants.id !== "") {
        return `${address} ${wants.id}`;
    }
    return `${address} ${wants?.kind ?? ""} ${wants?.since ?? wants?.said ?? ""}`;
}

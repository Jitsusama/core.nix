// What only Joel can do, for WaitingOnYou.qml and Sessions.qml (Needs You,
// VKCF3Z's, 2026-10-30), out of the files the agents keep, each quest's
// waiting/<address>.json, written by the session that needs him through
// mesh_wait; what the to do daemon found of each (verdicts.json, its
// alone), their quests' READMEs, and what Joel has said of each (the
// shell's own settled.json). Each item is written once, by its owner, and
// says how it knows it's done, so this decides which stand, now or later:
// one goes once its check passes or its signal comes, its quest ends, its
// owner takes it out or Joel says it's done; one is ready once its owner or
// a verdict says so. Nothing goes by its age. Kept apart from Quickshell's
// types, so the `waiting-on-you-is-true` check runs it in Qt's own engine
// (tests/waiting-on-you.qml).
.pragma library

// A quest's README status that means it's over, so nothing of it waits.
const ENDED = ["concluded", "retired"];

// A path's folders and its .json, which a file's name is without.
const FOLDERS = /^.*\//;
const JSON_END = /\.json$/;
// A quest's id, which a README stands behind.
const QUEST = /^QEST-/;
// A README's front-matter line naming its status.
const STATUS = /^status:\s*(\S+)/;

// Each file as a source of items, by its file's name, which is its
// quest's short address: `from`, who keeps it.
function sources(files) {
    return Object.keys(files).sort().map(path => ({
        from: path.replace(FOLDERS, "").replace(JSON_END, ""),
        read: files[path]
    }));
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

// Whether a quest has ended, from what its README said: unknown until it's
// read, so a shell just started drops nothing; ended when the README is
// missing or says concluded or retired.
function ended(owners, quest) {
    const owner = owners[quest];
    return owner === null || (typeof owner?.status === "string" && ENDED.includes(owner.status));
}

// Each item from a source, with who it's from, whether it's ready and since
// when it's been said (its own `since`, else its file's `updated`), or null
// for one that isn't an item.
function itemsOf(source) {
    const read = source.read;
    const listed = Array.isArray(read) ? read : read?.items;
    const quest = typeof read?.quest === "string" ? read.quest : source.from;
    return (Array.isArray(listed) ? listed : [])
        .filter(raw => typeof raw?.title === "string" && raw.title !== "")
        .map(raw => Object.assign({}, raw, {
            quest: typeof raw.quest === "string" ? raw.quest : quest,
            from: source.from,
            // Its file's version: 2 since mesh_wait writes how each is done.
            v: typeof read?.v === "number" ? read.v : 1,
            verdict: null,
            since: Date.parse(raw.since ?? read?.updated ?? ""),
            putOff: false
        }));
}

// Everything that waits, in two lists: `now`, what Joel can do now; and
// `later`, what he can't yet or has put off; each in the order he does
// them (`before`). Joel's `settled` is { done: { identity: when }, notNow:
// { identity: { said, when } } }: one done stays gone, and one put off
// waits under Later until its owner says something new of it. The to do
// daemon's `verdicts` are { items: { identity: { state, at, how } } }: one
// done is gone, one ready is for now.
function sorted(files, owners, settled, verdicts) {
    const now = [];
    const later = [];
    const done = settled?.done ?? {};
    const notNow = settled?.notNow ?? {};
    const found = verdicts?.items ?? {};
    let listed = 0;
    for (const source of sources(files)) {
        for (const item of itemsOf(source)) {
            item.listed = listed;
            listed += 1;
            const verdict = found[identity(item)] ?? null;
            const gone = (QUEST.test(item.quest) && ended(owners, item.quest))
                || done[identity(item)] !== undefined || verdict?.state === "done";
            if (gone) {
                continue;
            }
            item.ready = readyOf(item, verdict);
            item.verdict = verdict;
            item.putOff = notNow[identity(item)]?.said === said(item);
            (item.ready && !item.putOff ? now : later).push(item);
        }
    }
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

// The quests named by any file, whose READMEs say whether they've ended.
function quests(files) {
    const named = {};
    for (const source of sources(files)) {
        for (const item of itemsOf(source)) {
            named[item.quest] = true;
        }
    }
    return Object.keys(named).filter(quest => QUEST.test(quest)).sort();
}

// What a README says of its quest: its status, from its front matter, or
// "" when it gives none, and its title, its first heading.
function readme(text) {
    const lines = text.split("\n");
    let status = "";
    let body = 0;
    if (lines[0].trim() === "---") {
        body = lines.findIndex((line, i) => i > 0 && line.trim() === "---") + 1;
        for (const line of lines.slice(1, body)) {
            const found = STATUS.exec(line);
            if (found) {
                status = found[1];
            }
        }
    }
    const heading = lines.slice(body).find(line => line.startsWith("# "));
    return { status, title: heading ? heading.slice(2).trim() : "" };
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

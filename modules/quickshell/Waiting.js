// What truly waits on Joel, for WaitingOnYou.qml and Sessions.qml, out of
// the files the agents keep (the orchestrator's queue.json and each quest's
// waiting/<address>.json), their quests' READMEs, and what Joel has said of
// each (the shell's own settled.json). Those files are written by many
// sessions, each when it thinks of it, so they hold items Joel can't do
// yet, a quest's copy of what the orchestrator's queue already says, and
// files whose quest has ended. This decides which stand, now or later, so
// Waiting on You is true. An item goes only on an event, never by its age:
// its owner takes it out or its file goes, its quest ends, or Joel says
// it's done or not now. Kept apart from Quickshell's types, so the
// `waiting-on-you-is-true` check runs it in Qt's own engine
// (tests/waiting-on-you.qml).
.pragma library

// A quest's README status that means it's over, so nothing of it waits.
const ENDED = ["concluded", "retired"];

// A path's folders and its .json, which a file's name is without.
const FOLDERS = /^.*\//;
const JSON_END = /\.json$/;
// Any run of spacing, which two owners might write differently.
const SPACING = /\s+/g;
// A quest's id, which a README stands behind.
const QUEST = /^QEST-/;
// A README's front-matter line naming its status.
const STATUS = /^status:\s*(\S+)/;

// Each file as a source of items: the orchestrator's first, then each
// quest's by its file's name, so the first is the orchestrator's choice.
// `from` is who keeps it: the orchestrator's address, or the file's name,
// which is the quest's short address.
function sources(queue, files) {
    const all = [];
    if (queue !== null && typeof queue === "object") {
        all.push({ from: orchestratorOf(queue), read: queue, queue: true });
    }
    for (const path of Object.keys(files).sort()) {
        all.push({
            from: path.replace(FOLDERS, "").replace(JSON_END, ""),
            read: files[path],
            queue: false
        });
    }
    return all;
}

// The orchestrator's address, which the file once called the steward.
function orchestratorOf(queue) {
    return [queue?.orchestrator, queue?.steward].find(address => typeof address === "string") ?? "";
}

// Whether Joel can do an item now, as its owner says: `ready`, or, where a
// file gives no `ready`, as the orchestrator's doesn't, its when saying now.
function readyOf(item) {
    if (typeof item.ready === "boolean") {
        return item.ready;
    }
    return typeof item.when === "string" && item.when.trim().toLowerCase() === "now";
}

// A title as two owners might both write it, case and spacing aside.
function plain(title) {
    return title.toLowerCase().replace(SPACING, " ").trim();
}

// Who an item is, across every file: its quest and its id.
function identity(item) {
    return `${item.quest}:${item.id ?? item.title}`;
}

// What an item says, so one Joel put off comes back once its owner says
// something new of it, and not when its file is rewritten for another.
function said(item) {
    return JSON.stringify([item.title, item.why ?? null, item.detail ?? null, item.when ?? null,
        item.ready ?? null, item.minutes ?? null, item.window ?? null]);
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
            ready: readyOf(raw),
            since: Date.parse(raw.since ?? read?.updated ?? ""),
            putOff: false
        }));
}

// Everything that waits, in two lists: `now`, what Joel can do now, in the
// owners' order; and `later`, what he can't yet or has put off. Joel's
// `settled` is { done: { identity: when }, notNow: { identity: { said,
// when } } }: one done stays gone, and one put off waits under Later until
// its owner says something new of it.
function sorted(queue, files, owners, settled) {
    const covered = {};
    const titled = {};
    const now = [];
    const later = [];
    const done = settled?.done ?? {};
    const notNow = settled?.notNow ?? {};
    for (const source of sources(queue, files)) {
        for (const item of itemsOf(source)) {
            const id = item.id ?? item.title;
            const gone = (QUEST.test(item.quest) && ended(owners, item.quest))
                || done[identity(item)] !== undefined
                || (!source.queue && (covered[`${item.quest}:${id}`]
                    || covered[`${item.from}:${id}`] || titled[plain(item.title)]));
            if (gone) {
                continue;
            }
            if (source.queue) {
                for (const one of Array.isArray(item.covers) ? item.covers : []) {
                    covered[one] = true;
                }
                titled[plain(item.title)] = true;
            }
            item.putOff = notNow[identity(item)]?.said === said(item);
            (item.ready && !item.putOff ? now : later).push(item);
        }
    }
    return { now, later };
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
function quests(queue, files) {
    const named = {};
    for (const source of sources(queue, files)) {
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

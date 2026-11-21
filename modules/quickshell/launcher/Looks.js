// The looks folder's files, as For You reads them (Now.qml, ForYouHolds.qml): what each look is,
// which wait on Joel and which are history, and how one is written again once it's done with.
// They're read apart from the launcher, so tests/for-you.qml holds them in Qt's own engine. The
// to-do daemon reads the looks for the shell (KP0L6A's #4032), and serves each done with as
// History's row ready to draw (modules/to-do/src/history.rs), which these rules must agree with.
.pragma library
.import "ForYou.js" as ForYou
.import "Order.js" as Order

// What a look is about, and how it stands, as the harness writes them, and
// the one version of its file this reads.
const KINDS = ["pr", "page", "file", "diff", "terminal"];
const STATES = ["waiting", "opened", "cleared", "resolved", "expired"];
const VERSION = 1;
// What's read from a look's file only for the shell's own use, never
// written back.
const OWN = ["since", "resolved", "file", "words", "offers"];
// How many of a look's choices are offered, as the harness writes at most
// four (85PKB2, 2026-11-12).
const OFFERED = 4;
// How long an answer shows as given while the look's file still waits:
// its session writes the look resolved as it reads the steer, so one
// still waiting a minute on didn't take, and its choices come back.
const ANSWER_TAKES = 60 * 1000;
// How long a look waits before it goes quiet: it leaves what waits, and so
// every count of it, for History, where it's still opened as it was. By then
// why it came is lost (Joel, 2026-11-11: "by the time i find them the context
// of why they exist is lost"), and VKCF3Z's flows say "a look older than
// three days goes quiet".
const QUIET_AFTER = 3 * 24 * 60 * 60 * 1000;

// Whether a file's JSON is a look this reads whole: version 1, with what
// it's about and how it stands.
function whole(read) {
    const named = value => typeof value === "string" && value !== "";
    return read !== null && typeof read === "object" && read.v === VERSION && named(read.id)
        && named(read.title) && named(read.target) && KINDS.includes(read.kind)
        && STATES.includes(read.state);
}

// One look from its file's text, or null for one that can't be read
// whole: not JSON, not version 1, or missing what it's about.
function look(text) {
    let read = null;
    try {
        read = JSON.parse(text);
    } catch (_malformed) {
        return null;
    }
    if (!whole(read)) {
        return null;
    }
    return Object.assign({}, read, {
        why: typeof read.why === "string" ? read.why : "",
        // Its drawer says why, then the ask and its choices, to act on hours on (Joel, 2026-11-11).
        words: [read.why, read.ask].filter(part => part && typeof part === "string").join("\n\n"),
        since: ForYou.when(read.at),
        resolved: ForYou.when(read.resolvedAt),
        offers: offers(read.choices)
    });
}

// A quest's id as its title ends with it, "(INV-31016)", which says
// nothing to Joel (attachments/what-needs-me-table.md, 2026-11-19).
const TITLE_ID = /\s*\([A-Z][A-Z0-9]*-\d+\)\s*$/u;

// Who a look comes from, in plain words: what its session says, else its
// quest's title without its id (`quest`, the index's entry, or null), else
// "". Never the quest's id, which tells Joel nothing.
function askerOf(one, quest) {
    const asker = written(one.asker);
    return asker !== "" ? asker : String(quest?.title ?? "").replace(TITLE_ID, "");
}

// What a look asks of Joel, and what happens if he does nothing, as its
// session writes them (85PKB2, 2026-11-19), or "". Read here rather than
// into the look, since a look is written back as it was read (`rewritten`).
function askOf(one) {
    return written(one.ask);
}

function ifNothingOf(one) {
    return written(one.ifNothing);
}

function written(value) {
    return typeof value === "string" ? value : "";
}

// A look's choices as it's offered them, each { value, label }: one
// naming no value left out, one naming no label labelled by its value,
// and at most OFFERED.
function offers(choices) {
    const named = value => typeof value === "string" && value !== "";
    return (Array.isArray(choices) ? choices : [])
        .filter(one => one !== null && typeof one === "object" && named(one.value))
        .slice(0, OFFERED)
        .map(one => ({ value: one.value, label: named(one.label) ? one.label : one.value }));
}

// Joel's answer to a look, as the steer that tells it to the session that
// left it, `{ address, steer }`, at `at`: to that session while it runs,
// else to its quest's, whose address is the quest's own last part, so the
// look is answered where the asking goes on (85PKB2, 2026-11-12). Null
// where neither is known.
function answer(one, choice, live, at) {
    const quest = String(one.quest ?? "").split("-").pop();
    const address = String((live ? one.address : quest) || one.address || "").toUpperCase();
    if (address === "") {
        return null;
    }
    return { address, steer: { act: "answer", look: one.id, value: choice.value,
        from: "joel", at: new Date(at).toISOString() } };
}

// The cell a look leads to, the session a look's Talk and Go To reach (Joel,
// 2026-11-16: "i have no way to go from the thing it shows to engaging with
// the thing working on it"): the one that left it, running or parked, else,
// once that has ended, the quest's own session, whose address is the
// quest's last part and where a look's answers go then (`answer`), else any
// other on its quest, a running one before a parked one. Null where none is
// held.
function cellOf(one, running, parked) {
    const held = (Array.isArray(running) ? running : [])
        .concat(Array.isArray(parked) ? parked : []);
    const left = held.find(session => ForYou.leftBy(one, session));
    const quest = typeof one.quest === "string" ? one.quest : "";
    if (left !== undefined || quest === "") {
        return left ?? null;
    }
    const own = quest.split("-").pop().toUpperCase();
    const on = held.filter(session => questOf(session) === quest);
    return on.find(session => String(session.address ?? "").toUpperCase() === own)
        ?? on[0] ?? null;
}

// The quest a session works on, as pi publishes it: its questInfo's, else
// its `quest` (Holders.js and Sessions.qml read it so too).
function questOf(session) {
    return session.presence?.questInfo?.id ?? session.presence?.quest;
}

// The address the agents daemon says the cell a look names (`cell`) goes
// to now, by its `agent-cell` records by key, `cells`
// (modules/agents/src/cells.rs): null where the record says it goes
// nowhere, and "" where it names no cell or the daemon holds no record.
function cellGoes(one, cells) {
    const cell = typeof one.cell === "string" ? one.cell : "";
    const record = cell === "" ? undefined : cells?.[`agent-cell:agents:${cell}`];
    if (record !== undefined && record !== null && record.goes === null) {
        return null;
    }
    return typeof record?.goes === "string" ? record.goes : "";
}

// The cell a look leads to: the cell itself while the lists hold it, else
// the live session the agents daemon names (cellGoes), found among
// `running` and `parked` by the address it goes to. Where the daemon names
// a session those lists don't hold, says the cell goes nowhere, or has not
// answered (away, or not yet served), cellOf's search decides among the
// sessions on the look's quest alone; a look on none, sharing no quest,
// reaches only the session that left it, by that session's own id: the
// daemon and the lists each read the node and either can be a beat
// behind, so a cell back again wins over an older answer. A session that
// merely lists the cell's address is never found by that search; the
// daemon names one only where cells.rs finds it was the cell, its one
// lister while no session goes by the name. A look naming no cell searches
// every session until every look names its cell, by Mon 23 Nov 2026.
function cellFor(one, cells, running, parked) {
    if (typeof one.cell !== "string" || one.cell === "") {
        return cellOf(one, running, parked);
    }
    const goes = cellGoes(one, cells);
    const held = (Array.isArray(running) ? running : [])
        .concat(Array.isArray(parked) ? parked : []);
    const quest = typeof one.quest === "string" ? one.quest : "";
    const named = held.find(session => session.address === one.cell)
        ?? held.find(session => goes !== null && goes !== "" && session.address === goes);
    if (named !== undefined) {
        return named;
    }
    if (quest === "") {
        return held.find(session => typeof one.session === "string" && one.session !== ""
            && session.presence?.session === one.session) ?? null;
    }
    return cellOf(one, held.filter(session => questOf(session) === quest), []);
}

// A look done with, as History leads it to its cell: one written before
// looks named their cell names it by its address, the session that left
// it, so cellFor holds it to that cell, the daemon's answer for it and its
// own quest, never the open search, which History, keeping every look done
// with, would otherwise keep past Mon 23 Nov 2026. One gone quiet and opened
// is written back naming that cell, as the harness names every look's now.
function doneWith(one) {
    const named = typeof one.cell === "string" && one.cell !== "";
    return named || typeof one.address !== "string" || one.address === "" ? one
        : Object.assign({}, one, { cell: one.address });
}

// The label of the answer Joel gave a look still waiting, by `answered`'s
// { [look id]: { label, at } }, while it may yet take, or "".
function answering(one, answered, now) {
    const given = answered?.[one.id];
    return one.state === "waiting" && given && now - given.at < ANSWER_TAKES ? given.label : "";
}

// The looks as `looks` splits them, with each waiting one whose answer the
// node took (`answered`'s { [look id]: { label, at, taken } }) in History,
// answered then, its side as `saying` words its choice: Joel's answer has
// handled it, so it leaves what waits at once (VKCF3Z, 2026-11-12), though
// its file waits until its session reads the steer and writes it resolved.
function taken(split, answered, saying) {
    const given = answered ?? {};
    const moved = split.waiting.filter(one => given[one.id]?.taken === true);
    if (moved.length === 0) {
        return split;
    }
    const done = moved.map(one => Object.assign({}, one, { state: "answered",
        how: saying(given[one.id].label), resolved: given[one.id].at }));
    return {
        waiting: split.waiting.filter(one => given[one.id]?.taken !== true),
        reading: split.reading ?? [],
        history: Order.sorted(done.concat(split.history),
            Order.stream(one => one.resolved ?? one.since))
    };
}

// A look waiting past its time, as History holds it: expired at its expiresAt, which the
// harness gives one asking nothing (85PKB2, 2026-11-12), or gone quiet QUIET_AFTER after it
// came, which History's side says. Its file still says it waits until a session of its quest
// writes it expired; nothing's written here, and the look as its file was read, kept to be
// read again, is left as it was.
function quieted(one, now) {
    if (one.state !== "waiting") {
        return one;
    }
    const expires = ForYou.when(one.expiresAt);
    if (expires !== null && now >= expires) {
        return Object.assign({}, one, { state: "expired", resolved: expires });
    }
    const gone = one.since !== null && now - one.since > QUIET_AFTER;
    return gone ? Object.assign({}, one, { state: "quiet", resolved: one.since + QUIET_AFTER })
        : one;
}

// Whether a look asks nothing of Joel, only to be seen, so it's to read rather than to look
// at: the harness says so as `asks: false`, and one saying nothing asks (85PKB2, 2026-11-12).
function asksNothing(one) {
    return one.asks === false;
}

// The looks folder's files, each { file, text }, as { waiting, reading, history }: those
// waiting that ask something, then those that ask nothing (To Read, VKCF3Z, 2026-11-12), each
// in the order they came, the oldest first, the rest newest first by when each was resolved,
// expired or went quiet, as at `now`. One still being written, `*.writing`, or unreadable is
// left out. A file whose text is as `kept.by` holds it isn't parsed again: an act changes one of
// Joel's 166, and parsing all took 9 ms (2026-11-10).
function looks(files, kept, now) {
    const was = kept.by ?? {};
    const read = {};
    const listed = (files ?? []).filter(one => !String(one.file ?? "").endsWith(".writing"))
        .map(one => {
            const file = String(one.file ?? "");
            const same = file in was && was[file].text === one.text;
            const each = same ? was[file].look : look(one.text);
            const made = each === null ? null : Object.assign(each, { file });
            read[file] = { text: one.text, look: made };
            return made === null ? null : quieted(made, now);
        }).filter(one => one !== null);
    kept.by = read;
    const waits = listed.filter(one => one.state === "waiting");
    return {
        waiting: Order.sorted(waits.filter(one => !asksNothing(one)),
            Order.smaller(one => one.since)),
        reading: Order.sorted(waits.filter(asksNothing), Order.smaller(one => one.since)),
        history: Order.sorted(listed.filter(one => one.state !== "waiting"),
            Order.stream(one => one.resolved ?? one.since))
    };
}

// A look's file as it's written again once it's done with, whole, opened
// by Joel or cleared: everything it said, and how and when it ended.
function rewritten(one, state, how, now) {
    const kept = {};
    for (const key of Object.keys(one).filter(name => !OWN.includes(name))) {
        kept[key] = one[key];
    }
    return JSON.stringify(Object.assign(kept, { state, how,
        resolvedAt: new Date(now).toISOString() }), null, 2);
}

// Whether the to-do daemon's text of a look, `told`, says just what the
// text the shell showed it settled in, `shown`, did, but for when it was
// settled: the daemon writes the same look in its own key order at its own
// moment, so taking its text made the look's row again a moment after the
// act's own frame, and For You was drawn again for nothing.
function settledAlike(shown, told) {
    const read = text => {
        try {
            const one = JSON.parse(text);
            return one !== null && typeof one === "object" ? one : null;
        } catch (_unread) {
            return null;
        }
    };
    const was = read(shown);
    const is = read(told);
    if (was === null || is === null) {
        return false;
    }
    const unsettled = one => canonical(Object.assign({}, one, { resolvedAt: null }));
    return unsettled(was) === unsettled(is);
}

// A value as JSON with every object's keys in order, at any depth, since
// the daemon may order them otherwise than the look's session did.
function canonical(value) {
    if (Array.isArray(value)) {
        return `[${value.map(canonical).join(",")}]`;
    }
    if (value !== null && typeof value === "object") {
        return `{${Object.keys(value).sort()
            .map(key => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
    }
    return JSON.stringify(value);
}

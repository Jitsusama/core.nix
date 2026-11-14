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

// The looks folder's files, as For You reads them (Now.qml, ForYouHolds.qml): what each look is,
// which wait on Joel and which are history, and how one is written again once it's done with.
// They're read apart from the launcher, so tests/for-you.qml holds them in Qt's own engine. A
// stopgap: KP0L6A's agents daemon takes the looks over by Sat 14 Nov, and serves them ready.
.pragma library
.import "ForYou.js" as ForYou
.import "Order.js" as Order

// What a look is about, and how it stands, as the harness writes them, and
// the one version of its file this reads.
const KINDS = ["pr", "page", "file", "diff", "terminal"];
const STATES = ["waiting", "opened", "cleared", "resolved"];
const VERSION = 1;
// What's read from a look's file only for the shell's own use, never
// written back.
const OWN = ["since", "resolved", "file", "words"];
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
        resolved: ForYou.when(read.resolvedAt)
    });
}

// A look waiting since longer ago than QUIET_AFTER, as History holds it: gone quiet when its
// time ran out, which History's side says. Its file still says it waits; nothing's written,
// and the look as its file was read, kept to be read again, is left as it was.
function quieted(one, now) {
    const gone = one.state === "waiting" && one.since !== null && now - one.since > QUIET_AFTER;
    return gone ? Object.assign({}, one, { state: "quiet", resolved: one.since + QUIET_AFTER })
        : one;
}

// The looks folder's files, each { file, text }, as { waiting, history }: those waiting in the
// order they came, the oldest first, the rest newest first by when each was resolved or went
// quiet, as at `now`. One still being written, `*.writing`, or unreadable is left out. A file
// whose text is as `kept.by` holds it isn't parsed again: an act changes one of Joel's 166, and
// parsing all took 9 ms (2026-11-10).
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
    return {
        waiting: Order.sorted(listed.filter(one => one.state === "waiting"),
            Order.smaller(one => one.since)),
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

// Lately, For You's band below what needs Joel (Now.qml): what came for
// him and is still unread, one row for each source, the mail, Slack and
// the desktop's other notices, each done once its source says so (Joel,
// 2026-11-08: "i shouldn't be required to mark a notice as read if I
// looked at the slack message in slack"). What's done goes to For You's
// History, saying how: mail as Google's daemon says it went, and notices
// as read here or withdrawn, and Slack's messages as its daemon says
// (NewSlack.qml). Slack's notices stand for them only while the daemon
// doesn't answer (held).
// Kept apart from the QML so Qt's own engine holds it (tests/lately.qml).
.pragma library
.import "Order.js" as Order

// The sources, in the order rows alike in weight stand: mail and Slack
// first, since a person wrote each, then what programs said.
const SOURCES = ["mail", "slack", "desktop"];
// The senders the shell signs its own notices for what For You lists above
// Lately itself with, so neither is counted twice: a Needs You item's
// (Y2YBTS, 2026-11-08) and a pi session's want, For You's Agents row
// (Sessions.qml). Each is marked read once its item stops waiting.
const LISTED = ["waiting", "pi"];
// Slack's sender, by the name its program gives and by its desktop entry.
const SLACK = "slack";

// A time as milliseconds, from a number or an ISO time, or null.
function when(at) {
    if (typeof at === "number") {
        return Number.isFinite(at) ? at : null;
    }
    if (typeof at !== "string" || at === "") {
        return null;
    }
    const read = Date.parse(at);
    return Number.isNaN(read) ? null : read;
}

// Which source a notice is from: Slack's, by its program's name or its
// desktop entry, else the desktop's.
function sourceOf(notice) {
    const named = value => String(value ?? "").toLowerCase();
    return named(notice.app) === SLACK || named(notice.entry) === SLACK ? "slack" : "desktop";
}

// Whether a notice is one of Lately's: not the shell's own for something
// For You lists. One forgotten is gone from the history, so it isn't here
// either.
function ours(notice) {
    return notice !== null && typeof notice === "object" && !LISTED.includes(notice.app);
}

// How a Slack message or a piece of mail was done with, as its daemon says
// it, { how, at }, or null while it still wants Joel.
function doneOf(thing) {
    const ended = thing?.done;
    return ended !== null && typeof ended === "object" && typeof ended.how === "string"
        ? ended : null;
}

// The unread notices of each source, as { slack, desktop }, newest first
// as Notices' history holds them. A source in `answering`, one whose own
// piece says what's done with by itself, holds none: Slack's notices
// stand for Slack's row only while its daemon doesn't answer, away or not
// there, never because it has nothing unread, since what Joel read in
// Slack mustn't come back as notices (Y2YBTS, 2026-11-08).
function held(notices, answering) {
    const unread = (notices ?? []).filter(notice => ours(notice) && notice.read !== true
        && !(answering ?? []).includes(sourceOf(notice)));
    return {
        slack: unread.filter(notice => sourceOf(notice) === "slack"),
        desktop: unread.filter(notice => sourceOf(notice) === "desktop")
    };
}

// The notices as For You holds them while the launcher is open, since
// `since`, when it opened (Infinity while it's closed): one Joel read
// since is unread still, so Desktop holds still while he looks at it
// (VKCF3Z, 2026-11-08) and u brings back what d did there. One withdrawn
// goes at once, since Joel never read it (Y2YBTS, 2026-11-08).
function holdingStill(notices, since) {
    return (notices ?? []).map(notice => notice.read === true && readHow(notice) === "read"
        && when(notice.readAt) >= since ? Object.assign({}, notice, { read: false }) : notice);
}

// The rows Lately shows, { source, count, weight }, a source holding
// nothing left out, as Now's own pieces go once none want him. They stand
// by the heaviest each holds, as `weigh(thing, source)` (ForYou.js's)
// weighs it, so Lately orders as the rest of For You does, and those
// alike in the sources' order (VKCF3Z, 2026-11-08).
function bands(holding, weigh) {
    const rows = SOURCES.filter(source => holding[source].length > 0)
        .map(source => ({ source, count: holding[source].length,
            weight: Math.max(...holding[source].map(thing => weigh(thing, source))) }));
    return Order.sorted(rows, Order.larger(row => row.weight));
}

// Who a source's things are from, newest first, each once, as its row's
// note names them (VKCF3Z, 2026-11-08): `nameOf(thing)` says who, and one
// who says nothing is left out.
function members(things, nameOf) {
    const named = [];
    for (const thing of things) {
        const name = String(nameOf(thing) ?? "");
        if (name !== "" && !named.includes(name)) {
            named.push(name);
        }
    }
    return named;
}

// How a notice read was done with: withdrawn, by its sender or the shell;
// read elsewhere, as one of Slack's whose message was read in Slack
// (Notices.qml's readHow), "read in Slack 14:02" (VKCF3Z, 2026-11-09);
// read, here, when the history says when; or read before it kept when.
function readHow(notice) {
    if (notice.withdrawn === true || notice.left === "withdrawn") {
        return "withdrawn";
    }
    if (typeof notice.readHow === "string" && notice.readHow !== "") {
        return notice.readHow;
    }
    return when(notice.readAt) === null ? "earlier" : "read";
}

// What's been done with, for History, each { title, how, at, source, of },
// newest first: `of` is what it was, `how` its source's word for how it
// went, worded by `say` ({ mail, slack, notice }, words.json's, each keyed
// by the source's own id for it), or as the source said it where `say`
// has no word. `given` is { notices, messages, finished }: Notices'
// history, the messages Slack's daemon holds or null while it can't say,
// and the mail done with (Google's). A notice read is done; one withdrawn,
// by its sender or by the shell once what it said no longer stands, is
// read too (Notices.qml's left and mark), and says so; one read before
// the history kept when says only that it was read. Each is when it was
// read, else when it left, else when it came. While Slack's daemon
// answers, its read messages are Slack's history and its notices aren't,
// so none is told twice.
function done(given, say) {
    const worded = (words, how) => words?.[how] ?? how;
    const messages = Array.isArray(given.messages) ? given.messages : null;
    const mail = (Array.isArray(given.finished) ? given.finished : [])
        .filter(one => doneOf(one) !== null)
        .map(one => ({ title: String(one.subject ?? ""), how: worded(say.mail, one.done.how),
            at: when(one.done.at), source: "mail", of: one }));
    const slack = (messages ?? []).filter(one => doneOf(one) !== null)
        .map(one => ({ title: String(one.who ?? ""), how: worded(say.slack, one.done.how),
            at: when(one.done.at), source: "slack", of: one }));
    const notices = (given.notices ?? [])
        .filter(notice => ours(notice) && notice.read === true
            && (messages === null || sourceOf(notice) !== "slack"))
        .map(notice => ({ title: String(notice.summary ?? ""),
            how: worded(say.notice, readHow(notice)),
            at: when(notice.readAt) ?? when(notice.leftAt) ?? when(notice.at),
            source: sourceOf(notice),
            of: notice }));
    return Order.sorted(mail.concat(slack, notices), Order.stream(one => one.at));
}

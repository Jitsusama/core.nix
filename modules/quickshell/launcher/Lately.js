// Lately, For You's band below what needs Joel (Now.qml): what came for
// him and is still unread, one row for each source, the mail, Slack and
// the desktop's other notices, each done once its source says so, so a
// message read in Slack leaves here by itself (Joel, 2026-11-08: "i
// shouldn't be required to mark a notice as read if I looked at the slack
// message in slack"). What's done goes to For You's History, saying how.
// Kept apart from the QML so Qt's own engine holds it (tests/lately.qml).
.pragma library
.import "Order.js" as Order

// The sources, in the order rows alike in weight stand: mail and Slack
// first, since a person wrote each, then what programs said.
const SOURCES = ["mail", "slack", "desktop"];
// The sender the shell signs its own notice for a Needs You item with,
// which For You lists above Lately itself, so it's never counted twice
// (Y2YBTS, 2026-11-08).
const WAITING = "waiting";
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

// Whether a notice is one of Lately's: not the shell's own for a Needs You
// item. One forgotten is gone from the history, so it isn't here either.
function ours(notice) {
    return notice !== null && typeof notice === "object" && notice.app !== WAITING;
}

// How a Slack message or a piece of mail was done with, as its daemon says
// it, { how, at }, or null while it still wants Joel.
function doneOf(thing) {
    const ended = thing?.done;
    return ended !== null && typeof ended === "object" && typeof ended.how === "string"
        ? ended : null;
}

// The notices the history keeps, in their order, newest first: every one
// unread whatever its age, since one stays until it's read or forgotten
// (Y2YBTS, 2026-11-08), and those read for `readFor`. Past `most`, the
// oldest read go first, and the oldest unread only once none read are
// left, so what Joel hasn't seen outlasts what he has.
function kept(notices, now, readFor, most) {
    const young = notices.filter(notice => notice.read !== true || now - notice.at < readFor);
    let over = young.length - most;
    const gone = new Set();
    for (const read of [true, false]) {
        for (let at = young.length - 1; at >= 0 && over > 0; at--) {
            if ((young[at].read === true) === read) {
                gone.add(at);
                over--;
            }
        }
    }
    return young.filter((_notice, at) => !gone.has(at));
}

// What each source holds now, as { mail, slack, desktop }, each in the
// order its source gives, newest first. `given` is { notices, mail,
// messages }: Notices' history, the mail that wants him (Google's, which
// leaves done mail out), and the messages Slack's daemon holds, or null
// while it doesn't answer. While it answers, Slack's row is its messages
// still unread, and Slack's own notices are cards only, since they say no
// conversation to follow; while it doesn't, Slack's unread notices stand
// for them (Y2YBTS, 2026-11-08, 2abd047a).
function held(given) {
    const unread = (given.notices ?? []).filter(notice => ours(notice) && notice.read !== true);
    const messages = Array.isArray(given.messages) ? given.messages : null;
    return {
        mail: Array.isArray(given.mail) ? given.mail : [],
        slack: messages === null ? unread.filter(notice => sourceOf(notice) === "slack")
            : messages.filter(message => doneOf(message) === null),
        desktop: unread.filter(notice => sourceOf(notice) === "desktop")
    };
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

// What's been done with, for History, each { title, how, at, source, of },
// newest first: `of` is what it was, `how` its source's word for how it
// went, worded by `say` ({ mail, slack, notice }, words.json's, each keyed
// by the source's own id for it), or as the source said it where `say`
// has no word. `given` is held's, with `finished` the mail done with
// (Google's). A notice read is done; one withdrawn by its sender is read
// too (Notices.qml's left), and says so. While Slack's daemon answers,
// its read messages are Slack's history and its notices aren't, so none
// is told twice.
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
        .map(notice => {
            const how = notice.left === "withdrawn" ? "withdrawn" : "read";
            return { title: String(notice.summary ?? ""), how: worded(say.notice, how),
                at: when(notice.leftAt) ?? when(notice.at), source: sourceOf(notice),
                of: notice };
        });
    return Order.sorted(mail.concat(slack, notices), Order.stream(one => one.at));
}

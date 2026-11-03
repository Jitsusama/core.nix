// What in Slack is unread and what Joel read last, as Slack's daemon holds
// it (src/daemon.rs) and the shell watches it (SlackUnreads.qml), Now's
// Slack piece says what of it wants him (NewSlack.qml) and Slack's
// program lists it as Unread and Recent (SlackProgram.qml): how what's
// held stands, how many to read, from whom, and each conversation's row's
// words. A count is shown only while the daemon answers and Slack answered
// it, since the daemon holds conversations only then and the shell lets
// them go once it stops answering: an old count is never shown as though
// it were Slack's (Joel, 2026-10-28: "out of date information also sucks
// ass"). Why there's no count is said on Slack's program row
// (SlackProgram.qml), not in Now, since Slack out of reach doesn't want
// Joel (VKCF3Z, b8c8920e). Kept apart from the QML so Qt's own engine runs
// it (tests/slack-unreads.qml).
.pragma library

// The order what's unread comes in, as Slack's daemon says it: direct
// messages first, then channels mentioning him, then threads, then
// channels merely unread (VKCF3Z, f0c8ef61 and d7c0f933).
const groups = { dm: 0, mention: 1, thread: 2, unread: 3 };

// Newest first by one of Slack's timestamps, which are of fixed width and
// so sort as text, and by id where two are the same, so the order never
// rests on the order records were heard in.
function newest(field) {
    return (one, other) => {
        const at = String(other[field] ?? "").localeCompare(String(one[field] ?? ""));
        return at !== 0 ? at : one.id.localeCompare(other.id);
    };
}

// What the daemon holds, by key (Daemon.qml's `held`), as { state,
// conversations, recent }, or null while it doesn't answer or hasn't yet
// said how Slack stands. A conversation is kept when it's whole: a channel
// merely unread, or one in Recent, has no count, and anything else has
// one.
function gathered(held, answering) {
    if (!answering || held === null || typeof held !== "object") {
        return null;
    }
    const account = held["slack-account:slack"]?.["slack-account"];
    if (account === null || typeof account !== "object"
        || typeof account.standing !== "string") {
        return null;
    }
    const whole = one => one !== null && typeof one === "object"
        && typeof one.id === "string" && typeof one.name === "string"
        && typeof one.link === "string";
    const of = kind => Object.values(held)
        .filter(record => record?.kind === kind)
        .map(record => record[kind])
        .filter(whole);
    const first = newest("latest");
    return {
        state: account.standing,
        conversations: of("slack-unread")
            .filter(one => one.kind === "unread" || one.count > 0)
            .sort((one, other) => ((groups[one.kind] ?? 4) - (groups[other.kind] ?? 4))
                || first(one, other)),
        recent: of("slack-recent").sort(newest("read"))
    };
}

// Whether a conversation wants Joel, as Now lists only those that do: a
// channel merely unread doesn't.
function wants(one) {
    return one.kind !== "unread";
}

function wanting(told) {
    return told.conversations.filter(wants);
}

// How what's held stands: "none" while the daemon isn't answering or has
// no sign-in to ask Slack with, "ok" while Slack answers, "signedOut" once
// Slack refused pi's sign-in, and "unknown" while Slack can't be reached.
function standing(told) {
    if (told === null || told.state === "none") {
        return "none";
    }
    if (told.state === "signedOut") {
        return "signedOut";
    }
    return told.state === "ok" ? "ok" : "unknown";
}

function count(told) {
    return wanting(told).reduce((all, one) => all + one.count, 0);
}

// Whether Now lists it: only with something to read, which wants Joel.
function shown(told, stands) {
    return stands === "ok" && count(told) > 0;
}

// Beside its name: how many to read.
function value(told, stands) {
    return stands === "ok" ? `${count(told)} to read` : "";
}

// What the margin's mark says when pointed at, as the mail's does.
function hint(told, stands) {
    if (stands !== "ok") {
        return "";
    }
    const all = count(told);
    return all === 1 ? "A Slack message wants you" : `${all} Slack messages want you`;
}

// Why there's no count, as Slack's program row's note: signed out, or that
// Slack can't be reached; nothing while there's a count or nothing to ask.
function programNote(stands) {
    if (stands === "signedOut") {
        return "signed out";
    }
    return stands === "unknown" ? "Slack can't be reached" : "";
}

// Its note: who and where, the first two named and how many more, of
// those wanting him in Now, or of every one in Slack's Unread.
function context(told, stands, every) {
    if (stands !== "ok") {
        return "";
    }
    const names = (every ? told.conversations : wanting(told)).map(one => one.name);
    if (names.length <= 2) {
        return names.join(" and ");
    }
    return `${names.slice(0, 2).join(", ")} and ${names.length - 2} more`;
}

// A conversation's row's words: how many to read beside it, when it's
// counted, and what kind it is in its note.
function beside(one) {
    return one.count > 0 ? `${one.count} to read` : "";
}

const kinds = {
    dm: "a direct message",
    mention: "mentions you",
    thread: "replies in threads",
    unread: "a channel",
    channel: "a channel"
};

function note(one) {
    return kinds[one.kind] ?? "";
}

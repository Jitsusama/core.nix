// What in Slack wants Joel, as slack-unreads keeps it in the runtime
// folder's slack-unreads.json (src/bin/slack-unreads.rs) and Now's Slack
// piece says it (SlackUnreads.qml, NewSlack.qml): whether what the file says is current,
// how many to read, from whom, and each conversation's row's words. A
// count is current only while Slack answered and the file was written in
// the last ten minutes, since it's written at least every five: an old
// count is never shown as though it were Slack's (Joel, 2026-10-28: "out
// of date information also sucks ass"). Why there's no count is said on
// Slack's program row (SlackProgram.qml), not in Now, since Slack out of
// reach doesn't want Joel (VKCF3Z, b8c8920e). Kept apart from the QML so
// Qt's own engine runs it (tests/slack-unreads.qml).
.pragma library

// How long a check holds: twice the longest the service goes between
// writes, so one slow ask doesn't make Now say it can't check.
const fresh = 10 * 60 * 1000;

// The file's words as { checked, state, conversations }, or null when
// there's no file or it isn't one slack-unreads wrote.
function read(text) {
    let told;
    try {
        told = JSON.parse(text);
    } catch (_error) {
        return null;
    }
    if (told === null || typeof told !== "object" || typeof told.checked !== "number") {
        return null;
    }
    const conversations = Array.isArray(told.conversations) ? told.conversations : [];
    return {
        checked: told.checked,
        state: typeof told.state === "string" ? told.state : "",
        conversations: conversations.filter(one => one !== null && typeof one === "object"
            && typeof one.id === "string" && typeof one.name === "string"
            && typeof one.link === "string" && one.count > 0)
    };
}

// How what it says stands: "none" with no file, "ok" while Slack answered
// and it's fresh, "signedOut" once Slack refused pi's sign-in, and
// "unknown" when Slack couldn't be reached or the file has gone stale.
function standing(told, now) {
    if (told === null) {
        return "none";
    }
    if (told.state === "signedOut") {
        return "signedOut";
    }
    if (told.state !== "ok" || now - told.checked > fresh) {
        return "unknown";
    }
    return "ok";
}

// When a fresh file goes stale, as a time, or null when it can't.
function staleAt(told) {
    return told === null || told.state !== "ok" ? null : told.checked + fresh;
}

function count(told) {
    return told.conversations.reduce((all, one) => all + one.count, 0);
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

// Why there's no count, as Slack's program row's note: signed out, or
// since when it can't be checked, as a time of day, which stays true
// without a clock ticking to keep it so; nothing while there's a count or
// no file.
function programNote(told, stands) {
    if (stands === "signedOut") {
        return "signed out";
    }
    if (stands !== "unknown") {
        return "";
    }
    return told.state === "ok" ? `can't be checked since ${clock(told.checked)}`
        : "Slack can't be reached";
}

// Its note: who and where, the first two named and how many more.
function context(told, stands) {
    if (stands !== "ok") {
        return "";
    }
    const names = told.conversations.map(one => one.name);
    if (names.length <= 2) {
        return names.join(" and ");
    }
    return `${names.slice(0, 2).join(", ")} and ${names.length - 2} more`;
}

function clock(at) {
    const when = new Date(at);
    const two = number => String(number).padStart(2, "0");
    return `${two(when.getHours())}:${two(when.getMinutes())}`;
}

// A conversation's row's words: how many to read beside it, and what
// kind it is in its note.
function beside(one) {
    return `${one.count} to read`;
}

const kinds = {
    dm: "a direct message",
    mention: "mentions you",
    thread: "replies in threads"
};

function note(one) {
    return kinds[one.kind] ?? "";
}

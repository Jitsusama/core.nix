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
// Joel (VKCF3Z, b8c8920e). Slack's own desktop notices of what was read in
// Slack are found here too, so the shell reads them. Kept apart from the
// QML so Qt's own engine runs it (tests/slack-unreads.qml).
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

// Whether a message the daemon holds can be listed: it names its
// conversation, Slack's time for it, who said it and when.
function listable(one) {
    return one !== null && typeof one === "object" && typeof one.channel === "string"
        && typeof one.ts === "string" && typeof one.who === "string"
        && typeof one.at === "number";
}

// What the daemon holds, by key (Daemon.qml's `held`), as { state,
// conversations, recent, messages, whole }, or null while it doesn't
// answer or hasn't yet said how Slack stands. A conversation is kept when
// it's whole: a channel merely unread, or one in Recent, has no count, and
// anything else has one. `messages` are each message wanting Joel, done
// ones too, the newest first (src/messages.rs), and `whole` whether the
// daemon says it holds every one, since a history Slack didn't give
// leaves some out.
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
        recent: of("slack-recent").sort(newest("read")),
        messages: Object.values(held)
            .filter(record => record?.kind === "slack-message")
            .map(record => record["slack-message"])
            .filter(listable)
            .sort((one, other) => (other.at - one.at) || other.ts.localeCompare(one.ts)),
        whole: account.messages === "whole"
    };
}

// The messages wanting Joel, one each, while the daemon holds every one,
// done ones too, the newest first, or null where it holds none or not
// all, so Slack's conversations stand for them, as they did before the
// daemon held messages. A thread's replies are never held, so Threads
// stays a conversation beside them (src/messages.rs).
function messages(told, stands) {
    return stands === "ok" && told.whole ? told.messages : null;
}

// Whether a message is done with, as its daemon says: read in Slack, or
// gone from it.
function done(one) {
    return one.done !== null && typeof one.done === "object" && typeof one.done.how === "string";
}

// The conversations a message never stands for, listed beside the
// messages: Threads, whose replies the daemon doesn't hold.
function unheld(told) {
    return wanting(told).filter(one => one.kind === "thread");
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

// How many to read: while the daemon holds every message, each still to
// read and Threads' replies, as For You's Slack row lists them; else
// what the conversations count.
function count(told) {
    if (told.whole) {
        return told.messages.filter(one => !done(one)).length + counted(unheld(told));
    }
    return conversationsCount(told);
}

// How many the conversations wanting Joel hold, for a row listing them,
// as Slack's own Unread does, so its count is what it lists whether or
// not the daemon holds each message.
function conversationsCount(told) {
    return counted(wanting(told));
}

function counted(conversations) {
    return conversations.reduce((all, one) => all + one.count, 0);
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

// Its note in Now: who and where, the first two named and how many more,
// of those wanting him. Slack's own Unread names them after its name
// instead, as every row holding things does (SlackProgram.qml).
function context(told, stands) {
    if (stands !== "ok") {
        return "";
    }
    const names = wanting(told).map(one => one.name);
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

// Slack's own desktop notices say what they're of only in words: a direct
// message's summary is "[team] from {who}" and its body the text, and
// another's is "[team] in {place}" and its body "{who}: {text}", with no
// id, link or time of the message to know it by.
const NOTICE = /^\[[^\]]*\] (from|in) (.+)$/;
// An emoji as Slack's markup names it, ":heart:", which the daemon keeps
// and the notice shows as the emoji itself, so neither is a word.
const SHORTCODE = /:[a-z0-9_+'-]+:/g;
const DIGIT = /[0-9]/;
// How long after Slack's time for a message its notice may come: Slack's
// raise one within a second or so (median 0.3 s over 108 kept, the
// slowest under two minutes), and its clock and the machine's may differ
// by a few seconds either way.
const NOTICED_WITHIN = 5 * 60 * 1000;
const CLOCKS_APART = 10 * 1000;
// The fewest words two texts must share where one is cut short, so a
// greeting or a lone "ok" stands only for its own words.
const FEWEST_SHARED = 3;

// What a Slack notice says of its message, { kind, place, who, text, at },
// `at` when it came, a kind of "direct", "channel" or "group", or null for one that isn't
// Slack's or doesn't say it as Slack does.
function noticed(notice) {
    const named = field => String(field ?? "").toLowerCase();
    if (named(notice?.app) !== "slack" && named(notice?.entry) !== "slack") {
        return null;
    }
    const said = NOTICE.exec(String(notice.summary ?? ""));
    if (said === null) {
        return null;
    }
    const body = String(notice.body ?? "");
    if (said[1] === "from") {
        return { kind: "direct", place: said[2], who: said[2], text: body, at: notice.at };
    }
    const by = body.indexOf(": ");
    if (by <= 0) {
        return null;
    }
    return { kind: said[2].startsWith("#") ? "channel" : "group", place: said[2],
        who: body.slice(0, by), text: body.slice(by + 2), at: notice.at };
}

// A text's words, lower case, its letters and digits alone: an emoji,
// named or shown, and punctuation are neither.
function words(text) {
    const all = [];
    let word = "";
    for (const each of String(text ?? "").toLowerCase().replace(SHORTCODE, " ")) {
        if (each.toUpperCase() !== each || DIGIT.test(each)) {
            word += each;
        } else if (word !== "") {
            all.push(word);
            word = "";
        }
    }
    return word === "" ? all : all.concat([word]);
}

// Whether a notice's text and a message's say the same: every word alike,
// or, where either is cut short, the shorter's words but its last, which
// the cut may have split, opening the longer's, and enough of them.
function sameWords(text, other) {
    const one = words(text);
    const two = words(other);
    if (one.length === two.length) {
        return one.length > 0 && one.every((word, at) => word === two[at]);
    }
    const [shorter, longer] = one.length < two.length ? [one, two] : [two, one];
    const kept = shorter.slice(0, -1);
    return kept.length >= FEWEST_SHARED && kept.every((word, at) => word === longer[at]);
}

// Whether a message the daemon holds is the one a notice says: its
// sender, its conversation, a direct message by who it's with and a
// channel by its name (a group's notice names it by its title and the
// daemon by its members, so a group is known by being one), its words,
// and its notice coming as Slack raises one.
function isNoticed(said, one) {
    const place = String(one.place ?? "");
    const placed = {
        direct: place === said.who,
        channel: place === said.place,
        group: !place.startsWith("#") && place !== one.who
    }[said.kind];
    const after = said.at - one.at;
    return one.who === said.who && placed && after >= -CLOCKS_APART
        && after <= NOTICED_WITHIN && sameWords(said.text, one.text);
}

// The ids of Slack's own notices still unread whose message the daemon
// says was read in Slack, so each is read in Notices too (Joel,
// 2026-11-08: "i shouldn't be required to mark a notice as read if I
// looked at the slack message in slack"). `doneWith` are the daemon's
// messages done with (SlackUnreads.qml's), or null while it doesn't hold
// every one. Nothing is read on a guess: a notice two messages could be,
// or one of two notices a message could be, is left as it is.
function readInSlack(notices, doneWith) {
    const read = (doneWith ?? []).filter(one => listable(one) && one.done?.how === "readInSlack");
    if (read.length === 0) {
        return [];
    }
    const matched = [];
    for (const notice of notices ?? []) {
        const said = notice?.read === true ? null : noticed(notice);
        if (said !== null) {
            matched.push({ id: notice.id, of: read.filter(one => isNoticed(said, one)) });
        }
    }
    const claimed = new Map();
    for (const each of matched) {
        for (const one of each.of) {
            const key = `${one.channel}:${one.ts}`;
            claimed.set(key, (claimed.get(key) ?? 0) + 1);
        }
    }
    return matched.filter(each => each.of.length === 1
        && claimed.get(`${each.of[0].channel}:${each.of[0].ts}`) === 1).map(each => each.id);
}

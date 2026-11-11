// What in Slack is unread and what Joel read last, as Slack's daemon holds
// it (src/daemon.rs) and the shell watches it (SlackUnreads.qml), Now's
// Slack piece says what of it wants him (NewSlack.qml) and Slack's
// program lists it as Unread and Recent (SlackProgram.qml): how what's
// held stands, how many to read and from whom; each conversation's row
// and each message's are the daemon's, served ready (SlackRows.qml). A
// count is shown only while the daemon answers and Slack answered it,
// since the daemon holds conversations only then and the shell lets them
// go once it stops answering: an old count is never shown as though it
// were Slack's (Joel, 2026-10-28: "out of date information also sucks
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

// Each kind of record the daemon holds, by the part of what's gathered
// it makes, and the one key the account is held by.
const PARTS = { "slack-unread": "conversations", "slack-recent": "recent",
    "slack-message": "messages", "slack-account": "account" };
const ACCOUNT = "slack-account:slack";

// What's kept of the daemon's records: the account, and each other kind's
// by key, so a change remakes only the parts whose keys it names
// (Daemon.qml's heard) rather than walking all held, 400 on Joel's shell
// at a change a minute, with the launcher shut too (RCTRF8, 2026-11-10).
function keeping() {
    return { account: null, "slack-unread": {}, "slack-recent": {}, "slack-message": {} };
}

// Takes what `held` (Daemon.qml's) says of these keys into `kept`, and
// says which parts moved, as { account, conversations, recent, messages }
// with those that did. A key's kind is how it begins, as the daemon makes
// them (src/daemon.rs), so a key gone is placed without its record.
function keep(kept, held, keys) {
    const moved = {};
    for (const key of keys) {
        const kind = key.slice(0, key.indexOf(":"));
        const part = PARTS[kind];
        if (part === undefined || (part === "account" && key !== ACCOUNT)) {
            continue;
        }
        const record = held?.[key];
        const one = record?.kind === kind ? record[kind] : undefined;
        if (part === "account") {
            kept.account = one ?? null;
        } else if (one === undefined || one === null) {
            delete kept[kind][key];
        } else {
            kept[kind][key] = one;
        }
        moved[part] = true;
    }
    return moved;
}

// Whether a conversation can be listed: it says its id, its name and the
// link that opens it.
function openable(one) {
    return one !== null && typeof one === "object" && typeof one.id === "string"
        && typeof one.name === "string" && typeof one.link === "string";
}

// The conversations kept, in Slack's order. One is listed when it's whole:
// a channel merely unread, or one in Recent, has no count, and anything
// else has one.
function conversationsOf(kept) {
    const first = newest("latest");
    return Object.values(kept["slack-unread"]).filter(openable)
        .filter(one => one.kind === "unread" || one.count > 0)
        .sort((one, other) => ((groups[one.kind] ?? 4) - (groups[other.kind] ?? 4))
            || first(one, other));
}

// What Joel read last, newest first.
function recentOf(kept) {
    return Object.values(kept["slack-recent"]).filter(openable).sort(newest("read"));
}

// Each message wanting Joel, done ones too, the newest first
// (src/messages.rs).
function messagesOf(kept) {
    return Object.values(kept["slack-message"]).filter(listable)
        .sort((one, other) => (other.at - one.at) || other.ts.localeCompare(one.ts));
}

// Every part kept, listed.
function listed(kept) {
    return { conversations: conversationsOf(kept), recent: recentOf(kept),
        messages: messagesOf(kept) };
}

// What the daemon holds as { state, conversations, recent, messages,
// whole }, from its account and each part listed, or null while it
// doesn't answer or hasn't yet said how Slack stands. `whole` is whether
// the daemon says it holds every message, since a history Slack didn't
// give leaves some out.
function toldBy(account, lists, answering) {
    if (!answering || account === null || typeof account !== "object"
        || typeof account.standing !== "string") {
        return null;
    }
    return { state: account.standing, conversations: lists.conversations,
        recent: lists.recent, messages: lists.messages, whole: account.messages === "whole" };
}

// What the daemon holds, by key (Daemon.qml's `held`), gathered whole at
// once, as toldBy says it.
function gathered(held, answering) {
    if (!answering || held === null || typeof held !== "object") {
        return null;
    }
    const kept = keeping();
    keep(kept, held, Object.keys(held));
    return toldBy(kept.account, listed(kept), true);
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
    return threads(wanting(told));
}

function threads(conversations) {
    return conversations.filter(one => one.kind === "thread");
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

// Slack's own desktop notices say what they're of only in words: a direct
// message's summary is "[team] from {who}" and its body the text, and
// another's is "[team] in {place}" and its body "{who}: {text}", with no
// id, link or time of the message to know it by. The team is its name
// where the daemon keeps its id, so it isn't compared: the daemon holds
// one workspace's messages, and a notice of another's would need its
// sender, conversation, words and time alike to be taken for one.
const NOTICE = /^\[[^\]]*\] (from|in) (.+)$/;
// An emoji as Slack's markup names it, ":heart:", which the daemon keeps
// and the notice shows as the emoji itself, so neither is a word.
const SHORTCODE = /:[a-z0-9_+'-]+:/g;
// What a word is made of besides a letter with case: a digit, or a
// letter of a script without one, by its block (Hebrew, Arabic, the
// Indic scripts, Thai and Lao, Hangul, kana and CJK). Qt's engine has no
// \p{L}, so a letter beyond these isn't known as one.
const UNCASED = /[0-9\u0590-\u06ff\u0900-\u0d7f\u0e00-\u0eff\u1100-\u11ff\u3040-\u30ff]/;
const HAN_OR_HANGUL = /[\u3400-\u4dbf\u4e00-\u9fff\uac00-\ud7af\uf900-\ufaff]/;
// What parts words beyond ASCII: Latin-1's signs, general punctuation and
// the symbols, arrows and dingbats after it, CJK's and the fullwidth
// punctuation, emoji's variation selectors, and emoji themselves, each a
// pair of halves past U+1F000 or a flag's tag.
const PARTING = /[\u00a0-\u00bf\u00d7\u00f7\u2000-\u2bff\u3000-\u303f\ufe30-\ufe6f]/;
const VARIATIONS = [0xfe00, 0xfe0f];
const EMOJI = /^(\ud83c|\ud83d|\ud83e|\udb40)[\udc00-\udfff]$/;
const FULLWIDTH_SIGN = /[\uff01-\uff0f\uff1a-\uff20\uff3b-\uff40\uff5b-\uff65]/;
// A notice's text Slack cut short ends so, maybe within a word.
const ELLIPSIS = /\u2026\s*$/;
// The daemon cuts a message's text at a space to 400 characters (LONGEST
// in src/lib.rs), marking nothing, so one past 360 may have lost its end;
// one cut before a longer word than the rest is taken as whole, and so
// is never matched to a longer notice, which errs as nothing marked.
const CUT_PAST = 360;
// How long after Slack's time for a message its notice may come: Slack's
// raise one within a second or so (median 0.3 s over 108 kept, the
// slowest under two minutes), and its clock and the machine's may differ
// by a few seconds either way.
const NOTICED_WITHIN = 5 * 60 * 1000;
const CLOCKS_APART = 10 * 1000;
// The fewest words two texts must share where one is cut short, so a
// greeting or a lone "ok" stands only for its own words.
const FEWEST_SHARED = 3;

// What a Slack notice says of its message, { kind, place, who, words,
// cut, at }: a kind of "direct", "channel" or "group", its text's words,
// whether Slack cut them short, and when it came; or null for one that
// isn't Slack's or doesn't say it as Slack does.
function noticed(notice) {
    if (!isSlacks(notice)) {
        return null;
    }
    const said = NOTICE.exec(String(notice.summary ?? ""));
    if (said === null) {
        return null;
    }
    const body = String(notice.body ?? "");
    const by = body.indexOf(": ");
    const direct = said[1] === "from";
    if (!direct && by <= 0) {
        return null;
    }
    const text = direct ? body : body.slice(by + 2);
    const among = said[2].startsWith("#") ? "channel" : "group";
    return { kind: direct ? "direct" : among, place: said[2],
        who: direct ? said[2] : body.slice(0, by), words: words(text),
        cut: ELLIPSIS.test(text), at: notice.at };
}

// Whether a notice is Slack's own.
function isSlacks(notice) {
    const named = field => String(field ?? "").toLowerCase();
    return named(notice?.app) === "slack" || named(notice?.entry) === "slack";
}

// A text's words, lower case, its letters and digits alone: an emoji,
// named or shown, and punctuation are neither. A text holding a character
// that's none of these it knows has no words, so it's never matched on
// the words left beside it.
function words(text) {
    const all = [];
    let word = "";
    for (const each of String(text ?? "").toLowerCase().replace(SHORTCODE, " ")) {
        const code = each.charCodeAt(0);
        const parts = code < 0x80 || (code >= VARIATIONS[0] && code <= VARIATIONS[1])
            || PARTING.test(each) || EMOJI.test(each) || FULLWIDTH_SIGN.test(each);
        if (each.toUpperCase() !== each || UNCASED.test(each) || HAN_OR_HANGUL.test(each)) {
            word += each;
        } else if (!parts) {
            return [];
        } else if (word !== "") {
            all.push(word);
            word = "";
        }
    }
    return word === "" ? all : all.concat([word]);
}

// Whether `part` opens `whole`, word for word, with enough of them.
function opens(part, whole) {
    return part.length >= FEWEST_SHARED && part.length <= whole.length
        && part.every((word, at) => word === whole[at]);
}

// Whether a notice's words and a held message's say the same: every word
// alike, or, where a side could have been cut short, what's left of it
// opening the other's. Slack's cut may split its last word, so that one
// goes; the daemon's is at a space, so every one of its stays.
function sameWords(said, held) {
    const whole = said.words.length === held.words.length
        && said.words.every((word, at) => word === held.words[at]);
    if (whole) {
        return said.words.length > 0;
    }
    const kept = said.cut ? said.words.slice(0, -1) : said.words;
    return (said.cut && opens(kept, held.words)) || (held.cut && opens(held.words, kept));
}

// Whether a message the daemon holds is the one a notice says: its
// sender, its notice coming as Slack raises one, its conversation, a
// direct message by who it's with and a channel by its name (a group's
// notice names it by its title and the daemon by its members' handles,
// "ada, grace", so a group is known by being such a list), and its words.
function isNoticed(said, held) {
    const one = held.one;
    const after = said.at - one.at;
    if (one.who !== said.who || !(after >= -CLOCKS_APART && after <= NOTICED_WITHIN)) {
        return false;
    }
    const place = String(one.place ?? "");
    const placed = {
        direct: place === said.who,
        channel: place === said.place,
        group: !place.startsWith("#") && place.includes(", ")
    }[said.kind];
    return placed && sameWords(said, held);
}

// The ids of Slack's own notices still unread whose message the daemon
// says was read in Slack, so each is read in Notices too (Joel,
// 2026-11-08: "i shouldn't be required to mark a notice as read if I
// looked at the slack message in slack"). `all` are every message the
// daemon holds, read or not (SlackUnreads.qml's messages), or null while
// it doesn't hold every one. Nothing is read on a guess: a notice is read
// only where it could be one message alone, that one read in Slack after
// the notice came (the daemon looks only now and then, so a notice that
// came later is of another it hasn't heard yet), and no other notice,
// read or not, could be of it too.
function readInSlack(notices, all) {
    if (!(notices ?? []).some(notice => notice?.read !== true && isSlacks(notice))) {
        return [];
    }
    const held = (all ?? []).filter(listable).map(worded);
    if (!held.some(each => each.one.done?.how === "readInSlack")) {
        return [];
    }
    const matched = [];
    for (const notice of notices ?? []) {
        const said = noticed(notice);
        if (said !== null) {
            matched.push({ id: notice.id, at: said.at, unread: notice.read !== true,
                of: held.filter(each => isNoticed(said, each)).map(each => each.one) });
        }
    }
    const keyOf = one => `${one.channel}:${one.ts}`;
    const claimed = new Map();
    for (const each of matched) {
        for (const one of each.of) {
            claimed.set(keyOf(one), (claimed.get(keyOf(one)) ?? 0) + 1);
        }
    }
    return matched.filter(each => each.unread && each.of.length === 1
        && claimed.get(keyOf(each.of[0])) === 1 && readAfter(each.of[0], each.at))
        .map(each => each.id);
}

// A message held with its words, and whether the daemon may have cut it,
// kept by the message, which the daemon replaces whole as it changes, so
// a notice or a message heard words only the messages new since rather
// than every one held again.
const wordings = new WeakMap();

function worded(one) {
    let said = wordings.get(one);
    if (said === undefined) {
        said = { one, words: words(one.text),
            cut: [...String(one.text ?? "")].length > CUT_PAST };
        wordings.set(one, said);
    }
    return said;
}

// Whether a message was read in Slack at or after `at`, the daemon's time
// for it in milliseconds or as an ISO time.
function readAfter(one, at) {
    const finished = one.done;
    if (finished?.how !== "readInSlack") {
        return false;
    }
    const read = typeof finished.at === "number" ? finished.at
        : Date.parse(String(finished.at ?? ""));
    return at <= read;
}

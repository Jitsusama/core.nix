// What Tuple's log and links say, for Tuple.qml and Call.qml. Kept apart
// from Quickshell's types, so the `the-shell-follows-tuple-calls` check can
// run it in Qt's own engine over lines like the daemon's (tests/tuple.qml).
// A line reads "02:17:41.512|<process>|<thread>|<level>|<source>|<message>",
// its time of day in UTC.
//
// Only a call with nobody else in it has been seen to write its lines; what
// the daemon says of the others in a call, of a call ringing, and of its
// own mute comes from the messages its program holds, each with a blank
// where something about the call goes. What fills the blank isn't known,
// so a person is known by whatever fills it, and named only when it holds
// the number Tuple knows them by, as its contacts list them.
.pragma library

// What a call's end sounds like: Tuple ending it, the other side ending it,
// or the daemon starting or stopping, which ends any call it had.
const ending = [
    "invalidating call",
    "call ended",
    "daemon loop started, waiting for events...",
    "received 'off' message, quitting"
];

// What a ring's end sounds like, other than the call connecting or ending:
// Joel answering it in Tuple's window, or Tuple turning it away.
const taken = [/^accepting call$/, /^answering call with /, /^auto-answering /,
    /^call connected$/];
const answered = taken.concat([/^ignoring incoming call: /]);

// The two ways the daemon says someone came and went, each with the same
// blank in both. Which it writes isn't known, so each is kept apart, and
// the one that's heard the most people counts.
const comings = [
    { came: /^peer joined call: (.+)$/, went: /^peer left call: (.+)$/ },
    { came: /^EngineEvent: OnPeerAdded \(peer (.+?) cursor_mode=/,
        went: /^EngineEvent: OnPeerRemoved \(peer (.+?) reason=/ }
];

// A call nobody's in, as the shell starts: `others` holds, for each way
// of telling, who's in it besides Joel, `ringing` who's calling him and
// `rang` since when, `muted` whether Tuple sends his voice, when it's
// said, and `showing` who's sharing their screen into it.
const none = { calling: false, since: 0, others: [[], []], ringing: "", rang: 0,
    muted: null, showing: [] };

// A call link, as tuple call join takes one.
const CALL_LINK = /https:\/\/(?:production\.)?tuple\.app\/c\/[A-Za-z0-9_-]+/;
// A line's time of day, hours, minutes, seconds and milliseconds.
const TIME_OF_DAY = /^(\d\d):(\d\d):(\d\d)\.(\d\d\d)\|/;
// How far past now a line's time of day can be, in milliseconds, and still
// be today's: a minute, for a clock a little ahead of the shell's.
const AHEAD = 60_000;
// A day, in milliseconds.
const DAY = 86_400_000;
// Someone calling, with the blank that says who.
const RINGING = /^received incoming call: (.+)$/;
// Tuple saying whether it sends Joel's voice.
const VOICE = /^local user .+ sending audio: (true|false)$/;
// Someone starting or stopping sharing their screen, and the blank for who.
const SHOWS = /^peer (started|stopped) sharing screen: (.+)$/;
// What fills a blank when it's only a number, and when it holds one as an id.
const BARE_NUMBER = /^\s*(\d+)\s*$/;
const ID_NUMBER = /\bid[=: ]+(\d+)/;

// The first call link in some text, as tuple call join takes one, or "".
function link(text) {
    const found = CALL_LINK.exec(text ?? "");
    return found ? found[0] : "";
}

// When a line was written, in milliseconds since 1970, `now` being when it's
// read: a time of day later than now was yesterday's. A line with no time
// is taken as written now.
function written(line, now) {
    const time = TIME_OF_DAY.exec(line);
    if (!time) {
        return now;
    }
    const today = new Date(now);
    const at = Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate(),
        Number(time[1]), Number(time[2]), Number(time[3]), Number(time[4]));
    return at > now + AHEAD ? at - DAY : at;
}

// Whether a line says a ring was answered, rather than missed.
function answering(line) {
    const said = line.slice(line.lastIndexOf("|") + 1).trim();
    return taken.some(pattern => pattern.test(said));
}

function without(list, one) {
    return list.filter(each => each !== one);
}

// A copy of a call, for the next one to differ from.
function copied(call) {
    return Object.assign({}, call);
}

// The call after a line that says a ring began or ended without the call
// connecting, or null when the line says neither.
function rung(call, said, line, now) {
    const ringing = RINGING.exec(said);
    if (ringing) {
        const next = copied(call);
        next.ringing = ringing[1];
        next.rang = written(line, now);
        return next;
    }
    if (answered.some(pattern => pattern.test(said))) {
        const next = copied(call);
        next.ringing = "";
        return next;
    }
    return null;
}

// The call after a line that says Tuple's mute or someone's screen changed,
// or null when the line says neither.
function changed(call, said) {
    const voice = VOICE.exec(said);
    if (voice) {
        const next = copied(call);
        next.muted = voice[1] === "false";
        return next;
    }
    const shows = SHOWS.exec(said);
    if (shows) {
        const next = copied(call);
        next.showing = without(call.showing, shows[2])
            .concat(shows[1] === "started" ? [shows[2]] : []);
        return next;
    }
    return null;
}

// The call after a line that says someone came or went, by the way of
// telling it's in, or null when the line says neither.
function moved(call, said) {
    for (const [way, { came, went }] of comings.entries()) {
        const coming = came.exec(said) ?? went.exec(said);
        if (coming) {
            const next = copied(call);
            next.others = call.others.slice();
            next.others[way] = without(call.others[way], coming[1])
                .concat(came.test(said) ? [coming[1]] : []);
            return next;
        }
    }
    return null;
}

// The call after a line, from the call before it (none, to begin). A call
// connecting again keeps when it began, and who's in it.
function after(call, line, now) {
    const said = line.slice(line.lastIndexOf("|") + 1).trim();
    if (said === "call connected") {
        const next = copied(call);
        next.since = call.calling ? call.since : written(line, now);
        next.calling = true;
        next.ringing = "";
        return next;
    }
    if (ending.includes(said)) {
        return none;
    }
    return rung(call, said, line, now) ?? changed(call, said) ?? moved(call, said) ?? call;
}

// Who's in the call besides Joel, as the way of telling that's heard the
// most of them says.
function others(call) {
    return call.others.reduce((most, each) => each.length > most.length ? each : most, []);
}

// The number Tuple knows someone by, from what fills a line's blank, or "".
function number(who) {
    const found = BARE_NUMBER.exec(who ?? "") ?? ID_NUMBER.exec(who ?? "");
    return found ? found[1] : "";
}

// Someone's name, from the contacts `tuple --format json contacts list`
// gives, each as { id, full_name, short_name, email, status, ... }; "" if
// they aren't listed, or the list isn't one.
function named(listing, who) {
    const id = number(who);
    if (id === "") {
        return "";
    }
    let contacts = null;
    try {
        contacts = JSON.parse(listing ?? "");
    } catch (_error) {
        return "";
    }
    if (!Array.isArray(contacts)) {
        return "";
    }
    const found = contacts.find(contact => String(contact?.id) === id);
    return found ? found.full_name || found.short_name || "" : "";
}

// The people in a call as a sentence says them, from their names, "" for
// one not known: "with Aadi", "with Aadi and Ben", "with Aadi and one other",
// "with 3 others", or "on your own".
function company(names) {
    if (names.length === 0) {
        return "on your own";
    }
    const known = names.filter(name => name !== "");
    const rest = names.length - Math.min(known.length, 1);
    if (known.length === 0) {
        return names.length === 1 ? "with someone" : `with ${names.length} others`;
    }
    if (names.length === 2 && known.length === 2) {
        return `with ${known[0]} and ${known[1]}`;
    }
    if (rest === 0) {
        return `with ${known[0]}`;
    }
    return `with ${known[0]} and ${rest === 1 ? "one other" : `${rest} others`}`;
}

// How long a call has gone on, as a sentence says it.
function length(minutes) {
    if (minutes < 1) {
        return "under a minute";
    }
    if (minutes === 1) {
        return "a minute";
    }
    if (minutes < 60) {
        return `${minutes} minutes`;
    }
    const hours = `${Math.floor(minutes / 60)} h`;
    return minutes % 60 === 0 ? hours : `${hours} ${minutes % 60} min`;
}

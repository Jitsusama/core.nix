// What Tuple's log and links say, for Tuple.qml and Call.qml. Kept apart
// from Quickshell's types, so the `the-shell-follows-tuple-calls` check can
// run it in Qt's own engine over lines like the daemon's (tests/tuple.qml).
// A line reads "02:17:41.512|<process>|<thread>|<level>|<source>|<message>",
// its time of day in UTC.
.pragma library

// What a call's end sounds like: Tuple ending it, the other side ending it,
// or the daemon starting or stopping, which ends any call it had.
const ending = [
    "invalidating call",
    "call ended",
    "daemon loop started, waiting for events...",
    "received 'off' message, quitting"
];

// The first call link in some text, as tuple join takes one, or "".
function link(text) {
    const found = /https:\/\/(?:production\.)?tuple\.app\/c\/[A-Za-z0-9_-]+/.exec(text ?? "");
    return found ? found[0] : "";
}

// When a line was written, in milliseconds since 1970, `now` being when it's
// read: a time of day later than now was yesterday's. A line with no time
// is taken as written now.
function written(line, now) {
    const time = /^(\d\d):(\d\d):(\d\d)\.(\d\d\d)\|/.exec(line);
    if (!time)
        return now;
    const today = new Date(now);
    const at = Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate(),
        Number(time[1]), Number(time[2]), Number(time[3]), Number(time[4]));
    return at > now + 60 * 1000 ? at - 24 * 60 * 60 * 1000 : at;
}

// The call after a line, from the call before it: { calling, since }, since
// being when it connected. A call connecting again keeps when it began.
function after(call, line, now) {
    const said = line.slice(line.lastIndexOf("|") + 1).trim();
    if (said === "call connected")
        return { calling: true, since: call.calling ? call.since : written(line, now) };
    if (ending.includes(said))
        return { calling: false, since: 0 };
    return call;
}

// How long a call has gone on, as a sentence says it.
function length(minutes) {
    if (minutes < 1)
        return "under a minute";
    if (minutes === 1)
        return "a minute";
    if (minutes < 60)
        return `${minutes} minutes`;
    const hours = `${Math.floor(minutes / 60)} h`;
    return minutes % 60 === 0 ? hours : `${hours} ${minutes % 60} min`;
}

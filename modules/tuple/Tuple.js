// What Tuple's links say, and how a call is put in words, for Tuple.qml
// and Call.qml. Kept apart from Quickshell's types, so the
// `the-shell-follows-tuple-calls` check can run it in Qt's own engine
// (tests/tuple.qml). How a call stands is tuple-call's to say (call/).
.pragma library

// A call link, as tuple call join takes one.
const CALL_LINK = /https:\/\/(?:production\.)?tuple\.app\/c\/[A-Za-z0-9_-]+/;

// The first call link in some text, as tuple call join takes one, or "".
function link(text) {
    const found = CALL_LINK.exec(text ?? "");
    return found ? found[0] : "";
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

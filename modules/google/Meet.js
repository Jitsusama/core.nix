// What the shell works out of Meet's calls: which call a window is, the
// code Joel types to join one, and whose meeting a call is, from the
// meetings the google program keeps (src/state.rs), and how a call Joel
// is in is said in Now (Call.qml): its side, who's in it and its plate,
// from what Meet says of it (src/meet.rs). Calendar.qml and Call.qml ask
// these, for themselves and for Meet.qml, and the checks run them in Qt's
// own engine (tests/calendar.qml).
.pragma library
.import "Calendar.js" as Calendar

// A Meet call's code, as abc-defg-hij, from the app id Chrome gives the
// window it opens a call's link in as an app of its own: its host and
// path, the path's slash an underscore, then its profile, as
// chrome-meet.google.com__abc-defg-hij-Default. Any other window, Meet's
// own front page and a call in a browser tab among them, is no call: "".
const MEET_WINDOW = /^chrome-meet\.google\.com__([a-z]{3}-[a-z]{4}-[a-z]{3})-/;
// The code a Meet link joins by, before any ?authuser= that follows it.
const MEET_LINK = /^https:\/\/meet\.google\.com\/([a-z-]+)/;

function meetCall(appId) {
    const found = MEET_WINDOW.exec(appId ?? "");
    return found ? found[1] : "";
}

// A Meet code as Joel types or pastes it, three letters, four and three,
// with or without its dashes and spaces, or its whole link, written as
// Meet writes it: "abc-defg-hij"; or "" for anything else, which Meet
// wouldn't know.
const MEET_CODE = /^([a-z]{3})-?([a-z]{4})-?([a-z]{3})$/;
const SPACES = /\s+/g;

function meetCode(given) {
    const plain = String(given ?? "").trim().toLowerCase().replace(SPACES, "");
    const parts = MEET_CODE.exec(MEET_LINK.exec(plain)?.[1] ?? plain);
    return parts ? `${parts[1]}-${parts[2]}-${parts[3]}` : "";
}

// The meeting whose call has that code, or null for one nobody asked
// Joel to, such as a link someone pasted.
function calledBy(meetings, code) {
    return meetings.find(meeting => code !== ""
        && MEET_LINK.exec(meeting.link ?? "")?.[1] === code)
        ?? null;
}

const MINUTE = 60_000;
const HOUR = 60;

// Words with what's in braces filled in: "{count} in the call".
function filled(words, fill) {
    let said = String(words ?? "");
    for (const key of Object.keys(fill)) {
        said = said.replace(`{${key}}`, String(fill[key]));
    }
    return said;
}

// What Meet says of the call of `code`, from the google program's calls,
// or null before it has said.
function heard(calls, code) {
    return (calls ?? []).find(call => call.code === code) ?? null;
}

// When the call began: its conference's start, as Meet says it, else
// when its window opened, which is when Joel joined.
function began(call, opened) {
    return call?.started ?? opened;
}

// How long it has gone on, as a side says it: "under a minute", "23 min",
// "1 h 5 min".
function length(since, now, said) {
    const minutes = Math.max(0, Math.floor((now - since) / MINUTE));
    if (minutes < 1) {
        return said.under;
    }
    if (minutes < HOUR) {
        return filled(said.minutes, { minutes });
    }
    const hours = Math.floor(minutes / HOUR);
    return minutes % HOUR === 0 ? filled(said.hours, { hours })
        : filled(said.hoursMinutes, { hours, minutes: minutes % HOUR });
}

// Its side, how it stands first (VKCF3Z, 2026-11-08): whether Joel can be
// heard, then how long it has gone on, "muted \u00b7 23 min".
function side(muted, since, now, said) {
    return `${muted ? said.muted : said.live} \u00b7 ${length(since, now, said)}`;
}

// Whether someone has left the call, as Meet says once they have.
function gone(person) {
    return person.left !== null && person.left !== undefined;
}

// Everyone in the call now, by when they joined, then those who left.
function people(call) {
    const all = call?.people ?? [];
    return all.filter(person => !gone(person)).concat(all.filter(gone));
}

// The names of those in it now, as its row says them after its name,
// giving way by whole names.
function inIt(call) {
    return people(call).filter(person => !gone(person)).map(person => person.name);
}

// What People says beside it: how many are in the call, or that reading
// who is waits on Joel signing in to Google again.
function counted(call, said) {
    if (call && call.granted === false) {
        return said.signIn;
    }
    return filled(said.inCall, { count: inIt(call).length });
}

// A person's side inside People: since when they're in, or when they left.
function personSide(person, said) {
    return gone(person)
        ? filled(said.left, { time: Calendar.clock(new Date(person.left)) })
        : filled(said.since, { time: Calendar.clock(new Date(person.joined)) });
}

// The call's plate (VKCF3Z, 2026-11-08): over its name, where it is and
// what it is, "on Meet \u00b7 call"; then who's in it, when it began, when
// its meeting runs and, last, its code, an identifier set as written. How
// it stands and how long it has gone on are its side's, and never said
// again here; a call on no meeting has no times to give. `of` is the
// call as its piece holds it: { named, code, heard, since, meeting }.
function details(of, said) {
    const names = inIt(of.heard);
    const facts = [];
    if (names.length > 0) {
        facts.push({ label: said.people, value: names.join(", ") });
    }
    facts.push({ label: said.started, value: Calendar.clock(new Date(of.since)) });
    if (of.meeting) {
        facts.push({ label: said.meeting, value: Calendar.span(of.meeting) });
    }
    facts.push({ label: said.code, value: of.code, identifies: true });
    return { eyebrow: said.eyebrow, eyebrowNames: true, title: of.named, facts, body: "" };
}

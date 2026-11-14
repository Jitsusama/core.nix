// What Super+S's talk line says and sends, apart from how it's drawn
// (TalkLine.qml): what goes with each line of the window it's said over,
// for the void to pass it to whoever owns it (QCXZEB, 2026-11-13
// [ae5dfdb6]), the quiet line under the field naming who hears it and what
// goes with it, and how the talk it began stands, sent, then answered, then
// what became of it, all inside the line (VKCF3Z, 2026-11-13 [092d52ca]).
// tests/talk-line.qml holds it to its cases in Qt's own engine.
.pragma library

.import "Told.js" as Told

// The heading the void's Context reads what's attached under (QCXZEB's
// slice C): a word of the protocol between the line and the void, so it's
// never reworded with the line's own words.
const ATTACHED = "Attached:";

// How each kind of row is named in the Attached block, which the void
// reads, so like ATTACHED these are the protocol's words, never the line's
// (QCXZEB keeps how the void reads them); the theme's mark each wears on
// the line, and its hue: a program's for what's a program's, and none, the
// row's muted, for his own words.
const KINDS = {
    window: { named: "window", mark: "windows", hue: "program" },
    tab: { named: "tab", mark: "web", hue: "program" },
    session: { named: "pi session", mark: "agent", hue: "program" },
    selection: { named: "selection", mark: "lines", hue: "" },
    clipboard: { named: "clipboard", mark: "clipboard", hue: "" }
};

// The most of a selection or a copy a line sends, in characters, so a
// copied log doesn't bury his words: the clipboard keeps copies far larger.
const LONGEST = 4000;

// The name pi goes by, which a pi session's row notes.
const PI = "pi";

// pi's letter and its joint before a session's title, "π - name", which
// the row's mark already says, set small and odd in the words' face
// (VKCF3Z, 2026-11-13 [00c78048]). A stopgap: the windows daemon's `named`
// drops it itself from about Sat 14 01:30 (ZINAWZ [5dc6c596]), and then
// this finds nothing to drop.
const PI_LEAD = /^\u03c0(?: - | \u2013 | \u2014 | \| )/u;

// What a window says of itself, as Niri.qml holds it from the windows
// daemon: its title without the program's name (`named`, core #4242).
function ownTitle(window) {
    return String(window?.named ?? window?.title ?? "").trim().replace(PI_LEAD, "");
}

// What the programs daemon is asked for the tab in front of a browser's
// window, by niri's id for it: it matches the window's title to that tab and
// answers nothing where two could be it (FEH1HE, core #4190).
function tabKey(app, window) {
    return `front:${app}:${window}`;
}

// The tab its answer names, or null where it names none or one without an
// address: the window it's in front of, its title and its address.
function tabFrom(answer, app, window) {
    const key = tabKey(app, window);
    const front = (answer?.records ?? []).find(record => record?.key === key);
    const address = String(front?.address ?? "");
    if (address === "") {
        return null;
    }
    return { window, title: String(front.title ?? "").trim() || address, address };
}

// One row: its kind, the mark it wears and its hue, what it says on the
// line and its note, which gives way first where the row is short of room,
// and the words it's sent as under the Attached heading.
function row(kind, text, note, sent) {
    return { kind, mark: KINDS[kind].mark, hue: KINDS[kind].hue, text, note, sent };
}

// What an address begins and ends with that its note leaves out.
const SCHEME = /^https?:\/\/(?:www\.)?/u;
const TRAILING_SLASH = /\/$/u;

// An address as a row's note says it, without the scheme every page has,
// so the note gives way later; it's sent whole.
function shortAddress(address) {
    return String(address ?? "").replace(SCHEME, "").replace(TRAILING_SLASH, "");
}

// What goes with a line, a row each: the window it's said over, by its own
// title and noted by its program, or by its program alone where it has no
// title of its own; the tab in front, where it's a browser's (tabFrom); the
// pi session that window is, where it's one, by its name and noted as pi's,
// sent with its address; and what he attached of the registers
// (registered). A window with no title, or none focused, attaches nothing
// of itself.
function attached({ window, session, tab, registers }) {
    const rows = [];
    const title = String(window?.title ?? "").trim();
    // The program as the daemon names it, or, while niri's stream stands
    // in and says none, the app id.
    const program = String(window?.program || window?.app_id || "");
    if (title !== "" && window?.untitled === true) {
        rows.push(row("window", program, "", `${program} \u00b7 ${title}`));
    } else if (title !== "") {
        rows.push(row("window", ownTitle(window), program, `${program} \u00b7 ${title}`));
    }
    if (tab) {
        rows.push(row("tab", tab.title, shortAddress(tab.address),
            `${tab.title} \u00b7 ${tab.address}`));
    }
    if (session) {
        rows.push(row("session", session.name, PI, `${session.name} \u00b7 ${session.to}`));
    }
    return rows.concat(registers ?? []);
}

// The registers Ctrl+R attaches, as Vim names them: * the selection, + the
// clipboard.
const REGISTERS = { "*": "selection", "+": "clipboard" };

// The blank lines before what a register holds, and the space after it.
const LEADING_BLANKS = /^(?:[ \t]*\n)+/u;
const TRAILING_SPACE = /\s+$/u;

// The registers attached once Ctrl+R and `name` put `text`, in the order
// they were first attached: one row a register, the same again replacing
// it where it stood; a register with nothing in it attaches nothing, and
// lets go of what it held. Any other register is no attachment of a line's.
// On the line, its first line, and as its note, which register, and how
// many lines in all where it's many, in `words`' "{count} lines"; sent
// whole, to LONGEST.
function registered(registers, name, text, words) {
    const kind = REGISTERS[name];
    if (kind === undefined) {
        return registers ?? [];
    }
    // Blank lines round it are dropped, and a line alone is trimmed, but
    // the indents of many are kept, as code's mean something.
    const trimmed = String(text ?? "").replace(LEADING_BLANKS, "").replace(TRAILING_SPACE, "");
    const whole = trimmed.includes("\n") ? trimmed : trimmed.trim();
    const lines = whole.split("\n");
    const first = lines[0].trim().replace(/\s+/gu, " ");
    const count = lines.length;
    const named = String(words?.[kind] ?? "");
    const note = count > 1
        ? `${named} \u00b7 ${String(words?.lines ?? "").replace("{count}", grouped(count))}`
        : named;
    const cut = whole.length > LONGEST ? `${whole.slice(0, LONGEST)}\n[cut at ${LONGEST} characters]`
        : whole;
    const made = first === "" ? null : row(kind, first, note, cut);
    const at = (registers ?? []).findIndex(one => one.kind === kind);
    if (at < 0) {
        return made === null ? registers ?? [] : (registers ?? []).concat([made]);
    }
    const now = registers.slice();
    now.splice(at, 1, ...(made === null ? [] : [made]));
    return now;
}

// A count with its thousands set off by commas, as every count of ours is.
function grouped(count) {
    return String(count).replace(/\B(?=(?:\d{3})+(?!\d))/gu, ",");
}

// The rows the line shows of what's attached: every one, but the window
// where a row after it already names what it holds, as a browser's tab or a
// pi session does, since its title says the same.
function shownRows(rows) {
    const named = (rows ?? []).some(one => one.kind === "tab" || one.kind === "session");
    return (rows ?? []).filter(one => !(named && one.kind === "window"));
}

// The words a line sends: his own, then what's attached under its
// heading, a row a line, so the void reads which window it's about; a
// selection or a copy of many lines goes under its row, indented.
function said(text, rows) {
    const words = String(text ?? "").trim();
    if ((rows ?? []).length === 0) {
        return words;
    }
    const lines = rows.map(one => {
        const sent = String(one.sent ?? one.text);
        const named = `- ${KINDS[one.kind]?.named ?? one.kind}:`;
        return sent.includes("\n") ? `${named}\n  ${sent.split("\n").join("\n  ")}`
            : `${named} ${sent}`;
    });
    return [words, "", ATTACHED].concat(lines).join("\n");
}


// A talk begun by Return: to whom, his words as shown, the words sent,
// and since when, waiting on its answer.
function begun(to, line, sent, since) {
    return { to, line, sent, since, said: "sent", heard: "", reply: "", undo: "", kept: "",
        taking: "", takenSince: 0, told: "" };
}

// How many lines he has seen through a talk line keeps under the rest, newest first, so two
// questions in a row both show what became of them (WCJQSG [cf8639ab], Joel at 07:46 on
// 2026-11-13: "it only shows the first response").
const KEPT_TALKS = 3;

// A talk the void keeps, as Agents serves it (Orchestrators.qml's talks), as the line's own:
// his words without what went with them, when he said them by the talk's id, and what the
// void said back. The void keeps every talk on disk, so a line said before a restart, or
// on an earlier day, is the same line after (WCJQSG [a2eb9575]).
const TALK_ID = /^talk-(\d+)$/u;
function fromServed(to, served) {
    const id = String(served?.id ?? "").match(TALK_ID);
    const since = id === null ? Date.parse(served?.at ?? "") : Number(id[1]);
    const sent = String(served?.words ?? "");
    const heading = sent.indexOf(`\n\n${ATTACHED}`);
    const talk = begun(to, heading < 0 ? sent : sent.slice(0, heading), sent, since);
    return followed(talk, [served]) ?? talk;
}

// A talk the void keeps that's no line of his: one not delivered yet, which holds no words
// until it is; one it ended without a word back, failed or ended with the session; or
// :take's own asking to take a line back (Told.js's undoing).
const UNDOING = /^undo \d{1,2}:\d{2}$/u;
function unanswered(served) {
    const words = String(served?.words ?? "").trim();
    return words === "" || UNDOING.test(words)
        || (String(served?.reply ?? "") === "" && ["failed", "over"].includes(served?.stage));
}

// Whether a line said here is one the void serves: the same words, which the void kept a
// moment after, never before, so the same words said again are a line of their own.
const KEPT_AFTER_MS = 30000;
const SAID_BEFORE_MS = 2000;
function same(here, served) {
    const after = served.since - here.since;
    return here.sent === served.sent && after > -SAID_BEFORE_MS && after < KEPT_AFTER_MS;
}

// What only this shell knows of a line: that it's being taken back, or was.
const TAKING = ["taking", "taken", "told"];

// The events that carry an answer's words: answered, landed once it's done, and taken where
// whoever took it up said something (QCXZEB [203d9e2c]); one with no words is dropped.
const ANSWERING = ["answered", "landed", "taken"];
// The events after which someone has the line and it waits on them.
const HOLDING = ["routed", "taken"];

// Whether a line is still open: it has an answer he hasn't seen, nobody has answered it yet,
// or he's taking it back and the void hasn't said; `seen` holds how many of each line's
// answers he has, by its key. One whose every answer he has seen is done, whoever has it
// (Joel, 2026-11-13 23:39: "tons of things still waiting even though they were done hours ago").
function open(talk, events, seen) {
    if (TAKING.includes(talk.said)) {
        return talk.said === "taking";
    }
    const key = keyOf(talk);
    const heard = past(events, key);
    if (heard.last === "") {
        return talk.said === "sent";
    }
    return heard.answers.length > (seen?.[key] ?? 0)
        || (heard.answers.length === 0 && HOLDING.includes(heard.last));
}

// The lines a talk line shows, each once: those the void keeps, and those said in this
// shell it hasn't yet, or that are being taken back here. Lines still waiting, and lines
// with answers he hasn't seen, come first and all of them, newest first; then the newest
// KEPT_TALKS of the rest (WCJQSG [a2eb9575]).
function gathered(here, served, events, seen) {
    const lines = (served ?? []).filter(one => Number.isFinite(one.since));
    for (const one of here ?? []) {
        const kept = lines.findIndex(line => same(one, line));
        if (kept < 0) {
            lines.push(one);
        } else if (TAKING.includes(one.said)) {
            lines[kept] = one;
        }
    }
    lines.sort((one, other) => other.since - one.since);
    const lined = keyed(lines, events);
    const opened = lined.filter(line => open(line, events, seen));
    return opened.concat(lined.filter(line => !opened.includes(line)).slice(0, KEPT_TALKS));
}

// Each line's key from the void's own routings: a talk carries only its latest reply, and
// the steer inbox can file a later line's reply under an earlier one (8GYNJ6 [0f78d301]),
// whose undo then names the later line's key. So, newest first, a line holds its reply's
// key where the void routed it and no newer line holds it, else the earliest routing free
// between the minute it was said and that key.
function keyed(lines, events) {
    const routed = (events ?? []).filter(event => event.event === "routed")
        .map(event => ({ key: event.line, at: Date.parse(String(event.line).replace(" ", "T")) }))
        .sort((one, other) => one.at - other.at);
    const held = [];
    return lines.map(line => {
        const own = keyOf(line);
        const minute = line.since - (line.since % 60000);
        const free = one => !held.includes(one.key) && one.at >= minute;
        const owned = Date.parse(own.replace(" ", "T"));
        const key = routed.find(one => one.key === own && free(one))
            ?? routed.find(one => free(one) && one.at <= owned);
        if (own === "" || key === undefined) {
            return line;
        }
        held.push(key.key);
        return Object.assign({}, line, { key: key.key });
    });
}

// What he has seen once the line is put away: every answer of each line it showed, added to
// what he saw before, a month's lines kept.
const SEEN_DAYS = 30;
const DAY_MS = 86400000;
function seenAfter(seen, shown, events, now) {
    const after = {};
    for (const key of Object.keys(seen ?? {})) {
        if (now - Date.parse(key.replace(" ", "T")) < SEEN_DAYS * DAY_MS) {
            after[key] = seen[key];
        }
    }
    for (const talk of shown ?? []) {
        const key = keyOf(talk);
        if (key !== "") {
            after[key] = Math.max(past(events, key).answers.length, after[key] ?? 0);
        }
    }
    return after;
}

// What a file of what he has seen holds, or nothing where it's missing or not one.
function seenIn(text) {
    const read = eventOf(text);
    return read !== null && typeof read === "object" && !Array.isArray(read) ? read : {};
}

// The line's key in the void's day file, "YYYY-MM-DD HH:MM": the day it was said, on the local
// clock, and the time the void keeps it at, its routing (keyed) or else its first answer's
// undo; "" before the void has said.
function keyOf(talk) {
    if (typeof talk?.key === "string") {
        return talk.key;
    }
    if (talk === null || talk.kept === "") {
        return "";
    }
    const day = new Date(talk.since);
    const two = number => String(number).padStart(2, "0");
    const time = talk.kept.split(":");
    return `${day.getFullYear()}-${two(day.getMonth() + 1)}-${two(day.getDate())} `
        + `${two(time[0])}:${time[1]}`;
}

// What the void wrote as it happened, in the order written, as the node serves its file:
// each line an object, or its text where it wasn't read; a line that isn't JSON is left
// out, as a line half written is.
function eventsOf(lines) {
    return (Array.isArray(lines) ? lines : [])
        .map(line => (typeof line === "string" ? eventOf(line) : line))
        .filter(event => event !== null && typeof event === "object");
}

// One line's event, or null where it's half written.
function eventOf(line) {
    try {
        return JSON.parse(line);
    } catch (_halfWritten) {
        return null;
    }
}

// Every answer in the order written, one that supersedes another in its place.
function answersIn(own) {
    const answers = [];
    for (const event of own.filter(one => ANSWERING.includes(one.event))) {
        const answer = { msg: event.msg ?? "", text: String(event.text ?? "").trim(),
            who: midSentence(event.who), at: event.at ?? "" };
        const replaced = answers.findIndex(one => event.supersedes && one.msg === event.supersedes);
        answers.splice(replaced < 0 ? answers.length : replaced, replaced < 0 ? 0 : 1, answer);
    }
    return answers.filter(answer => answer.text !== "");
}

// What became of a line past the void, from its events in the order written: every answer;
// who has it while the last is its routing or its taking up, else ""; and the last event.
function past(events, key) {
    const own = key === "" ? [] : (events ?? []).filter(event => event.line === key);
    const last = own.length > 0 ? own[own.length - 1] : { event: "", who: "" };
    return { answers: answersIn(own), last: last.event,
        waiting: HOLDING.includes(last.event) ? midSentence(last.who) : "",
        lastWho: midSentence(last.who) };
}

// "Your orchestrator has it", as the void's first answer says who has a line.
const HOLDER = /^(?<who>.+) has it$/u;
const YOUR = /^Your /u;

// Who the void's events name, mid-sentence: it writes "Your orchestrator", lower-cased here.
function midSentence(who) {
    return String(who ?? "").replace(YOUR, "your ");
}

// Who the void's first answer says has a line, mid-sentence; "" where it names nobody.
function holderIn(reply) {
    const named = String(reply ?? "").split("\n")[0].trim().match(HOLDER);
    // By its number, since Qt's engine fills no `groups`.
    return named === null ? "" : named[1].replace(YOUR, "your ");
}

function worded(words, name, who) {
    return String(words?.[name] ?? "").replace("{who}", who);
}

function clockOf(at) {
    const day = new Date(at);
    return `${String(day.getHours()).padStart(2, "0")}:${String(day.getMinutes()).padStart(2, "0")}`;
}

// Whom a talk waits on before the void has written anything of it: the void while it's only
// sent, then whoever its answer says has it; or, where it says no one has, that answer alone.
function before(talk, words) {
    if (talk.said === "sent") {
        return { answers: [], waiting: worded(words, "waiting", String(words?.void ?? "")) };
    }
    const holder = talk.kept === "" ? "" : holderIn(talk.reply);
    return holder === "" ? { answers: [talk.reply], waiting: "" }
        : { answers: [], waiting: worded(words, "waiting", holder) };
}

// How it stands where the void's last event ends it: undone, or let go.
function ended(heard, words) {
    if (heard.last === "undone") {
        return String(words?.taken ?? "");
    }
    return heard.last === "let-go" ? worded(words, "letGo", heard.lastWho) : "";
}

// How a kept talk reads under his words: `answers`, each in the words, or else `waiting`, in
// the voice while someone has it, saying on whom, or how it otherwise `stands`; at its right,
// `side`, when he said it, whatever came after; and at its answer's right, `by`, who answered
// (WCJQSG [e05f6231], QCXZEB [5aab8439]).
function exchange(talk, events, words, dayOf) {
    const day = typeof dayOf === "function" ? dayOf(talk.since) : "";
    const shown = { line: talk.line, answers: [], waiting: "", stands: "", by: "",
        side: day === "" ? clockOf(talk.since) : `${day} ${clockOf(talk.since)}` };
    if (TAKING.includes(talk.said)) {
        return Object.assign(shown, { stands: standing(talk, words) });
    }
    const heard = past(events, keyOf(talk));
    if (heard.last === "") {
        return Object.assign(shown, before(talk, words));
    }
    const newest = heard.answers.length > 0 ? heard.answers[heard.answers.length - 1] : null;
    return Object.assign(shown, { answers: heard.answers.map(answer => answer.text),
        waiting: heard.waiting === "" ? "" : worded(words, "waiting", heard.waiting),
        stands: ended(heard, words),
        by: newest === null ? "" : String(newest.who) });
}

// How a talk stands once its session's talks have moved, or null where nothing it shows
// changed: answered once the reply is whole, its undo made an act; and once Take It Back is
// asked, the void's answer to that.
function followed(talk, talks) {
    if (talk === null) {
        return null;
    }
    if (talk.taking !== "") {
        const answer = Told.replyOf(Told.talkOf(talks, talk.taking, talk.takenSince));
        return answer === "" ? null : Object.assign({}, talk, Told.takenBy(answer), { taking: "" });
    }
    if (talk.said === "taken" || talk.said === "told") {
        return null;
    }
    const reply = Told.replyOf(Told.talkOf(talks, talk.sent, talk.since));
    if (reply === "" || reply === talk.heard) {
        return null;
    }
    const went = Told.went(reply);
    return Object.assign({}, talk, { said: "answered", heard: reply, reply: went.reply,
        undo: went.undo, kept: talk.kept || went.undo });
}

// Take It Back asked of a talk the void can undo: what the void is told, and the talk
// waiting on its answer, never offering it twice.
function taking(talk, now) {
    if (talk === null || talk.undo === "") {
        return null;
    }
    const asked = Told.undoing(talk.undo);
    return { asked, talk: Object.assign({}, talk, { said: "taking", taking: asked,
        takenSince: now, undo: "" }) };
}

// What the line says under his words as the talk stands: the reply once answered, else how
// it stands in the line's own words.
function standing(talk, words) {
    if (talk === null) {
        return "";
    }
    if (talk.said === "answered") {
        return talk.reply;
    }
    if (talk.said === "told") {
        return talk.told;
    }
    return String(words?.[talk.said] ?? "");
}

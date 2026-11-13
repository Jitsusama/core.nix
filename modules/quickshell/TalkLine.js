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

// What goes with a line, a row each: the window it's said over, as its
// program names itself, and its title; and the pi session that window is,
// where it's one, by its name and address. A window with no title, or none
// focused, attaches nothing of itself.
function attached(window, program, session) {
    const rows = [];
    const title = String(window?.title ?? "").trim();
    if (title !== "") {
        rows.push({ kind: "window", text: `${program} \u00b7 ${title}` });
    }
    if (session) {
        rows.push({ kind: "session", text: `${session.name} \u00b7 ${session.to}` });
    }
    return rows;
}

// The words a line sends: his own, then what's attached under its
// heading, a row a line, so the void reads which window it's about.
function said(text, rows) {
    const words = String(text ?? "").trim();
    if ((rows ?? []).length === 0) {
        return words;
    }
    const lines = rows.map(row => `- ${row.kind === "session" ? "pi session" : "window"}: ${row.text}`);
    return [words, "", ATTACHED].concat(lines).join("\n");
}

// The quiet line under the field: who hears it, short, then what goes
// with it, the window first, joined with dots. Where the window is a pi
// session's, its title already names the session, so only the window is
// said; the address goes with the words all the same.
function about(hearer, rows) {
    const window = (rows ?? []).find(row => row.kind === "window");
    return [hearer].concat(window ? [window.text] : []).join(" \u00b7 ");
}

// A talk begun by Return: to whom, his words as shown, the words sent,
// and since when, waiting on its answer.
function begun(to, line, sent, since) {
    return { to, line, sent, since, said: "sent", heard: "", reply: "", undo: "", kept: "",
        taking: "", takenSince: 0, told: "" };
}

// The lines a talk line keeps under it, newest first: the talk just begun
// and the two before it, so two questions in a row both show what became
// of them (WCJQSG [cf8639ab], Joel at 07:46 on 2026-11-13: "it only shows
// the first response").
const KEPT_TALKS = 3;

function kept(talks, talk) {
    return [talk].concat(talks ?? []).slice(0, KEPT_TALKS);
}

// The line's key in the void's day file, "YYYY-MM-DD HH:MM": the day it
// was said, on the local clock, and the time the void keeps it at, which
// its first answer's undo says; "" before the void has said.
function keyOf(talk) {
    if (talk === null || talk.kept === "") {
        return "";
    }
    const day = new Date(talk.since);
    const two = number => String(number).padStart(2, "0");
    const time = talk.kept.split(":");
    return `${day.getFullYear()}-${two(day.getMonth() + 1)}-${two(day.getDate())} `
        + `${two(time[0])}:${time[1]}`;
}

// What the void wrote as it happened, a JSON object a line, in the order
// written; a line that isn't one is left out, as a line half written is.
function eventsIn(text) {
    return String(text ?? "").split("\n").filter(line => line.trim() !== "").map(eventOf)
        .filter(event => event !== null);
}

// One line's event, or null where it's half written.
function eventOf(line) {
    try {
        return JSON.parse(line);
    } catch (_halfWritten) {
        return null;
    }
}

// The events that carry an answer's words: answered, and landed once it's done.
const ANSWERING = ["answered", "landed"];
// The events after which someone has the line and it waits on them.
const HOLDING = ["routed", "taken"];

// Every answer in the order written, one that supersedes another in its place.
function answersIn(own) {
    const answers = [];
    for (const event of own.filter(one => ANSWERING.includes(one.event))) {
        const answer = { msg: event.msg ?? "", text: String(event.text ?? "").trim(),
            who: event.who ?? "", at: event.at ?? "" };
        const replaced = answers.findIndex(one => event.supersedes && one.msg === event.supersedes);
        answers.splice(replaced < 0 ? answers.length : replaced, replaced < 0 ? 0 : 1, answer);
    }
    return answers.filter(answer => answer.text !== "");
}

// What became of a line past the void, from its events in the order
// written: every answer; who has it while the last is its routing or its
// taking up, else ""; and the last event, with who it names.
function past(events, key) {
    const own = key === "" ? [] : (events ?? []).filter(event => event.line === key);
    const last = own.length > 0 ? own[own.length - 1] : { event: "", who: "" };
    return { answers: answersIn(own), last: last.event,
        waiting: HOLDING.includes(last.event) ? String(last.who ?? "") : "",
        lastWho: String(last.who ?? "") };
}

// "Your orchestrator has it", as the void's first answer says who has a line.
const HOLDER = /^(?<who>.+) has it$/u;
const YOUR = /^Your /u;

// Who the void says has a line, from its first answer's words, as they'd
// stand mid-sentence; "" where it says something else.
function holderIn(reply) {
    const named = String(reply ?? "").split("\n")[0].trim().match(HOLDER);
    return named === null ? "" : named.groups.who.replace(YOUR, "your ");
}

function worded(words, name, who) {
    return String(words?.[name] ?? "").replace("{who}", who);
}

function clockOf(at) {
    const day = new Date(at);
    return `${String(day.getHours()).padStart(2, "0")}:${String(day.getMinutes()).padStart(2, "0")}`;
}

// Whom a talk waits on before the void has written anything of it: the void
// while it's only sent, then whoever the void's own answer says has it; or,
// where its answer says no one has it, that answer is all there is.
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

// How a kept talk reads under his words: `answers`, each in the words, or
// else `waiting`, in the voice while someone has it, saying on whom, or how
// it otherwise `stands`; and at its right, `side`, when he said it, or once
// answered who answered and when.
function exchange(talk, events, words) {
    const shown = { line: talk.line, answers: [], waiting: "", stands: "",
        side: clockOf(talk.since) };
    if (["taking", "taken", "told"].includes(talk.said)) {
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
        side: newest === null || newest.who === "" ? shown.side
            : `${newest.who} \u00b7 ${newest.at}` });
}

// How a talk stands once its session's talks have moved, or null where
// nothing it shows changed: answered once the reply is whole, where it
// went kept and its undo made an act; and once Take It Back is asked, the
// void's answer to that.
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

// Take It Back asked of a talk the void can undo: what the void is told,
// and the talk waiting on its answer, never offering it twice.
function taking(talk, now) {
    if (talk === null || talk.undo === "") {
        return null;
    }
    const asked = Told.undoing(talk.undo);
    return { asked, talk: Object.assign({}, talk, { said: "taking", taking: asked,
        takenSince: now, undo: "" }) };
}

// What the line says under his words as the talk stands: the reply once
// answered, else how it stands in the line's own words.
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

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
    return { to, line, sent, since, said: "sent", heard: "", reply: "", undo: "", taking: "",
        takenSince: 0, told: "" };
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
        undo: went.undo });
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

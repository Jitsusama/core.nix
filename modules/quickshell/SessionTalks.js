// Talking to a session from the launcher, Talk to It and its kin, which shows the talk under
// the field as Super+S's line shows the void's (Joel at 19:30 on 2026-11-13: "they need to
// feel more like the void"), in TalkLine.js's exchange shape and on its clock and words. A
// session answers on its talk's own lines, as Orchestrators.qml folds them,
// `{ id, at, words, stage, reply, why }`, and the daemon writes `read` once he has heard it,
// so what's open outlives the shell. tests/talk-line.qml holds it to its cases in Qt's own
// engine.
//
// A stopgap: merging what's said here with what's served, and choosing which talks show and in
// what order, is the agents daemon's work, as the daemons are the data layer. It stays here
// until KP0L6A's agents daemon serves each session's talks so, due Sat 2026-11-21 (Y2YBTS's
// review of #4272), and then this file goes; a talk said but not yet recorded may stay the
// shell's, as KP0L6A tells by Mon 2026-11-16.
.pragma library

.import "TalkLine.js" as Said
.import "Told.js" as Told

// The stages after which a talk is done with: heard, ended, or never delivered.
const CLOSED = ["read", "over", "failed"];
// The stages whose reply is whole.
const WHOLE = ["replied", "over", "read"];

// A talk said in this shell, as the session's talks hold one, until it's delivered.
function toSession(words, since) {
    return { id: "", at: new Date(since).toISOString(), words: String(words ?? "").trim(),
        stage: "sent", reply: "", why: "" };
}

// Whether a talk said here is the one the session's talks hold, delivered a moment after.
function delivers(served, here) {
    return Told.talkOf([served], here.words, Date.parse(here.at)) !== null;
}

// A session's talks as the launcher shows them, each once: those served, and those said here
// not yet delivered. Open ones, waiting or answered and not yet heard, come first and all of
// them; then the newest of the rest, as many as the line keeps of its own.
function sessionTalks(here, served) {
    const talks = (served ?? []).filter(one => String(one?.words ?? "") !== "");
    const lines = talks.concat((here ?? []).filter(one => !talks.some(kept => delivers(kept,
        one))));
    lines.sort((one, other) => Date.parse(other.at) - Date.parse(one.at));
    const opened = lines.filter(one => !CLOSED.includes(one.stage));
    return opened.concat(lines.filter(one => !opened.includes(one)).slice(0, Said.KEPT_TALKS));
}

// How a session's talk reads under the field, in exchange()'s shape: his words, when he said
// them at their right; sent until it's delivered, then, breathing, the word the session's row
// says of it (delivered, reading, replying: `words.stages`, Talks' own), so one talk is told
// one way wherever Joel looks (VKCF3Z [2dbb1c2f]), or waiting on whom for a stage it has no
// word for, with what it has written so far; its reply, and who answered at its right; or why
// it never went.
function spoken(talk, name, words, dayOf) {
    const since = Date.parse(talk.at);
    const day = typeof dayOf === "function" ? dayOf(since) : "";
    const shown = { line: talk.words, answers: [], waiting: "", stands: "", by: "",
        side: day === "" ? Said.clockOf(since) : `${day} ${Said.clockOf(since)}` };
    const reply = String(talk.reply ?? "").trim();
    if (talk.stage === "sent") {
        return Object.assign(shown, { stands: String(words?.sent ?? "") });
    }
    if (talk.stage === "failed") {
        const why = String(talk.why ?? "");
        return Object.assign(shown, { stands: [String(words?.unsent ?? "")]
            .concat(why === "" ? [] : [why]).join(" \u00b7 ") });
    }
    const whole = WHOLE.includes(talk.stage);
    return Object.assign(shown, { answers: reply === "" ? [] : [reply],
        waiting: whole ? "" : String(words?.stages?.[talk.stage] ?? "")
            || Said.worded(words, "waiting", name),
        by: whole && reply !== "" ? String(name ?? "") : "" });
}

// Whether a talk shown has an answer he hasn't heard, for the daemon to be told he has.
function unheard(talk) {
    return talk.stage === "replied";
}

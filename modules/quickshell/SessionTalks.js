// Talking to a session from the launcher, Talk to It and its kin, which shows the talk under
// the field as Super+S's line shows the void's (Joel at 19:30 on 2026-11-13: "they need to
// feel more like the void"), in TalkLine.js's exchange shape and on its clock and words. A
// session answers on its talk's own lines, as Orchestrators.qml folds them,
// `{ id, at, words, stage, reply, why }`, and the daemon writes `read` once he has heard it,
// so what's open outlives the shell. tests/talk-line.qml holds it to its cases in Qt's own
// engine.
//
// Which talks show, and in what order, is the agents daemon's, as the daemons are the data
// layer (modules/agents/src/inboxes.rs): each open one, then the newest few, a talk still
// waiting in its session's inbox among them as sent. Only the words just said stay this
// shell's, for the moment before the daemon hears them land, so Return shows them at once.
.pragma library

.import "TalkLine.js" as Said
.import "Told.js" as Told

// The stages whose reply is whole.
const WHOLE = ["replied", "over", "read"];

// A talk said in this shell, as the session's talks hold one, until it's delivered.
function toSession(words, since) {
    return { id: "", at: new Date(since).toISOString(), words: String(words ?? "").trim(),
        stage: "sent", reply: "", why: "" };
}

// Whether a talk said here is one the daemon serves, waiting in its inbox or delivered: the
// same words, served no earlier than said, by Told.js's slack for the harness's clock.
function delivers(served, here) {
    return Told.talkOf([served], here.words, Date.parse(here.at)) !== null;
}

// A session's talks as the launcher shows them, each once: those said here that the daemon
// hasn't served yet, newest first, then those it serves, as it picked and ordered them.
function sessionTalks(here, served) {
    const talks = served ?? [];
    return (here ?? []).filter(one => !talks.some(kept => delivers(kept, one))).concat(talks);
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

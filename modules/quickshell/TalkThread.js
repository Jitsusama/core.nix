// A line of Super+S's opened as its thread, apart from how it's drawn
// (TalkThread.qml): his words whole, then everything the void wrote of it in
// the order written, each with who and when, and the nerdy bits under a fold
// (Joel at 17:48 on 2026-11-13, 8GYNJ6 [851e94b2]: "i need a way to drill
// into them and continue those threads of conversation ... they lack the
// nerdiness of other things"). Which line j and k pick in the list is here
// too. tests/talk-line.qml holds it to its cases in Qt's own engine.
.pragma library
.import "TalkLine.js" as Said

// The events whose words are someone's answer, set as answers are in the list.
const ANSWERS = ["answered", "landed", "taken"];

// One event as the thread says it: `text` in the words' ink where it's an
// answer, else in the voice, saying who has it or how it ended; `by`, who
// wrote it, and `side`, when, at its right; and `nerd`, what the void calls
// it and the message it is, and what it answers or replaces, for the fold.
function stepOf(event, words) {
    // Who opens each of these, so keeps its capital, as the list's `by` does.
    const who = String(event.who ?? "");
    const text = String(event.text ?? "").trim();
    const answer = ANSWERS.includes(event.event) && text !== "";
    const said = {
        routed: Said.worded(words, "has", who),
        taken: Said.worded(words, "tookUp", who),
        "let-go": Said.worded(words, "letGo", who),
        undone: String(words?.taken ?? "")
    }[event.event] ?? String(event.event ?? "");
    const nerd = [String(event.event ?? ""), event.msg ? `msg ${event.msg}` : "",
        event.re ? `re ${event.re}` : "", event.supersedes ? `replaces ${event.supersedes}` : "",
        event.owner ? `to ${event.owner}` : ""].filter(part => part !== "").join(" \u00b7 ");
    return { text: answer ? text : said, answer, by: answer ? who : "",
        side: String(event.at ?? ""), nerd };
}

// The thread of `talk`, a line as the list holds it: his words and when he
// said them, a day said beside the time where it was an earlier one
// (`dayOf`); each event of its key, every one, superseded too, since the
// thread is where nothing is left out; and its key in the void's day file,
// which the fold names first. A line the void hasn't written of yet holds
// only his words.
function threadOf(talk, events, words, dayOf) {
    const key = Said.keyOf(talk);
    const day = typeof dayOf === "function" ? dayOf(talk.since) : "";
    const clock = Said.clockOf(talk.since);
    const own = key === "" ? [] : (events ?? []).filter(event => event.line === key);
    return { key, line: String(talk.line ?? ""), side: day === "" ? clock : `${day} ${clock}`,
        steps: own.map(event => stepOf(event, words)),
        nerd: key === "" ? "" : `line ${key}` };
}

// The line j and k pick, `count` down from `picked` or up where it's less than
// nothing, among `length` lines: the first one down picks the newest where
// none was, and neither goes past either end.
function picking(picked, count, length) {
    if (length <= 0) {
        return -1;
    }
    const from = picked < 0 ? (count > 0 ? -1 : 0) : picked;
    return Math.max(0, Math.min(length - 1, from + count));
}

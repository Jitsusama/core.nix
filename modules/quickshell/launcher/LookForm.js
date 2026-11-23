// A look's second form, as 85PKB2's harness #2008 writes it beside the
// first: what happened, why it matters, what Joel presses and what it wants
// back. Looks.js reads a look's words through it, and Now.qml its side, its
// act and whether it asks him back; tests/for-you.qml holds it in Qt's own
// engine.
.pragma library

// How a sentence ends, so a second form's parts read as sentences, its
// closing quote or bracket after; and the most words an act runs to, as
// the harness writes it (ACT_WORDS).
const ENDED = /[.!?\u2026]["'\u2019\u201d)\]]*$/u;
const ACT_WORDS = 4;
const SPACES = /\s+/gu;
// What draws as nothing, or as a space, which a name may hide in to pass
// for another, the blank fillers of Hangul and Braille among them; and the
// stops a name may end in. Ranges rather than property escapes, which Qt's
// engine may not read.
const UNSEEN = /[\u200b-\u200f\u202a-\u202e\u2060-\u2064\u2066-\u206f\ufeff]/gu;
const UNMARKED = /[\u00ad\u061c\u180e]|\u034f/gu;
const FILLERS = /[\u115f\u1160\u2800\u3164\uffa0]/gu;
const STOPS = /[\s.!?,;:\u2026]+$/u;

// `value` where it's a string, else "".
function written(value) {
    return typeof value === "string" ? value : "";
}

// Whether a look is said in its second form, beside the first form's why
// and ask, which its harness made from it.
function secondForm(one) {
    return written(one.happened).trim() !== "";
}

// What its drawer leads with, to act on hours on (Joel, 2026-11-11): in
// its second form what happened then why it matters, each ending as a
// sentence ends, as its harness writes a why; else its why, then its ask.
function wordsOf(read) {
    const ended = text => text === "" || ENDED.test(text) ? text : `${text}.`;
    return secondForm(read)
        ? [read.happened, read.matters].map(part => ended(written(part).trim()))
            .filter(part => part !== "").join(" ")
        : [read.why, read.ask].filter(part => part && typeof part === "string").join("\n\n");
}

// A name as it reads, to tell two apart: what draws as nothing gone, its
// spaces one, its closing stops gone, in small letters.
function readAs(name) {
    return String(name).replace(UNSEEN, "").replace(UNMARKED, "").replace(FILLERS, " ")
        .replace(SPACES, " ").replace(STOPS, "").trim().toLowerCase();
}

// What Joel presses for a look in its second form, in its own words, as
// "Review It", else "": never past the harness's four words, a choice's
// label, or a name another of its acts goes by (`taken`), however it's
// spaced, stopped or hides what draws as nothing, so it can't pass itself
// off as Clear It or an answer. A letter from another script that draws
// alike still could; the drawer's eyebrow says whose words they are.
function actOf(one, taken = []) {
    const act = secondForm(one) ? written(one.act).trim() : "";
    const read = readAs(act);
    return read === "" || read.split(" ").length > ACT_WORDS || poses(act, taken)
        || (one.offers ?? []).some(choice => readAs(choice.label) === read) ? "" : act;
}

// Whether a name reads as one of `taken`, however it's spaced, stopped or
// hides what draws as nothing, so a choice can't pass for Clear It.
function poses(name, taken) {
    const read = readAs(name);
    return taken.some(each => readAs(each) === read);
}

// Whether a look in its second form asks Joel back: it wants a choice or
// his own words, or offers choices. One in its first form says so in its ask.
function asksBack(one) {
    return secondForm(one) && (one.wants?.words === true || (one.offers ?? []).length > 0
        || (Array.isArray(one.wants?.choices) && one.wants.choices.length > 0));
}

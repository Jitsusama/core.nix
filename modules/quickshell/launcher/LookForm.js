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
const SPACES = /\s+/u;

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

// What Joel presses for a look in its second form, in its own words, as
// "Review It", else "": never past the harness's four words, a choice's
// label, or a name another of its acts goes by (`taken`), so it can't pass
// itself off as Clear It or an answer.
function actOf(one, taken = []) {
    const act = secondForm(one) ? written(one.act).trim() : "";
    const same = name => String(name).toLowerCase() === act.toLowerCase();
    return act.split(SPACES).length > ACT_WORDS || taken.some(same)
        || (one.offers ?? []).some(choice => same(choice.label)) ? "" : act;
}

// Whether a look in its second form asks Joel back: it wants a choice or
// his own words, or offers choices. One in its first form says so in its ask.
function asksBack(one) {
    return secondForm(one) && (one.wants?.words === true || (one.offers ?? []).length > 0
        || (Array.isArray(one.wants?.choices) && one.wants.choices.length > 0));
}

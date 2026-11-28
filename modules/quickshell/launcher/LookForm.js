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
const UNSEEN = /[\u200b-\u200f\u202a-\u202e\u2060-\u206f\ufeff\ufff0-\ufff8]/gu;
const UNMARKED = /[\u00ad\u061c\u180e]|\u034f/gu;
const FILLERS = /[\u115f\u1160\u2800\u3164\uffa0]/gu;
const STOPS = /[\s.!?,;:\u2026]+$/u;
// What turns the letters after it around, a mark, an embedding, an override
// or an isolate: left out of a name it's read as nothing, yet "\u202etI
// raelC" still draws as Clear It, so a name holding one is never trusted.
const TURNS = /[\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u;
// And the rest of what Unicode says may draw as nothing, some of it past
// U+FFFF: selectors of a letter's variant (Mongolian's and Unicode's),
// Khmer's inherent vowels, shorthand's and music's format controls, and
// the whole of the tags' plane, its selectors' supplement among them;
// each by code point, from first to last.
const UNDRAWN = [[0x17b4, 0x17b5], [0x180b, 0x180d], [0x180f, 0x180f], [0xfe00, 0xfe0f],
    [0x1bca0, 0x1bca3], [0x1d173, 0x1d17a], [0xe0000, 0xe0fff]];

// `text` with what UNDRAWN names left out. It walks the text by code
// point itself, since Qt's engine's Array.from walks a string by halves
// of a pair, as a case past U+FFFF showed (23 Nov).
function drawnOf(text) {
    let kept = "";
    for (let at = 0; at < text.length;) {
        const point = text.codePointAt(at);
        const width = point > 0xffff ? 2 : 1;
        if (!UNDRAWN.some(([first, last]) => point >= first && point <= last)) {
            kept += text.slice(at, at + width);
        }
        at += width;
    }
    return kept;
}

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
// Each trimmed, as the sites' ask row reads them, so words only of spaces
// leave no blank.
function wordsOf(read) {
    const ended = text => text === "" || ENDED.test(text) ? text : `${text}.`;
    return secondForm(read)
        ? [read.happened, read.matters].map(part => ended(written(part).trim()))
            .filter(part => part !== "").join(" ")
        : [read.why, read.ask].map(part => written(part).trim())
            .filter(part => part !== "").join("\n\n");
}

// A name as it reads, to tell two apart: what draws as nothing gone, its
// spaces one, its closing stops gone, in small letters.
function readAs(name) {
    return drawnOf(String(name)).replace(UNSEEN, "").replace(UNMARKED, "").replace(FILLERS, " ")
        .replace(SPACES, " ").replace(STOPS, "").trim().toLowerCase();
}

// What Joel presses for a look in its second form, in its own words, as
// "Review It", else "": never past the harness's four words, a choice's
// label, or a name another of its acts goes by (`taken`), however it's
// spaced, stopped or hides what draws as nothing, so it can't pass itself
// off as Clear It or an answer, or turn its letters around. A letter from
// another script that draws alike still could; the drawer's eyebrow says
// whose words they are.
function actOf(one, taken = []) {
    const act = secondForm(one) ? written(one.act).trim() : "";
    const read = readAs(act);
    return read === "" || read.split(" ").length > ACT_WORDS || TURNS.test(act)
        || poses(act, taken)
        || (one.offers ?? []).some(choice => readAs(choice.label) === read) ? "" : act;
}

// Whether a name reads as one of `taken`, however it's spaced, stopped or
// hides what draws as nothing, so a choice can't pass for Clear It; one
// that turns its letters around poses as any, since it can draw as either.
function poses(name, taken) {
    const read = readAs(name);
    return taken.length > 0
        && (TURNS.test(String(name)) || taken.some(each => readAs(each) === read));
}

// What Explain It is told a look's row offers, by the names the row shows
// them under: `cellActs`, those on the cell behind it, then a terminal
// look's own Open (`openName`), which goes to that cell's window. Any other
// look's Open, like its choices, answers the look, not what the cell asks
// now. A terminal's Open may go by the look's own words, as its row shows
// it, which actOf keeps from passing for any act the launcher names.
function presses(one, cellActs, openName) {
    return one.kind === "terminal" ? cellActs.concat([openName]) : cellActs.slice();
}

// Whether a look in its second form asks Joel back: it wants a choice or
// his own words, or offers choices. One in its first form says so in its ask.
function asksBack(one) {
    return secondForm(one) && (one.wants?.words === true || (one.offers ?? []).length > 0
        || (Array.isArray(one.wants?.choices) && one.wants.choices.length > 0));
}

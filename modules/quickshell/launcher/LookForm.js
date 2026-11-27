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
// The most characters a choice opening with a one-word act runs to and is
// still taken to draw whole on the narrowest verb line (poses).
const WHOLE_CHOICE = 24;
const SPACES = /\s+/gu;
// What draws as nothing, or as a space, which a name may hide in to pass
// for another, the blank fillers of Hangul and Braille among them; and the
// stops a name may end in. Ranges rather than property escapes, which Qt's
// engine may not read.
const UNSEEN = /[\u200b-\u200f\u202a-\u202e\u2060-\u206f\ufeff\ufff0-\ufff8]/gu;
const UNMARKED = /[\u00ad\u061c\u180e]|\u034f/gu;
// The marks that combine with the letter before them, drawn on it and
// taking no place of their own, so "Tal\u0327k" draws as Talk with a
// cedilla under its l; read as nothing, it's read as Talk (round 14 on
// #6562, 201).
const MARKS = /[\u0300-\u036f\u1ab0-\u1aff\u1dc0-\u1dff\u20d0-\u20ff\ufe20-\ufe2f]/gu;
const FILLERS = /[\u115f\u1160\u2800\u3164\uffa0]/gu;
const STOPS = /[\s.!?,;:\u2026]+$/u;
// The letters an act's are told apart from: ASCII's, and Latin's accented
// ones. Any other a name holds where an act's letter stands may draw as
// it, a Cyrillic Es as C, a Lisu letter as a Latin capital, a Roman
// numeral as I, a fullwidth or a mathematical letter as its plain one, so
// it's read as able to be any letter (drawsLike). A list of the letters
// that draw alike never ends, as three rounds on #6562 found (F170, then
// round 13's 177 to 187), so none is kept. It refuses more than it must:
// a choice in another script, as long as an act or longer, may be held
// back too, which a look written in English never is.
const LATIN = /[\x21-\x7e\u00c0-\u017f]/u;
// And Latin's own letters that draw alike in a plain face, each read as
// the one it draws like: a capital I, a small l, a dotless i, a one and a
// bar as a stem, a zero as an o, so "Clear lt", "CIear It" and "Explain It"
// with a dotless i draw as the act and read as it (round 14, 198 and 207).
// Unlike the letters beyond Latin, these are few, and known.
const STEMS = { i: "l", 1: "l", "|": "l", "\u0131": "l", 0: "o" };
// A digit, which goes on a word as a letter does (opensWith).
const DIGIT = /[0-9]/u;
// Where a verb's one line stops showing what follows: a line break, or two
// spaces or more, past which the rest may run out of room.
const CUTS = /[\n\r\v\f\u0085\u2028\u2029]|\s{2,}/u;
// What turns the letters after it around, a mark, an embedding, an override
// or an isolate: left out of a name it's read as nothing, yet "\u202etI
// raelC" still draws as Clear It, so a name holding one is never trusted.
const TURNS = /[\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u;
// What splits a name into variants Qt's Text draws only the first of that
// fits, eliding the last alone: "Talk\u009cX" draws as Talk however it reads
// here, so a name holding one is never trusted either.
const VARIANTS = /\u009c/u;
// What may follow a one-word act in a choice that's still trusted to draw
// whole: printable ASCII, Latin's accented letters, Greek's and Cyrillic's
// letters, each weighed as wide as a W, which none draws wider than
// (round 13 on #6562, 180; round 14, 189, 200 and 205), and
// typographic quotes and dashes, whose width the row's cut can be judged
// by (weighed), as "Open Jos\u00e9\u2019s PR" or "Answer \u0414\u0430". A wide
// glyph, a fullwidth letter among them, or a mark drawn like a stop
// ("Open \u65e5\u65e5", "Talk\u2024\u2024") may draw as the act and an
// ellipsis however few its characters.
const PLAIN =
    /^[\x20-\x7e\u00c0-\u017f\u0386-\u03ce\u0400-\u045f\u2013\u2014\u2018\u2019\u201c\u201d]*$/u;
// What of it draws wider than a small letter, which weighed counts by its
// width: about twice, "Talk " and 19 capital W's drawing as wide as 43
// small x's, as their accented and joined kin do, and every Greek or
// Cyrillic letter PLAIN lets in, since Ш, ш, Ω and Μ draw as wide as W or
// m, and weighing them all so only ever weighs them too wide (WIDEST); or
// half as wide again, as every other capital, which weighed tells by its
// case.
const WIDEST = /[MWmw@%\u2014\u00c6\u00e6\u0152\u0153\u0174\u0175\u0386-\u03ce\u0400-\u045f]/u;
// A space PLAIN can't see, since the read makes every space one: a tab or
// a wide space draws past its count, "Talk x\t" and 15 y's as Talk.
const SPACED = /[^\S \u00a0]/u;
// What a verb's line cut short strips from its end before its ellipsis,
// as Reading.js's DANGLING, which draws the cut: a stop or a pause, or a
// dash or a bar after a space, so "Talk - " and a long word draws as Talk.
const DANGLING = /(?:[\s\u00b7,;:]|\s[-|\u2013])+$/u;
// What ends a name drawn as nothing, trimmed by hand, as Qt's engine has
// no trimEnd.
const TRAILING = /\s+$/u;
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

// `text` by code point, as drawnOf walks it, since Qt's engine's
// Array.from walks a string by halves of a pair.
function pointsOf(text) {
    const points = [];
    for (let at = 0; at < text.length;) {
        const width = text.codePointAt(at) > 0xffff ? 2 : 1;
        points.push(text.slice(at, at + width));
        at += width;
    }
    return points;
}

// Whether the first of `read`'s letters, as many as `each` has, may draw as
// `each`: each the act's own letter, or one not LATIN where the act has a
// letter, which may draw as any, or one STEMS reads as the act's. A space
// draws only as a space.
function drawsLike(read, each) {
    const ours = pointsOf(read);
    const acts = pointsOf(each);
    const stem = one => STEMS[one] ?? one;
    return ours.length >= acts.length && acts.every((one, at) => stem(ours[at]) === stem(one)
        || (one !== " " && ours[at] !== " " && !LATIN.test(ours[at])));
}

// A name as it reads, to tell two apart: what draws as nothing gone, its
// spaces one, its closing stops gone, in small letters.
function readAs(name) {
    return drawnOf(String(name)).replace(UNSEEN, "")
        .replace(UNMARKED, "").replace(MARKS, "").replace(FILLERS, " ")
        .replace(SPACES, " ").replace(STOPS, "").trim().toLowerCase();
}

// What Joel presses for a look in its second form, in its own words, as
// "Review It", else "": never past the harness's four words, a choice's
// label, one a label opens with or opening with one, or a name another of
// its acts goes by (`taken`), however it's
// spaced, stopped or hides what draws as nothing, so it can't pass itself
// off as Clear It or an answer, or turn its letters around, nor hold,
// where an act's letters stand, a letter from beyond Latin that may draw
// as one (drawsLike); the drawer's eyebrow says whose words they are.
function actOf(one, taken = []) {
    const act = secondForm(one) ? written(one.act).trim() : "";
    const read = readAs(act);
    return read === "" || read.split(" ").length > ACT_WORDS || TURNS.test(act)
        || VARIANTS.test(act)
        || poses(act, taken)
        || (one.offers ?? []).some(choice => poses(act, [written(choice.label)])
            || poses(written(choice.label), [act])) ? "" : act;
}

// The words of a name before a line break or a run of spaces, which is
// all a verb drawn on one line shows once the rest is cut short.
function headOf(name) {
    return drawnOf(String(name)).replace(UNSEEN, "").replace(UNMARKED, "")
        .replace(MARKS, "").replace(FILLERS, " ").trim().split(CUTS)[0];
}

// Whether a name reads as one of `taken`, however it's spaced, stopped or
// hides what draws as nothing, or as one once what follows a line break or
// a run of spaces is cut from sight, so a choice can't pass for Clear It;
// one that turns its letters around poses as any, since it can draw as
// either. One that opens with a taken name's words poses too, whatever
// follows, since a verb's line cut short where it runs out of room shows
// only those, as Clear It and one long word draws as Clear It and an
// ellipsis; the line's room isn't known here, so it's never trusted to
// show the rest. A one-word act, as Open, Answer or Talk, begins many a
// choice Joel should press (Open a PR, Answer Later, Talk It Through), so
// one opening with it poses only once it's long enough to be cut short to
// it: past ACT_WORDS words or WHOLE_CHOICE characters read, or with the
// word after it past WHOLE_CHOICE drawn, its stops and capitals weighed
// (weighed), or once what follows it isn't PLAIN or holds a space wider
// than one (SPACED), whose width its count can't tell.
function poses(name, taken) {
    const read = readAs(name);
    const head = readAs(headOf(name));
    const cut = read.split(" ").length > ACT_WORDS || read.length > WHOLE_CHOICE;
    // Only its end trimmed: a row's name is drawn with what opens it, so a
    // choice opening with a space opens with no act's letters and poses,
    // rather than measured a space short (round 11 on #6562, F158).
    const face = drawnOf(String(name)).replace(TRAILING, "");
    // The least of it a verb's line cut short shows, since it's cut at a
    // word (Reading.wordCuts): the act and the word after it and an
    // ellipsis, or the whole of a choice of two words. Only where that
    // won't fit does the act show alone, so "Talk With Mom Tomorrow" is
    // offered and "Talk" and one long word isn't (round 9 on #6562, F130).
    // It's measured on the face as drawn, which the read's offsets match
    // only where the face opens with the act's own letters (plainly): one
    // where something that draws as nothing sits before or inside the act
    // is refused rather than measured at the wrong place (round 10, F132,
    // F139, F143). Drawn whole where it fits; else the least wordCuts
    // gives that reads as more than the act, its words split at a space
    // and what it strips as DANGLING gone, since "Talk, " or "Talk - " and a
    // long word draws as Talk (F133, F135).
    const drawn = each => {
        const uncut = actWeighed(face, each.length) + weighed(face.slice(each.length));
        const words = face.split(" ");
        for (let count = 1; uncut > WHOLE_CHOICE && count < words.length; count += 1) {
            const cutTo = words.slice(0, count).join(" ").replace(DANGLING, "");
            // Read as it's read, so a cut of the act and what draws as
            // nothing, "Talk\u200b", isn't taken for more than the act.
            if (readAs(cutTo).length > each.length) {
                return actWeighed(face, each.length) + weighed(cutTo.slice(each.length)) + 1;
            }
        }
        return uncut;
    };
    const plainly = each => face.slice(0, each.length).toLowerCase() === each;
    const opened = each => each.includes(" ") || cut || !plainly(each)
        || !PLAIN.test(read.slice(each.length)) || SPACED.test(face.slice(each.length))
        || drawn(each) > WHOLE_CHOICE;
    return taken.length > 0 && (TURNS.test(String(name)) || VARIANTS.test(String(name))
        || taken.map(readAs).some(each => each !== "" && (reads(read, each)
            || reads(head, each) || (opensWith(read, each) && opened(each)))));
}

// Whether `read` may draw as `each` whole: as long, by code point, and
// each letter alike (drawsLike).
function reads(read, each) {
    return pointsOf(read).length === pointsOf(each).length && drawsLike(read, each);
}

// How wide the act that opens `face` draws, its first `length` characters,
// as weighed says but for its first capital, which every act opens with
// and the room was set by: an act written in capitals is charged its
// width, "ANSWER" three letters more than "Answer" (round 10, F140).
function actWeighed(face, length) {
    const act = face.slice(0, length);
    return weighed(act.charAt(0).toLowerCase() + act.slice(1));
}

// How wide `text` draws, in small letters: each character one, a capital
// half as wide again, and one about twice as wide (WIDEST) two. It walks
// by code point, as ask.ts's does, so a character past U+FFFF is one.
function weighed(text) {
    let width = 0;
    for (let at = 0; at < text.length;) {
        const point = text.codePointAt(at);
        const each = String.fromCodePoint(point);
        width += WIDEST.test(each) ? 2 : each === each.toLowerCase() ? 1 : 1.5;
        at += each.length;
    }
    return width;
}

// Whether `read` opens with the words `each` and then a break between
// words: a space, a stop or a mark, as "clear it, now" opens with "clear
// it", where "clearance" doesn't open with "clear". A letter is one with
// a case, so a letter of a script without one counts as a break, refusing
// more rather than less.
function opensWith(read, each) {
    const ours = pointsOf(read);
    const length = pointsOf(each).length;
    if (ours.length === length || !drawsLike(read, each)) {
        return false;
    }
    const next = ours[length];
    return next.toLowerCase() === next.toUpperCase() && !DIGIT.test(next);
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

// Whether a look in its second form asks Joel back: it wants his own
// words, or offers choices he can press (`offered`, those poses left), or
// wants a choice it offers none for. One whose every choice was held back
// asks nothing he can give, so it doesn't say it asks. One in its first
// form says so in its ask.
function asksBack(one, offered = one.offers ?? []) {
    const wanted = Array.isArray(one.wants?.choices) && one.wants.choices.length > 0;
    return secondForm(one) && (one.wants?.words === true || offered.length > 0
        || (wanted && (one.offers ?? []).length === 0));
}

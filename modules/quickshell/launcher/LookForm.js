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
// The longest a choice is read at: past four times WHOLE_CHOICE it poses
// unread, since one in another script was once walked whole against every
// act, about (m+1)^2 a step on the shell's thread (#6562, 221).
const LONGEST = WHOLE_CHOICE * 4;
// The accents a choice's decomposed letters carry, which draw on the letter
// before them, so "Ta\u013ck" decomposed reads as "Tal\u0327k" and poses alike
// (#6562, 215).
const COMBINING = /[\u0300-\u036f]/gu;
// The only characters a choice is trusted to hold once its accents are
// gone, each with how wide it draws in small letters: ASCII's letters and
// digits, the space, the marks a choice in English writes, and typographic
// quotes and dashes. Any other may draw as an act's letter or as nothing,
// a Cyrillic Es as C, a kra as k, a times sign as x, a mark no list names
// over a letter (#6562, F170, 177 to 187, 211, 218 and 224). A list of
// those never ends, as round after round on #6562 found, so the set is
// closed and a choice holding anything outside it poses as any act. It
// refuses more than it must: a choice in another script, or with a letter
// as \u00f8, is held back, which a look written in English rarely is. A
// capital or a digit draws half as wide again as a small letter, and an m,
// a w and the marks drawn as wide as one about twice, so "Talk " and 19
// #'s, weighed as small letters, drew cut to Talk (212).
// The marks sessions write all the time are in it too, since leaving them
// out held back "Merge into `main`", "Ship at 50%" and "Not yet\u2026"
// (230, 234, 241 and 247): each draws as a mark of its own, never as
// nothing, and each is weighed as the widest, which a $ or a _ draws as
// (214) and the narrow ones, a bar or a backtick, draw well within, so a
// run of any after an act is never weighed short. The ellipsis is a stop,
// as STOPS and ENDED read it, but draws as three, about as wide as an m.
// Those that may stand for a letter read as it (ALIKE); the rest draw as
// no letter in a plain face: _ and ~ as a rule, as the dash already in the
// set does, [ ] < > as brackets open on one side, as ( ) are, and % = *
// as signs.
const WIDTHS = widthsOf([
    ["abcdefghijklnopqrstuvxyz .,'\"-!?:;()/\u2018\u2019\u201c\u201d\u2013", 1],
    ["ABCDEFGHIJKLNOPQRSTUVXYZ0123456789", 1.5],
    ["mwMW&#+\u2014`_@%=*[]<>~$|\u2026", 2]]);
// The set's characters that draw alike in a plain face, each read as the
// one it draws like: a capital I, a small l, a one and a bar as a stem, a
// zero as an o, two v's as a w, r and n as an m, c and l as a d, a $ as an
// s, an @ as an a, and a backtick as an apostrophe, so "Clear lt",
// "Ansvver", "Rnerge", "Cle@r It" and "$ave" draw as an act and read as it
// (round 14 on #6562, 198 and 207; 218 and 224). Unlike the letters beyond
// the set, these are few, and known.
const ALIKE = [[/[i1|]/gu, "l"], [/0/gu, "o"], [/vv/gu, "w"], [/rn/gu, "m"], [/cl/gu, "d"],
    [/\$/gu, "s"], [/@/gu, "a"], [/`/gu, "'"]];
// What goes on a word, so what follows an act's words is a break only where
// it's none of these (opensWith): a letter, or a digit as a letter does.
const WORDED = /[a-z0-9]/u;
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

// The table of WIDTHS from its `groups`, each of characters drawn alike
// wide and that width, so the set is the table's keys.
function widthsOf(groups) {
    const table = new Map();
    for (const [characters, width] of groups) {
        for (const one of characters.split("")) {
            table.set(one, width);
        }
    }
    return table;
}

// `read` as it draws in a plain face, its letters that draw alike as one
// (ALIKE), to tell an act apart from what draws as it.
function skeletonOf(read) {
    return ALIKE.reduce((drawn, [alike, as]) => drawn.replace(alike, as), read.toLowerCase());
}

// A name as it reads, to tell two apart: decomposed, what draws as nothing
// gone, its spaces one, its closing stops gone, in small letters.
function readAs(name) {
    return drawnOf(String(name).normalize("NFD")).replace(UNSEEN, "")
        .replace(UNMARKED, "").replace(MARKS, "").replace(FILLERS, " ")
        .replace(SPACES, " ").replace(STOPS, "").trim().toLowerCase();
}

// What Joel presses for a look in its second form, in its own words, as
// "Review It", else "": never past the harness's four words, a choice's
// label, one a label opens with or opening with one, or a name another of
// its acts goes by (`taken`), however it's
// spaced, stopped or hides what draws as nothing, so it can't pass itself
// off as Clear It or an answer, or turn its letters around, nor hold,
// beside a name it's told apart from, a character outside the closed set
// (WIDTHS) that may draw as an act's letter or as nothing; the drawer's
// eyebrow says whose words they are. It's told apart only from the choices
// offered beside it, those poses leaves of `taken`, as Now.qml's
// lookOffers leaves them: one held back is never drawn beside it, so one
// holding a character outside the set cost the act its words for nothing
// (#6562, 231).
function actOf(one, taken = []) {
    const act = secondForm(one) ? written(one.act).trim() : "";
    const read = readAs(act);
    const beside = (one.offers ?? []).map(choice => written(choice.label))
        .filter(label => !poses(label, taken));
    return read === "" || read.split(" ").length > ACT_WORDS || TURNS.test(act)
        || VARIANTS.test(act)
        || poses(act, taken)
        || beside.some(label => poses(act, [label]) || poses(label, [act])) ? "" : act;
}

// The words of a name before a line break or a run of spaces, which is
// all a verb drawn on one line shows once the rest is cut short.
function headOf(name) {
    return drawnOf(String(name)).replace(UNSEEN, "").replace(UNMARKED, "")
        .replace(MARKS, "").replace(FILLERS, " ").trim().split(CUTS)[0];
}

// Whether a name reads as one of `taken`, however it's spaced, stopped or
// accented, or as one once what follows a line break or a run of spaces is
// cut from sight, so a choice can't pass for Clear It. One past LONGEST,
// or holding, once its accents are gone, a character outside the closed
// set (WIDTHS), poses as any, since it may draw as either: a turn, a tab,
// a letter from beyond Latin. One that opens with a taken name's words
// poses too, whatever follows, since a verb's line cut short where it runs
// out of room shows only those, as Clear It and one long word draws as
// Clear It and an ellipsis; the line's room isn't known here, so it's
// never trusted to show the rest. A one-word act, as Open, Answer or Talk,
// begins many a choice Joel should press (Open a PR, Answer Later, Talk It
// Through), so one opening with it poses only once it's long enough to be
// cut short to it: past ACT_WORDS words or WHOLE_CHOICE characters read,
// or with the word after it past WHOLE_CHOICE drawn, each character
// weighed as WIDTHS says.
function poses(name, taken) {
    if (taken.length === 0) {
        return false;
    }
    // Counted by halves of a pair, which only a character past U+FFFF has
    // two of, and none of those is in the set, so a name it counts too
    // long poses all the same.
    if (String(name).length > LONGEST) {
        return true;
    }
    const spelled = String(name).normalize("NFD");
    const bare = spelled.replace(COMBINING, "");
    if (!trusted(bare)) {
        return true;
    }
    const read = readAs(bare);
    const head = readAs(headOf(bare));
    const cut = read.split(" ").length > ACT_WORDS || read.length > WHOLE_CHOICE;
    // Only its end trimmed: a row's name is drawn with what opens it, so a
    // choice opening with a space opens with no act's letters and poses,
    // rather than measured a space short (round 11 on #6562, F158).
    const face = bare.replace(TRAILING, "");
    // The least of it a verb's line cut short shows, since it's cut at a
    // word (Reading.wordCuts): the act and the word after it and an
    // ellipsis, or the whole of a choice of two words. Only where that
    // won't fit does the act show alone, so "Talk With Mom Tomorrow" is
    // offered and "Talk" and one long word isn't (round 9 on #6562, F130).
    // It's measured on the face as drawn, which the read's offsets match
    // only where the name opens with the act's own letters, unaccented
    // (plainly): one where a stop, a space or an accent sits before or
    // inside the act, or one of ALIKE stands for its letter, is refused
    // rather than measured at the wrong place (round 10, F132, F139,
    // F143). Drawn whole where it fits; else the least wordCuts gives
    // that reads as more than the act, its words split at a space and
    // what it strips as DANGLING gone, since "Talk, " or "Talk - " and a
    // long word draws as Talk (F133, F135).
    const drawn = each => {
        const uncut = actWeighed(face, each.length) + weighed(face.slice(each.length));
        const words = face.split(" ");
        for (let count = 1; uncut > WHOLE_CHOICE && count < words.length; count += 1) {
            const cutTo = words.slice(0, count).join(" ").replace(DANGLING, "");
            // Read as it's read, so a cut of the act and a stop, "Talk.",
            // isn't taken for more than the act.
            if (readAs(cutTo).length > each.length) {
                return actWeighed(face, each.length) + weighed(cutTo.slice(each.length)) + 1;
            }
        }
        return uncut;
    };
    const plainly = each => spelled.slice(0, each.length).toLowerCase() === each;
    const opened = each => each.includes(" ") || cut || !plainly(each)
        || drawn(each) > WHOLE_CHOICE;
    return taken.map(readAs).some(each => each !== "" && (reads(read, each)
        || reads(head, each) || (opensWith(read, each) && opened(each))));
}

// Whether every character of `name`, once its accents are gone, is in the
// closed set (WIDTHS).
function trusted(name) {
    return String(name).normalize("NFD").replace(COMBINING, "").split("")
        .every(one => WIDTHS.has(one));
}

// Why poses held back a choice named `name`, so its drawer says only
// what's true of it: "length" past LONGEST, read no further; "character"
// holding one outside the closed set, which may draw as anything; else
// "worded", read as one of the launcher's acts (#6562, 229 and 235). Asked
// only of a choice poses refused, in the order poses refuses it.
function heldFor(name) {
    if (String(name).length > LONGEST) {
        return "length";
    }
    return trusted(name) ? "worded" : "character";
}

// Whether `read` draws as `each` whole: alike once the letters that draw
// alike are read as one (skeletonOf).
function reads(read, each) {
    return skeletonOf(read) === skeletonOf(each);
}

// How wide the act that opens `face` draws, its first `length` characters,
// as weighed says but for its first capital, which every act opens with
// and the room was set by: an act written in capitals is charged its
// width, "ANSWER" three letters more than "Answer" (round 10, F140).
function actWeighed(face, length) {
    const act = face.slice(0, length);
    return weighed(act.charAt(0).toLowerCase() + act.slice(1));
}

// How wide `text` draws, in small letters, each character as WIDTHS says.
// It walks by code point, as ask.ts's does, so a character past U+FFFF is
// one.
function weighed(text) {
    let width = 0;
    for (let at = 0; at < text.length;) {
        const each = String.fromCodePoint(text.codePointAt(at));
        width += widthOf(each);
        at += each.length;
    }
    return width;
}

// How wide one character draws, as weighed counts it: as WIDTHS says, and
// one outside the set as the widest, which poses never weighs.
function widthOf(each) {
    return WIDTHS.get(each) ?? 2;
}

// Whether `read` opens with the words `each` and then a break between
// words: a space, a stop or a mark, as "clear it, now" opens with "clear
// it", where "clearance" doesn't open with "clear". Any drawing of it
// counts (skeletonOf), so "ansvver later" opens with answer.
function opensWith(read, each) {
    const act = skeletonOf(each);
    for (let end = 1; end < read.length; end += 1) {
        if (!WORDED.test(read.charAt(end)) && skeletonOf(read.slice(0, end)) === act) {
            return true;
        }
    }
    return false;
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

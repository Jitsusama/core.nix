.pragma library
// Matching by words: each word typed has to be the start of a word in the
// item, its letters unbroken, so what's found is what was typed and
// nothing that merely holds its letters scattered about. Joel asked for it
// (2026-10-23): scattered matches filled the list with things he had to
// read to rule out. "vol up" finds "Turn the volume up", "chro" finds
// "Google Chrome", "pass" finds "1Password", and "gc" finds nothing.
//
// - Words match on their own, in any order, so "open term" finds "Open a
//   terminal".
// - A word starts where fzf says one does: after a space, a delimiter or
//   other punctuation, at a capital after a small letter or a digit, and
//   at a digit after a letter.
// - Matches rank by fzf's constants, so a word landing at the start of the
//   item, or on a stronger boundary, ranks first.
// - An item is prepared once, when it appears, so a query only compares
//   character codes and reads bonuses it already has.
//
// It runs on every key, in Qt's JavaScript engine, so a miss allocates
// nothing and a hit only its answer.

const scoreMatch = 16;
const gapStart = -3;
const gapExtension = -1;
const bonusBoundary = scoreMatch / 2;
const bonusNonWord = scoreMatch / 2;
const bonusCamel = bonusBoundary + gapExtension;
const bonusConsecutive = -(gapStart + gapExtension);
const bonusFirstLetter = 2;
const bonusBoundaryWhite = bonusBoundary + 2;
const bonusBoundaryDelimiter = bonusBoundary + 1;
// Ours, not fzf's: a word typed whole beats one it only begins, so "pi"
// finds pi before a picture.
const bonusWhole = bonusBoundary;

// The kinds of character a bonus depends on, in fzf's order: anything
// past nonWord is part of a word.
const white = 0;
const nonWord = 1;
const delimiter = 2;
const lower = 3;
const upper = 4;
const letter = 5;
const number = 6;

const delimiters = "/,:;|";

function kindOf(c) {
    if (c === " " || c === "\t" || c === "\n") return white;
    if (c >= "0" && c <= "9") return number;
    if (c >= "a" && c <= "z") return lower;
    if (c >= "A" && c <= "Z") return upper;
    if (delimiters.includes(c)) return delimiter;
    if (c.charCodeAt(0) < 128) return nonWord;
    if (c.toLowerCase() !== c) return upper;
    if (c.toUpperCase() !== c) return lower;
    return /\s/.test(c) ? white : letter;
}

// What landing on a character is worth, given the one before it. A
// capital after a digit counts as a new word too, so the P of
// "1Password" is a start.
function bonusFor(before, kind) {
    if (kind > nonWord) {
        if (before === white) return bonusBoundaryWhite;
        if (before === delimiter) return bonusBoundaryDelimiter;
        if (before === nonWord) return bonusBoundary;
    }
    if (before === lower && kind === upper) return bonusCamel;
    if (before === number && kind === upper) return bonusCamel;
    if (before !== number && kind === number) return bonusCamel;
    if (kind === nonWord || kind === delimiter) return bonusNonWord;
    if (kind === white) return bonusBoundaryWhite;
    return 0;
}

// Which letters a text holds, one bit each for a to z and one for any
// digit, so an item missing a letter of the query is passed over with one
// test and no call. Anything else shares the last bit. A caller looping
// over many items tests (item.mask & letters(words)) itself before calling
// match, since in Qt's engine the call costs more than the test.
function bitOf(code) {
    if (code >= 97 && code <= 122) return 1 << (code - 97);
    if (code >= 48 && code <= 57) return 1 << 26;
    return 1 << 27;
}

function letters(ws) {
    let mask = 0;
    for (const w of ws)
        for (let i = 0; i < w.length; i++) mask |= bitOf(w.charCodeAt(i));
    return mask;
}

// An item ready to be matched: its text, its letters in lower case (one
// for one, so positions line up) as a string and as character codes, each
// letter's bonus, and which letters it holds.
function prepare(text) {
    const source = String(text);
    const n = source.length;
    const bonus = new Int8Array(n);
    const codes = new Uint16Array(n);
    let folded = "";
    let mask = 0;
    let before = white;
    for (let j = 0; j < n; j++) {
        const c = source[j];
        const kind = kindOf(c);
        bonus[j] = bonusFor(before, kind);
        before = kind;
        const l = c.toLowerCase();
        const f = l.length === 1 ? l : c;
        folded += f;
        codes[j] = f.charCodeAt(0);
        mask |= bitOf(codes[j]);
    }
    return { text: source, folded: folded, codes: codes, bonus: bonus, mask: mask };
}

// What a word is worth landing at `at`: fzf's score for a run of letters,
// the first worth more where a stronger word starts and doubled, each
// later one keeping the bonus of the start of its run, unless a stronger
// word start inside it begins a new one. A run ending where its word ends
// is worth more again.
function worth(bonus, at, m) {
    let opened = bonus[at];
    let score = scoreMatch + opened * bonusFirstLetter;
    for (let j = at + 1; j < at + m; j++) {
        const b = bonus[j];
        if (b >= bonusBoundary && b > opened) {
            opened = b;
            score += scoreMatch + b;
        } else {
            score += scoreMatch + Math.max(b, bonusConsecutive, opened);
        }
    }
    if (at + m >= bonus.length || bonus[at + m] > 0) score += bonusWhole;
    return score;
}

// The best place one word of a query (already in lower case) starts a word
// in a prepared item: its score, or -1 if it starts none. Given an array,
// it puts where each letter landed into it.
function align(item, word, positions) {
    const text = item.folded;
    const bonus = item.bonus;
    const m = word.length;
    if (m === 0) return 0;
    let best = -1;
    let bestAt = -1;
    for (let at = text.indexOf(word); at >= 0; at = text.indexOf(word, at + 1)) {
        if (bonus[at] <= 0) continue;
        const score = worth(bonus, at, m);
        if (score > best) {
            best = score;
            bestAt = at;
        }
    }
    if (positions && bestAt >= 0)
        for (let j = bestAt; j < bestAt + m; j++) positions.push(j);
    return best;
}

// What's typed as a question is read: in lower case, its spaces single,
// without a closing question mark.
function plainly(typed) {
    return String(typed).trim().toLowerCase().replace(/\s+/g, " ").replace(/\s*\?$/, "");
}

// The words of a query, in lower case.
function words(query) {
    return String(query).toLowerCase().split(/\s+/).filter(w => w.length > 0);
}

// What match answers for an item that doesn't match, shared so a miss
// costs nothing.
const missed = Object.freeze({ score: -1, grade: 3 });

// How well a prepared item answers a query whose words may land in any
// order: the sum of each word's best landing, or -1 if any word starts
// none of its words. Its grade is 0, every word landed whole, which is the
// only way one matches now; callers that keep the tidiest of their matches
// still read it. With wantPositions, the letters to bring forward, in
// order.
function match(item, query, wantPositions) {
    const ws = Array.isArray(query) ? query : words(query);
    if (!wantPositions) {
        const need = letters(ws);
        if ((item.mask & need) !== need) return missed;
    }
    let total = 0;
    const marks = wantPositions ? [] : null;
    for (let k = 0; k < ws.length; k++) {
        const score = align(item, ws[k], marks);
        if (score < 0) return missed;
        total += score;
    }
    if (!marks) return { score: total, grade: 0 };
    const positions = Array.from(new Set(marks)).sort((a, b) => a - b);
    return { score: total, grade: 0, positions: positions };
}

// Matches worth showing, best first: those as tidy as the tidiest, and a
// thing marked kept (one picked for this query before) whatever its grade.
// Ties go to the shorter text, then the alphabet. Each found thing is
// { item, score, grade } plus whatever the caller put on it.
function keep(found) {
    let finest = 3;
    for (const f of found) if (f.score >= 0 && f.grade < finest) finest = f.grade;
    return found
        .filter(f => f.score >= 0 && (f.grade <= finest || f.kept))
        .sort((a, b) => b.score - a.score || a.item.text.length - b.item.text.length
            || (a.item.text < b.item.text ? -1 : a.item.text > b.item.text ? 1 : 0));
}

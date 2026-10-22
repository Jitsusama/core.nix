.pragma library
// Fuzzy matching, after fzf's FuzzyMatchV2: a Smith-Waterman alignment that
// rewards letters landing where words start and runs of letters together,
// and charges for the gaps between them. Its constants are fzf's, so a
// query ranks here as it would there.
//
// What this adds to fzf:
// - an item is prepared once, when it appears, so a query only compares
//   character codes and reads bonuses it already has;
// - words in a query match on their own, in any order, so "open term"
//   finds "Open a terminal";
// - a word's first letter has to land where a word starts, as VS Code
//   asks, which is what keeps the list short, unless the whole word
//   lands unbroken;
// - a match made mostly of gaps is dropped rather than ranked last, and
//   one more scattered than the best is left out (keep).
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

// Room for the alignment, grown as longer items arrive and reused, so a
// query allocates nothing. For each letter of the word and each place in
// the item: the best score with that letter landing there, the length of
// the run it ends, and where the letter before it landed.
const none = -1000000;
let room = 0;
let hits = new Int32Array(0);
let runs = new Int32Array(0);
let from = new Int32Array(0);
let firsts = new Int32Array(32);

function reserve(cells) {
    if (cells <= room) return;
    room = Math.max(cells, room * 2, 1024);
    hits = new Int32Array(room);
    runs = new Int32Array(room);
    from = new Int32Array(room);
}

// How many pieces the last word aligned broke into.
let pieces = 1;

// The best alignment of one word of a query (already in lower case)
// against a prepared item: its score, or -1 for no alignment or one the
// rules refuse. It leaves how many pieces the word broke into in pieces,
// and, given an array, puts where each letter landed into it.
function align(item, word, positions) {
    const text = item.folded;
    const codes = item.codes;
    const bonus = item.bonus;
    const m = word.length;
    const n = codes.length;
    pieces = 1;
    if (m === 0) return 0;
    if (m > n) return -1;

    // One letter: the best place it lands, which has to start a word.
    if (m === 1) {
        const letterHere = word.charCodeAt(0);
        let at = -1;
        for (let j = text.indexOf(word); j >= 0 && j < n; j++)
            if (codes[j] === letterHere && (at < 0 || bonus[j] > bonus[at])) at = j;
        if (at < 0 || bonus[at] <= 0) return -1;
        if (positions) positions.push(at);
        return scoreMatch + bonus[at] * bonusFirstLetter;
    }

    // Where each letter can first land, in order, and the last place the
    // final letter appears; anything missing is no match at all.
    if (firsts.length < m) firsts = new Int32Array(m * 2);
    let after = 0;
    for (let i = 0; i < m; i++) {
        const at = text.indexOf(word[i], after);
        if (at < 0) return -1;
        firsts[i] = at;
        after = at + 1;
    }
    const last = text.lastIndexOf(word[m - 1]);
    if (last < firsts[m - 1]) return -1;

    const start = firsts[0];
    const width = last - start + 1;
    reserve(m * width);

    // The first letter: worth more where a word starts, and doubled.
    const first = word.charCodeAt(0);
    for (let j = start; j <= last; j++) {
        const cell = j - start;
        hits[cell] = codes[j] === first ? scoreMatch + bonus[j] * bonusFirstLetter : none;
        runs[cell] = 1;
    }

    // Every later letter lands either straight after the one before it,
    // continuing a run, or after a gap, which costs gapStart and then
    // gapExtension for each letter past the first. Keeping the two apart,
    // as nucleo does, means a later landing never hides an earlier, better
    // one. A run keeps the bonus of the letter that began it, unless a
    // stronger word start breaks it.
    for (let i = 1; i < m; i++) {
        const row = i * width;
        const up = row - width;
        const letterHere = word.charCodeAt(i);
        const earliest = firsts[i] - start;
        // Nothing lands before the letter before could; the cell just ahead
        // of that is read by the next row, so it says so too.
        const opening = firsts[i - 1] - start;
        if (opening > 0) hits[row + opening - 1] = none;
        // The best way to have placed the letter before, a gap ago.
        let gapped = none;
        let gappedFrom = -1;
        for (let cell = opening; cell < width; cell++) {
            hits[row + cell] = none;
            if (cell >= 2) {
                const opened = hits[up + cell - 2] + gapStart;
                if (opened >= gapped + gapExtension) {
                    gapped = opened;
                    gappedFrom = start + cell - 2;
                } else {
                    gapped += gapExtension;
                }
            }
            const j = start + cell;
            if (cell < earliest || codes[j] !== letterHere) continue;
            const before = hits[up + cell - 1];
            let together = none;
            let run = 1;
            if (before > none) {
                let b = bonus[j];
                run = runs[up + cell - 1] + 1;
                const opened = bonus[j - run + 1];
                if (b >= bonusBoundary && b > opened) run = 1;
                else b = Math.max(b, bonusConsecutive, opened);
                together = before + scoreMatch + b;
            }
            const apart = gapped > none / 2 ? gapped + scoreMatch + bonus[j] : none;
            if (together >= apart && together > none) {
                hits[row + cell] = together;
                runs[row + cell] = run;
                from[row + cell] = j - 1;
            } else if (apart > none) {
                hits[row + cell] = apart;
                runs[row + cell] = 1;
                from[row + cell] = gappedFrom;
            }
        }
    }

    // The best place for the last letter; whatever follows it is free.
    const lastRow = (m - 1) * width;
    let best = none;
    let bestAt = -1;
    for (let cell = firsts[m - 1] - start; cell < width; cell++) {
        if (hits[lastRow + cell] > best) {
            best = hits[lastRow + cell];
            bestAt = start + cell;
        }
    }
    if (bestAt < 0 || best <= none / 2) return -1;

    // Walk back through where each letter came from, counting the pieces.
    let landed = bestAt;
    for (let i = m - 1, j = bestAt; i >= 0; i--) {
        if (positions) positions.push(j);
        const previous = i > 0 ? from[i * width + j - start] : -1;
        if (i > 0 && previous !== j - 1) pieces++;
        landed = j;
        j = previous;
    }
    if (positions) positions.reverse();

    // The first letter has to land where a word starts, unless three or
    // more letters land together inside one, as "shot" does in
    // "screenshot".
    if (bonus[landed] <= 0 && (m < 3 || pieces > 1)) return -1;

    // A match whose letters are mostly gaps isn't one: each letter past
    // the first has to keep at least half its worth on average.
    const firstWorth = scoreMatch + bonus[landed] * bonusFirstLetter;
    if ((best - firstWorth) / (m - 1) < scoreMatch / 2) return -1;

    // One piece when it could have landed whole, since the best score may
    // come from a scattered landing ("screen...hot" outscores "shot" in
    // "screenshot").
    if (pieces > 1 && wholeIn(item, word)) pieces = 1;
    return best;
}

// Whether a word appears unbroken where it may land: where a word starts,
// or anywhere when it is three letters or more.
function wholeIn(item, word) {
    for (let at = item.folded.indexOf(word); at >= 0; at = item.folded.indexOf(word, at + 1))
        if (item.bonus[at] > 0 || word.length >= 3) return true;
    return false;
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
// order: the sum of each word's best alignment, or -1 if any word fails.
// Its grade says how scattered it is: 0 when every word landed whole, 1
// when one broke in two, 2 when one broke further. With wantPositions,
// the letters to bring forward, in order.
function match(item, query, wantPositions) {
    const ws = Array.isArray(query) ? query : words(query);
    if (!wantPositions) {
        const need = letters(ws);
        if ((item.mask & need) !== need) return missed;
    }
    let total = 0;
    let grade = 0;
    const marks = wantPositions ? [] : null;
    for (let k = 0; k < ws.length; k++) {
        const score = align(item, ws[k], marks);
        if (score < 0) return missed;
        total += score;
        if (pieces - 1 > grade) grade = Math.min(pieces - 1, 2);
    }
    if (!marks) return { score: total, grade: grade };
    const positions = Array.from(new Set(marks)).sort((a, b) => a - b);
    return { score: total, grade: grade, positions: positions };
}

// Matches worth showing, best first. A match more scattered than the
// finest is noise beside it: once "close" lands whole in "Close the
// window", "column left or screen" isn't what was meant, and once "tab"
// lands whole in "tabs", "the screen above" isn't either. A thing marked
// kept (one picked for this query before) is shown whatever its grade.
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

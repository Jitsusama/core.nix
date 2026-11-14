// An identifier in a drawer's value, a command's arguments or a path,
// shown whole on as many lines as it needs, never elided (VKCF3Z,
// 2026-11-08): broken at its own marks when it's too long for a line
// alone, and a short last token kept beside the one before rather than
// standing alone. DrawerValue.js lays a value's items with it, and
// tests/query-keys.qml holds both in Qt's own engine. Every width is
// `width(text)`, as the drawer measures what it draws.
.pragma library

// The marks an identifier too long for a line breaks after: these first,
// then a hyphen's, then anywhere, so "--player-operation-mode=pseudo-gui"
// breaks at its "=" before its words, and a desktop entry's
// "com.onepassword.OnePassword" at its dots (VKCF3Z, 2026-11-09). A space
// is one too, its own word's end, for an identifier that holds one.
const STRONG_MARKS = " =/,:.";
// A dot between two digits is a figure's, "6.12" or "0.50", never a
// mark to break after.
const DIGIT = /\d/u;
const WEAK_MARKS = "-";
// A hash, as a store path's or a commit's: a run of 20 or more lower-case
// letters and digits, a digit among them, with no mark to break at. It has
// no words to keep whole, so it's read letter by letter and may break at
// any of them, once the marks around it can't make it fit
// (VKCF3Z, 2026-11-09). No name in the launcher runs that long unbroken.
const HASH = /^(?=[0-9a-z]*\d)[0-9a-z]{20,}$/u;
// The mark a piece ends on, which stays with its last letter.
const ENDING_MARK = /[-=/,:.]$/u;
// A last item this short never stands alone on its line: "--".
const SHORT_TOKEN = 2;
// The space a line's joiner leaves at a line's end, which goes, since Qt's
// engine has no trimEnd.
const TRAILING = /\s+$/u;

// Whether `said` is short enough that it never stands alone on a line.
function short(said) {
    return [...said].length <= SHORT_TOKEN;
}

// Every mark a line may end on, and so none may open with.
const MARKS = `${STRONG_MARKS}${WEAK_MARKS}`;

// Whether the letter at `at` is a figure's dot, between two digits.
function figureDot(letters, at) {
    return letters[at] === "." && DIGIT.test(letters[at - 1] ?? "")
        && DIGIT.test(letters[at + 1] ?? "");
}

// Words cut after each letter `breaksAfter` says, each piece keeping the
// marks it ends on, a run of any of them together: "mode=pseudo-gui" at
// "=" is "mode=" and "pseudo-gui", "fe80::8a2e" at ":" is "fe80::" and
// "8a2e", and "LDFLAGS=-lfoo" is "LDFLAGS=-" and "lfoo", so no piece but
// the first opens with a mark (Y2YBTS, 2026-11-09). After a space the next
// piece opens at once, as a command's next argument "--verbose" does, and
// marks opening a piece stay with what follows them, so "--player-" is one.
function cutWhere(words, breaksAfter) {
    const pieces = [""];
    const letters = [...words];
    let worded = false;
    let broke = false;
    let spaced = false;
    for (const [at, letter] of letters.entries()) {
        const mark = breaksAfter(letters, at);
        if (broke && !(spaced ? letter === " " : mark || MARKS.includes(letter))) {
            pieces.push("");
            broke = false;
            worded = false;
        }
        pieces[pieces.length - 1] += letter;
        if (broke || (mark && worded)) {
            broke = true;
            spaced = letter === " ";
        } else if (!mark) {
            worded = true;
        }
    }
    return pieces.filter(piece => piece !== "");
}

// Words cut after each of `marks`, a figure's dot never among them.
function cutAfter(words, marks) {
    return cutWhere(words, (letters, at) => marks.includes(letters[at])
        && !figureDot(letters, at));
}

// Words cut after their figures' dots, "192." and "168.", so a number
// too long for a line breaks between its figures before inside one
// (Y2YBTS, 2026-11-09).
function cutAtFigures(words) {
    return cutWhere(words, figureDot);
}

// Words cut to their letters, each mark kept with the letter before it
// while the two fit `room`, so a line broken anywhere opens with no mark,
// and a run of marks too long for a line still breaks rather than draw
// past the frame (Y2YBTS, 2026-11-09).
function markedLetters(words, room, width) {
    return [...words].reduce((all, letter) => {
        const before = all[all.length - 1];
        return before !== undefined && MARKS.includes(letter)
            && fitting(`${before}${letter}`, room, width)
            ? all.slice(0, -1).concat([`${before}${letter}`]) : all.concat([letter]);
    }, []);
}

// Whether `said` fits `room`, the spaces it ends on left out, since they
// go where a line ends: "Ploopy " fits as wide as "Ploopy".
function fitting(said, room, width) {
    return width(said.replace(TRAILING, "")) <= room;
}

// The marks inside a title's word it may break after, as a server's dots
// or a path's slashes, where the word is too wide for a line alone.
const TITLE_BREAKS = /([./_])(?=\S)/gu;
// The spaces between a title's words, whatever they are, so a tab or a
// line's end still ends a word, kept where they are.
const SPACES = /(\s+)/u;
const ONLY_SPACES = /^\s*$/u;

// A drawer's title with its break chances: its spaces, and a zero-width
// space after a mark only in a word too wide for `room` alone, so a name
// with spaces breaks at them, "Drawing 2026-11-09" over "10.12.03.svg"
// rather than inside its time (RTOIMQ, 2026-11-09), and one with none
// still breaks at its marks rather than mid-word.
function titleBreaks(title, room, width) {
    return title.split(SPACES).map(word => ONLY_SPACES.test(word)
        || fitting(word, room, width) ? word
        : word.replace(TITLE_BREAKS, "$1\u200b")).join("");
}

// A drawer's title as the lines it's drawn on: one where it fits, else two
// as near the same width as its words allow, never one word alone on the
// second where three or more could share them unless only that fits, and
// past two the second ends at a whole word in `ellipsis` (VKCF3Z's wrap rule 4, 2026-11-11:
// Joel "i hate gross line wraps"). A title with a word too wide for a line
// alone keeps titleBreaks' breaks inside that word instead, since no
// whole-word split can hold it.
function titleLines(title, room, width, ellipsis) {
    const words = title.split(SPACES).filter(word => !ONLY_SPACES.test(word));
    if (words.length === 0 || fitting(words.join(" "), room, width)) {
        return words.length === 0 ? [] : [words.join(" ")];
    }
    if (words.some(word => !fitting(word, room, width))) {
        return [titleBreaks(title, room, width)];
    }
    const shared = balanced(words, room, width, words.length > 2 ? 2 : 1)
        ?? balanced(words, room, width, 1);
    return shared ?? cutToTwo(words, room, width, ellipsis);
}

// The two lines of `words` nearest the same width that both fit `room`,
// the second holding at least `least` of them, or null where none fit.
function balanced(words, room, width, least) {
    let best = null;
    for (let at = 1; at <= words.length - least; at += 1) {
        const lines = [words.slice(0, at).join(" "), words.slice(at).join(" ")];
        const wider = Math.max(width(lines[0]), width(lines[1]));
        if (wider <= room && (best === null || wider < best.wider)) {
            best = { lines, wider };
        }
    }
    return best === null ? null : best.lines;
}

// A title shown whole, as a landing asking Joel something is (VKCF3Z,
// 2026-11-13, c2d01d53): as titleLines lays it where that cuts none of
// it, else on as many lines as its words fill, each as full as it fits,
// so nothing of it is left out.
function titleWhole(title, room, width, ellipsis) {
    const two = titleLines(title, room, width, ellipsis);
    if (two.length < 2 || !two[1].endsWith(ellipsis)) {
        return two;
    }
    const words = title.split(SPACES).filter(word => !ONLY_SPACES.test(word));
    return words.reduce((lines, word) => {
        const last = lines.length - 1;
        const longer = last < 0 ? word : `${lines[last]} ${word}`;
        return last >= 0 && fitting(longer, room, width)
            ? lines.slice(0, last).concat([longer]) : lines.concat([word]);
    }, []);
}

// Words too many for two lines: the first as full as it fits, the second
// the words after it that fit with `ellipsis`, so it ends at a whole word.
function cutToTwo(words, room, width, ellipsis) {
    let at = 1;
    while (at < words.length && fitting(words.slice(0, at + 1).join(" "), room, width)) {
        at += 1;
    }
    let end = at + 1;
    while (end < words.length
        && fitting(`${words.slice(at, end + 1).join(" ")}${ellipsis}`, room, width)) {
        end += 1;
    }
    return [words.slice(0, at).join(" "), `${words.slice(at, end).join(" ")}${ellipsis}`];
}

// Whether a line laid opens with a mark, which only the identifier's
// own first may.
function opensOnMark(line) {
    return MARKS.includes([...line][0] ?? "");
}

// The pieces an identifier breaks into to fit `room`: at its strong marks,
// a piece still too wide at its hyphens, then its figures' dots, then by
// its letters, so nothing of it is ever left out. Each is cut in turn,
// gathered by concat, since Qt's engine has no flatMap.
function piecesOf(words, room, width) {
    const finer = (pieces, cutting) => pieces.reduce((all, piece) => all.concat(
        fitting(piece, room, width) ? [piece] : cutting(piece)), []);
    return [piece => cutAfter(piece, WEAK_MARKS), cutAtFigures,
        piece => markedLetters(piece, room, width)]
        .reduce(finer, cutAfter(words, STRONG_MARKS));
}

// Whether no line laid from `said` after its first opens on a mark, but one
// a space comes before, as a command's next argument "--verbose" may or an
// item after a run's dot (Y2YBTS, 2026-11-09). Each line is found in
// `said` after the one before, its kept spaces and joiners read as they
// are; one that isn't, given way from its middle, opens on its item's own
// start.
function laidWell(said, lines) {
    const whole = String(said);
    let from = 0;
    return lines.every((line, at) => {
        const plain = line.replace(/\u00a0/gu, " ").replace(/\u2060/gu, "");
        const found = whole.indexOf(plain, from);
        if (found < 0) {
            return true;
        }
        from = found + plain.length;
        return at === 0 || !opensOnMark(plain) || whole[found - 1] === " ";
    });
}

// An identifier too wide for a line, whole on as many lines as it needs:
// as many of its pieces to a line as fit, a space a line ends on going.
function wholeLines(words, room, width) {
    return piecesOf(words, room, width).reduce((lines, piece) => {
        const last = lines.length - 1;
        if (last >= 0 && fitting(`${lines[last]}${piece}`, room, width)) {
            lines[last] = `${lines[last]}${piece}`;
        } else {
            lines.push(piece);
        }
        return lines;
    }, []).map(line => line.replace(TRAILING, ""));
}

// The words an identifier is read as, each kept whole on its line: its
// pieces between its marks, a hyphen's among them, so a UUID broken at its
// hyphens or a desktop entry's id at its dots breaks none of its words.
function wordsOf(said) {
    return cutAfter(String(said), STRONG_MARKS)
        .reduce((all, piece) => all.concat(cutAfter(piece, WEAK_MARKS)), [])
        .map(piece => piece.replace(TRAILING, ""))
        .reduce((all, piece) => all.concat(lettersOf(piece)), []);
}

// A piece as the words it's read as: a hash's letters, the mark it ends on
// kept with the last, or the piece whole.
function lettersOf(piece) {
    const mark = ENDING_MARK.test(piece) ? piece.slice(-1) : "";
    const letters = [...piece.slice(0, piece.length - mark.length)];
    if (!HASH.test(letters.join(""))) {
        return [piece];
    }
    letters[letters.length - 1] += mark;
    return letters;
}

// Lines whose last is a short token alone, with the end of the line before
// brought down to join it: the item that ended it, else its words after
// their last strong mark, else after a hyphen, the longest that fits. The
// line it leaves ends on no dot, since a dot goes with a break (VKCF3Z,
// 2026-11-09). They stay as they were where nothing fits beside it.
function joinedDown(lines, between, wide, width) {
    const alone = lines[lines.length - 1];
    const before = lines[lines.length - 2];
    const fits = ending => ending !== "" && ending !== before
        && width(`${ending}${between}${alone}`) <= wide;
    const spaced = before.lastIndexOf(between);
    const strong = cutAfter(before, STRONG_MARKS);
    const weak = cutAfter(before, WEAK_MARKS);
    const tails = [spaced > 0 ? before.slice(spaced + between.length) : "",
        strong.length > 1 ? strong[strong.length - 1] : ""]
        .concat(weak.slice(1).map((_piece, at) => weak.slice(at + 1).join("")));
    const tail = tails.find(fits);
    if (tail === undefined) {
        return lines;
    }
    const mark = between.trim();
    const kept = before.slice(0, before.length - tail.length).replace(TRAILING, "");
    const left = mark !== "" && kept.endsWith(mark)
        ? kept.slice(0, kept.length - mark.length).replace(TRAILING, "") : kept;
    return lines.slice(0, -2).concat([left, `${tail}${between}${alone}`]);
}

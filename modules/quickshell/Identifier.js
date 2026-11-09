// An identifier in a drawer's value, a command's arguments or a path,
// shown whole on as many lines as it needs, never elided (VKCF3Z,
// 2026-11-08): broken at its own marks when it's too long for a line
// alone, and a short last token kept beside the one before rather than
// standing alone. DrawerPage.js lays a value's items with it, and
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

// Words cut after each letter `breaksAfter` says, each piece keeping its
// marks at its end, a run of them together: "mode=pseudo-gui" at "=" is
// "mode=" and "pseudo-gui", "fe80::8a2e" at ":" is "fe80::" and "8a2e",
// so no piece but the first opens with a mark (Y2YBTS, 2026-11-09). Marks
// opening the words stay with what follows them, so "--player-" is one.
function cutWhere(words, breaksAfter) {
    const pieces = [""];
    const letters = [...words];
    let worded = false;
    let broke = false;
    for (const [at, letter] of letters.entries()) {
        const mark = breaksAfter(letters, at);
        if (!mark && broke) {
            pieces.push("");
            broke = false;
        }
        pieces[pieces.length - 1] += letter;
        if (mark && worded) {
            broke = true;
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

// Words cut to their letters, each mark kept with the letter before it, so
// a line broken anywhere still opens with no mark.
function markedLetters(words) {
    return [...words].reduce((all, letter) => all.length > 0 && MARKS.includes(letter)
        ? all.slice(0, -1).concat([`${all[all.length - 1]}${letter}`])
        : all.concat([letter]), []);
}

// Whether `said` fits `room`, the spaces it ends on left out, since they
// go where a line ends: "Ploopy " fits as wide as "Ploopy".
function fitting(said, room, width) {
    return width(said.replace(TRAILING, "")) <= room;
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
    return [piece => cutAfter(piece, WEAK_MARKS), cutAtFigures, markedLetters]
        .reduce(finer, cutAfter(words, STRONG_MARKS));
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

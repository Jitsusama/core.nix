// Vim's n and N, f and F, ; and , on the launcher's rows, and the
// drawer's own search, apart from QueryKeys.js, which reads their keys,
// and Launcher.qml, which moves by them. n and N go through what the one
// find found, as Vim's go through a search's matches; f and F go to a row
// by its name's first letter, as Vim's go to a letter on the line, and ;
// and , go again.
.pragma library
.import "LineEditing.js" as LineEditing

// The field names a Ctrl chord by its letter as typed, and the line's
// editing (LineEditing.js) by the letter's capital, as Vim's help writes it.
const ctrlLetter = /^Ctrl\+[a-z]$/;

// A key typed in the drawer's search, held in how the keys stand as
// `seek`, { pattern, forward }, while it's open, as Vim types / and ? on
// its command line: Return runs it, with the count typed before the /,
// and says what it sought, an empty pattern seeking the last again as
// Vim's does; Escape or Ctrl+C lets it go, as Backspace or Ctrl+H does
// with nothing left, the keys standing as `rested`. Every other key edits
// it as the find line and the command line are edited (LineEditing.js):
// Ctrl+W takes the word before, Ctrl+U all of it, and anything printable
// is typed, a page's keys among them, as a space is.
function seekRead(standing, key, rested) {
    const seek = standing.seek;
    const open = pattern => ({ act: "", count: 0,
        standing: { count: standing.count, typed: [], seek: { pattern, forward: seek.forward } },
        ours: true });
    if (key === "Return" || key === "Enter") {
        return { act: "seek", count: standing.count === "" ? 0 : Number(standing.count),
            sought: seek, standing: rested, ours: true };
    }
    const chord = ctrlLetter.test(key) ? `Ctrl+${key.slice(-1).toUpperCase()}` : key;
    const edited = LineEditing.edit(LineEditing.line(seek.pattern), chord, false);
    if (key === "Escape" || chord === "Ctrl+C" || edited.act === "leave") {
        return { act: "", count: 0, standing: rested, ours: true };
    }
    return open(edited.line.text);
}

// How the keys read while f or F waits for its letter (the standing's
// `letter`, { forward }), or as f or F begins the wait, in normal or
// visual at rest; null for any other key, which the table reads. The
// letter is any key that types, with the count typed before the f; any
// key that types nothing, Escape among them, lets it go, as Vim's does.
// On a prompt's words f means nothing yet (`line`, { mode, words }).
function letterRead(standing, key, line, rested) {
    const waiting = standing.letter;
    if (waiting !== undefined) {
        const count = standing.count === "" ? 0 : Number(standing.count);
        return key.length !== 1 ? { act: "", count: 0, standing: rested, ours: true }
            : { act: waiting.forward ? "toLetter" : "toLetterBack", count, letter: key,
                standing: rested, ours: true };
    }
    const resting = standing.typed.length === 0 && standing.operator === undefined;
    if ((key !== "f" && key !== "F") || !resting
        || (line.mode !== "normal" && line.mode !== "visual")) {
        return null;
    }
    return line.words ? { act: "", count: 0, standing: rested, ours: true, idle: true }
        : { act: "", count: 0, ours: true,
            standing: { count: standing.count, typed: [], letter: { forward: key === "f" } } };
}

// Where n or N goes from the row `at` among the rows the find found, 0 to
// `last`, a count's rows on (none being one), round past the end as
// Vim's wrapscan goes, saying which end it wrapped past: "bottom" or
// "top". With nothing typed there's nothing to go to, as Vim's E35 says,
// and where the find found nothing, E486 (`none`).
function matchTo(going) {
    if (going.typed === "") {
        return { to: going.at, wrapped: "", none: "E35" };
    }
    if (going.last < 0) {
        return { to: going.at, wrapped: "", none: "E486" };
    }
    const rows = going.last + 1;
    const steps = Math.max(going.count, 1);
    const from = Math.max(going.at, 0);
    const ahead = going.forward ? from + steps : from - steps;
    const to = (ahead % rows + rows) % rows;
    const wrapped = ahead > going.last ? "bottom" : ahead < 0 ? "top" : "";
    return { to, wrapped };
}

// The row f or F goes to from `at`: the count's next (none being one)
// whose name begins with the letter sought, down or up, never round past
// an end, as Vim's f stays on its line; -1 where there's none. A small
// letter finds either case and a capital its own, as his smartcase does.
function letterTo(going) {
    const letter = going.sought.letter;
    const folds = letter === letter.toLowerCase();
    const begins = name => {
        const first = String(name ?? "").charAt(0);
        return (folds ? first.toLowerCase() : first) === letter;
    };
    const way = going.sought.forward ? 1 : -1;
    let left = Math.max(going.count, 1);
    for (let at = going.at + way; at >= 0 && at < going.names.length; at += way) {
        if (begins(going.names[at])) {
            left -= 1;
            if (left === 0) {
                return at;
            }
        }
    }
    return -1;
}

// What ; and , seek: the last f or F's letter, the same way or the other;
// an f or F (`act`, with its `letter`) makes itself the last.
function again(last, act, letter) {
    if (act === "toLetter" || act === "toLetterBack") {
        return { letter, forward: act === "toLetter" };
    }
    if (last === null) {
        return null;
    }
    return act === "letterAgainBack" ? { letter: last.letter, forward: !last.forward } : last;
}

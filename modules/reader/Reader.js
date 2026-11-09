// What the reader's provider in the launcher works out (ReaderProgram.qml),
// kept apart from Quickshell's types so tests/reader-program.qml runs it in
// Qt's own engine: the places `reader --places` lists (src/place.rs), read;
// what each file's row says beside it, the heading it was left under, how
// far down the page and when, or that it was read to its end; its drawer's
// plate; what Programs › Reader's drawer says is still to finish; and the
// commands that read a file where it was left or at one of its marks. When
// a file was left is said as the launcher says when of any file
// (FileFacts.js), so a file reads alike in the reader's Recent, mpv's and
// the chooser.
.pragma library
.import "../FileFacts.js" as FileFacts

const MILLISECONDS = 1000;
const PERCENT = 100;
const BETWEEN = " \u00b7 ";

// A share kept, or null where the reader couldn't say one: a place kept
// before the whole page was set, or before it said one at all.
function shareOf(through) {
    return typeof through === "number" && Number.isFinite(through)
        ? Math.min(1, Math.max(0, through)) : null;
}

// A heading kept, or null where the place sits under none.
function headingOf(heading) {
    return typeof heading === "string" && heading.trim() !== "" ? heading : null;
}

// One line reader --places printed, read, or null where it doesn't read as
// a place of a file by its whole path.
function placeOf(line) {
    let kept = null;
    try {
        kept = JSON.parse(line);
    } catch (_malformed) {
        return null;
    }
    const path = String(kept?.file ?? "");
    const left = Number(kept?.left);
    if (!(path.startsWith("/") && Number.isFinite(left))) {
        return null;
    }
    const marks = (Array.isArray(kept.marks) ? kept.marks : [])
        .filter(mark => typeof mark?.letter === "string" && mark.letter.length === 1)
        .map(mark => ({
            letter: mark.letter,
            heading: headingOf(mark.heading),
            through: shareOf(mark.through)
        }))
        .sort((mine, theirs) => mine.letter.localeCompare(theirs.letter));
    return {
        path,
        at: left * MILLISECONDS,
        end: kept.at_end === true,
        heading: headingOf(kept.heading),
        through: shareOf(kept.through),
        marks
    };
}

// The places reader --places printed, a line of JSON each, newest left
// first: each { path, at, end, heading, through, marks }, `at` in
// milliseconds, as the shell keeps a moment, `heading` and `through` null
// where the reader couldn't say them, and `marks` each { letter, heading,
// through }, in the letters' order, as :marks lists them. A line that
// doesn't read so is passed over.
function places(lines) {
    return lines.map(placeOf).filter(one => one !== null);
}

// A share as it's said, in whole percent, rounded down so a page short of
// its end never reads as the whole of it: "40%".
function share(through, said) {
    if (through === null) {
        return "";
    }
    return String(said.share ?? "{share}%")
        .replace("{share}", String(Math.floor(through * PERCENT)));
}

// Where a place or a mark is, as a person reads it: the heading it sits
// under and how far down the page, "Installing \u00b7 40%", either alone
// where the other isn't known, and nothing where neither is.
function where(one, said) {
    return [one.heading ?? "", share(one.through, said)]
        .filter(part => part !== "").join(BETWEEN);
}

// Where a file was left: at its end, said so instead, since the heading
// over the page's last screen says nothing more; else where.
function whereLeft(one, said) {
    return one.end ? String(said.end ?? "") : where(one, said);
}

// The words the reader's rows and drawers say, each thing's as `of` gives
// it, as Words.of does, and a day's as `date(date, format)` says it, as
// Qt.formatDate names days and months in the locale's words: what
// Resume's entry says (`said`), the labels of a plate's facts, and the
// words a moment is said in, as FileFacts.when takes them.
function wordsOf(of, date) {
    const said = of("verb:reader:on").said ?? {};
    const name = id => of(id).name ?? "";
    return {
        said,
        heading: name("reader:left:heading"),
        read: name("reader:left:read"),
        mark: name("reader:left:mark"),
        path: name("reader:left:path"),
        when: {
            ago: { now: said.now, minutes: said.minutes },
            days: { yesterday: said.yesterday, date }
        }
    };
}

// What a file's row says beside its name: where it was left and when,
// "Installing \u00b7 40% \u00b7 14:32", or "read to the end \u00b7
// yesterday"; `now` in milliseconds. Its folder is the drawer's eyebrow's
// instead, as a file mpv left says.
function side(one, now, words) {
    return [whereLeft(one, words.said), FileFacts.when(one.at, now, words.when)]
        .filter(part => part !== "").join(BETWEEN);
}

// A mark's name, its words' name and then its letter, as Vim names one:
// "Mark a", or "Read From Mark a" for the act reading from it. The letter
// is the mark's own, never capitalised, since a and A are two marks.
function markName(name, letter) {
    return `${String(name ?? "")} ${letter}`;
}

// A file's drawer, `shown` its name, its folder and its path as the
// launcher shows them (Files.qml): its eyebrow where it is and then its
// kind, "in ~/src/core.nix \u00b7 markdown", as written, since a folder is
// the world's name; its name; then the heading it was left under, how far
// down it was read, or that it was read to its end, each mark by its
// letter with where it is, and its path last, an identifier, in the Name
// face. When it was left is its side's, so it isn't said again here.
function plate(one, shown, words) {
    const said = words.said;
    const far = one.end ? String(said.whole ?? "") : share(one.through, said);
    const heading = one.heading === null ? [] : [{ label: words.heading, value: one.heading }];
    const read = far === "" ? [] : [{ label: words.read, value: far }];
    const marks = one.marks.map(mark => ({
        label: markName(words.mark, mark.letter),
        value: where(mark, said) || String(said.kept ?? "")
    }));
    return {
        eyebrow: String(said.eyebrow ?? "").replace("{folder}", shown.folder)
            .replace("{kind}", String(said.markdown ?? "")),
        eyebrowNames: true,
        title: shown.name,
        facts: heading.concat(read, marks, [
            { label: words.path, value: shown.path, identifies: true }
        ])
    };
}

// What Programs › Reader's drawer says is still to finish, by its id in the
// launcher's words: how many files were left short of their end, "3
// files", in `said`'s words; nothing while none is.
function held(left, said) {
    const count = left.filter(one => !one.end).length;
    if (count === 0) {
        return [];
    }
    const value = String((count === 1 ? said.file1 : said.files) ?? "")
        .replace("{count}", String(count));
    return [{ id: "reader:to-finish", value }];
}

// What reads a file in a window of its own: where it was left, which the
// reader does by itself, or at the mark kept under `letter`.
function reading(reader, path, letter) {
    return (letter ?? "") === "" ? [reader, path] : [reader, "--at", letter, path];
}

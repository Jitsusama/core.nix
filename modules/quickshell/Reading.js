// How a name too long for its row slides to show its end (Choice.qml): a
// reading motion, steady with soft ends, not an arrival. It holds one
// reading pace from end to end but for a short ramp at each, where it
// gathers speed from rest and settles back to it, so it never starts or
// stops hard and the eye follows one speed (Joel, 2026-10-29: the slide
// was too slow and started and stopped abruptly; VKCF3Z's [e804ce8d]).
// Tested as it is by tests/reading.qml.
.pragma library

// What a cut leaves dangling at its end, a separator or a pause, which
// goes before the ellipsis does.
const DANGLING = /[\s\u00b7,;:]+$/u;

// Words as `fitting` fits them, kept by `key` where `width` is a
// measurer's (Measures.js), whose face is all else the fit depends on;
// fitted anew by any other.
function fitted(width, key, fitting) {
    return width.kept === undefined ? fitting() : width.kept(key, fitting);
}

// The longest each ramp lasts, in seconds. A slide the pace alone would
// take less than two ramps for ramps for half that time, so the pace
// still holds between them.
const RAMP = 0.2;

// The slide over `over` pixels at `pace` pixels a second: how long it
// lasts, in seconds, and the easing curve QML's BezierSpline takes, as
// three pieces: speeding up evenly, the pace held, and slowing evenly.
// Speeding up evenly from rest to the pace covers half the ground the
// pace would in that time, so the slide lasts a ramp longer than the
// pace alone would take, and the pieces meet at the same speed.
function slide(over, pace) {
    if (over <= 0 || pace <= 0) {
        return { seconds: 0, curve: [1 / 3, 1 / 3, 2 / 3, 2 / 3, 1, 1] };
    }
    const steady = over / pace;
    const ramp = Math.min(RAMP, steady / 2);
    const seconds = steady + ramp;
    // The share of the time each ramp takes, and of the ground.
    const time = ramp / seconds;
    const ground = pace * ramp / 2 / over;
    const third = (from, to, part) => from + (to - from) * part;
    return {
        seconds,
        curve: [
            time / 3, 0, 2 * time / 3, ground / 3, time, ground,
            third(time, 1 - time, 1 / 3), third(ground, 1 - ground, 1 / 3),
            third(time, 1 - time, 2 / 3), third(ground, 1 - ground, 2 / 3),
            1 - time, 1 - ground,
            1 - 2 * time / 3, 1 - ground / 3, 1 - time / 3, 1, 1, 1
        ]
    };
}

// What's said between a side's items, as the status line joins its facts.
const ITEMS_APART = " \u00b7 ";

// What stands between names in a run of them.
const NAMES_APART = ", ";

// What a row says at its right, as items, given way whole: the least
// needed first, by each one's `keep`, until what's left fits `room`, and
// the rest said in their own order between dots. The one needed most is
// said whole even where it doesn't fit, so a side is never cut mid-word
// or slid from its left, and only then does the name give up letters
// (VKCF3Z, 2026-11-04: a session's tree goes, then its address, then
// its state, and what it cost stays).
function wholeItems(items, room, width) {
    return keptItems(items, room, width).join(ITEMS_APART);
}

// What a row says at its right: what a second Return does while that
// shows (`landed`), else a noun's kind, whose side then follows its name
// as its detail does (design.md, "A row says what it is"), else how it
// stands. The key an ending act is carried by is the foot's alone, never
// the row's, so the cursor row reads as its neighbours do (VKCF3Z,
// 2026-11-09).
function rightSays(landed, kind, stands) {
    if (landed !== "") {
        return landed;
    }
    return kind !== "" ? kind : stands;
}

// What the cursor row says beyond its name and its side, from what it
// has to say: `state`, its toggle's state where it has one (undefined
// where it hasn't); `once`, asked for the name it was once called by, if
// found by that; `note`, its own note, and `noteInDrawer`, whether its
// drawer says that note now; `answer`, the answer it's part of, which
// says where it came from unless it `missed` (null where it's no
// answer's); and `context`, asked for where a held thing is. `once` and
// `context` are asked only where nothing before them is said. Its note
// follows its name, muted (`note`), never at the right, which says how it
// stands (`stands`), state first, and never gives way to the note
// (VKCF3Z, 2026-11-09 [6918348b]: Wi-Fi reads "Lantern" then "a
// password", with "connected" at its right). A state that only says its
// note again, as a row's does while nothing's happening to it (a network
// joining, a copy clearing), is that note, said once after the name. A
// row with a note says no answer's or held thing's context, as it never
// has. `footer` is what the foot's report has always named the cursor
// row's words: the one of them it said before the note moved.
function cursorSays(row) {
    const note = row.note ?? "";
    const said = row.noteInDrawer === true ? "" : note;
    if (row.state !== undefined && (note === "" || row.state !== note)) {
        return { stands: row.state, note: said, footer: row.state };
    }
    const once = row.once ? row.once() : "";
    if (once !== "") {
        return { stands: once, note: said, footer: once };
    }
    if (note !== "") {
        return { stands: "", note: said, footer: said };
    }
    const stands = answerSays(row.answer ?? null, row.context);
    return { stands, note: "", footer: stands };
}

// Where an answer came from, unless it missed, or where a held thing is.
function answerSays(answer, context) {
    if (answer !== null) {
        return answer.missed === true ? "" : answer.note ?? "";
    }
    return context ? context() : "";
}

// A note as it fits `room` after its row's name: given way by whole items
// from its end, and a note of one item by whole words, never with an
// ellipsis, which no note wears; "" where not even its first word fits.
function noteFitted(note, room, width) {
    return note.includes(ITEMS_APART) ? wholeItemsFromEnd(note, room, width)
        : wordsFromEnd(note, room, width);
}

// The items `wholeItems` keeps, each as it's said, in their own order.
// An item that `stays` is never given way whole, as Now's minutes and how
// many more wait never are (VKCF3Z, bd5cb965).
function keptItems(items, room, width) {
    const kept = items.filter(item => item.said !== "");
    const joined = () => kept.map(item => item.said).join(ITEMS_APART);
    const losable = () => kept.filter(item => item.stays !== true);
    while (kept.length > 1 && losable().length > 0 && width(joined()) > room) {
        const least = losable().reduce((low, item) => (item.keep < low.keep ? item : low));
        kept.splice(kept.indexOf(least), 1);
    }
    return pathsFitted(kept, room, width);
}

// The items kept, as said: one naming its `path`, where they still don't
// fit, gives that way from its middle as a path does (pathTries: "in
// ~/\u2026/Screenshots"), its last folder whole, so a folder kept is never
// lost (VKCF3Z, 2026-11-08), and one naming its `file` gives that way
// from its middle too (fileTries); the longest that fits, or the
// shortest where none does.
function pathsFitted(kept, room, width) {
    const ways = kept.map(item => itemWays(item));
    const most = Math.max(1, ...ways.map(way => way.length));
    let said = [];
    for (let step = 0; step < most; step += 1) {
        said = ways.map(way => way[Math.min(step, way.length - 1)]);
        if (width(said.join(ITEMS_APART)) <= room) {
            break;
        }
    }
    return said;
}

// What a styled text needs written as its entity, so a side's words are
// drawn as they read.
const MARKUP = /[&<>]/g;
const ENTITIES = { "&": "&amp;", "<": "&lt;", ">": "&gt;" };

// Kept items as styled text, their dots in `dotColour` while the words
// keep the side's own ink: the status line's facts are joined so, and an
// agent row's "working \u00b7 $4.20" too, its dot muted on the cursor row
// (VKCF3Z, 2026-11-06). An item `inks` gives a colour, by its place, is
// drawn in that colour, as Reopen Them All's count of folders gone is in
// the alert's (VKCF3Z, 2026-11-07).
function styledItems(said, dotColour, inks) {
    const dot = ` <font color="${dotColour}">\u00b7</font> `;
    return said.map((words, at) => {
        const written = words.replace(MARKUP, mark => ENTITIES[mark]);
        const ink = (inks ?? [])[at] ?? "";
        return ink === "" ? written : `<font color="${ink}">${written}</font>`;
    }).join(dot);
}

// What stands for whatever a crumb leaves out.
const FOLDED = "\u2026";

// The ways a folder's path reads, longest first, each shorter than the
// one before and each of whole folder names: whole, its middle folders
// as one mark, all but its last folder as one, and the mark alone, but
// for the place Joel is in, which keeps its last folder's name.
function pathWays(path, width, here) {
    const [whole, ...candidates] = pathTries(path, here);
    const ways = [whole];
    for (const way of candidates) {
        if (width(way) < width(ways[ways.length - 1])) {
            ways.push(way);
        }
    }
    return ways;
}

// Every way a folder's path may be said, whole first, before any is
// measured: down to its root and last folder, its last folder, and, where
// it isn't `here`, the mark alone.
function pathTries(path, here) {
    const parts = path.split("/").filter((part, at) => part !== "" || at === 0);
    const last = parts[parts.length - 1];
    const tries = [path];
    if (parts.length > 2) {
        tries.push(`${parts[0]}/${FOLDED}/${last}`);
    }
    tries.push(`${FOLDED}/${last}`);
    if (!here) {
        tries.push(FOLDED);
    }
    return tries;
}

// Every way an item may be said, whole first: the ways it gives itself,
// its path's ways, its file's, or only as it is.
function itemWays(item) {
    // An item that knows how it gives way says so, as who a sent mail went
    // to gives way by whole names, "to Ada Lovelace and 4 more" (VKCF3Z,
    // 635ac574).
    if (Array.isArray(item.ways) && item.ways.length > 0) {
        return item.ways;
    }
    if (item.path) {
        return pathTries(item.path, true).map(way => item.said.replace(item.path, way));
    }
    if (item.file) {
        return fileTries(item.file).map(way => item.said.replace(item.file, way));
    }
    // A title gives way from its end by whole words, behind an ellipsis as
    // a cut sentence does (VKCF3Z, bd5cb965: Now's first thing waiting).
    if (item.words === true) {
        return [item.said].concat(wordCuts(item.said));
    }
    return [item.said];
}

// Every way a file's name may be said, whole first and each a letter
// shorter, given way from its middle with its start and its extension
// kept, "second-\u2026take.mp3", as a path keeps its ends: the one item
// telling two rows apart is never lost (VKCF3Z, 2026-11-08).
function fileTries(name) {
    const dot = name.lastIndexOf(".");
    const stem = dot > 0 ? name.slice(0, dot) : name;
    const extension = name.slice(stem.length);
    const tries = [name];
    for (let kept = stem.length - 1; kept >= 1; kept -= 1) {
        const head = Math.ceil(kept / 2);
        const tail = stem.slice(stem.length - (kept - head));
        tries.push(`${stem.slice(0, head)}${FOLDED}${tail}${extension}`);
    }
    return tries;
}

// Every text crumbs and crumbsLeast may measure for `steps`, so each can
// be measured as it's drawn: each step's ways of giving way, the mark, and
// the first step in fewer words.
function crumbTexts(steps) {
    const texts = [FOLDED];
    for (const [at, step] of steps.entries()) {
        texts.push(...step.yields ? pathTries(step.text, at === steps.length - 1) : [step.text]);
    }
    if (steps.length > 1) {
        texts.push(...wordCuts(steps[0].text));
    }
    return Array.from(new Set(texts));
}

// The field's path, its steps given way at whole names so it fits `room`
// with `between` beside each step after the first, never a piece of a
// name (VKCF3Z, 2026-11-05: "~\u2026e"). A folder's path, a step that
// `yields`, gives way first, a folder at a time; then the steps between
// the first and the last fold into one mark, those nearest the first
// going first; then the first says fewer words, or joins the mark. The
// last, where Joel is, is said whole, or a folder's path there down to
// its last folder. Gives what each drawn step
// `said`, and whether each step is said `whole`.
function crumbs(steps, room, width, between) {
    if (steps.length === 0) {
        return { said: [], whole: [] };
    }
    const wide = said => said.reduce((sum, text) => sum + width(text), 0)
        + Math.max(0, said.length - 1) * between;
    const ways = steps.map((step, at) => (step.yields
        ? pathWays(step.text, width, at === steps.length - 1) : [step.text]));
    const deepest = Math.max(...ways.map(way => way.length));
    // Each step as it reads at `depth`, then with `folded` steps after the
    // first folded into one mark, merged with a mark beside it.
    const read = (depth, folded, first) => {
        const texts = ways.map(way => way[Math.min(depth, way.length - 1)]);
        if (first !== undefined) {
            texts[0] = first;
        }
        const said = [];
        const whole = [];
        texts.forEach((text, at) => {
            const fold = text === FOLDED || (at > 0 && at <= folded && at < texts.length - 1);
            whole.push(!fold && text === steps[at].text);
            if (!(fold && said[said.length - 1] === FOLDED)) {
                said.push(fold ? FOLDED : text);
            }
        });
        return { said, whole };
    };
    for (let depth = 0; depth < deepest; depth += 1) {
        const tried = read(depth, 0);
        if (wide(tried.said) <= room) {
            return tried;
        }
    }
    const least = deepest - 1;
    for (let folded = 1; folded < steps.length - 1; folded += 1) {
        const tried = read(least, folded);
        if (wide(tried.said) <= room) {
            return tried;
        }
    }
    // The first step in fewer words, in the room the rest leave it.
    const rest = read(least, steps.length - 2, "");
    const others = rest.said.slice(1);
    const left = room - wide(others) - (others.length > 0 ? between : 0);
    const fewer = steps.length > 1 ? wholeWords(steps[0].text, left, width) : steps[0].text;
    return read(least, steps.length - 2, fewer === "" ? FOLDED : fewer);
}

// How wide the path is at the least it says before its first step gives
// way: every step at its least, and the steps between the first and the
// last folded into one mark. The ask and where Joel is stay whole down to
// this, and what's typed scrolls in what's left (VKCF3Z, 2026-11-06).
function crumbsLeast(steps, width, between) {
    if (steps.length === 0) {
        return 0;
    }
    const least = (step, at) => (step.yields
        ? pathWays(step.text, width, at === steps.length - 1).pop() : step.text);
    const said = [least(steps[0], 0)];
    if (steps.length > 2) {
        said.push(FOLDED);
    }
    if (steps.length > 1) {
        said.push(least(steps[steps.length - 1], steps.length - 1));
    }
    return said.reduce((sum, text) => sum + width(text), 0) + (said.length - 1) * between;
}

// Words too long for their room, given way at the last whole word that
// fits, an ellipsis after it, so what's said is never cut mid-word
// (docs/design.md, Collapsing; VKCF3Z, 2026-11-02: "Open-Meteo didn't
// a\u2026"). `width` measures a text in the face it's drawn in. Where not
// even the first word fits, nothing is said, rather than a piece of one.
function wholeWords(words, room, width) {
    return fitted(width, `whole\u241f${room}\u241f${words}`,
        () => wholeWordsFitted(words, room, width));
}
function wholeWordsFitted(words, room, width) {
    if (width(words) <= room) {
        return words;
    }
    for (const cut of wordCuts(words)) {
        if (width(cut) <= room) {
            return cut;
        }
    }
    return "";
}

// Words said as items between dots, given way by whole items from their
// end, never inside one, so a note reads "Example ISP \u00b7 370 Mb/s down",
// or "Example ISP" alone, never "11.9 Mb/s\u2026" (VKCF3Z, 2026-11-07).
// Where it's several items, its first, where it alone doesn't fit, isn't
// said at all, since no ellipsis is drawn on a note, never "found
// by\u2026" (VKCF3Z, 2026-11-08); words that are one item, a sentence,
// give way at a whole word, as any words do. `apart` is what parts the
// items, the dot unless a run of names is given way by its commas.
function wholeItemsFromEnd(words, room, width, apart) {
    const between = apart ?? ITEMS_APART;
    return fitted(width, `items${between}\u241f${room}\u241f${words}`,
        () => wholeItemsFitted(words, room, width, between));
}
function wholeItemsFitted(words, room, width, between) {
    const items = words.split(between);
    if (items.length === 1) {
        return wholeWords(words, room, width);
    }
    while (items.length > 0 && width(items.join(between)) > room) {
        items.pop();
    }
    return items.join(between);
}

// A drawer's eyebrow as it's drawn in `room`: one given as items
// (`eyebrowItems`) gives way by whole items, those it keeps least first,
// a path from its middle; any other gives way as a note does, by whole
// items from its end. Neither is ever cut inside a word, as an ellipsis
// at its end cut "IN ~/PICTURES/SCREENS\u2026" (VKCF3Z, 2026-11-08).
function eyebrowOf(drawer, room, width) {
    const items = drawer?.eyebrowItems ?? [];
    return items.length > 0 ? wholeItems(items, room, width)
        : wholeItemsFromEnd(String(drawer?.eyebrow ?? ""), room, width);
}

// What a row holds, as a run of names, given way by whole names from its
// end and ending on the last that fits, with no ellipsis after it, so home
// reads "kitty, 1Password, btop++, Calendar", never ", \u2026" or
// "Calenda\u2026" (VKCF3Z, 2026-11-08: "no ellipsis anywhere"). A name
// that doesn't fit alone isn't said; words of items between dots give
// way as a note's items do.
function wholeNamesFromEnd(words, room, width) {
    if (words.includes(NAMES_APART)) {
        const kept = wholeItemsFromEnd(words, room, width, NAMES_APART);
        if (kept.includes(NAMES_APART)) {
            return kept;
        }
    }
    const one = words.split(NAMES_APART)[0];
    if (one.includes(ITEMS_APART)) {
        return wholeItemsFromEnd(one, room, width);
    }
    return width(one) <= room ? one : "";
}

// A note given way by whole words from its end, with no ellipsis, since
// none is drawn on a note: a notice's line reads "line 7 of the", never
// "line 7 of the de" (VKCF3Z, 2026-11-08). Where not even its first
// word fits, nothing is said.
function wordsFromEnd(words, room, width) {
    return fitted(width, `ends\u241f${room}\u241f${words}`,
        () => wordsFromEndFitted(words, room, width));
}
function wordsFromEndFitted(words, room, width) {
    if (width(words) <= room) {
        return words;
    }
    const parts = words.split(" ");
    for (let count = parts.length - 1; count > 0; count -= 1) {
        const kept = parts.slice(0, count).join(" ").replace(DANGLING, "");
        if (width(kept) <= room) {
            return kept;
        }
    }
    return "";
}

// Words cut at each whole word but the last, longest first, an ellipsis
// after each.
function wordCuts(words) {
    const parts = words.split(" ");
    const cuts = [];
    for (let count = parts.length - 1; count > 0; count -= 1) {
        cuts.push(`${parts.slice(0, count).join(" ").replace(DANGLING, "")}\u2026`);
    }
    return cuts;
}

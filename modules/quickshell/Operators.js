// What the launcher's operators do to its rows, as Vim's d and y do to a
// buffer's lines (QueryKeys.js reads their keys): which of a row's actions
// d does and which y does, what a copy copies, and when d asks before it
// does what can't be put back. Read apart from the launcher so
// tests/query-keys.qml can hold it in Qt's own engine.
.pragma library

// The operators, by their key, and the act each does over the rows it
// takes.
const operators = { d: "ending", y: "copy" };

// The moves an operator takes as its motion, each linewise on the rows,
// as Vim's j k gg G H M L are on lines. Ctrl+D, Ctrl+U and the screens
// scroll rather than move, so Vim takes none of them as a motion.
const motions = ["next", "previous", "first", "last", "viewTop", "viewMiddle", "viewBottom",
    "paragraphOn", "paragraphBack"];

// The text objects an operator takes in a motion's place, as dip does.
const objects = ["inParagraph", "aParagraph"];

// Whether a key is taken with the visual operator before it, `after`, its
// key and when it was pressed: the same operator's key within `wait` ms,
// as the spec's V j j d d, whose second d would otherwise wait for a
// motion on the rows left.
function takenWith(after, key, now, wait) {
    return after !== null && key === after.key && key in operators && now - after.at < wait;
}

// Two counts as one, as Vim multiplies the count before an operator and
// the one after it: 2d3j goes six rows. Either alone is itself, and
// neither is no count.
function multiplied(before, after) {
    if (before === "" && after === "") {
        return 0;
    }
    return Number(before === "" ? 1 : before) * Number(after === "" ? 1 : after);
}

// A key while an operator waits: the operator's own key again does it
// over the row and those below, as dd does; a motion does it as far as
// that goes, as dj does, with the counts multiplied; a sequence's first
// key (gg's g) waits for the rest; anything else lets it go. It waits as
// long as it takes for its motion, as Vim's does. What a mode's table
// makes of the keys typed (`matching`, given them and the mode) and how
// the keys stand at rest (`rested`) are QueryKeys.js's, which reads every
// other key.
function operated(standing, key, matching, rested) {
    const { operator } = standing;
    const act = operators[operator.key];
    const count = multiplied(operator.count, standing.count);
    if (standing.typed.length === 0 && key === operator.key) {
        return { act, count, motion: null, standing: rested, ours: true };
    }
    // Another operator's key lets this one go, as dy does in Vim. A text
    // object is read first, so the i of ip waits rather than inserting.
    const typed = standing.typed.concat([key]);
    const object = matching(typed, "object");
    const matched = key in operators ? { act: "", begun: false }
        : object.act !== "" || object.begun ? object : matching(typed, "normal");
    if (motions.includes(matched.act) || objects.includes(matched.act)) {
        return { act, count: 0, motion: { act: matched.act, count }, standing: rested,
            ours: true };
    }
    if (matched.act === "" && matched.begun) {
        return { act: "", count: 0, standing: { count: standing.count, typed, operator },
            ours: true };
    }
    return { act: "", count: 0, standing: rested, ours: true };
}

// The rows an operator takes, from where the rows stand (as moved takes
// them): over the row and count - 1 below it with no motion, as dd
// and 3dd, to the last at most, as Neovim's 9dd near the end; or from the
// row to wherever the motion goes, whichever way, every row between
// included, as a linewise motion takes lines. No rows span nothing.
// A paragraph's text object takes its run and count - 1 after it. } and {
// are exclusive, as Vim's are, leaving out the later end: d} the row it
// lands on, unless the run it began in ends the list there, and d{ the
// cursor's own.
function spanned(count, motion, place) {
    if (place.last < place.highest) {
        return { from: 0, to: -1 };
    }
    if (motion === null) {
        return { from: place.at, to: Math.min(place.last, place.at + Math.max(1, count) - 1) };
    }
    if (motion.act === "inParagraph" || motion.act === "aParagraph") {
        return paragraph(motion.count, place);
    }
    const to = moved(motion.act, motion.count, place);
    const exclusive = (motion.act === "paragraphOn" || motion.act === "paragraphBack")
        && to !== place.at && !(motion.act === "paragraphOn" && sameRun(place, to));
    if (exclusive) {
        return to > place.at ? { from: place.at, to: to - 1 } : { from: to, to: place.at - 1 };
    }
    return { from: Math.min(place.at, to), to: Math.max(place.at, to) };
}

// Where a move takes the selection, from where the rows stand: the row
// selected, the highest it may go (-1 where an answer's headline sits
// above the rows) and the last, the first and last rows wholly in view,
// how many rows a screen holds, and the run a row is in (runOf, from
// runAt), a place without one being one run. A count moves that many
// times, or, with gg and G, goes to that row, counted from 1, as a line's
// number is. H and L count from the view's top and bottom. Ctrl+D and
// Ctrl+U move half the rows in view, or a count's rows, as Vim's 'scroll'
// takes it. Every move stops at the first and last rows, never going
// round. The moves' keys are QueryKeys.js's.
function moved(act, count, place) {
    const times = Math.max(1, count);
    const within = at => Math.max(place.highest, Math.min(at, place.last));
    const half = count > 0 ? count : Math.max(1, Math.floor((place.bottom - place.top + 1) / 2));
    switch (act) {
    case "next":
        return within(place.at + times);
    case "previous":
        return within(place.at - times);
    case "first":
        return count > 0 ? within(count - 1) : place.highest;
    case "last":
        return count > 0 ? within(count - 1) : within(place.last);
    case "viewTop":
        return within(Math.min(place.top + times - 1, place.bottom));
    case "viewMiddle":
        return within(place.top + Math.floor((place.bottom - place.top) / 2));
    case "viewBottom":
        return within(Math.max(place.bottom - times + 1, place.top));
    case "halfDown":
        return within(place.at + half);
    case "halfUp":
        return within(place.at - half);
    case "pageDown":
        return within(place.at + times * place.screen);
    case "pageUp":
        return within(place.at - times * place.screen);
    case "paragraphOn":
    case "paragraphBack":
        return paragraphed(act === "paragraphOn", times, place);
    default:
        return place.at;
    }
}

// The run a row is in, by the place's runOf, or the whole list.
function runIn(place, at) {
    return place.runOf ? place.runOf(at) : { from: place.highest, to: place.last };
}

// Where } and { go, `times` runs on or back: to the next run's first row,
// or the last row past the last run; and to this run's first row, or the
// run before's from there.
function paragraphed(on, times, place) {
    let at = place.at;
    for (let time = 0; time < times; time += 1) {
        const run = runIn(place, at);
        if (on) {
            at = Math.min(run.to + 1, place.last);
        } else {
            at = at > run.from ? run.from
                : runIn(place, Math.max(place.highest, at - 1)).from;
        }
    }
    return at;
}

// The run the cursor's in and count - 1 after it, to the last row.
function paragraph(count, place) {
    const first = runIn(place, place.at);
    let last = first;
    for (let more = 1; more < Math.max(1, count) && last.to < place.last; more += 1) {
        last = runIn(place, last.to + 1);
    }
    return { from: first.from, to: last.to };
}

// Whether a row is in the cursor's run, as } landing on the last row of
// the last run is, which d} takes.
function sameRun(place, at) {
    const run = runIn(place, place.at);
    return at >= run.from && at <= run.to;
}

// The run of rows a row is in: the rows round it of the same `of`, its kind
// or a notice's sender, none set apart by a gap above it (VKCF3Z,
// 2026-11-05), as Vim's paragraph is the lines between blank ones.
function runAt(rows, at, of) {
    const key = of(rows[at]);
    let from = at;
    while (from > 0 && !rows[from].gap && of(rows[from - 1]) === key) {
        from -= 1;
    }
    let to = at;
    while (to < rows.length - 1 && !rows[to + 1].gap && of(rows[to + 1]) === key) {
        to += 1;
    }
    return { from, to };
}

// A row's ending action, the one that ends what it is, as a window's Close
// or a notice's Forget It: the key language's act ending (modules/keys/
// language.toml), which words.json names on each such action as "act".
// The row itself, where it's one, as an action listed in a step into a
// window is; else the first of its actions that is; else none. A verb of
// close alone isn't one, since an answer's Return that only closes the
// launcher says close too.
function endingOf(row, actions) {
    const ends = one => one.act === "ending" && doing(one) !== null;
    if (ends(row)) {
        return row;
    }
    return actions.find(ends) ?? null;
}

// What d runs for an ending action: its own run, or, for one that runs
// nothing itself and steps in to the row that does, as a program's Quit
// asks once that way, that row's, since d's own ask stands for it. Null
// for one with nothing to run.
function doing(action) {
    if (typeof action.run === "function") {
        return action.run;
    }
    const inside = typeof action.targets === "function" ? action.targets() : [];
    return inside.find(one => typeof one.run === "function")?.run ?? null;
}

// A row's copy: the row itself, where Return copies it, as an answer's
// rows and a figure do; else the first of its actions that copies, as a
// file's Copy the Path or a window's Copy Title; else none. `copies` says
// whether a row copies, as the launcher reads its verb.
function copyOf(row, actions, copies) {
    if (copies(row)) {
        return row;
    }
    return actions.find(copies) ?? null;
}

// What a copy copies, which its row says as `copies`, a text or a function
// giving one as it's asked, so a live figure copies what it shows: "" for
// one that says nothing.
function textOf(action) {
    const copies = action.copies;
    const text = typeof copies === "function" ? copies() : copies;
    return typeof text === "string" ? text : "";
}

// What several copies put on the clipboard: each a line, in the rows'
// order, as a linewise yank holds lines, those copying nothing left out.
function copied(actions) {
    return actions.map(textOf).filter(text => text !== "").join("\n");
}

// The rows a span takes, each with its place, the row at -1 being an
// answer's headline above them, where there's one. A row whose thing has
// gone, which stays in its place until a key, is left out, since there's
// nothing there to end or copy.
function taken(rows, span, headline) {
    const these = [];
    for (let at = span.from; at <= span.to; at++) {
        const row = at === -1 ? headline : rows[at];
        if (row && !row.gone) {
            these.push({ at, row });
        }
    }
    return these;
}

// Each row's ending action over the rows taken, with its place, rows with
// none left out: what d does there.
function endings(rows, actionsOf) {
    return rows
        .map(({ at, row }) => ({ at, action: endingOf(row, actionsOf(row)) }))
        .filter(ending => ending.action !== null);
}

// Each row's copy over the rows taken, with its place, rows with none
// left out: what y does there.
function copiesIn(rows, actionsOf, copies) {
    return rows
        .map(({ at, row }) => ({ at, action: copyOf(row, actionsOf(row), copies) }))
        .filter(copy => copy.action !== null);
}

// Whether d asks once before it does these, as the key language says by
// each action's command (`commands`, Options.commands): where any can't be
// undone, unless it's guarded, as a window's Close is, whose program keeps
// its own work, and it's the only one; never for an action that says how
// it's put back (`undo`), for u to take back. What the language calls
// undoable asks too until its row says how, since nothing could bring it
// back; one the language doesn't name asks, as nothing says it can be.
function asks(actions, commands) {
    const alone = actions.length === 1;
    return actions.some(action => {
        if (typeof action.undo === "function") {
            return false;
        }
        const flags = commands[action.command ?? ""] ?? {};
        return !(flags.guarded === true && alone);
    });
}

// An operator waiting for what it acts on, from how the keys stand: the
// keys typed since its count, as 3d or d2 or dg, and the count its rows
// would be as the two multiply, 0 where none was typed. Null with none
// waiting.
function pending(standing) {
    const { operator } = standing;
    if (operator === undefined) {
        return null;
    }
    return {
        typed: operator.count + operator.key + standing.count + standing.typed.join(""),
        count: multiplied(operator.count, standing.count)
    };
}

// What a pending operator will hit, as the mode word's room says it: the
// keys typed, then the verb and the cursor row's name, or the count's
// rows where it takes more than one, as "d · disconnect Home-5G" and
// "3d · close 3"; where the row has nothing it takes, the operator's own
// word for that. `hit` is { typed, act, verb, name, count }, its verb
// null for nothing.
function pendingSays(hit, words) {
    const nothing = hit.act === "copy" ? words.noCopy : words.nothing;
    const what = hit.verb === null ? nothing
        : (hit.count > 1 ? words.hits : words.hit)
            .replace("{verb}", hit.verb)
            .replace("{name}", hit.name)
            .replace("{count}", String(hit.count));
    return words.pending.replace("{typed}", hit.typed).replace("{what}", what ?? "");
}

// The rows visual line takes, from where it began to the cursor, either
// way; none before it begins, its start being -1.
function selection(from, at) {
    if (from < 0) {
        return { from: 0, to: -1 };
    }
    return { from: Math.min(from, at), to: Math.max(from, at) };
}

// Where visual line's start goes as the rows change under it: onto the
// row it began on, found by what it is (`id`, as `identity` reads a row),
// else where it was, within the rows; -1, none begun, stays.
function heldFrom(rows, id, from, identity) {
    if (from < 0) {
        return from;
    }
    const found = id === "" ? -1 : rows.findIndex(row => identity(row) === id);
    return found >= 0 ? found : Math.max(0, Math.min(from, rows.length - 1));
}

// What the foot says Return does over a selection: open its rows, by
// their count, as "open 3".
// Visual's mode word, counting the rows chosen (`span`, as selection gives
// it), as the spec's "visual line · 3".
function visualSaid(mode, span, words) {
    return (words.counted ?? "{mode}").replace("{mode}", mode)
        .replace("{count}", String(span.to - span.from + 1));
}

function opensSaid(count, words) {
    return (words.opens ?? "").replace("{count}", String(count));
}

// What's said once Return has opened a selection's rows where it couldn't
// open them all, "2 of 3 open", and nothing where it opened every one.
function openedSaid(open, count, words) {
    if (open === count) {
        return "";
    }
    return (words.opened ?? "").replace("{open}", String(open))
        .replace("{count}", String(count));
}


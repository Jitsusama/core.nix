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
const motions = ["next", "previous", "first", "last", "viewTop", "viewMiddle", "viewBottom"];

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
// long as it takes for its motion, as Vim's does. What normal's table
// makes of the keys typed (`matching`) and how the keys stand at rest
// (`rested`) are QueryKeys.js's, which reads every other key.
function operated(standing, key, matching, rested) {
    const { operator } = standing;
    const act = operators[operator.key];
    const count = multiplied(operator.count, standing.count);
    if (standing.typed.length === 0 && key === operator.key) {
        return { act, count, motion: null, standing: rested, ours: true };
    }
    // Another operator's key lets this one go, as dy does in Vim.
    const typed = standing.typed.concat([key]);
    const matched = key in operators ? { act: "", begun: false } : matching(typed);
    if (motions.includes(matched.act)) {
        return { act, count: 0, motion: { act: matched.act, count }, standing: rested,
            ours: true };
    }
    if (matched.act === "" && matched.begun) {
        return { act: "", count: 0, standing: { count: standing.count, typed, operator },
            ours: true };
    }
    return { act: "", count: 0, standing: rested, ours: true };
}

// Where a move takes the selection, from where the rows stand: the row
// selected, the highest it may go (-1 where an answer's headline sits
// above the rows) and the last, the first and last rows wholly in view,
// and how many rows a screen holds. A count moves that many times, or,
// with gg and G, goes to that row, counted from 1, as a line's number is.
// H and L count from the view's top and bottom. Ctrl+D and Ctrl+U move
// half the rows in view, or a count's rows, as Vim's 'scroll' takes it.
// Every move stops at the first and last rows, never going round. The
// moves' keys are QueryKeys.js's; it's here beside the operators that
// take a move as their motion, QueryKeys.js having outgrown its room.
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
    default:
        return place.at;
    }
}

// The rows an operator takes, from where the rows stand (as moved takes
// them): over the row and count - 1 below it with no motion, as dd
// and 3dd, to the last at most, as Neovim's 9dd near the end; or from the
// row to wherever the motion goes, whichever way, every row between
// included, as a linewise motion takes lines. No rows span nothing.
function spanned(count, motion, place) {
    if (place.last < place.highest) {
        return { from: 0, to: -1 };
    }
    if (motion === null) {
        return { from: place.at, to: Math.min(place.last, place.at + Math.max(1, count) - 1) };
    }
    const to = moved(motion.act, motion.count, place);
    return { from: Math.min(place.at, to), to: Math.max(place.at, to) };
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

// A word that's only a mark, as the dot between the keys and the verb.
const LONE_MARK = /^[\u00b7\u2022:,.-]*$/;

// Words cut at a whole word to what fits, as `fits` measures them, never
// ending on a lone mark; the first word stands, however wide.
function wholeWords(text, fits) {
    const words = text.split(" ");
    const lone = word => LONE_MARK.test(word);
    while (words.length > 1 && (!fits(words.join(" ")) || lone(words[words.length - 1]))) {
        words.pop();
    }
    return words.join(" ");
}

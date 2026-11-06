// The held notices as Vim's quickfix list (VKCF3Z's vi-paradigm spec,
// Notices): a line a card, the newest on line 1, a cursor on one, and what
// the keys (NoticeKeys.js) do to them worked out apart from drawing them
// (Notifications.qml): where a motion lands, which cards an operator or a
// range takes, the view kept around the cursor, the gutter's numbers, the
// changes u takes back, and the command line's menu. Apart from the shell,
// so tests/notice-list.qml can hold it in Qt's own engine.
.pragma library

// How many lines show at once, and how far the view keeps from the cursor,
// as Joel's scrolloff of 4, which Vim holds to half the lines in view.
const room = 8;
const scrolloff = 4;

// The cards' ids, the newest first, by when each arrived.
function ordered(ids, arrived) {
    return ids.slice().sort((a, b) => (arrived[b] ?? 0) - (arrived[a] ?? 0));
}

// The first line in view, from the one before, so the cursor stays as far
// from the view's edges as scrolloff asks while there are lines beyond.
function viewTop(top, cursor, count, shown) {
    const lines = shown ?? room;
    const off = Math.min(scrolloff, Math.floor((lines - 1) / 2));
    const last = Math.max(count - lines, 0);
    let first = Math.min(Math.max(top, 0), last);
    if (cursor - off < first) {
        first = cursor - off;
    }
    if (cursor + off > first + lines - 1) {
        first = cursor + off - lines + 1;
    }
    return Math.max(0, Math.min(first, last));
}

// The view scrolled as Vim scrolls it, and the cursor kept in it by
// scrolloff: Ctrl+E and Ctrl+Y move the view a line, the wheel three, each
// times the count, the cursor going only as far as it must; zz, zt and zb
// put the cursor's line, or the counted one, in the middle, at the top or
// at the foot. It gives the view's first line and the cursor's.
function scrolled(motion, at, count, list) {
    const total = list.count;
    const lines = Math.min(list.shown ?? room, total);
    const off = Math.min(scrolloff, Math.floor((lines - 1) / 2));
    const lastTop = Math.max(total - lines, 0);
    const clampTop = wanted => Math.max(0, Math.min(wanted, lastTop));
    const by = Math.max(count, 1);
    const line = motion.startsWith("view") && count > 0
        ? Math.max(0, Math.min(count - 1, total - 1)) : at;
    const tops = {
        lineDown: (list.top ?? 0) + by,
        lineUp: (list.top ?? 0) - by,
        viewCentre: line - Math.floor((lines - 1) / 2),
        viewAtTop: line - off,
        viewAtBottom: line - lines + 1 + off
    };
    const top = clampTop(tops[motion] ?? list.top ?? 0);
    const lowest = top > 0 ? top + off : 0;
    const highest = top + lines - 1 < total - 1 ? top + lines - 1 - off : total - 1;
    return { top, cursor: Math.max(lowest, Math.min(line, highest)) };
}

// A line's number in the gutter, as Joel's Neovim sets number and
// relativenumber: the cursor line's own, from 1, and the rest how far from
// it, or every line its own once relative numbers are off.
function number(at, cursor, relative) {
    return relative && at !== cursor ? `${Math.abs(at - cursor)}` : `${at + 1}`;
}

// The run of lines from one sender around a line, as a list's paragraph:
// its first and last.
function run(senders, at) {
    let from = at;
    let to = at;
    while (from > 0 && senders[from - 1] === senders[at]) {
        from -= 1;
    }
    while (to < senders.length - 1 && senders[to + 1] === senders[at]) {
        to += 1;
    }
    return { from, to };
}

// Where a motion lands from a line, with its count (0 for none), in a list
// of `count` lines whose view starts at `top`, each line's sender given for
// { and }. gg and G go to the line a count numbers, as Vim's do.
function moved(motion, at, count, list) {
    const by = Math.max(count, 1);
    const total = list.count;
    const last = Math.max(total - 1, 0);
    const lines = Math.min(room, total);
    const top = list.top ?? 0;
    const clamp = line => Math.max(0, Math.min(line, last));
    switch (motion) {
    case "down":
        return clamp(at + by);
    case "up":
        return clamp(at - by);
    case "first":
        return clamp(count > 0 ? count - 1 : 0);
    case "last":
        return clamp(count > 0 ? count - 1 : last);
    case "viewTop":
        return clamp(top + (count > 0 ? count - 1 : 0));
    case "viewMiddle":
        return clamp(top + Math.floor((lines - 1) / 2));
    case "viewBottom":
        return clamp(top + lines - 1 - (count > 0 ? count - 1 : 0));
    case "runOn":
    case "runBack":
        return runs(motion, at, by, list.senders ?? []);
    case "halfDown":
    case "halfUp":
        return clamp(at + (motion === "halfDown" ? 1 : -1) * Math.floor(room / 2) * by);
    case "viewDown":
    case "viewUp":
        return clamp(at + (motion === "viewDown" ? 1 : -1) * room * by);
    default:
        return at;
    }
}

// Where } and { land: the first line of the next sender's run, or of this
// or the last one's, as many runs as counted.
function runs(motion, at, by, senders) {
    let line = at;
    for (let step = 0; step < by; step++) {
        if (motion === "runOn") {
            line = Math.min(run(senders, line).to + 1, Math.max(senders.length - 1, 0));
        } else {
            const here = run(senders, line).from;
            line = here < line ? here : run(senders, Math.max(here - 1, 0)).from;
        }
    }
    return line;
}

// The lines an operator takes from the cursor, first and last: the
// operator again takes this and those after it, as many as counted; a
// motion, everything from here to where it lands, as Vim's linewise d
// does; an object, the sender's run.
function taken(motion, at, count, list) {
    const last = Math.max(list.count - 1, 0);
    if (motion === "line") {
        return { from: at, to: Math.min(at + Math.max(count, 1) - 1, last) };
    }
    if (motion === "inRun") {
        return run(list.senders ?? [], at);
    }
    const to = moved(motion, at, count, list);
    return { from: Math.min(at, to), to: Math.max(at, to) };
}

// The lines chosen in visual, between where it began and the cursor.
function chosen(anchor, cursor) {
    return { from: Math.min(anchor, cursor), to: Math.max(anchor, cursor) };
}

// The ids of lines from first to last.
function between(lines, span) {
    return lines.slice(span.from, span.to + 1);
}

// The ids that are away: what every change still standing took.
function away(standing) {
    return standing.reduce((all, change) => all.concat(change.ids), []);
}

// The changes u and Ctrl+R move between: what's done, as a list each of
// { ids, kind }, the last last, and what was taken back.
function changes() {
    return { done: [], undone: [] };
}

function changed(history, change) {
    return { done: history.done.concat([change]), undone: [] };
}

// u takes back the last change, as many as counted; Ctrl+R does again what
// u took back. Each says what moved, or null where there was nothing.
function undo(history, count) {
    const by = Math.min(Math.max(count, 1), history.done.length);
    if (by === 0) {
        return { history, moved: null };
    }
    const back = history.done.slice(-by);
    return {
        history: { done: history.done.slice(0, -by), undone: history.undone.concat(back.reverse()) },
        moved: back
    };
}

function redo(history, count) {
    const by = Math.min(Math.max(count, 1), history.undone.length);
    if (by === 0) {
        return { history, moved: null };
    }
    const again = history.undone.slice(-by).reverse();
    return {
        history: { done: history.done.concat(again), undone: history.undone.slice(0, -by) },
        moved: again
    };
}

// How many lines words take `wide` across, broken at their spaces as
// Qt's wrap breaks them, `measure` giving a run's width; a word too wide
// alone takes the lines it is broken over. Laying stops once past `most`
// lines, so a long mail costs only what a card can show.
function laid(words, wide, measure, most = Number.POSITIVE_INFINITY) {
    const linesOf = part => (part === "" ? 0 : Math.max(Math.ceil(measure(part) / wide), 1));
    let lines = 0;
    let line = "";
    for (const word of words) {
        const longer = line === "" ? word : `${line} ${word}`;
        if (line !== "" && measure(longer) <= wide) {
            line = longer;
        } else {
            lines += linesOf(line);
            line = word;
            if (lines > most) {
                return lines;
            }
        }
    }
    return lines + linesOf(line);
}

const spaces = /\s+/;

// What a card's words show in `lines` lines `wide` across: null when they
// all fit, kept as they are; else the whole words that fit, the last one
// ending in an ellipsis, never cut mid-word nor a line cut in half
// (VKCF3Z, 2026-11-05).
function wholeWords(text, lines, wide, measure) {
    const words = text.split(spaces).filter(word => word !== "");
    if (laid(words, wide, measure, lines) <= lines) {
        return null;
    }
    const ending = kept => words.slice(0, kept - 1).concat(`${words[kept - 1]}\u2026`);
    // The most words kept, found by halving, since keeping more never
    // takes fewer lines; each try lays at most what the lines hold, so the
    // cursor card measures by its room, not by the mail's length (MVL7F9,
    // Y2YBTS, 2026-11-06).
    let fits = 0;
    let over = words.length;
    while (over - fits > 1) {
        const kept = Math.floor((fits + over) / 2);
        if (laid(ending(kept), wide, measure, lines) <= lines) {
            fits = kept;
        } else {
            over = kept;
        }
    }
    return fits === 0 ? `${words[0] ?? ""}\u2026` : ending(fits).join(" ");
}

const ranged = /^(%|[.$0-9'<>,;+-]*)/;

// The menu for what's typed on the command line: a range typed first is
// kept, and the lines are those whose command starts with what follows it,
// then those with a word of the act's name that does. Each line is { command, short,
// name, key }.
function menu(entries, text) {
    const prefix = text.match(ranged)?.[0] ?? "";
    const typed = text.slice(prefix.length).trim().toLowerCase();
    if (typed === "") {
        return { prefix, shown: entries };
    }
    const word = typed.split(" ")[0];
    const starts = entries.filter(one => one.command.startsWith(word));
    const holds = entries.filter(one => !starts.includes(one)
        && one.name.toLowerCase().split(" ").some(name => name.startsWith(word)));
    return { prefix, shown: starts.concat(holds) };
}

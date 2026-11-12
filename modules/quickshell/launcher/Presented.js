// How long what the launcher showed took to reach niri's screen, as its
// journal says it as it closes: keys from their press, and a daemon's
// changes from the line that made them, each place on its own (Y2YBTS,
// 2026-11-11), so a slow update path is seen against Joel's 50 ms bar as
// keys are, and a drawer from the key that selected its row (Joel,
// 2026-11-11 20:28: "the drawer in the launcher and its time to appear
// needs to be kept to the same performance requirements as everything
// else"). Apart from the shell, so tests/pacing.qml holds it in Qt's own
// engine.
.pragma library

const bar = 50;

// How long each of a set took, from `from` to `at`: how many, the median,
// the slowest, the p99 by nearest rank and how many within the bar. The
// daily loop reads a day's p99 from these lines, since one slow key in a
// hundred is what a median hides and the slowest overstates (7QEY1S,
// 2026-11-11).
function took(times) {
    const ms = times.map(one => Math.round(one.at - one.from)).sort((a, b) => a - b);
    return {
        count: ms.length,
        median: median(ms),
        slowest: ms[ms.length - 1],
        p99: ms[Math.max(0, Math.ceil(ms.length * 0.99) - 1)],
        within: ms.filter(each => each <= bar).length
    };
}

// How long a set took, as every line here says it after what it timed:
// `verb` as it came, presented or landed, and `from` what it's timed from.
function tookSaid(one, from, verb) {
    return `${verb ?? "presented"} a median ${one.median} ms from ${from}, `
        + `the slowest ${one.slowest} ms, `
        + `the p99 ${one.p99} ms, ${one.within} within ${bar} ms`;
}

function median(ms) {
    const sorted = ms.slice().sort((a, b) => a - b);
    return sorted[(sorted.length - 1) >> 1];
}

// What the journal says of the keys presented, each `{from, at}` and, once
// its frame began, `heard` and `begun`, or of those `verb` names landed:
// how long they took, and with `apart` where that went, each part's median,
// so a key whose own frame came in 20 ms but was presented 100 ms after
// its press says whether it waited to be heard, behind a busy shell, or
// after its frame, in niri (ZINAWZ on Joel's 12:06 opening, 2026-11-11).
function keysSaid(verb, times, apart) {
    const one = took(times);
    if (one.count === 0) {
        return "";
    }
    const said = `launcher: ${one.count} keys ${tookSaid(one, "the key", verb)}`;
    return apart ? said + wentTo(times) : said;
}

// Where a set's time went, each part's median over those stamped with
// when the shell heard it and when its frame began, or "" where none
// was: a key from its press.
function wentTo(times) {
    return partsSwapped(times, [["from", "heard", "before the shell heard it"],
        ["heard", "begun", "in the shell"], ["begun", "at", "from its frame's start"]]);
}

// Each part's median as partsSaid says them, and where the time from the
// frame's start went, over those whose frame's swap the render thread
// timed (frame-presentation.patch's timesOf, `swapped`): to its swap,
// which holds the shell's polish, sync and render and any wait the swap
// made for the frame before it, then after it, from niri taking the frame
// to showing it. Joel's drawers spent 45 to 48 ms from their frame's start
// to the screen, whatever they held, and nothing said whose those were
// (KP0L6A, 2026-11-12).
function partsSwapped(times, parts) {
    const said = partsSaid(times, parts);
    const timed = times.filter(each => parts.every(([from, to]) => each[from] !== undefined
        && each[to] !== undefined) && each.swapped > 0);
    if (said === "" || timed.length === 0) {
        return said;
    }
    const toSwap = median(timed.map(each => Math.round(each.swapped - each.begun)));
    const after = median(timed.map(each => Math.round(each.at - each.swapped)));
    return `${said} (${toSwap} to its swap and ${after} after it)`;
}

// Each part's median, `[from, to, words]`, over those stamped with every
// part's ends, or "" where none was, said as ": a median A ms <words>, B
// <words> and C <words>".
function partsSaid(times, parts) {
    const split = times.filter(each => parts.every(([from, to]) => each[from] !== undefined
        && each[to] !== undefined));
    if (split.length === 0) {
        return "";
    }
    const said = parts.map(([from, to, words]) =>
        `${median(split.map(each => Math.round(each[to] - each[from])))} ${words}`);
    return `: a median ${said[0].replace(" ", " ms ")}`
        + said.slice(1, -1).map(one => `, ${one}`).join("") + ` and ${said[said.length - 1]}`;
}

// Where a drawer's time went from the key that selected its row: waiting
// for the selection to be followed, then until its frame began, which
// holds making it, then from its frame's start to the screen.
const drawerParts = [["from", "followed", "before it followed"],
    ["followed", "begun", "until its frame began"], ["begun", "at", "from its frame's start"]];

// A drawer presented, `{ key, place, from, followed, begun, swapped, at }`, said as
// it's presented, as an opening is, so each one's time from Joel's key can
// be read as he goes.
function drawerSaid(one) {
    return `launcher: drawer for "${one.key}" presented ${Math.round(one.at - one.from)} ms `
        + `from the key${partsSwapped([one], drawerParts)}, ${one.came ?? "the frame after"}`;
}

// The drawers presented since the launcher opened, a line a place, as a
// place's changes are.
function drawersSaid(drawers) {
    const places = [...new Set(drawers.map(one => one.place))].sort();
    return places.map(place => {
        const its = drawers.filter(each => each.place === place);
        const one = took(its);
        return `launcher: ${one.count} ${one.count === 1 ? "drawer" : "drawers"} in ${place} `
            + `${tookSaid(one, "the key")}${partsSwapped(its, drawerParts)}`;
    });
}

// A place's change heard from a line at `from`: the earliest line not yet
// drawn is the one it's timed from, since the draw shows them all.
function heard(waiting, place, from) {
    if (place in waiting && waiting[place] <= from) {
        return waiting;
    }
    const kept = Object.assign({}, waiting);
    kept[place] = from;
    return kept;
}

// What waits for a place once it's drawn, which it no longer does.
function drawn(waiting, place) {
    if (!(place in waiting)) {
        return waiting;
    }
    const kept = Object.assign({}, waiting);
    delete kept[place];
    return kept;
}

// Where a change's time went: before the shell heard its line, waiting on
// the pace for its refresh to begin (Pacing.js), which Joel's windows
// changes' 190 ms in the shell was (2026-11-12), in the shell's refresh
// and until its frame began, and from its frame's start to the screen.
const changeParts = [["from", "heard", "before the shell heard it"],
    ["heard", "refreshed", "waiting on the pace"], ["refreshed", "begun", "in the shell"],
    ["begun", "at", "from its frame's start"]];

// What the journal says of the changes presented, each `{place, from,
// at}` and, once the launcher took it, its refresh began and its frame
// began, `heard`, `refreshed` and `begun`: a line a place, in the
// places' order by name, saying where its time went as a key's does, so a slow update says whether it waited
// before the launcher took it, in the shell, or after its frame began
// (Joel's 20 changes in machine, 6 past 50 ms, 2026-11-11). Where they
// name the daemons whose lines set them going, `by`, it says how many
// each did, most first, as Now is made from many.
function changesSaid(changes) {
    const places = [...new Set(changes.map(one => one.place))].sort();
    return places.map(place => {
        const its = changes.filter(each => each.place === place);
        const one = took(its);
        return `launcher: ${one.count} ${one.count === 1 ? "change" : "changes"} in ${place} `
            + `${tookSaid(one, "the line")}${whose(its)}${partsSwapped(its, changeParts)}`;
    });
}

// Whose lines a place's changes came from, ", from machine 2, wifi 1", or
// "" where none says.
function whose(changes) {
    const counts = {};
    for (const one of changes.filter(each => (each.by ?? "") !== "")) {
        counts[one.by] = (counts[one.by] ?? 0) + 1;
    }
    const named = Object.keys(counts).sort((a, b) => counts[b] - counts[a] || (a < b ? -1 : 1));
    return named.length === 0 ? "" : `, from ${named.map(by => `${by} ${counts[by]}`).join(", ")}`;
}

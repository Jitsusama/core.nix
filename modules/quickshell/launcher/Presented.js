// How long what the launcher showed took to reach niri's screen, as its
// journal says it as it closes: keys from their press, and a daemon's
// changes from the line that made them, each place on its own (Y2YBTS,
// 2026-11-11), so a slow update path is seen against Joel's 50 ms bar as
// keys are. Apart from the shell, so tests/pacing.qml holds it in Qt's own
// engine.
.pragma library

const bar = 50;

// How long each of a set took, from `from` to `at`: how many, the median,
// the slowest and how many within the bar.
function took(times) {
    const ms = times.map(one => Math.round(one.at - one.from)).sort((a, b) => a - b);
    return {
        count: ms.length,
        median: median(ms),
        slowest: ms[ms.length - 1],
        within: ms.filter(each => each <= bar).length
    };
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
    const said = `launcher: ${one.count} keys ${verb} a median ${one.median} ms from the key, `
        + `the slowest ${one.slowest} ms, ${one.within} within ${bar} ms`;
    return apart ? said + wentTo(times) : said;
}

// Where a set's time went, each part's median over those stamped with
// when the shell heard it and when its frame began, or "" where none
// was: a key from its press, a change from its daemon's line.
function wentTo(times) {
    const split = times.filter(each => each.heard !== undefined && each.begun !== undefined);
    if (split.length === 0) {
        return "";
    }
    const part = (from, to) => median(split.map(each => Math.round(each[to] - each[from])));
    return `: a median ${part("from", "heard")} ms before the shell heard it, `
        + `${part("heard", "begun")} in the shell and ${part("begun", "at")} from its `
        + "frame's start";
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

// What the journal says of the changes presented, each `{place, from,
// at}` and, once the launcher took it and its frame began, `heard` and
// `begun`: a line a place, in the places' order by name, saying where its
// time went as a key's does, so a slow update says whether it waited
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
            + `presented a median ${one.median} ms from the line, the slowest `
            + `${one.slowest} ms, ${one.within} within ${bar} ms${whose(its)}${wentTo(its)}`;
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

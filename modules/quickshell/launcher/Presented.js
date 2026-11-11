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
        median: ms[(ms.length - 1) >> 1],
        slowest: ms[ms.length - 1],
        within: ms.filter(each => each <= bar).length
    };
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
// at}`: a line a place, in the places' order by name.
function changesSaid(changes) {
    const places = [...new Set(changes.map(one => one.place))].sort();
    return places.map(place => {
        const one = took(changes.filter(each => each.place === place));
        return `launcher: ${one.count} ${one.count === 1 ? "change" : "changes"} in ${place} `
            + `presented a median ${one.median} ms from the line, the slowest `
            + `${one.slowest} ms, ${one.within} within ${bar} ms`;
    });
}

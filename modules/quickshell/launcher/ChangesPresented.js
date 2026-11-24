// How long a daemon's changes took to reach niri's screen, a place at a time, as the journal says
// it as the launcher closes: which line each was timed from, the holds while home was found, and
// where the time went, as a key's line says it (Presented.js). Apart from the shell, so
// tests/pacing.qml holds it in Qt's own engine.
.pragma library
.import "Presented.js" as Presented

// A place's change heard from a line at `from`: the earliest line not yet drawn is the one it's
// timed from, since the draw shows them all.
function heard(waiting, place, from) {
    return place in waiting && waiting[place] <= from ? waiting
        : Object.assign({}, waiting, { [place]: from });
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

// Where a change's time went: before the shell heard its line, waiting on the pace for its refresh
// to begin (Pacing.js), which Joel's windows changes' 190 ms in the shell was (2026-11-12), in the
// shell's refresh and until its frame began, and from its frame's start to the screen.
const changeParts = [["from", "heard", "before the shell heard it"],
    ["heard", "refreshed", "waiting on the pace"], ["refreshed", "begun", "in the shell"],
    ["begun", "at", "from its frame's start"]];
// A refresh due while home was still to be found is held until it's found (Launcher.qml's
// refreshDue and refreshed, through heldFrom), which isn't the pace: where one of a place's
// changes was held, `held` being when, its line says the hold apart (heldOf), its wait on the pace
// ending as the hold began, and how many were held and the longest hold after the medians. One
// opening holds about one change a place, so a median of holds over all of them would read 0
// beside a hold of 150 ms.
const pacedParts = [["from", "heard", "before the shell heard it"],
    ["heard", "held", "waiting on the pace"], ["refreshed", "begun", "in the shell"],
    ["begun", "at", "from its frame's start"]];

// A place's changes, `its`, with the parts their line says.
function changesParted(its) {
    if (!its.some(one => one.held !== undefined)) {
        return Presented.partsSwapped(its, changeParts);
    }
    const filled = its.map(one => Object.assign({}, one, { held: heldOf(one) }));
    const ms = its.filter(one => one.held !== undefined && one.refreshed !== undefined)
        .map(one => Math.round(one.refreshed - heldOf(one)));
    const said = Presented.partsSwapped(filled, pacedParts);
    return said === "" || ms.length === 0 ? said
        : `${said}; ${ms.length} held while home was found, the longest ${Math.max(...ms)} ms`;
}

// When a change was held from: one not held is held for none, and one heard again after its hold
// began, an earlier line come late taking the place's change (heard, above), from when it was
// heard, so its wait on the pace is never less than none.
function heldOf(one) {
    return one.held === undefined ? one.refreshed : Math.max(one.heard, one.held);
}

// When each place's waiting change was first held for home's find, `held` by place, as a refresh
// comes due or a change is heard: while `holding`, each of `places` whose change was heard
// (`heardOf`, when by place) is held from `at`, and one held already keeps its time, as it's held
// from when it was first due; one heard of none, or any while nothing holds, has nothing held.
function heldFrom(held, heardOf, { places, at, holding }) {
    if (!holding) {
        return held;
    }
    const next = Object.assign({}, held);
    for (const place of places) {
        if (heardOf[place] !== undefined && next[place] === undefined) {
            next[place] = at;
        }
    }
    return next;
}

// What the journal says of the changes presented, each `{place, from, at}` and, once the launcher
// took it, its refresh began and its frame began, `heard`, `refreshed` and `begun`: a line a place,
// in the places' order by name, saying where its time went as a key's does, so a slow update says
// whether it waited before the launcher took it, in the shell, or after its frame began (Joel's 20
// changes in machine, 6 past 50 ms, 2026-11-11). Where they name the daemons whose lines set them
// going, `by`, it says how many each did, most first, as Now is made from many.
function changesSaid(changes) {
    const places = [...new Set(changes.map(one => one.place))].sort();
    return places.map(place => {
        const its = changes.filter(each => each.place === place);
        const one = Presented.took(its);
        return `launcher: ${one.count} ${one.count === 1 ? "change" : "changes"} in ${place} `
            + `${Presented.tookSaid(one, "the line")}${whose(its)}${changesParted(its)}`;
    });
}

// Whose lines a place's changes came from, ", from machine 2, wifi 1", or "" where none says.
function whose(changes) {
    const counts = {};
    for (const one of changes.filter(each => (each.by ?? "") !== "")) {
        counts[one.by] = (counts[one.by] ?? 0) + 1;
    }
    const named = Object.keys(counts).sort((a, b) => counts[b] - counts[a] || (a < b ? -1 : 1));
    return named.length === 0 ? "" : `, from ${named.map(by => `${by} ${counts[by]}`).join(", ")}`;
}

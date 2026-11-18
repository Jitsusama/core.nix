// How the steps the launcher holds, each a row stepped into and the rows
// it offers, are made again as what they were made from changes: a place's
// members coming, a session's verbs as it starts working. Apart from the
// launcher, so tests/held.qml can hold it in Qt's own engine.
//
// Each takes `ways`, the launcher's own answers: `same(row, was)` whether a
// row offered now is the one a step holds, `into(thing, at, steps)` the
// rows a thing stepped into at `at` offers now, or null for none, and
// `keep(step, into)` those rows kept in the order the step shows them.
// Each gives `steps` made again, `leave`, the first step to leave or -1,
// and `all`, whether every step was made again.
.pragma library

// Each step from `from` on made again from the rows the one above it
// offers now, `rows` for the first. A step entered from what was typed is
// made again from its own row, since what was typed found it, not the rows
// above.
function remade(steps, from, rows, ways) {
    const made = steps.slice();
    let offered = rows;
    for (let at = from; at < made.length; at++) {
        const step = made[at];
        const thing = offered.find(row => ways.same(row, step.row))
            ?? ((step.typed ?? "") !== "" ? step.row : undefined);
        const into = thing === undefined ? null : ways.into(thing, at, made);
        if (!into) {
            return { steps: made, leave: -1, all: false };
        }
        made[at] = Object.assign({}, step, { row: thing, rows: ways.keep(step, into) });
        offered = into;
    }
    return { steps: made, leave: -1, all: true };
}

// The step at `at`, stepped into from a source's own row, made again from
// what that row offers now; `offered` is everything the source offers.
function own(steps, at, _offered, ways) {
    const made = steps.slice();
    const step = made[at];
    const into = ways.into(step.row, at, made);
    if (into) {
        made[at] = Object.assign({}, step, { rows: ways.keep(step, into) });
    }
    return { steps: made, leave: -1, all: true };
}

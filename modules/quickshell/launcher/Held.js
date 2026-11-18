// How the steps the launcher holds, each a row stepped into and the rows
// it offers, are made again as what they were made from changes: a place's
// members coming, a session's verbs as it starts working. Apart from the
// launcher, so tests/held.qml can hold it in Qt's own engine.
//
// Each takes `ways`, the launcher's own answers: `same(row, was)` whether a
// row offered now is the one a step holds, `into(thing, at, steps)` the
// rows a thing stepped into at `at` offers now, or null for none, as
// `offers` decides, and
// `keep(step, into)` those rows kept in the order the step shows them.
// Each gives `steps` made again, `leave`, the first step to leave or -1,
// and `all`, whether every step was made again.
.pragma library

// Each step from `from` on made again from the rows the one above it
// offers now, `rows` for the first. A step entered from what was typed is
// made again from its own row, since what was typed found it, not the rows
// above. A step whose row is gone is kept as it was, dimmed where the list
// above shows it gone; one whose row is still offered but offers nothing
// now is left, as a session's verbs once it has ended, rather than kept
// showing what it offered before. A typed step whose own row offers
// nothing keeps its rows, each gone, as no row offered now says it's left.
function remade(steps, from, rows, ways) {
    const made = steps.slice();
    let offered = rows;
    for (let at = from; at < made.length; at++) {
        const step = made[at];
        const now = offered.find(row => ways.same(row, step.row));
        const thing = now ?? ((step.typed ?? "") !== "" ? step.row : undefined);
        const into = thing === undefined ? null : ways.into(thing, at, made);
        if (!into && (now !== undefined || thing === undefined)) {
            return { steps: made, leave: now === undefined ? -1 : at, all: false };
        }
        made[at] = Object.assign({}, step, { row: thing, rows: ways.keep(step, into ?? []) });
        offered = into ?? [];
    }
    return { steps: made, leave: -1, all: true };
}

// What a row offers a step into, from `rows`, what's inside it, or null
// for nothing: no rows is nothing, unless its vocabulary names `places`
// (Agents with no agent running still leads to its Quests), or it says
// what it holds when `empty`, as Waiting on You does. The launcher's step
// in and its held steps both ask this, so one is left exactly where the
// other is refused.
function offers(rows, places, empty) {
    if ((!rows || rows.length === 0) && places === 0 && !empty) {
        return null;
    }
    return rows ?? [];
}

// The step at `at`, stepped into from a source's own row, made again with
// that row as the source offers it now, among `offered`, everything it
// offers: kept as entered, its crumb and drawer said what it said then. It
// is the source's row as it is now, with only what the launcher gave it
// when found kept over it: `foundAs`, its side, the verbs it found and the
// rest, kept with it for the next time, or else `from`, the source that
// gave it, which the launcher finds the step by. What the search found,
// and where it lands, are each as the source offers them now (foundNow).
// It's never said in `given`, a row's way of asking for words. Nothing
// else the row held then survives, so a field the source no longer offers
// is gone. A row gone is kept, its rows made from it as it was, each gone
// where it now offers nothing, as a collection's own row, never among what
// it offers, is when its last row leaves; one still offered that offers
// nothing now is left.
function own(steps, at, offered, ways) {
    const made = steps.slice();
    const step = made[at];
    const now = offered.find(one => ways.same(one, step.row));
    const foundAs = step.row.foundAs === undefined ? undefined
        : foundNow(step.row.foundAs, offered, ways);
    const kept = foundAs !== undefined ? Object.assign({}, foundAs, { foundAs })
        : step.row.from === undefined ? {} : { from: step.row.from };
    const row = now === undefined ? step.row : Object.assign({}, now, kept);
    const into = ways.into(row, at, made);
    if (!into && now !== undefined) {
        return { steps: made, leave: at, all: false };
    }
    made[at] = Object.assign({}, step, { row, rows: ways.keep(step, into ?? []) });
    return { steps: made, leave: -1, all: true };
}

// What the launcher gave a row a search found, with each row the search
// found, and the one it lands in, as the source offers it now among
// `offered`: an ask hidden as a share begins is then listed hidden, never
// as it was found. One the source no longer offers is kept as found, as
// People's, found beyond what it lists, are.
function foundNow(foundAs, offered, ways) {
    const asNow = was => offered.find(one => ways.same(one, was)) ?? was;
    const found = typeof foundAs.verbs === "function" ? foundAs.verbs().map(asNow) : null;
    return Object.assign({}, foundAs, found === null ? {} : { verbs: () => found },
        foundAs.landsIn === undefined ? {} : { landsIn: asNow(foundAs.landsIn) });
}

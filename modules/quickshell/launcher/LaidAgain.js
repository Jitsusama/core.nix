// The heading rules Steady.js lays a list again by, apart so each file
// holds one thing: where a new row of a list with headings comes in, the
// heading each row reads as under, and what waits unseen while it would
// read as another's. Only Steady.js calls them, and tests/steady.qml holds
// them to it through Steady.js.
.pragma library

// The rows laid again, `rows`, with `lit(rows)` dimming each heading said
// again with no row of its own standing under it in `rows`, `late`, how
// many new ones wait to be shown afresh where `withholding`, and
// `putBack`, how many of `rows` the rows shown put back.
// Where a list has headings (Headings.js), as For You's cells do, what
// stands under a heading reads as its, so a new row of a heading that
// isn't the last above the end, dimmed or not, has it said again over it,
// the same heading in the same words, `repeated`; a heading new to the
// list comes in only so, over a new row. A new row of no heading there
// waits while the end reads as under a heading, by where each row stands
// (endingHeading), since there it would read as that heading's, unless
// it's `pressing`, as Agents Asking's row and a critical notice's are,
// since what's pressing never waits unseen: that one comes in first,
// above every heading, where ForYouCells.js's laidOut gathers the asks,
// since at the end it too would read as the last heading's; `raised` says
// how many came so. Where the end reads as under none, a new row of none
// comes in at the end, and ends nothing that isn't already. A row shown
// stays where it is, under the heading it was shown under, though its cell
// has changed. One said again stays where it was said, reading as its
// heading reads now, and is dimmed while no row of its stands under it, as
// a row is while gone; a list shown afresh says each heading once (Y2YBTS,
// 2026-11-22 [228cdeb9] and [4c5e154a]). One said again among `now`, as a
// held step's rows hand it back, is no row of the source's, and is said
// only as here, though it still says what follows it is its heading's.
// One that waits is withheld and counted in `late` where `withholding`,
// and else kept at the end marked `unseen`, as a row of no heading: it ends
// nothing, since it isn't shown, and once it's pressing, or has a heading
// or none above it, it comes in as any row does.
function laidAgain(shown, now, id, withholding) {
    // A list with no heading, as most are, has nothing to wait or say
    // again, so is spared the work of finding each row's.
    const headless = !shown.some(row => row.heading === true)
        && !now.some(row => row.heading === true);
    const owners = headless ? now.map(() => null) : headingsUnder(now, id);
    // Where each row shown stood: what the end reads as under goes by
    // that, not by its heading now, so one whose cell went still reads as
    // under the heading it stands under.
    const placed = headless ? shown.map(() => null) : headingsUnder(shown, id);
    const fresh = now.filter(row => !row.repeated);
    const under = owners.filter((_, at) => !now[at].repeated);
    // Each row's heading by its place in `out` (`lit` is given only it or
    // its first rows), not a Map, which Qt 6.12 walks whole (Rank.js).
    const owner = [];
    // Whether each row in `out` reads as under none where it stands.
    const none = [];
    const lit = headless ? rows => rows : rows => rows.map((row, at) => row.repeated
        && !row.went && !standsUnder(rows, at, owner, id) ? dimmed(row) : row);
    if (shown.length === 0) {
        return { rows: fresh.map(asSeen), late: 0, lit: rows => rows, putBack: 0, raised: 0 };
    }
    // A heading said again stands for its cell where `now` has it only so,
    // as a held step's rows hand back a new cell's, else it would go dim.
    const headingsNow = fresh.filter(row => row.heading === true)
        .concat(now.filter(row => row.heading === true && row.repeated));
    const waiting = Object.create(null);
    fresh.forEach((row, at) => {
        const said = id(row);
        if (waiting[said] === undefined) {
            waiting[said] = [];
        }
        waiting[said].push(at);
    });
    const taken = fresh.map(() => false);
    const out = [];
    const put = (row, its, alone) => {
        owner[out.length] = its;
        none[out.length] = alone;
        out.push(row);
    };
    let late = 0;
    const raised = [];
    for (const [place, row] of shown.entries()) {
        if (row.repeated) {
            const again = headingsNow.find(one => id(one) === id(row));
            out.push(again ? Object.assign({}, again, { repeated: true }) : dimmed(row));
            continue;
        }
        const left = waiting[id(row)];
        // One that waits unseen was never shown, so it has no place to
        // keep: it stays waiting while it waits, and else comes in as a new
        // one does, with its heading said over it, or leaves with nothing
        // to dim.
        if (row.unseen === true) {
            const at = left && left.length > 0 ? left[0] : -1;
            if (at >= 0 && waits(fresh[at], under[at], endingHeading(out, none) !== null)) {
                left.shift();
                taken[at] = true;
                late += withholding ? 1 : 0;
                if (!withholding) {
                    out.push(asUnseen(fresh[at]));
                }
            }
            continue;
        }
        // One shown is seen, though `now` hands it back marked, as a held
        // step does one a search found: marked, it'd next move to the end.
        if (left && left.length > 0) {
            const at = left.shift();
            const again = asSeen(fresh[at]);
            taken[at] = true;
            put(again, under[at], placed[place] === null);
        } else {
            put(dimmed(row), undefined, placed[place] === null);
        }
    }
    const putBack = out.length;
    let last = endingHeading(out, none);
    fresh.forEach((row, at) => {
        // One that left before it was ever shown, as a held step's rows
        // hand it back, has nothing to dim, and never waits to be shown.
        if (taken[at] || row.heading === true || row.went) {
            return;
        }
        const its = under[at];
        if (waits(row, its, last !== null)) {
            late += withholding ? 1 : 0;
            if (!withholding) {
                out.push(asUnseen(row));
            }
            return;
        }
        if (its === null && last !== null) {
            raised.push(asSeen(typeof row.ranked === "function" ? row.ranked() : row));
            return;
        }
        if (its !== null && (last === null || id(last) !== id(its))) {
            last = Object.assign({}, its, { repeated: true });
            out.push(last);
        }
        const coming = asSeen(typeof row.ranked === "function" ? row.ranked() : row);
        put(coming, its, its === null);
    });
    out.unshift(...raised);
    owner.unshift(...raised.map(() => null));
    none.unshift(...raised.map(() => true));
    return { rows: out, late, lit, putBack, raised: raised.length };
}

// Whether a row new to a list waits unseen: one of no heading (`its`)
// where the end reads as under one (`endsHeaded`), and not pressing.
function waits(row, its, endsHeaded) {
    return its === null && endsHeaded && !pressing(row);
}

// A row as it's shown, no longer marked as waiting unseen, or so marked:
// a copy, so the row given keeps its mark, laid from its descriptors as
// Member.js's copied lays one, a getter handed on unread and unfrozen.
function asSeen(row) {
    return row.unseen === true ? marked(row, {}) : row;
}
function asUnseen(row) {
    return marked(row, { unseen: { value: true, enumerable: true, writable: true } });
}
function marked(row, laid) {
    const own = Object.getOwnPropertyDescriptors(row);
    return Object.defineProperties({}, Object.keys(own).filter(name => name !== "unseen")
        .reduce((all, name) => Object.assign(all, { [name]: own[name] }), laid));
}

// The heading the end of `rows` reads as under: the last one, unless a row
// that reads as under none where it stands (`none`) comes after it, as a
// pressing one that came in does, or To Look At after the cells, which
// ends it however long ago it came.
function endingHeading(rows, none) {
    let last = null;
    for (const [at, row] of rows.entries()) {
        if (row.heading === true) {
            last = row;
        } else if (none[at] === true) {
            last = null;
        }
    }
    return last;
}

// Whether a row is pressing, so it never waits unseen: its own word, or
// what it says while asked, as a notices row is while it holds a critical
// one (Now.qml's latelyRows).
function pressing(row) {
    return (typeof row.pressing === "function" ? row.pressing() : row.pressing) === true;
}

// A row as it stands once it's left: dimmed where it was, `gone` and `went`.
function dimmed(row) {
    return row.went ? row : Object.assign({}, row, { gone: true, went: true });
}

// Whether a row that hasn't left, and is the heading's at `at` in `rows`
// (`owner`, each row's heading), stands under it, before the next heading.
function standsUnder(rows, at, owner, id) {
    for (let next = at + 1; next < rows.length && rows[next].heading !== true; next += 1) {
        const its = owner[next] ?? null;
        if (!rows[next].went && its !== null && id(its) === id(rows[at])) {
            return true;
        }
    }
    return false;
}

// The heading each row is under among `rows`, or null for a heading itself
// and a row under none. A heading that names the rows it's `holding`, by
// key, as For You's cells' do (ForYouCells.js's laidOut), has those
// wherever they stand, so what follows them, To Look At and History, is
// under none, and a row that came in at the end is still its cell's; one
// that doesn't has every row to the next. One said again stands for the
// heading it repeats.
//
// A heading said again holds what no heading it repeats is there to hold,
// as one a held step's rows hand back alone does, a new cell's said over
// its first row: else its rows would read as under none.
function headingsUnder(rows, id) {
    const holderOf = Object.create(null);
    const holders = rows.filter(row => row.heading === true && Array.isArray(row.holding));
    for (const one of holders.filter(row => !row.repeated)
        .concat(holders.filter(row => row.repeated))) {
        // A row with no key holds no place, so no row of none is a cell's.
        for (const key of one.holding.filter(held => typeof held === "string")) {
            holderOf[key] = holderOf[key] ?? one;
        }
    }
    let above = null;
    return rows.map(row => {
        if (row.heading === true) {
            above = row.repeated ? rows.find(one => one.heading === true && !one.repeated
                && id(one) === id(row)) ?? row : row;
            return null;
        }
        const holder = holderOf[row.key];
        if (holder !== undefined) {
            return holder;
        }
        return above !== null && !Array.isArray(above.holding) ? above : null;
    });
}


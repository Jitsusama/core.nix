.pragma library

// Whether a source's rows are wanted now, from where the launcher stands, so
// a source whose rows cost much builds them only then and a change nobody
// sees builds none (Provider.qml's wanted). One opening on Joel's shell made
// 6,933 rows, most of them sessions' that home only counts (Joel,
// 2026-11-08: "weird lag can happen").
//
// `stands` is what the launcher says of itself: whether its catalogue is
// walked from outside (`cataloguing`), whether it's `open`, the steps
// `held`, each with the row it stepped into, the acts `becoming`, each with
// the source whose rows it waits on, and what's `typed`. A source is
// `asleep` while it waits its turn to wake as a kept opening fills in.
function wanted(stands, source) {
    // Walked from outside, every row is read.
    if (stands.cataloguing) {
        return true;
    }
    if (!stands.open) {
        return false;
    }
    // Held, or with an act of its own waiting to be agreed, its rows are
    // read for what's shown and for what the act becomes.
    if (stands.held.some(step => step.row?.from === source)
        || stands.becoming.some(one => one.from === source)) {
        return true;
    }
    // In another place, only that place is shown and searched.
    if (stands.held.length > 0) {
        return false;
    }
    // At home, what's typed searches every place, and home names what each
    // holds, unless it says what it holds itself, as Agents does; while a
    // kept opening fills in, only once it's woken (wakingOrder).
    return stands.typed !== "" || !(source.saysAtHome || source.asleep);
}

// Whether rows home shows, or a few places read (`readers`), are read now:
// walked from outside, open at home, or with one of those places wanted.
// Opened by a chord straight into another place, as Super+V opens Copies,
// they aren't, where Agents made its own rows (Sessions to Reopen, Landed,
// Recent, Start and Budgets) as any opening began (H6EDLC, 2026-11-11).
function readAt(stands, readers) {
    if (stands.cataloguing) {
        return true;
    }
    if (!stands.open) {
        return false;
    }
    return stands.held.length === 0 || readers.some(one => one !== null && wanted(stands, one));
}

// The order a kept opening's sources wake in, a few a frame, rather than
// all at once as it opened, which held Joel's shell 195 to 220 ms past
// its first frame (2026-11-10 20:39): those home's rows read, in home's
// order, a place's own and then those it holds, so the rows shown fill
// first, then every other in `all`'s order. `place(id)` is the place an
// id names, as `{ stands, holds }`, or null, and `source(id)` the source
// an id or a place's `stands` reads, or null.
function wakingOrder(home, place, source, all) {
    const order = [];
    const add = one => {
        if (one && !order.includes(one)) {
            order.push(one);
        }
    };
    // A place holds places, as Machine holds Power, but never itself, so
    // the walk ends; the depth only guards a vocabulary that one day did.
    const walk = (id, depth) => {
        const placed = place(id);
        if (placed === null) {
            add(source(id));
        } else if (depth < 8) {
            add(placed.stands ? source(placed.stands) : null);
            for (const held of placed.holds) {
                walk(held, depth + 1);
            }
        }
    };
    for (const id of home) {
        walk(id, 0);
    }
    for (const one of all) {
        add(one);
    }
    return order;
}

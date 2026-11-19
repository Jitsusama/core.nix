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
// the source whose rows it waits on, and whether anything's typed
// (`typing`), never what, so a source's wanted is asked again only as that
// turns rather than on every key: 65 sources' were, about a millisecond of
// each key before it searched (2026-11-11). A source is
// `asleep` while it waits its turn to wake as an opening fills in, and
// `warming` while its home rows are made shut, so the first opening after
// a start draws from what's kept. The source a chord opens into is
// `entering` until its step is held, and the launcher is `viewing` while
// its command line is over a viewer's.
function wanted(stands, source) {
    // Walked from outside, every row is read.
    if (stands.cataloguing) {
        return true;
    }
    if (!(stands.open || source.warming)) {
        return false;
    }
    // Over a viewer, as : in swayimg opens it, the line hides every row, so
    // none is made for it: Joel's two at 23:43 made 84 and 38 home rows, and
    // took 110 and 75 ms to their frames (2026-11-14).
    if (stands.viewing) {
        return false;
    }
    // Opened into a place, only that place is shown, from the opening's
    // first moment: open before its step was held, the launcher stood at
    // home for an instant, and every source home names made its rows for
    // it, put away at once. Each Super+Tab reopening made 35 rows of
    // Capture, Today, Time and Agents (3TXW9E's measure, 2026-11-10).
    if (stands.entering) {
        return source === stands.entering
            || stands.becoming.some(one => one.from === source);
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
    return stands.typing || !(source.saysAtHome || source.asleep);
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
    if (!stands.open || stands.viewing) {
        return false;
    }
    // Opening into a place, nothing is held yet but home isn't shown either.
    return (stands.held.length === 0 && !stands.entering)
        || readers.some(one => one !== null && wanted(stands, one));
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

// The source an id names, by the name `provider(id)` gives it, the first in
// `all` whose `nameOf` is that, or null: each source's name read once, and
// each id's asked once. Searched for each id and each source, an opening
// asked the vocabulary about 2,100 times as it filled (H6EDLC's profile,
// 2026-11-12).
function sourceNamed(all, nameOf, provider) {
    const byName = new Map();
    for (const one of all) {
        const name = nameOf(one);
        if (!byName.has(name)) {
            byName.set(name, one);
        }
    }
    return id => byName.get(provider(id)) ?? null;
}

// Whether an opening leaves finding home and joining its sources to a
// slice after its fill's frame (Launcher.qml's fillFound), since the card
// already shows what was kept: only home's fill, run by `filled()` and so
// sure of a slice to follow, with a home kept for the card to stand on
// until then. A place's fill, whose steps into it run in the open, and any
// other opening find and join there as ever (H6EDLC, 2026-11-18). Nor
// while the screen may be shared (`shared`, Shared.js's hidden): home kept
// from before a share can name what it hides, an ask or a copy, which its
// first frame would draw on the shared screen (5GLTPH, 2026-11-19). An
// opening that never says counts as shared, as Shared.js fails closed.
function findsLater(opening) {
    return opening.byFill && !opening.intoPlace && opening.kept > 0 && opening.shared === false;
}

// Whether a chord may draw home's kept card before it's filled in
// (Launcher.qml's beginOpening), rather than open whole: only while
// nothing may see the screen (`shared`, Shared.js's hidden), since the
// card was kept before any share began and a shut launcher doesn't
// refresh it, so it can name a waiting ask or a copy (5GLTPH, 2026-11-19).
function opensKept(shared) {
    return shared === false;
}

// How a chord into a kept place lists it as it's drawn (Launcher.qml's
// intoKept), or "" to open it whole: "source" by its source as it stands,
// for one whose source lists it so (`listed`, Windows'); "asItStands" for
// a plain place asked to list now (`asNow`, a margin mark into On);
// "led" as a source leads to it, for one asked for by name (`named`) or
// led to (`led`, Super+N's). Each of those makes its rows now, each
// source hiding its own under a share. Otherwise "kept", its rows as
// they were kept, only while nothing may see the screen (`shared`,
// Shared.js's hidden), since they were made before any share began and a
// shut launcher never makes them again: Super+V's would show a copy,
// Super+O's or For You's a waiting ask. Nothing marks which kept rows
// are private, so every such place opens whole.
function keptPlaceListed(place) {
    if (place.listed === true) {
        return "source";
    }
    if (place.asNow === true) {
        return "asItStands";
    }
    if (place.named === true || place.led === true) {
        return "led";
    }
    return opensKept(place.shared) ? "kept" : "";
}

// Whether that slice, or a key, a click or a chord run before it, searches
// home again rather than only joining: only where the open would have
// found home, and never for a chord, which steps away from it at once, or
// once a step taken or a word typed has found its own.
function findsAgain(by, stands) {
    return stands.home && by !== "chord" && stands.held === 0 && stands.typed === "";
}

// How a kept opening's sources woke: the longest slice, and the longest
// any waited after the one before ended, or the first after the fill.
// Each is `{ from, took }`, in ms from the fill's end. Home found again
// after the fill, `found`, or null where it was found inside it, is one
// slice more before the sources' first, so its time is counted as a
// slice's and never read as the first's wait (H6EDLC, 2026-11-18).
function wakeTimes(found, slices) {
    const timed = (found === null ? [] : [found]).concat(slices);
    const longest = timed.reduce((most, one) => Math.max(most, Math.round(one.took)), 0);
    const waited = timed.reduce((most, one, at) => Math.max(most, at === 0 ? one.from
        : one.from - timed[at - 1].from - timed[at - 1].took), 0);
    return { longest, waited: Math.round(waited) };
}

// Whether the slice that found home woke behind a frame or fallback of its
// own, so no source woke in it: `after` names what woke the find, the
// frame's number or its fallback's turn, and each slice carries its own.
// One with no name to compare says nothing, so it's never vouched for.
function foundAlone(after, slices) {
    const named = one => one !== null && one !== undefined;
    return named(after) && slices.every(one => named(one.after) && one.after !== after);
}

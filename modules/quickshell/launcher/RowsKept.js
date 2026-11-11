.pragma library

// How the launcher's list turns the rows it holds into the rows it lists
// now, by what each is (Launcher.qml's pairedId), so a row listed before
// and now keeps what it drew wherever it moves, and only a row new to the
// list is drawn afresh. Kept only where the start and the end were the
// same, a letter narrowing a search drew every row it kept again: "s" to
// "so" drew 6 rows of 6 the list already held, 3 ms each in the sandbox,
// the most of what a letter's showing spent after its search (2026-11-09).

// Each key with how many times it came before, so a key listed twice is
// two rows, each kept in its turn.
function tagged(keys) {
    const seen = new Map();
    return keys.map(key => {
        const nth = seen.get(key) ?? 0;
        seen.set(key, nth + 1);
        return `${nth}\u0000${key}`;
    });
}

// The rows held that keep their places, the most of them that stand in
// the same order before and now, so every other row moves once rather
// than each row it passes: a row falling to the end is one move, not one
// for each row below it.
function staying(held, placeOf) {
    const places = held.map(key => placeOf.get(key));
    // tails[n]: the row ending the least rising run n + 1 long; before[at]:
    // the row before it in its run.
    const tails = [];
    const before = [];
    places.forEach((place, at) => {
        let low = 0;
        let high = tails.length;
        while (low < high) {
            const middle = (low + high) >> 1;
            if (places[tails[middle]] < place) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        before[at] = low > 0 ? tails[low - 1] : -1;
        tails[low] = at;
    });
    const stay = new Set();
    for (let at = tails.length > 0 ? tails[tails.length - 1] : -1; at >= 0; at = before[at]) {
        stay.add(held[at]);
    }
    return stay;
}

// Each key's place in `keys`.
function placesOf(keys) {
    return new Map(keys.map((key, at) => [key, at]));
}

// Every row held gone and every row now put in, in one step each.
function afresh(was, now) {
    return (was.length > 0 ? [{ remove: 0, count: was.length }] : [])
        .concat(now.length > 0 ? [{ insert: 0, keys: now.slice() }] : []);
}

// Whether `taken` (what steps gave) put every row of `was` in again, as
// afresh does, so the rows stand from the list's start, whatever it was
// scrolled to, where moved rows keep those in view where they were:
// Launcher.qml puts the row selected back in view after them (reshow).
function whole(was, taken) {
    return was.length > 0 && taken.length > 0 && taken[0].remove === 0
        && taken[0].count === was.length;
}

// The steps that make `was` into `now`, done in order to a ListModel: the
// rows gone, last first, as { remove, count }, so each place counts from
// the rows still there; then each row in its place, as { move, to }, or
// { insert, keys }, the rows in a run the list didn't hold, put in at
// once. A step into Emoji lists 1,924 rows, which went in one at a time,
// 16 to 20 ms in Qt's engine offscreen against a millisecond at once, after
// 28 to 34 ms tagging and placing each, where nothing listed before stays
// (2026-11-11). So two lists sharing no row take one step each way. A row
// that moves is moved up
// into its place, or, in a staying row's way, down to just before the
// next row listed after it now that stays or has already moved down, or
// the end. None where the two are the same.
// More than `most` rows moving are listed afresh too, as the same two
// steps. Each move shifts the rows in view, and each row shifted reads its
// row again: "f" in Emoji ranked 838 of its 1,914 rows, whose 550 moves
// took 2.4 to 6.7 s before the key's frame, and their steps 0.15 to 0.57 s
// more, in the sandbox (2026-11-11). Listed afresh, the list reads each
// row in view once, from the rows it set aside. Leaving `most` out moves
// every row, however many.
// Working the steps out read each list several times over, tagged, as
// sets and as places: 84 to 132 ms for that reorder, 113 to 151 for a
// narrowing as long and 58 to 94 for two lists the same, in Qt's engine
// offscreen (2026-11-11). Lists the same are now seen in one walk, keys
// are tagged only where one repeats, and one walk over the rows held
// tells how many stay listed and how many fall behind the row before
// them. Each that falls is a row that must move, so past `most` the list
// goes afresh there and then; with none falling and every row now held,
// it's a narrowing, only removed from.
function steps(was, now, most) {
    if (was.length === now.length && was.every((key, at) => key === now[at])) {
        return [];
    }
    let from = was;
    let into = now;
    let placeOf = placesOf(now);
    if (placeOf.size !== now.length || new Set(was).size !== was.length) {
        from = tagged(was);
        into = tagged(now);
        placeOf = placesOf(into);
    }
    let kept = 0;
    let fallen = 0;
    let last = -1;
    for (const key of from) {
        const place = placeOf.get(key);
        if (place === undefined) {
            continue;
        }
        kept += 1;
        if (place < last) {
            fallen += 1;
            if (most !== undefined && fallen > most) {
                return afresh(was, now);
            }
        }
        last = place;
    }
    if (kept === 0) {
        return afresh(was, now);
    }
    const done = [];
    let end = from.length;
    for (let at = from.length - 1; at >= -1; at -= 1) {
        if (at >= 0 && !placeOf.has(from[at])) {
            continue;
        }
        if (end - at > 1) {
            done.push({ remove: at + 1, count: end - at - 1 });
        }
        end = at;
    }
    if (fallen === 0 && kept === into.length) {
        return done;
    }
    const held = from.filter(key => placeOf.has(key));
    const stay = staying(held, placeOf);
    if (most !== undefined && held.length - stay.size > most) {
        return afresh(was, now);
    }
    const settled = new Set(stay);
    const holds = new Set(held);
    for (let at = 0; at < into.length; at += 1) {
        // A run of rows the list didn't hold goes in as one.
        if (!holds.has(into[at])) {
            let past = at + 1;
            while (past < into.length && !holds.has(into[past])) {
                past += 1;
            }
            done.push({ insert: at, keys: now.slice(at, past) });
            held.splice(at, 0, ...into.slice(at, past));
            at = past - 1;
            continue;
        }
        // Rows before a staying row's place that aren't yet in theirs
        // move, so it never does.
        while (stay.has(into[at]) && held[at] !== into[at]) {
            const falling = held[at];
            const next = into.slice(placeOf.get(falling) + 1).find(key => settled.has(key));
            const to = next === undefined ? held.length - 1 : held.indexOf(next) - 1;
            done.push({ move: at, to });
            held.splice(to, 0, ...held.splice(at, 1));
            settled.add(falling);
        }
        if (held[at] === into[at]) {
            continue;
        }
        const below = held.indexOf(into[at], at + 1);
        done.push({ move: below, to: at });
        held.splice(below, 1);
        held.splice(at, 0, into[at]);
    }
    return done;
}

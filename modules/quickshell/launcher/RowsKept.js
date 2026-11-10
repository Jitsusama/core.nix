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

// The steps that make `was` into `now`, done in order to a ListModel: the
// rows gone, last first, as { remove, count }, so each place counts from
// the rows still there; then each row in its place, as { move, to }, or
// { insert, key }, one the list didn't hold. A row that moves is moved up
// into its place, or, in a staying row's way, down to just before the
// next row listed after it now that stays or has already moved down, or
// the end. None where the two are the same.
function steps(was, now) {
    const from = tagged(was);
    const into = tagged(now);
    const wanted = new Set(into);
    const done = [];
    let end = from.length;
    for (let at = from.length - 1; at >= -1; at -= 1) {
        if (at >= 0 && !wanted.has(from[at])) {
            continue;
        }
        if (end - at > 1) {
            done.push({ remove: at + 1, count: end - at - 1 });
        }
        end = at;
    }
    const held = from.filter(key => wanted.has(key));
    // Each tagged key's place now, read once rather than searched for.
    const placeOf = new Map(into.map((key, at) => [key, at]));
    const stay = staying(held, placeOf);
    const settled = new Set(stay);
    for (let at = 0; at < into.length; at += 1) {
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
        if (below >= 0) {
            done.push({ move: below, to: at });
            held.splice(below, 1);
        } else {
            done.push({ insert: at, key: now[at] });
        }
        held.splice(at, 0, into[at]);
    }
    return done;
}

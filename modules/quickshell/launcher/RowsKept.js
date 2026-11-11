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

// Whether two lists share no row, looking up the longer's in the
// shorter's, so a step from home's 20 rows into Emoji's 1,924 reads each
// once and makes nothing of them.
function apart(was, now) {
    const fewer = was.length <= now.length ? was : now;
    const more = fewer === was ? now : was;
    const held = new Set(fewer);
    return !more.some(key => held.has(key));
}

// The steps that make `was` into `now`, done in order to a ListModel: the
// rows gone, last first, as { remove, count }, so each place counts from
// the rows still there; then each row in its place, as { move, to }, or
// { insert, keys }, the rows in a run the list didn't hold, put in at
// once. A step into Emoji lists 1,924 rows, which went in one at a time,
// 16 to 20 ms in Qt's engine offscreen against a millisecond at once, after
// 28 to 34 ms tagging and placing each, where nothing listed before stays
// (2026-11-11). So two lists sharing no row are told apart first, in under
// a millisecond, and take one step each way. A row that moves is moved up
// into its place, or, in a staying row's way, down to just before the
// next row listed after it now that stays or has already moved down, or
// the end. None where the two are the same.
function steps(was, now) {
    if (apart(was, now)) {
        return (was.length > 0 ? [{ remove: 0, count: was.length }] : [])
            .concat(now.length > 0 ? [{ insert: 0, keys: now.slice() }] : []);
    }
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
    const holds = new Set(held);
    for (let at = 0; at < into.length; at += 1) {
        // A run of rows the list didn't hold goes in as one.
        if (!holds.has(into[at])) {
            let end = at + 1;
            while (end < into.length && !holds.has(into[end])) {
                end += 1;
            }
            done.push({ insert: at, keys: now.slice(at, end) });
            held.splice(at, 0, ...into.slice(at, end));
            at = end - 1;
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

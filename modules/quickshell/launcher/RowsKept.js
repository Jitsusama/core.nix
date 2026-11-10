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

// The steps that make `was` into `now`, done in order to a ListModel: the
// rows gone, last first, as { remove, count }, so each place counts from
// the rows still there; then each row in its place, as { move, to }, a row
// held further down moved up, or { insert, key }, one the list didn't
// hold. None where the two are the same.
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
    for (let at = 0; at < into.length; at += 1) {
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

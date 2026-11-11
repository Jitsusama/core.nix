// The rows a daemon serves ready (Y2YBTS's ready-row form), kept in the
// order each row's `order` says, so a change places only the rows it names,
// each found its place by halving, and nothing sorts the rest. The daemon
// spaces its orders and keeps them from one change to the next, so a row
// that moves is said alone. Wireless.qml keeps wifid's rows by it, and
// tests/placed.qml runs it in Qt's own engine.
.pragma library

// The keys in their order once a message said the rows `changed` anew and
// let those `gone` go, the rows themselves held by key in `held`: a new
// list, so whatever reads it is told, and the one before left as it was.
function placed(keys, held, changed, gone) {
    const moved = new Set(changed.concat(gone));
    const now = keys.filter(key => !moved.has(key));
    for (const key of changed.filter(named => held[named])) {
        now.splice(after(now, held, held[key].order), 0, key);
    }
    return now;
}

// Where a row of order `order` goes among `keys`: after every row ordered
// at or before it, found by halving.
function after(keys, held, order) {
    let low = 0;
    let high = keys.length;
    while (low < high) {
        const middle = Math.floor((low + high) / 2);
        if (held[keys[middle]].order <= order) {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    return low;
}

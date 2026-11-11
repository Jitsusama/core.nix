// The rows a daemon serves ready (Y2YBTS's ready-row form), kept in the
// order each row's `order` says, so a change places only the rows it names,
// each found its place by halving, and nothing sorts the rest. The daemon
// spaces its orders and keeps them from one change to the next, so a row
// that moves is said alone. Machine.qml keeps machine's rows by it, and
// Wireless.qml each network's acts as made, and tests/placed.qml runs it
// in Qt's own engine.
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

// The keys under each parent, by the parent's key ("" for a place's own
// rows), once a message said the rows `changed` anew and let those `gone`
// go: for rows served in the crate's wrapped form, each record's fields
// under `row`, as the machine daemon serves Machine's, whose parts are a
// step in. Each is placed among its siblings as `placed` places them, so
// nothing sorts, and a parent whose rows are unmoved keeps its list.
function nested(groups, held, changed, gone) {
    const moved = new Set(changed.concat(gone));
    const now = {};
    for (const parent of Object.keys(groups)) {
        const kept = groups[parent].some(key => moved.has(key))
            ? groups[parent].filter(key => !moved.has(key)) : groups[parent];
        if (kept.length > 0) {
            now[parent] = kept;
        }
    }
    const rows = {};
    for (const key of Object.keys(held)) {
        rows[key] = held[key]?.row;
    }
    for (const key of changed.filter(named => rows[named])) {
        const parent = rows[key].parent ?? "";
        // A list kept from before is copied, so the one before stays as it was.
        const keys = now[parent] === undefined ? []
            : now[parent] === groups[parent] ? now[parent].slice() : now[parent];
        keys.splice(after(keys, rows, rows[key].order), 0, key);
        now[parent] = keys;
    }
    return now;
}

// What `make` made for `key` the last time it was asked, while each of
// `from`, what it was made from, is the very one it was then; else made
// again and kept in `kept`, by key, for the next time. A row served
// stays the one object until the daemon says it changed, so whatever's
// made of it is made once for each change rather than once for each ask.
function madeOnce(kept, key, from, make) {
    const before = kept[key];
    if (before && before.from.length === from.length
        && before.from.every((one, at) => one === from[at])) {
        return before.made;
    }
    const made = make();
    kept[key] = { from, made };
    return made;
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

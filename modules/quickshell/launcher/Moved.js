// Whether a row a daemon tells again only moved, so ReadyRows.qml keeps
// what it made of it (Y2YBTS, 2026-11-11). Apart from ReadyRows.js, which
// is at the length a file may be, and tests/ready-rows.qml holds it.
.pragma library

// Whether a row told again says all it said when made, `order` aside, so it
// only moves: a daemon making room tells every row it moved, remaking some
// 103 of Sound's rows every ten notification streams (1KZFHC, 2026-11-11).
function movedOnly(was, now) {
    return Boolean(was && now) && Object.keys(was).length === Object.keys(now).length
        && Object.keys(now).every(name => name === "order"
            ? Object.keys(was).includes(name) : alike(was[name], now[name]));
}

// Whether two values a daemon said are alike, as their JSON reads.
function alike(one, other) {
    if (one === other || typeof one !== "object" || typeof other !== "object" || !one
        || !other || Array.isArray(one) !== Array.isArray(other)) {
        return one === other;
    }
    const names = Object.keys(one);
    const theirs = Object.keys(other);
    return names.length === theirs.length
        && names.every(name => theirs.includes(name) && alike(one[name], other[name]));
}

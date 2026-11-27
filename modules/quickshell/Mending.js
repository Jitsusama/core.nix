// How a daemon's record held is mended by a change saying only what
// differs, as mycelium's node says one (Daemon.qml), and which of a
// kind's fields a change says deep. Apart from the shell, so
// tests/mending.qml holds it in Qt's own engine.
.pragma library

function plain(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

// A record held with a change laid over it: a new object, so a reader
// holding the old one still reads it as it was, each field the change
// says replacing the held one's, and each it says as null let go. A field
// said deep, an object both held and said, is mended the same way one
// level in, as the node says it: only what moved inside it. What didn't
// move is the very object it was, so comparing it costs nothing.
function mended(was, change, deep) {
    const now = Object.assign({}, was);
    for (const field of Object.keys(change)) {
        const said = change[field];
        if (said === null) {
            delete now[field];
        } else if (deep.includes(field) && plain(now[field]) && plain(said)) {
            now[field] = mended(now[field], said, []);
        } else {
            now[field] = said;
        }
    }
    return now;
}

// The fields each kind's changes say deep: those a watch asked deep that
// the daemon's hello offers for the kind, as `{"session": {"deep":
// ["presence"]}}`. A daemon offering none, or a watch asking none, says
// every change whole, so nothing is mended deep. Kept with no prototype,
// so a kind the hello names "__proto__" or "constructor" is a kind of
// its own, not what every object inherits.
function deepByKind(kinds, asked) {
    const by = Object.create(null);
    if (!(plain(kinds) && Array.isArray(asked))) {
        return by;
    }
    for (const kind of Object.keys(kinds)) {
        const offered = kinds[kind]?.deep;
        if (Array.isArray(offered)) {
            by[kind] = asked.filter(field => offered.includes(field));
        }
    }
    return by;
}

// The kind of a record a change names: the one it says, or else its key's
// before the first colon, as `session:kestrel` is a session's.
function kindOf(record) {
    if (typeof record.kind === "string") {
        return record.kind;
    }
    const at = record.key.indexOf(":");
    return at < 0 ? record.key : record.key.slice(0, at);
}

// The fields `record`'s change says deep, by its kind. Only a list is
// taken, since `by` may be a plain object a caller made: a record keyed
// "constructor" or "toString", or saying such a kind, names a kind it
// inherits a function for, and mended() would throw on that, holding
// the change's records before it unheard and losing those after.
function deepOf(by, record) {
    const deep = by[kindOf(record)];
    return Array.isArray(deep) ? deep : [];
}

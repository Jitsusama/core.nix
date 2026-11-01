// The order of the notices' stack, by their ids, the front first, worked
// out apart from the notices (Notifications.qml) so tests/stack.qml can
// hold it in Qt's own engine.
.pragma library

// The stack as it shows: those it knows, in its order, while they're
// still live, then any live one it never heard arrive, as one kept across
// a reload of the shell does, at the back.
function shown(order, live) {
    const known = order.filter((id) => live.includes(id));
    return known.concat(live.filter((id) => !known.includes(id)));
}

// The order once a notice arrives: a critical one goes to the front, and
// the rest in the order they came, so a newer one waits its turn behind
// the one before it.
function arrived(order, id, urgent) {
    return urgent ? [id].concat(order) : order.concat([id]);
}

// The stack turned, so the next comes to the front, or, turned back, the
// one before. A stack of one or none stays as it is.
function turned(stack, by) {
    if (stack.length < 2) {
        return stack;
    }
    return by > 0 ? stack.slice(1).concat(stack.slice(0, 1)) : stack.slice(-1).concat(stack.slice(0, -1));
}

// The stack turned until one is at the front, as turning by the keys
// would bring it, so the rest keep their turn after it. One it doesn't
// hold leaves it as it is.
function turnedTo(stack, id) {
    const at = stack.indexOf(id);
    if (at < 1) {
        return stack;
    }
    return stack.slice(at).concat(stack.slice(0, at));
}

// The stack with one brought to the front, the rest as they were.
function forward(stack, id) {
    return [id].concat(stack.filter((other) => other !== id));
}

.pragma library

// The names that counted most, as the launcher's journal lines say which
// sources made rows and which changed: the most first, then by name, each
// with its count, at most `few` of them, and none counted nothing.
// modules/quickshell/tests/most.qml holds it in Qt's engine.

// "name count" for each of the most, joined by commas, or "" for none.
function said(counts, few) {
    return Object.keys(counts)
        .filter(name => counts[name] > 0)
        .sort((a, b) => counts[b] - counts[a] || a.localeCompare(b))
        .slice(0, few)
        .map(name => `${name} ${counts[name]}`)
        .join(", ");
}

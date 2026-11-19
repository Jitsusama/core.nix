.pragma library

// A collection's own rows beside the parts its vocabulary names, as a step
// into it lists them (Launcher.qml's listedWhole), each thing once. The
// cursor is found by key, so a key listed twice left one row for the other
// as the rows were laid out again: Agents listed Spec among its own rows
// and as a part, and it came and went where Joel was looking (2026-11-18).

// What a row is, for telling two rows apart, as Rank.js tells them.
function identity(row) {
    return row.is ?? row.key ?? "";
}

// The rows, with the parts after them, or before them where they lead; a
// part the collection lists itself stands where it lists it. Rows with no
// key are never taken for each other.
function withParts(members, parts, partsLead) {
    const listed = Object.create(null);
    for (const row of members) {
        const id = identity(row);
        if (id !== "") {
            listed[id] = true;
        }
    }
    const own = parts.filter(row => {
        const id = identity(row);
        return id === "" || listed[id] !== true;
    });
    return partsLead ? own.concat(members) : members.concat(own);
}

.pragma library

// What a place's row says, at home and wherever else it stands, as one
// rule (Joel, 2026-11-08 19:58: "i don't like how the title and values on
// the home screen differ from every other sub-menu"; VKCF3Z's ruling,
// 2a429f67): after its name, what it holds by its members' own names, as
// many whole as fit and never an ellipsis; at its right, how it stands,
// as that row reads inside its parent, or else how many it holds.

// The room a row's name and its side leave apart: the two spaces the list
// keeps between what's said after a name and what's said at its right.
const BETWEEN = "  ".length;

// Names as one line, as many whole as fit in `room` characters, ending on
// the last that fits with nothing after it (VKCF3Z, 2026-11-08: "no
// ellipsis anywhere"). The row gives way by whole names again where its
// face is wider than a character a letter (Reading.wholeNamesFromEnd).
function listed(cased, room) {
    let said = "";
    for (const name of cased) {
        const next = said === "" ? name : `${said}, ${name}`;
        if (next.length > room) {
            return said;
        }
        said = next;
    }
    return said;
}

// What a place's row says after its name: its members' names in the room
// its `name` and what it `says` at its right leave in a row of `room`
// characters, or `none` where it holds nothing. A row whose first name
// doesn't fit says nothing there, since its right already says how many
// it holds.
function holds(names, row) {
    if (names.length === 0) {
        return row.none ?? "";
    }
    const beside = row.name.length + (row.says === "" ? 0 : row.says.length + BETWEEN);
    return listed(names, row.room - beside);
}

// What a place's row says at its right: how it stands, where it says
// (Processor's "99% busy", Power's charge, how Agents' sessions stand),
// or else how many it holds. One whose name is followed by its word for
// holding none, as Media's "none", says no count there, which would only
// say "empty" again.
function side(stands, counted, saidNone = false) {
    if (stands !== null && stands !== undefined && stands !== "") {
        return stands;
    }
    return saidNone ? "" : counted;
}

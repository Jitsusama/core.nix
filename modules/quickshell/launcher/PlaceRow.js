.pragma library

// What a place's row says, at home and wherever else it stands, as one
// rule (Joel, 2026-11-08 19:58: "i don't like how the title and values on
// the home screen differ from every other sub-menu"; VKCF3Z's ruling,
// 2a429f67): nothing after its name, as no other row says there (Joel,
// 2026-11-09: "they still have a lot more text right beside the title
// than most do"; VKCF3Z, dae7fb67); at its right, how it stands, as that
// row reads inside its parent; and what it holds, by its members' own
// names, in its drawer.

// Names as one line, as many whole as fit in `room` characters, ending on
// the last that fits with nothing after it (VKCF3Z, 2026-11-08: "no
// ellipsis anywhere"), as Now's rows say their parts.
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

// What a place's row says at its right: how it stands, where it says
// (Processor's "99% busy", Power's charge, how Agents' sessions stand);
// else its word for holding none, as Media's "none", while it holds
// nothing; else how many it holds, `counted`, which a place that only
// groups others (Machine, Network, Desktop) gives as "", since a count of
// menus tells Joel nothing (VKCF3Z, dae7fb67).
function side(stands, counted, none = "") {
    if (stands !== null && stands !== undefined && stands !== "") {
        return stands;
    }
    return none !== "" ? none : counted;
}

// A place's members as the run they are, joined by middle dots (VKCF3Z,
// 00ca9231: "Power · Bluetooth · Sound"). The drawer lays it
// as a run (`runs`), wrapping between two members with the dot going
// with the break, so no line ends or starts on one and Desk Light stays
// whole (VKCF3Z, 8e0712aa).
function run(names) {
    return names.map(String).join(" \u00b7 ");
}

// A place's drawer: its eyebrow saying it's a place, as Programs' says
// "program", and what it holds by its members' own names, in its own
// order, so one key shows what's inside without stepping in (VKCF3Z,
// dae7fb67, 00ca9231). A place holding things of one kind, Keyboard's
// keys or Programs' programs, says what they are in a sentence, `holding`,
// rather than naming each. Null where it holds nothing to name.
function drawer(name, names, eyebrow, holding = "") {
    if (holding !== "") {
        return { eyebrow, title: name, body: holding };
    }
    return names.length === 0 ? null
        : { eyebrow, title: name, body: run(names), runs: true };
}

// What a row being read again says, at its right and below its name, as a
// reading not back yet says it (Displays' Room Light, Recent's rows).
const READING = "\u2026";

// Home's rows as the launcher keeps them while it's shut, for the next
// opening to draw at once (Launcher.qml's keepHome): each where it stood,
// by the same name and key, so the rows read again land where they are
// and nothing moves in; but what each said of how it stands, at its right
// or below its name, is as old as the opening that read it, so it says
// it's being read rather than showing that as now (Y2YBTS, 2026-11-10).
// A row that said nothing there still says nothing.
function kept(rows) {
    return rows.map(row => {
        const stands = (row.side ?? "") !== "" || (row.sideItems ?? []).length > 0;
        const noted = (row.note ?? "") !== "";
        if (!stands && !noted) {
            return row;
        }
        return Object.assign({}, row, {
            side: stands ? READING : row.side,
            sideItems: [],
            note: noted ? READING : row.note,
            kept: true
        });
    });
}

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
// 00ca9231: "Power · Bluetooth · Sound"). Each name's own spaces don't
// break and the space before each dot doesn't either, so a line breaks
// only between members, after a dot, as Desk Light stays whole.
const within = / /g;
function run(names) {
    return names.map(name => String(name).replace(within, "\u00a0")).join("\u00a0\u00b7 ");
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
    return names.length === 0 ? null : { eyebrow, title: name, body: run(names) };
}

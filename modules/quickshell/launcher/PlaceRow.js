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

// A place's drawer: what it holds by its members' own names, in its own
// order, so one key shows what's inside without stepping in, or null where
// it holds nothing to name (VKCF3Z, dae7fb67).
function drawer(name, names) {
    return names.length === 0 ? null : { title: name, body: names.join(", ") };
}

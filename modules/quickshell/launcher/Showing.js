// What the launcher shows, asked from a terminal (`launcher show`, the
// program in ../in-a-terminal/, over `qs ipc call launcher show`): which
// row each step of a path names, the names nearest a step that names
// none, and each row and its drawer as plain JSON, so the terminal prints
// what the launcher would say and nothing a provider doesn't. Launcher.qml
// walks its own rows by these rules; tests/showing.qml holds them in Qt's
// own engine.
.pragma library

// How a step and a name are compared: in any case, a hyphen or an
// underscore as a space, so `start-a-session` names Start a Session.
function plain(text) {
    return String(text ?? "").toLowerCase().replace(/[-_\s]+/g, " ").trim();
}

// How many names a miss offers at most.
const NEAREST = 3;

// What share of the typed letters a near name may be apart by: a third.
const LEEWAY = 3;

// The row a step names among `rows`: the one whose steady id it is
// (`idOf`), else whose name it is, else the only one whose name starts
// with it. Gives { row }, or { near }, the names it might have meant,
// nearest first, where it names none or starts more than one.
function step(rows, said, idOf) {
    const named = rows.filter(row => (row?.name ?? "") !== "");
    const byId = named.find(row => idOf(row) === said);
    if (byId) {
        return { row: byId };
    }
    const wanted = plain(said);
    const byName = named.find(row => plain(row.name) === wanted);
    if (byName) {
        return { row: byName };
    }
    const starting = named.filter(row => plain(row.name).startsWith(wanted));
    if (starting.length === 1 && wanted !== "") {
        return { row: starting[0] };
    }
    if (starting.length > 1 && wanted !== "") {
        return { near: unique(starting.map(row => row.name)).slice(0, NEAREST) };
    }
    return { near: nearest(named.map(row => row.name), wanted) };
}

function unique(names) {
    return names.filter((name, at) => names.indexOf(name) === at);
}

// The names within a few letters of what was typed, nearest first: a
// letter added, dropped, changed or two swapped each count one, and a
// name is near when its start is within a third of the typed letters,
// at least one, so "mahcine" finds Machine and "dsk" finds Disk.
function nearest(names, wanted) {
    if (wanted === "") {
        return [];
    }
    const most = Math.max(1, Math.floor(wanted.length / LEEWAY));
    return unique(names)
        .map(name => ({ name, apart: apartAtStart(plain(name), wanted) }))
        .filter(one => one.apart <= most)
        .sort((one, other) => one.apart - other.apart || one.name.localeCompare(other.name))
        .slice(0, NEAREST)
        .map(one => one.name);
}

// How far `wanted` is from the name's start, or from the whole name, the
// nearer: "mach" is no distance from Machine, and "mahcine" one swap.
function apartAtStart(name, wanted) {
    const whole = apart(name, wanted);
    const start = apart(name.slice(0, wanted.length), wanted);
    return Math.min(whole, start);
}

// The optimal string alignment distance between two words.
function apart(one, other) {
    const rows = [];
    for (let at = 0; at <= one.length; at += 1) {
        rows.push([at]);
    }
    for (let to = 1; to <= other.length; to += 1) {
        rows[0][to] = to;
    }
    for (let at = 1; at <= one.length; at += 1) {
        for (let to = 1; to <= other.length; to += 1) {
            const same = one[at - 1] === other[to - 1] ? 0 : 1;
            let best = Math.min(rows[at - 1][to] + 1, rows[at][to - 1] + 1,
                rows[at - 1][to - 1] + same);
            if (at > 1 && to > 1 && one[at - 1] === other[to - 2]
                && one[at - 2] === other[to - 1]) {
                best = Math.min(best, rows[at - 2][to - 2] + 1);
            }
            rows[at][to] = best;
        }
    }
    return rows[one.length][other.length];
}

// Words a row may say, as a string or nothing: a getter's or a number's
// are read as the launcher draws them.
function words(value) {
    if (value === null || value === undefined || typeof value === "function") {
        return "";
    }
    return String(value);
}

// What a row is to a terminal: a figure, which only says a value, else
// what the catalogue calls it (Catalogue.js's kindOf), a place or an
// action, as `kind` gives it.
function kindOf(row, kind) {
    return row.figure === true || row.answered === true ? "figure" : kind;
}

// One row as plain JSON, every key a word words.json uses for it: its id
// (its path's steady ids joined as the catalogue's are), its key, name,
// glyph and hue, the count after its name, its side and its note, what
// Return does (verb), what it is (kind), whether a step into it holds
// more, whether its side says something needs Joel now (alerts) or soon
// (warns), as its side is inked, and its drawer where it has one. A word
// it doesn't say is left out rather than given empty.
function shaped(one, at) {
    const out = { id: at.id, name: words(one.name), kind: kindOf(one, at.kind),
        holds: at.holds === true };
    for (const key of ["key", "glyph", "hue", "count", "side", "note"]) {
        const said = words(one[key]);
        if (said !== "") {
            out[key] = said;
        }
    }
    if (at.verb !== "") {
        out.verb = at.verb;
    }
    for (const key of ["alerts", "warns"]) {
        if (one[key] === true) {
            out[key] = true;
        }
    }
    const plate = at.drawer ? drawer(at.drawer) : null;
    if (plate !== null) {
        out.drawer = plate;
    }
    return out;
}

// A drawer as plain JSON (Drawer.qml's shape): its eyebrow, title and
// body, its facts each a label and a value, a value not read yet null as
// the drawer's placeholder stands for, a heading as its own, and the
// moment's groups of facts; a picture by its path. A styled body is said
// as its words, its markup taken away.
function drawer(plate) {
    const out = {};
    for (const key of ["eyebrow", "title"]) {
        const said = words(plate[key]);
        if (said !== "") {
            out[key] = said;
        }
    }
    const body = plate.styled === true ? unstyled(words(plate.body ?? plate.say))
        : words(plate.body ?? plate.say);
    if (body.trim() !== "") {
        out.body = body.trim();
    }
    const facts = (plate.facts ?? []).map(fact).filter(one => one !== null);
    if (facts.length > 0) {
        out.facts = facts;
    }
    const groups = (plate.groups ?? []).map(group => (group ?? []).map(fact)
        .filter(one => one !== null)).filter(group => group.length > 0);
    if (groups.length > 0) {
        out.groups = groups;
    }
    const picture = words(plate.picture);
    if (picture !== "") {
        out.picture = picture;
    }
    return Object.keys(out).length > 0 ? out : null;
}

// One fact: a heading, or a label and its value, with its qualifier where
// it has one; null for none, as a fact left out is.
function fact(one) {
    if (!one) {
        return null;
    }
    if (one.heading !== undefined) {
        return { heading: words(one.heading) };
    }
    const value = one.value === null || one.value === undefined ? null : words(one.value);
    const out = { label: words(one.label), value };
    const qualifier = words(one.qualifier);
    if (qualifier !== "") {
        out.qualifier = qualifier;
    }
    return out;
}

// Styled text's words, its tags taken away and its entities read.
function unstyled(text) {
    return text.replace(/<br\s*\/?>/gi, "\n").replace(/<[^>]*>/g, "")
        .replace(/&nbsp;/g, " ").replace(/&lt;/g, "<").replace(/&gt;/g, ">")
        .replace(/&quot;/g, "\"").replace(/&#39;/g, "'").replace(/&amp;/g, "&");
}

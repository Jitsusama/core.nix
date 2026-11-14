// What For You narrows to when Joel types :filter, as VKCF3Z ruled for it
// (2026-11-08): a filter is a step in, never a state, so `-` comes back
// out to everything and nothing carries across launches. Its words are a
// kind (do, look or try) and a source, by its id in lowercase; two words
// narrow to both, and the head joins their names, "Mail · To Look At". A
// row says both under its `forYou` (Now.qml), and the sources are read
// from the rows, never a fixed table, since more come. Later's members and
// History say neither, so any filter leaves them out: `:filter do` shows
// only what can be done now, and `-` brings the rest back. Apart from the
// launcher, so tests/for-you-filter.qml can hold it in Qt's own engine.
.pragma library
.import "Order.js" as Order

// The kinds a filter names, in the order For You's bands are designed in.
const KINDS = ["do", "look", "try"];
// For You's own sources that give it one kind alone, so they're no source
// to filter by: their kind already reaches them, and typed, each answers as
// it (VKCF3Z, 2026-11-08). Every other source is the world's.
const AS_KIND = { "needs-you": "do", looks: "look" };
const FIELDS = ["kind", "source"];

const SPACES = /\s+/;
const ENDS_IN_SPACE = /\s$/;
// What the head puts between a filter's names, as it does between a
// place's parts.
const JOINED = " · ";

// Where the vocabulary names a filter's words (words.json): each kind as
// for-you:kind:<kind>, and each source it knows as for-you:source:<id>.
const KIND_ID = "for-you:kind:";
const SOURCE_ID = "for-you:source:";

// The mark each word wears in Tab's menu, by the theme's name for it, so
// the menu teaches what each word means (VKCF3Z, 2026-11-08). A source
// that's a program, as Slack is, wears its program's own mark. One the
// table doesn't know wears For You's own tray, never the to-do's, which
// would read as a to-do (VKCF3Z, 2026-11-08).
const MARKS = {
    kind: { do: "waitsOnJoel", look: "look", try: "toTry" },
    source: { mail: "mail", calendar: "calendar", desktop: "desktop", sessions: "agent",
        pineapple: "stopped" }
};
const PROGRAMS = { slack: "slack" };
const OWN_MARK = "forYou";

// The names a filter's head says, from the vocabulary's ids and how it
// names each: { kinds, sources }, each by its word.
function saidFrom(ids, nameOf) {
    const sources = {};
    for (const id of ids.filter(one => one.startsWith(SOURCE_ID))) {
        sources[id.slice(SOURCE_ID.length)] = nameOf(id);
    }
    const kinds = {};
    for (const kind of KINDS) {
        kinds[kind] = nameOf(`${KIND_ID}${kind}`);
    }
    return { kinds, sources };
}

// What a row says of a field for For You, or "" for a row that isn't one
// of its items, as Later and History aren't.
function fieldOf(row, field) {
    return String(row?.forYou?.[field] ?? "");
}

function wordsOf(arg) {
    return (arg ?? "").toLowerCase().split(SPACES).filter(word => word !== "");
}

// The sources Joel could name: each one a row comes from, and each the
// words name, so a source with nothing in it now is still a step into an
// empty place rather than an error.
function sourcesOf(rows, said) {
    const known = Object.keys(said?.sources ?? {});
    return known.concat(rows.map(row => fieldOf(row, "source"))).filter(source => source !== "");
}

// What a word names, as [field, value], or null for neither: a source of
// one kind alone names its kind, before any source of its name.
function meaning(word, sources) {
    if (KINDS.includes(word)) {
        return ["kind", word];
    }
    if (word in AS_KIND) {
        return ["kind", AS_KIND[word]];
    }
    return sources.includes(word) ? ["source", word] : null;
}

// What the words narrow `from` to (a filter, or none for everything), as
// { kind, source }, each "" where nothing names one; or { error } for a
// word that's neither a kind nor a source, or a second of one other than
// the first, which could only narrow to nothing. Vim's own E475 says it,
// as Ex.js says what Vim says.
function read(arg, rows, said, from) {
    const sources = sourcesOf(rows, said);
    const filter = { kind: from?.kind ?? "", source: from?.source ?? "" };
    for (const word of wordsOf(arg)) {
        const [field, value] = meaning(word, sources) ?? ["", ""];
        if (field === "" || (filter[field] !== "" && filter[field] !== value)) {
            return { error: `E475: Invalid argument: ${word}` };
        }
        filter[field] = value;
    }
    return filter;
}

// What `:filter` steps into: as `read` reads it, but a line with no words
// is Vim's E471, since a filter that names nothing is everything, which
// `-` already goes back to.
function asked(arg, rows, said, from) {
    return wordsOf(arg).length === 0 ? { error: "E471: Argument required" }
        : read(arg, rows, said, from);
}

// Whether a row stands in a filter: of its kind, where it names one, and
// from its source, where it names one.
function holds(filter, row) {
    return FIELDS.every(field => filter[field] === "" || fieldOf(row, field) === filter[field]);
}

// The rows a filter keeps, in the order For You lists them.
function filtered(rows, filter) {
    return rows.filter(row => holds(filter, row));
}

// A source's name as the head says it: the words' own, or its id with each
// part begun in capitals, for a source the words don't know yet.
function sourceName(source, said) {
    const own = said?.sources?.[source];
    if (own) {
        return own;
    }
    return source.split("-").map(part => part.charAt(0).toUpperCase() + part.slice(1)).join(" ");
}

// What the head says of a filter, after For You: the source, then the
// kind, with the head's own mark between them.
function heading(filter, said) {
    const names = [];
    if (filter.source !== "") {
        names.push(sourceName(filter.source, said));
    }
    if (filter.kind !== "") {
        names.push(said?.kinds?.[filter.kind] ?? filter.kind);
    }
    return names.join(JOINED);
}

// The mark an offer wears: { glyph } by the theme's name, or { program }
// by the desktop entry whose mark it wears.
function markOf(one) {
    if (one.field === "source" && one.value in PROGRAMS) {
        return { program: PROGRAMS[one.value] };
    }
    return { glyph: MARKS[one.field]?.[one.value] ?? OWN_MARK };
}

// A filter's own key, so two steps into one filter are one place.
function keyOf(filter) {
    return `${filter.source}:${filter.kind}`;
}

// What Tab offers after `:filter `: each kind and source not yet named,
// in `from` or the words already typed, that would keep something, with
// how many it would keep. Kinds come in their bands' order, then sources
// A to Z by the name the head would give them. Each reads as a row:
// the word, its name as the head would say it, how many it keeps at the
// side, and which field it fills, so the menu can give it that field's
// mark (VKCF3Z, 2026-11-08). The word being typed is the last, which
// Commands.ownArguments narrows these to.
function offered(rows, arg, said, from) {
    const typed = wordsOf(arg);
    const done = ENDS_IN_SPACE.test(arg ?? "") ? typed : typed.slice(0, -1);
    const so = read(done.join(" "), rows, said, from);
    if (so.error) {
        return [];
    }
    const kept = filtered(rows, so);
    const count = (field, word) => kept.filter(row => fieldOf(row, field) === word).length;
    const kinds = so.kind !== "" ? []
        : KINDS.map(kind => ({ value: kind, field: "kind", count: count("kind", kind),
            note: said?.kinds?.[kind] ?? kind }));
    const present = so.source !== "" ? []
        : Array.from(new Set(kept.map(row => fieldOf(row, "source"))
            .filter(one => one !== "" && !(one in AS_KIND))));
    const sources = Order.sorted(present, Order.byName(source => sourceName(source, said)))
        .map(source => ({ value: source, field: "source", count: count("source", source),
            note: sourceName(source, said) }));
    return kinds.concat(sources)
        .filter(one => one.count > 0)
        .map(one => ({ value: one.value, field: one.field, note: one.note,
            side: String(one.count) }));
}

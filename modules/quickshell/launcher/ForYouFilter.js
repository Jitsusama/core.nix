// What For You narrows to when Joel types :filter, as VKCF3Z ruled for it
// (2026-11-08): a filter is a step in, never a state, so `-` comes back
// out to everything and nothing carries across launches. Its words are a
// kind (do, look or try) and a source, by its id in lowercase; two words
// narrow to both, and the head joins their names, "Mail · To Look At". The
// sources are read from the items, never a fixed table, since more come.
// Apart from the launcher, so tests/for-you-filter.qml can hold it in Qt's
// own engine.
.pragma library
.import "Order.js" as Order

// The kinds a filter names, in the order For You's bands are designed in.
const KINDS = ["do", "look", "try"];

const SPACES = /\s+/;
const ENDS_IN_SPACE = /\s$/;
// What the head puts between a filter's names, as it does between a
// place's parts.
const JOINED = " · ";

function wordsOf(arg) {
    return (arg ?? "").toLowerCase().split(SPACES).filter(word => word !== "");
}

// The sources Joel could name: each one an item comes from, and each the
// words name, so a source with nothing in it now is still a step into an
// empty place rather than an error.
function sourcesOf(items, said) {
    const named = Object.keys(said?.sources ?? {});
    return named.concat(items.map(item => item.source)).filter(source => !!source);
}

// What `:filter` was given, as { kind, source }, each "" where it names
// none; or { error } for a word that's neither a kind nor a source, or a
// second of one, which could only narrow to nothing. Vim's own E475 says
// it, as Ex.js says what Vim says.
function read(arg, items, said) {
    const sources = sourcesOf(items, said);
    const filter = { kind: "", source: "" };
    for (const word of wordsOf(arg)) {
        const field = KINDS.includes(word) ? "kind" : sources.includes(word) ? "source" : "";
        if (field === "" || filter[field] !== "") {
            return { error: `E475: Invalid argument: ${word}` };
        }
        filter[field] = word;
    }
    return filter;
}

// Whether an item stands in a filter: of its kind, where it names one, and
// from its source, where it names one.
function holds(filter, item) {
    return (filter.kind === "" || item.kind === filter.kind)
        && (filter.source === "" || item.source === filter.source);
}

// The items a filter keeps, in the order For You lists them.
function filtered(items, filter) {
    return items.filter(item => holds(filter, item));
}

// A source's name as the head says it: the words' own, or its id with each
// part begun in capitals, for a source the words don't know yet.
function sourceName(source, said) {
    const named = said?.sources?.[source];
    if (named) {
        return named;
    }
    return source.split("-").map(part => part.charAt(0).toUpperCase() + part.slice(1)).join(" ");
}

// What the head says after "For You ›": the source, then the kind, joined
// with the head's own mark between them.
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

// What Tab offers after `:filter `: each kind and source not yet named
// that would keep something beside the words already typed, with how many
// it would keep. Kinds come in their bands' order, then sources A to Z by
// the name the head would give them. The word being typed is the last,
// which Commands.ownArguments narrows these to.
function offered(items, arg, said) {
    const typed = wordsOf(arg);
    const done = ENDS_IN_SPACE.test(arg ?? "") ? typed : typed.slice(0, -1);
    const so = read(done.join(" "), items, said);
    if (so.error) {
        return [];
    }
    const kept = filtered(items, so);
    const count = (field, word) => kept.filter(item => item[field] === word).length;
    const kinds = so.kind !== "" ? []
        : KINDS.map(kind => ({ value: kind, note: count("kind", kind) }));
    const present = so.source !== "" ? []
        : Array.from(new Set(kept.map(item => item.source).filter(source => !!source)));
    const sources = Order.sorted(present, Order.byName(source => sourceName(source, said)))
        .map(source => ({ value: source, note: count("source", source) }));
    return kinds.concat(sources)
        .filter(one => one.note > 0)
        .map(one => ({ value: one.value, note: String(one.note) }));
}

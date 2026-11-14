// How the launcher joins every source's rows into the one list a search
// finds from, and the indexes it reads that list by. An opening joins it
// again each time a daemon answers, and Joel's openings spent about 26 ms
// a time joining (perf, 2026-11-14), most of it remaking the rows of
// sources that hadn't changed and the indexes in three walks. Apart from
// Launcher.qml, so tests/joined.qml can hold it in Qt's own engine.
.pragma library

// How many names said plainly are kept beyond twice the rows, so a short
// list never clears them at each joining.
const NAMES_SPARE = 64;

// A source's rows as the search finds them, each made ready by `ready`
// (Launcher.qml's withSynonyms), but for a desktop action another row
// stands for. While its things are the same list and the stand-ins are
// too, the rows the last joining kept (`kept.before`) are its rows, made
// ready once: a source's things are a binding, so a change makes a new
// list. Each is kept in `kept.now`, which the next joining reads as its
// `before`, so a source that's gone is let go.
function rowsOf(source, standIns, kept, ready) {
    const was = kept.before.get(source);
    if (was !== undefined && was.things === source.things && was.standIns === standIns) {
        kept.now.set(source, was);
        return was.rows;
    }
    const rows = [];
    for (const row of source.things) {
        if (!(row.key.startsWith("action:") && standIns.includes(row.name))) {
            rows.push(ready(row));
        }
    }
    kept.now.set(source, { things: source.things, standIns, rows });
    return rows;
}

// Every row by its key (`keyed`); the keys of the rows named each name,
// by the name as `plainly` says it (`namedAs`); and the rows that are
// parts of another, by the key of what they're part of (`partsOfRows`).
// Made in one walk, each name said plainly once for as long as `plain`
// holds it: a joining names the same rows the one before it did.
function indexes(choices, plainly, plain) {
    const keyed = {};
    const namedAs = {};
    const partsOfRows = {};
    for (const choice of choices) {
        keyed[choice.key] = choice;
        let name = plain.get(choice.name);
        if (name === undefined) {
            name = plainly(choice.name);
            plain.set(choice.name, name);
        }
        if (namedAs[name] === undefined) {
            namedAs[name] = [];
        }
        namedAs[name].push(choice.key);
        if (choice.of) {
            if (partsOfRows[choice.of] === undefined) {
                partsOfRows[choice.of] = [];
            }
            partsOfRows[choice.of].push(choice);
        }
    }
    // The names said plainly are kept while they're about as many as the
    // rows, so names that have gone, as windows' titles do, don't pile up.
    if (plain.size > 2 * choices.length + NAMES_SPARE) {
        plain.clear();
    }
    return { keyed, namedAs, partsOfRows };
}

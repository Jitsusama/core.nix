// What Emoji lists for what's typed while the clipboard's history is
// still asked for its answer, kept apart from launcher/Emoji.qml so
// tests/ready-rows.qml runs it in Qt's own engine: the history finds
// emoji (clipboard/src/search.rs), and between a key and its answer the
// list must neither show rows the letters no longer hold nor put any in a
// place of their own.
.pragma library
.import "../Fuzzy.js" as Fuzzy

// Those of `rows` whose name or meaning holds every letter typed, by the
// letters each prepared text holds, as Fuzzy.js's masks say them.
function holding(rows, typed) {
    const need = Fuzzy.letters(Fuzzy.words(typed));
    const holds = item => !!item && (item.mask & need) === need;
    return rows.filter(row => holds(row.prepared) || holds(row.keywords));
}

// Whether what's typed goes on from the words the last answer was to, so
// that answer's rows may stand while the next is asked; `lastWords` is
// null while no answer has come. Asked before those rows are made, since
// making a hundred for nothing is what Emoji's search saves.
function goesOn(lastWords, typed) {
    const said = Fuzzy.words(typed).join(" ");
    return lastWords !== null && said.startsWith(Fuzzy.words(lastWords).join(" "));
}

// What stands while the answer to `typed` is asked, from `last`, the last
// answer's rows where what's typed goes on from it, else none: those that
// hold every letter typed, so a key takes rows away and puts none in a
// place of its own. Its grade is 0 where any stand, else 3, as a search
// finding nothing grades it.
function whileAsked(last, typed) {
    const rows = holding(last, typed);
    return { rows, grade: rows.length > 0 ? 0 : 3 };
}

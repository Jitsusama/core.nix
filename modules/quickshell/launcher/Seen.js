// Whether a provider's rows can be seen now, so it watches its daemon
// only then rather than from the first time it's asked until the launcher
// closes (docs/design.md: nothing is polled that needn't be; Y2YBTS's
// review of #2441). Derived from what the launcher shows, never written
// as a side effect of answering, so bindings that ask a provider change
// nothing. Apart from the providers so tests/answers.qml holds it in Qt's
// own engine.
.pragma library

// Whether a row is one of the provider's: its own row, the row of a
// place it says how things stand in, as Power's place is said by Power,
// or a row of a collection it is, as Screens' own row is Displays', or
// one of its members found from outside it.
function isOf(row, provider, ids) {
    return !!row && (row.live === provider || row.from === provider
        || ids.includes(row.category?.id ?? ""));
}

// Whether words typed, read plainly, ask one of `questions`, whole or in
// the plural, or start one by three letters or more, as the launcher asks
// "battery" of "batt" (Launcher.qml's answering).
function asks(questions, text) {
    return questions.some(question => question === text || `${question}s` === text
        || (text.length >= 3 && question.startsWith(text)));
}

// Whose the rows found are, as isOf reads a row, kept for the last rows
// found: every provider that watches asks again on every key, and a
// whole place lists rows by the thousand, so six providers each walking
// Emoji's 1914 read them 114,660 times over a lab's 20 keys, about 7 ms
// of each Backspace to its whole list (2026-11-11). This library is one
// for every provider importing it, and the launcher sets new rows found
// rather than changing them in place, so the rows are known again by
// being the same.
let foundLast = { rows: null, from: new Set(), ids: new Set() };

function foundIn(rows) {
    if (foundLast.rows !== rows) {
        const from = new Set();
        const ids = new Set();
        for (const row of rows) {
            if (row) {
                from.add(row.live);
                from.add(row.from);
                ids.add(row.category?.id ?? "");
            }
        }
        foundLast = { rows, from, ids };
    }
    return foundLast;
}

// Whether `provider`'s rows show in the open launcher: its answer, a row
// of it stepped into or found, or a place shown holding one of `ids`, as
// Machine holds Heat, or about to: what's typed asks it, by `asking`, a
// function of the words typed, since its answer is made only from what
// its daemon says. A place's holdings count before their conditions
// hold them back, since a row whose condition is what the daemon says,
// as Heat's is, would otherwise never be read to show. A terminal's `launcher show` counts the rows it
// walked and listed as seen for a few seconds after it asks, where the
// launcher keeps them (its `terminalSees`), so the terminal reads what
// the open launcher would.
function seen(launcher, provider, ids, asking) {
    if (!launcher) {
        return false;
    }
    const mine = row => isOf(row, provider, ids);
    const terminal = launcher.terminalSees ?? { walked: [], listed: [] };
    if (terminal.walked.some(mine) || terminal.listed.some(mine)) {
        return true;
    }
    if (!launcher.open) {
        return false;
    }
    const found = foundIn(launcher.matches);
    return mine(launcher.answer)
        || launcher.held.some(step => mine(step.row))
        || found.from.has(provider)
        || ids.some(id => found.ids.has(id))
        || launcher.placed.some(id => ids.includes(id))
        || (launcher.category?.holds ?? []).some(id => ids.includes(id))
        || (typeof asking === "function" && asking(launcher.typed ?? ""));
}

// Whether a provider's rows can be seen now, so it watches its daemon
// only then rather than from the first time it's asked until the launcher
// closes (docs/design.md: nothing is polled that needn't be; Y2YBTS's
// review of #2441). Derived from what the launcher shows, never written
// as a side effect of answering, so bindings that ask a provider change
// nothing. Apart from the providers so tests/answers.qml holds it in Qt's
// own engine.
.pragma library

// Whether a row is one of the provider's: its own row, or the row of a
// place it says how things stand in, as Power's place is said by Power.
function isOf(row, provider, ids) {
    return !!row && (row.live === provider || ids.includes(row.category?.id ?? ""));
}

// Whether `provider`'s rows show in the open launcher: its answer, a row
// of it stepped into or found, or a place shown holding one of `ids`, as
// Machine holds Heat, or about to: what's typed asks it, by `asks`, a
// function of the words typed, since its answer is made only from what
// its daemon says. A place's holdings count before their conditions
// hold them back, since a row whose condition is what the daemon says,
// as Heat's is, would otherwise never be read to show. A terminal's `launcher show` counts the rows it
// walked and listed as seen for a few seconds after it asks, where the
// launcher keeps them (its `terminalSees`), so the terminal reads what
// the open launcher would.
function seen(launcher, provider, ids, asks) {
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
    return mine(launcher.answer)
        || launcher.held.some(step => mine(step.row))
        || launcher.matches.some(mine)
        || launcher.placed.some(id => ids.includes(id))
        || (launcher.category?.holds ?? []).some(id => ids.includes(id))
        || (typeof asks === "function" && asks(launcher.typed ?? ""));
}

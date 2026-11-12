// What a served row holds beneath its Return, so one holding only what the
// keys already reach opens no list of its acts (Joel, 2026-11-11 22:00: "a
// number of menu items that i open to only see verb:copy, which is stupid
// as shit. if they have nothing under them, we shouldn't open a submenu for
// them"). Apart from ReadyRows.js, which is at the length a file may be,
// and tests/ready-rows.qml holds it.
.pragma library

// Whether a row's acts hold nothing a list would add: none steps into rows,
// and past Return's first, each is a copy, which y does, or its details,
// which open its drawer. A row with any other act keeps its step in, so
// that act is still reached, until its drawer offers its acts. Its daemon
// may say so where the acts alone can't, as an emoji's copy is asked of
// the history and still done by y (`served`, ready-rows.md's actsOnly).
function actsOnly(acts, served) {
    if (served === true) {
        return acts.length > 0;
    }
    return acts.length > 0 && !acts.some(one => one.step === "rows")
        && acts.every((one, at) => at === 0 || one.step === "drawer"
            || (typeof one.copy === "string" && !one.ask));
}

// The row a step into rows enters: the row itself, or, where the act names
// one `into`, that row inside it, under the row's own name, so the path
// still says the meeting as Joel's hand learned (VKCF3Z's 74464630). It's
// made over the inside row, so what that row works out stays live.
function entered(row, named) {
    return named ? Object.assign(Object.create(named), { name: row?.name ?? named.name }) : row;
}

// The rows inside a row, the one its Return steps into first, as Return's
// own act leads a shell row's (Catalogue.js's leading).
function led(rows, first) {
    return first && rows.includes(first) ? [first].concat(rows.filter(one => one !== first))
        : rows;
}

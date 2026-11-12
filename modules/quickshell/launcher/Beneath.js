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
// that act is still reached, until its drawer offers its acts.
function actsOnly(acts) {
    return acts.length > 0 && !acts.some(one => one.step === "rows")
        && acts.every((one, at) => at === 0 || one.step === "drawer"
            || (typeof one.copy === "string" && !one.ask));
}

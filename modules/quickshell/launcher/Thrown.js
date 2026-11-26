.pragma library

// A throw kept from ending more than its own work: said as one line of the
// journal, and what waits on a frame run each in turn though one throws.

// A throw as one line of the journal: its kind and where it threw first,
// never its message, which could carry what a source served, nor every
// frame, which would bury the journal. The kind always ends "Error:", as
// the VM tests' error nets look for, a thrown value that isn't an Error, or
// one named otherwise, said as "Error"; and a first line not shaped wholly
// as a frame, as another engine's "Error: message" is, is left out, so no
// message can come in through it. A throw that can't be said even so, as
// an Error whose name is a Symbol can't, is said as a bare "Error: thrown",
// so saying a throw never throws in its turn.
const errorKind = /^[A-Za-z]*Error$/;
const frameLine = /^[^\s@]*@[^\s@]+:\d+(:\d+)?$/;
function shown(error) {
    try {
        const real = error instanceof Error;
        const kind = real && errorKind.test(`${error.name}`) ? error.name : "Error";
        const first = real ? `${error.stack ?? ""}`.split("\n")[0] : "";
        const where = frameLine.test(first) ? ` at ${first}` : "";
        return `${kind}: ${real ? "" : `${typeof error} `}thrown${where}`;
    } catch (_unsaid) {
        return "Error: thrown";
    }
}

// What waited on a frame, each run in turn, one that throws said in a line and the rest run
// still: the list is let go before it runs, so a throw ending the loop would drop every
// callback after it for good, among them Quests' made-ahead one, which alone clears its
// wait (launcher/ReadyRows.qml's aheadSoon), so its rows would never be made ahead again.
function runEach(due, said) {
    for (const then of due) {
        try {
            then();
        } catch (error) {
            said(shown(error));
        }
    }
}

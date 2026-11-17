.pragma library

// Whom `qs ipc call launcher talkTo <said>` opens a talk to (Launcher.qml's
// talkTo), as a cell on the realm's map asks it (XUBK2E, 2026-11-15
// [2b5c927a]). What's said names one and nothing more: its first word
// alone, so a page asking it can never fill the field or send anything.
// The void is talked to on Super+S's line, its one place; a session by its
// address, in either case, gets its own Talk; a network, which serves no
// Talk, is told apart so the launcher can say why there's none (VKCF3Z,
// [e1eb0ebe]); anything else is nobody here.
//
// `known` says what the launcher holds: `isVoid(said)`, `session(address)`
// for an address in capitals, and `network(name)`. Gives `{ to }`: "void",
// "session" with its `address`, "network" with its `name`, or "none" with
// what was `said`.
const spaces = /\s+/u;

// The one name `said` gives: its first word, as talkTo and network read it.
function named(said) {
    const [name] = String(said ?? "").trim().split(spaces);
    return name;
}

// The address of the session `qs ipc call launcher goTo <said>` goes to,
// as a notice's Go To asks it (Launcher.qml's goToCell): whom `said`
// names, if a session, since only a session has a window; "" for the void,
// a network or nobody. `known` is as whom's.
function goesTo(said, known) {
    const found = whom(said, known);
    return found.to === "session" ? found.address : "";
}

function whom(said, known) {
    const name = named(said);
    if (name === "") {
        return { to: "none", said: name };
    }
    if (known.isVoid(name)) {
        return { to: "void" };
    }
    const address = name.toUpperCase();
    if (known.session(address)) {
        return { to: "session", address };
    }
    if (known.network(name)) {
        return { to: "network", name };
    }
    return { to: "none", said: name };
}

.pragma library
.import "Presented.js" as Presented

// Carry Back on a carry Agents lists under Carried, which an orchestrator
// made in Joel's stead: one act, asked of the realm's node by the shell
// itself (ISXEG8's fourth condition, 26dbc61f). The node judges who asks
// by what the kernel says of them, and carries a quest only for the
// person's own login, so the ask goes from the shell, never through the
// agents daemon, which would ask in a daemon's name (KP0L6A's DCU5J4).
//
// The daemon serves the act with the node's ask in it. Only that ask's
// own shape is sent, rebuilt from its parts, so a row can never have the
// shell ask the node anything else: `carry`, a quest's id, and a machine's
// name.
const QUEST = /^QEST-[0-9]{8}-[A-Z0-9]{6}$/u;
const MACHINE = /^[A-Za-z0-9][A-Za-z0-9._-]*$/u;

// The key a carried row reads as carrying back under, and has its refusal
// said on: the one ReadyRows.qml made it with from its record's key
// (Presented.js's keyMade), so what it becomes is that row, and a thing
// claiming a kind of the launcher's own stays under `row:`.
function keyed(row, key) {
    return Presented.keyMade(row, key);
}

// The node's ask `served` names, or null for anything but a carry of a
// quest to a machine.
function asked(served) {
    const quest = String(served?.quest ?? "");
    const to = String(served?.to ?? "");
    if (served?.ask !== "carry" || !QUEST.test(quest) || !MACHINE.test(to)) {
        return null;
    }
    return { ask: "carry", quest, to };
}

// How the node's answer to it ended: `{ done: true }` once the carry has
// ended carried, else `{ done: false, why }` with the node's own words
// for its refusal, or `said`'s where it gave none or never answered.
function ended(answer, said) {
    if (answer === null || answer === undefined) {
        return { done: false, why: said.unanswered ?? "" };
    }
    if (answer.carried?.state === "done") {
        return { done: true };
    }
    const why = answer.error?.message;
    return { done: false, why: typeof why === "string" && why !== "" ? why : said.unread ?? "" };
}

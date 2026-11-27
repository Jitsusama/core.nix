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

// Carry Back as `served` holds it, its `row` served under `key` with the
// node's `ask`: the row, keyed as keyed says, reads as carrying back in
// `said`'s words until the node says the carry has ended, and a refusal is
// said on it in the node's own words. Only the ask asked rebuilds is sent;
// one that isn't a carry is never sent, and is refused on the row in
// `said`'s words, since only a row carrying back has a refusal said on it.
// All of it is here, where carry-back.qml reaches it, so Agents.qml
// only hands over the provider's becomes and refused and the node's find;
// left in Agents.qml, the key it chose went unchecked (core.nix#7099's round).
function carry(served, said, shell) {
    const { row, key } = served;
    const shown = keyed(row, key);
    const ask = asked(served.ask);
    let answered = false;
    shell.becomes(shown, {
        side: said.carrying ?? "",
        sideItems: [{ said: said.carrying ?? "", keep: 4 }]
    }, row.name ?? "", "carry back", { done: () => answered });
    if (ask === null) {
        shell.refused(shown, said.unasked ?? "");
        return;
    }
    shell.find(ask, answer => {
        const end = ended(answer, said);
        if (end.done) {
            answered = true;
        } else {
            shell.refused(shown, end.why);
        }
    });
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

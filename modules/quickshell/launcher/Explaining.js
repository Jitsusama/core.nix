.pragma library

// Explain It, the third act beside Talk and Go To on a row a session is
// behind (WCJQSG, 17 November 17:41: every signal stays short and carries a
// link, and a fast, cheap model says on the spot what it leads to). It asks
// the realm's node what the session is, as `mycelium cells <ADDRESS>` says
// it, and hands Ask's warm pi, on the machine's quick model, a question
// holding only what Joel would know the work by: its title, repository,
// machine, how it stands and its plan's latest and next. Never its ids,
// quest, folders, trees or spend, which say nothing to him and would only
// come back in the answer.
//
// Only that one question is asked, of the address alone, as the notices
// daemon and mycelium-link read one: capitals and digits, never an option.
const ADDRESS = /^[A-Z0-9]{1,12}$/u;
const REALM = /^[A-Za-z0-9][A-Za-z0-9._-]*$/u;
const FACTS = "{facts}";

// The words that ask `realm`'s node what the session at `address` is, or
// null when either isn't one.
function command(realm, address) {
    return REALM.test(String(realm ?? "")) && ADDRESS.test(String(address ?? ""))
        ? ["mycelium", "--realm", String(realm), "cells", String(address), "-o", "json"]
        : null;
}

// The session the node's answer `text` says, or null when it says no
// session: a machine, a network, an error or nothing.
function cellOf(text) {
    try {
        const said = JSON.parse(String(text ?? ""));
        return said?.kind === "cell" && said.type === "session" ? said : null;
    } catch (_unread) {
        // Words that aren't JSON, such as mycelium's "no cell is named", say
        // no session, which is what null means.
        return null;
    }
}

// What the quick model is told of `cell`, a line each, only those it says.
function facts(cell) {
    const plan = cell?.plan ?? {};
    return [
        ["Its title", cell?.title],
        ["Its repository", cell?.repo],
        ["Its machine", cell?.machine],
        ["How it stands", cell?.standing],
        ["Doing now", cell?.doing],
        ["Running since", cell?.since],
        ["Latest in its plan", plan.chose],
        ["Next in its plan", plan.next]
    ].filter(([, value]) => String(value ?? "").trim() !== "")
        .map(([label, value]) => `${label}: ${String(value).trim()}`);
}

// The question for `cell`: `asks`, the act's words, with its facts where
// they say {facts}, or after them when they don't.
function message(cell, asks) {
    const told = facts(cell).join("\n");
    const words = String(asks ?? "");
    // A replacer, so a $ in the facts is never read as a pattern.
    return words.includes(FACTS) ? words.replace(FACTS, () => told) : `${words}\n\n${told}`;
}

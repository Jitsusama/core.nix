.pragma library
.import "OnSelection.js" as OnSelection

// Explain It, the act beside Talk and Go To on a row a session is behind
// (WCJQSG, 17 November 17:41: every signal stays short and carries a link,
// and a fast, cheap model says on the spot what it leads to). It asks the
// realm's node what the session is, as `mycelium cells <ADDRESS>` says it,
// and hands a pi of its own on the machine's quick model, one with no
// tools (Explainer.qml), a question holding only what Joel would know the
// work by: its title, repository, machine, how it stands, what it's doing,
// since when, and its plan's latest and next, each on one line of its own.
// Never its ids, quest, folders, trees, spend or networks, which say
// nothing to him and would only come back in the answer. Its title and
// plan are an agent's own words, so each is flattened to one line and cut
// short, and the question asks the model to name no ids either.
//
// Only that one question is asked, of the address alone, as the notices
// daemon and mycelium-link read one: capitals and digits, never an option.
const ADDRESS = /^[A-Z0-9]{1,12}$/u;
const REALM = /^[A-Za-z0-9][A-Za-z0-9._-]*$/u;
const FACTS = "{facts}";
const SPACES = /\s+/gu;
const UNKNOWN = /no cell is named/u;
// The longest a fact is told, in characters: a sentence or two.
const LONGEST = 240;
// How long what an act said stands before it's the act again, so an
// answer saying where work stands now is never hours old (ten minutes).
const FRESH = 600_000;

// The words that ask `realm`'s node what the session at `address` is, or
// null when either isn't one.
function command(realm, address) {
    const named = String(address ?? "").toUpperCase();
    return REALM.test(String(realm ?? "")) && ADDRESS.test(named)
        ? ["mycelium", "--realm", String(realm), "cells", named, "-o", "json"]
        : null;
}

// The session a line of the node's answer says, or null when it says no
// session: a machine, a network, an error or nothing.
function sessionOf(line) {
    try {
        const said = JSON.parse(line);
        return said?.kind === "cell" && said.type === "session" ? said : null;
    } catch (_unread) {
        // Words that aren't JSON, such as a warning, say no session, which
        // is what null means.
        return null;
    }
}

// The session the node's answer `text` says, from its JSON lines, or null.
function cellOf(text) {
    return String(text ?? "").split("\n").map(sessionOf).find(one => one !== null) ?? null;
}

// How the node's answer ended, from what it wrote out and its errors: the
// session, "unknown" when the node knows no such cell, or "failed" when it
// couldn't say, as when the node is down or refused the realm.
function outcome(out, errors) {
    const cell = cellOf(out);
    if (cell !== null) {
        return { cell };
    }
    return { stage: UNKNOWN.test(String(errors ?? "")) ? "unknown" : "failed" };
}

// A fact's words on one line and cut short, so none can start a line or a
// paragraph of its own in the question.
function plain(value) {
    const flat = String(value ?? "").replace(SPACES, " ").trim();
    return flat.length <= LONGEST ? flat : `${flat.slice(0, LONGEST - 1).trim()}\u2026`;
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
    ].map(([label, value]) => [label, plain(value)])
        .filter(([, value]) => value !== "")
        .map(([label, value]) => `${label}: ${value}`);
}

// The question for `cell`: `asks`, the act's words, with its facts where
// they say {facts}, or after them when they don't.
function message(cell, asks) {
    const told = facts(cell).join("\n");
    const words = String(asks ?? "");
    // A replacer, so a $ in the facts is never read as a pattern.
    return words.includes(FACTS) ? words.replace(FACTS, () => told) : `${words}\n\n${told}`;
}

// What an act explained shows at `now`, from how it stands, `one` ({ stage,
// message, at }), and its pi (`ask`: posed, said, stage), in the act's
// words `said`: { name, reads, note, hue }, or null where it's the act as
// it was: never asked, stale, asked of another question, or stopped as
// the launcher closed, so Return asks again.
function shown(one, ask, said, now) {
    if (!one || now - one.at > FRESH) {
        return null;
    }
    const words = key => String(said?.[key] ?? "");
    if (one.stage !== "asking") {
        return {
            name: words(one.stage),
            reads: "",
            note: "",
            hue: one.stage === "reading" ? "voice" : "alert"
        };
    }
    if (ask?.posed !== one.message || ask.stage === "stopped" || ask.stage === "") {
        return null;
    }
    if (ask.stage === "failed") {
        return { name: words("failed"), reads: "", note: "", hue: "alert" };
    }
    const whole = String(ask.said ?? "").trim();
    const first = OnSelection.firstSentence(whole);
    return {
        name: whole === "" ? "\u2026" : first,
        reads: first === whole ? "" : whole,
        note: words(ask.stage === "answered" ? "answered" : "asking"),
        hue: "voice"
    };
}

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
const ACTS = "{acts}";
// The fact saying what a session asks Joel, in its own words, so quoted.
const ASKS = "It asks the person";
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

// How a row whose words ask nothing stands: a realm that isn't one is no
// answer to be had, and an address that isn't one is a session no node
// knows.
function unasked(realm) {
    return REALM.test(String(realm ?? "")) ? "unknown" : "failed";
}

// The session at `address` a line of the node's answer says, or null when
// it says none: a machine, a network, another cell, an error or nothing.
// `mycelium cells` also finds cells by their titles' words, so only the one
// the address names is it.
function sessionOf(line, address) {
    const said = cellSaid(line);
    const named = String(address ?? "").toUpperCase();
    return said?.type === "session" && said.name === named ? said : null;
}

// The cell a line of the node's answer says, or null.
function cellSaid(line) {
    try {
        const said = JSON.parse(line);
        return said?.kind === "cell" ? said : null;
    } catch (_unread) {
        // Words that aren't JSON, such as a warning, say no cell, which is
        // what null means.
        return null;
    }
}

// The session at `address` the node's answer `text` says, from its JSON
// lines, or null.
function cellOf(text, address) {
    return String(text ?? "").split("\n").map(line => sessionOf(line, address))
        .find(one => one !== null) ?? null;
}

// How the node's answer about `address` ended, from what it wrote out and
// its errors: the session, "unknown" when the node knows no such cell, or
// "failed" when it couldn't say, as when the node is down or refused the
// realm.
function outcome(out, errors, address) {
    const cell = cellOf(out, address);
    if (cell !== null) {
        return { cell };
    }
    // Cells whose titles hold its words, and none at it, are an answer too.
    const others = String(out ?? "").split("\n").some(line => cellSaid(line) !== null);
    return { stage: others || UNKNOWN.test(String(errors ?? "")) ? "unknown" : "failed" };
}

// A fact's words on one line and cut short, so none can start a line or a
// paragraph of its own in the question.
function plain(value) {
    const flat = String(value ?? "").replace(SPACES, " ").trim();
    return flat.length <= LONGEST ? flat : `${flat.slice(0, LONGEST - 1).trim()}\u2026`;
}

// The names of the acts `beside` a row, each on one line and none blank:
// `beside` may be a list Qt handed through a property, no array of
// JavaScript's, so it's read as anything with a length, and anything else
// names none.
function names(beside) {
    return typeof beside === "string" || typeof beside?.length !== "number" ? []
        : Array.from(beside, plain).filter(name => name !== "");
}

// The acts on its row Joel can answer `cell` by, `beside`'s names: none
// while it asks him nothing, since nothing then waits on a press.
function presses(cell, beside) {
    return plain(cell?.asks) === "" ? [] : names(beside);
}

// What the quick model is told of `cell`, a line each, only those it says.
// One waiting on Joel says what it asks him, its want's own words quoted,
// so none of them reads as a fact of the launcher's, and then the acts on
// its row he can answer it by (presses), so the answer says what it waits
// on and which to press.
function facts(cell, beside) {
    const plan = cell?.plan ?? {};
    return [
        ["Its title", cell?.title],
        ["Its repository", cell?.repo],
        ["Its machine", cell?.machine],
        ["How it stands", cell?.standing],
        [ASKS, cell?.asks],
        ["The person can press", presses(cell, beside)],
        ["Doing now", cell?.doing],
        ["Running since", cell?.since],
        ["Latest in its plan", plan.chose],
        ["Next in its plan", plan.next]
    ].map(([label, value]) => [label, Array.isArray(value) ? value.join(", ") : plain(value)])
        .filter(([, value]) => value !== "")
        .map(([label, value]) => `${label}: ${label === ASKS ? JSON.stringify(value) : value}`);
}

// The question for `cell`: `asks`, the act's words, with its facts where
// they say {facts}, or after them when they don't, and `acts`, the words
// asking the answer to end with the act that answers it, where they say
// {acts}, only while its row has one to name (presses): told to end with
// one when none is listed, the model makes one up. `beside` names the
// acts on its row.
function message(cell, asks, beside, acts) {
    const told = facts(cell, beside).join("\n");
    const pressing = presses(cell, beside).length > 0 ? String(acts ?? "") : "";
    // Replacers, so a $ in the facts is never read as a pattern.
    const words = String(asks ?? "").replace(ACTS, () => pressing);
    return words.includes(FACTS) ? words.replace(FACTS, () => told) : `${words}\n\n${told}`;
}

// Whether an answer asked with the acts `asked` still fits a row whose
// acts are `beside` now: one naming an act the row no longer has, or
// leaving out one it has since, is asked again.
function fits(asked, beside) {
    const then = names(asked);
    const now = names(beside);
    return then.length === now.length && then.every((name, at) => name === now[at]);
}

// What an act explained shows at `now`, from how it stands, `one` ({ stage,
// message, at }), and its pi (`ask`: posed, said, stage), in the act's
// words `said`: { name, reads, note, hue }, or null where it's the act as
// it was: never asked, stale, of another question, or stopped as the
// launcher closed, so Return asks again. One asked with other acts on its
// row is the act again too, as Explainer.qml's shown holds by fits.
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

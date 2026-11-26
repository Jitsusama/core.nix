.pragma library
// A pi session as the launcher shows it, said once, and named apart from
// Session.qml, which is the desktop session: how it stands and the acts it
// offers in their order, wherever the launcher shows it (Agents, an
// orchestrator's row, a quest's Go To Its Session). Its drawer, figures
// and all, is the agents daemon's (modules/agents/src/drawer.rs).
// Joel, 2026-10-28: "we need to not have different versions of these sort
// of elements. they need to be treated the same everywhere." The providers
// turn what this says into rows, each through its own `thing`.

// What a session is, as the launcher's components name their kinds.
const KIND = "session";

// How a session stands, and where that comes in the one order every list
// keeps, are the agents daemon's (modules/agents/src/standing.rs), served
// on each row as its `state` and `rank` and read by Agents.qml's stateOf
// and rankOf, so nothing here works them out a second way.

// Where a session stands in that order, from the agents daemon's row for
// it (`row.held.rank`). Only a number is a rank: one not served yet, or
// served as anything else, goes after every one ranked, since Number would
// make null, "" or false 0, the first place, ahead of one needing Joel.
const UNRANKED = Number.MAX_SAFE_INTEGER;
function rankOf(row) {
    const rank = row?.held?.rank;
    return Number.isFinite(rank) ? rank : UNRANKED;
}

// Which acts a session's own row offers, and in what order, is the agents
// daemon's (standing.rs's acts, served on its row by row.rs), so nothing
// here says it a second way: before its row is held, the shell offers
// only what can't wake one parked already (fallbackOf).

// The acts a session offers from a row it's behind but isn't its own, as
// a landing's in For You: Go To, then Talk, in the order of its own acts,
// so Joel reaches whoever made the thing from the thing (Joel,
// 2026-11-16: "i have no way to go from the thing it shows to engaging
// with the thing working on it"). None steers or ends it, since that row
// is about what it made, not how it runs, so where it runs, which only
// Close weighs, never changes them. One `asking` a question or a yes has
// no Talk, as What Needs Me gives none (Now.qml's talkTo): its ask runs
// until he answers, so words said to it would come after the answer they
// were meant to shape. Parked, it runs no ask, so Talk stays, and it has
// no window known to go to, so no Go To.
function behindActs(standing, windowed, asking) {
    const parked = standing === "parked";
    const ids = windowed && !parked ? ["switch"] : [];
    if (!asking || parked) {
        ids.push("talk");
    }
    return ids;
}

// Whether a session's presence holds an ask open, a question or a yes,
// the one reading behind both a landing's acts and What Needs Me's Talk,
// so a kind of want added later is weighed in one place.
function asks(presence) {
    const kind = presence?.wants?.kind;
    return kind === "question" || kind === "approval";
}

// The acts a session's own row can do, as Agents.qml's sessionAct makes
// each. Only these are taken from what the daemon serves: a switch
// restarts the daemon at once but leaves the running shell alone until its
// handover, so a daemon serving an act newer than the shell would
// otherwise offer what the shell can't do.
const AGENT = "verb:agent:";
const KNOWN = ["switch", "talk", "wake", "pause", "resume", "park", "stop", "close"];

// The id of a session's act its key names, as "verb:agent:park" names
// park, or "" for a key that isn't one the shell makes, which is how a
// row's Return (Agents.qml's sessionRow) and its acts are both read.
function known(key) {
    const said = String(key ?? "");
    const id = said.startsWith(AGENT) ? said.slice(AGENT.length) : "";
    return KNOWN.includes(id) ? id : "";
}

// The acts Return may fall to when the row names none it lists: those
// that leave a session as it is, or wake it, as Wake is Return for a
// parked one, whose list holds it.
const HARMLESS = ["switch", "talk", "wake"];

// The acts a session's row offers, by their ids, and `marked`, the one its
// Return does: those its row serves (`rowActs`) that the shell makes, in
// their order, and its `returns` (modules/agents/src/row.rs), else
// `fallback`, the shell's own list. A Return it doesn't list, which only
// a daemon out of step with the shell serves (standing.rs gives each
// Return in its acts), isn't trusted, as Go To on a row with no window or
// Wake on one running would be: Return is then, as with no Return the
// shell makes, the first act that leaves the session as it is, else Talk,
// put first, never one that steers or ends it, whatever order a newer
// daemon serves. Its drawer and its step in read the same list.
function offered(rowActs, returns, fallback) {
    const served = (Array.isArray(rowActs) ? rowActs : [])
        .map(one => known(one?.key))
        .filter(id => id !== "");
    const ids = served.length > 0 ? served : fallback.slice();
    const named = known(returns);
    const returning = ids.includes(named) ? named
        : ids.find(id => HARMLESS.includes(id)) ?? "talk";
    if (!ids.includes(returning)) {
        ids.unshift(returning);
    }
    return { ids, marked: returning };
}

// What a session's drawer offers and its Return does, from its `row` as
// the agents daemon serves it, or null before it's held, whether it's
// `windowed`, whether the shell holds it `parked`, and whether it stands
// at the `top`, as the void does (standsAtTop). Parked by the shell's
// word, it offers a parked one's acts whatever its row still serves,
// since `withheld` keeps back every other act then, so what's offered and
// what goes agree; its Return is Wake, but Talk for one at the top, which
// is gone to to be told something however it stands, as row.rs serves it.
function drawn(row, windowed, parked, top) {
    const fallback = fallbackOf(windowed, parked);
    return parked ? offered([], top ? `${AGENT}talk` : "", fallback)
        : offered(row?.acts, row?.held?.returns, fallback);
}

// Whether a session stands at the top, gone to by name as the void is,
// from its `presence` as session.rs's at_top reads it: a name it says.
function standsAtTop(presence) {
    return String(presence?.standsAtTop?.name ?? "") !== "";
}

// The shell's own acts for a session's row, `offered`'s fallback: for one
// the shell holds `parked`, a parked one's, Wake and Talk, as Wake works
// with no daemon to serve its row; else Go To where it has a window, and
// Talk, which every session takes, and none that steers or ends it, since
// how it stands isn't known: before its row is held it may be parked
// already, and the node wakes a parked session for any steer; held with
// no act the shell makes, as from a daemon newer than it, it's no better
// known, so it gets the same. `withheld` keeps back the rest.
function fallbackOf(windowed, parked) {
    if (parked) {
        return ["wake", "talk"];
    }
    return windowed ? ["switch", "talk"] : ["talk"];
}

// The addresses of the sessions the shell holds parked (Sessions.parked),
// upper-cased, each by every address it goes by, as Sessions.parkedAs
// finds one, so a row asks whether its session is parked in one lookup
// and by the same rule as the shell's other places.
function parkedSet(parked) {
    const addresses = new Set();
    for (const one of Array.isArray(parked) ? parked : []) {
        for (const address of one?.presence?.addresses ?? [one?.address]) {
            const upper = String(address ?? "").toUpperCase();
            if (upper !== "") {
                addresses.add(upper);
            }
        }
    }
    return addresses;
}

// The act a served orchestration's row names by `key` that the shell
// makes itself, as a session's own (Agents.qml's topSessionAct), or ""
// for one the daemon does, Close among them, which asks before it's sent.
const TOP = ["switch", "park", "pause", "resume", "stop"];
function topActOf(key) {
    const id = known(key);
    return TOP.includes(id) ? id : "";
}


// Why an act pressed on a session's row isn't steered, asked of the row
// as it's served at the press, and whether the shell holds it `parked`,
// or "" when it is: one parked since (the shell holds it parked, or its
// row offers Wake) takes only Wake and Talk, since the node wakes a
// parked session for any steer. One whose row isn't held, or serves no
// act the shell makes, takes only Go To, Wake and Talk, as it may be
// parked already. Park goes only while its row offers it. Any other act
// goes for the node or the harness to answer, so a Stop pressed as its
// turn comes to ask Joel still stops it.
function withheld(row, id, parked) {
    const served = (Array.isArray(row?.acts) ? row.acts : [])
        .map(one => known(one?.key))
        .filter(one => one !== "");
    if (parked || served.includes("wake")) {
        return id === "wake" || id === "talk" ? "" : "unsentParked";
    }
    if (served.length === 0) {
        return ["switch", "wake", "talk"].includes(id) ? "" : "unsentUnheld";
    }
    return id !== "park" || served.includes("park") ? "" : "unsent";
}

// What the foot says of an act `withheld` kept back, by its word's key,
// in the act's own name: said plainly, as nothing failed.
function unsent(said, why, act) {
    return String(said?.[why] ?? "").replace("{act}", () => act);
}

// The theme's role its glyph is drawn in: a program's, as a live one
// is, and muted for one paused, parked or closed, which never looks live
// (#2052; VKCF3Z, 2026-11-07), colour going on glyphs while its name
// stays in text and the far-end word says how it stands. One waiting on
// Joel is in the alert's, ahead of a nearing limit, as the agents daemon
// serves its row (modules/agents/src/standing.rs's hue; Joel, 2026-11-20).
function hue(stands, warns) {
    if (stands === "needs") {
        return "alert";
    }
    if (["paused", "parked", "off"].includes(stands)) {
        return "muted";
    }
    // A live one with a tier 2 figure is nearing a limit, in the warning's
    // hue, as a budget near its allowance is; action would say a thing to do
    // (VKCF3Z, 2026-11-07).
    return warns ? "warning" : "program";
}

// What a session's row says at its right, each item with how much it's
// needed, so where room runs short they give way whole (Choice.qml): how
// it stands, then what it spent today, last and kept longest, and nothing
// more (VKCF3Z, 2026-11-06: "working \u00b7 $4.20"). Its address and
// folder are its drawer's eyebrow, and typing its address still finds it.
// A tier 2 figure leads while it's true, "84% full \u00b7 working \u00b7
// $4.20", and is kept longest, since it's why the row asks for a look;
// beside it how the session stands outlasts the money, which gives way
// first, since the drawer and Today say the spend (VKCF3Z, 2026-11-07).
function side(doing, spent, filled) {
    const tier = filled ?? "";
    const [stands, costs] = tier !== "" ? [4, 3] : [3, 4];
    return [{ said: tier, keep: 5 }, { said: doing, keep: stands }, { said: spent, keep: costs }]
        .filter(item => item.said !== "");
}

// How full a session's context is, as a whole percent of its window, from
// its record's `usage.context` ({ used, limit }), the harness's count of
// what the next turn sends; 0 where it isn't said. Past FULL_AT it's tier 2
// on its row (the design's figures, 2026-11-06): a compaction is near, and
// each turn sends most of a window.
const FULL_AT = 80;
const PERCENT = 100;
function fullness(usage) {
    const used = Number(usage?.context?.used ?? 0);
    const limit = Number(usage?.context?.limit ?? 0);
    return limit > 0 && used > 0 ? Math.floor(used / limit * PERCENT) : 0;
}

// What its row says of how full it is, in the vocabulary's words, only past
// FULL_AT; "" below it.
function full(usage, said) {
    const share = fullness(usage);
    return share >= FULL_AT ? String(said.full ?? "").replace("{share}", share) : "";
}

// When a closed session ended, at its right: "closed 23:41" today, and
// with its day before, "closed Fri 23:41", as `day` names a date. `said`
// is the vocabulary's agents words, and `endedAt` when it ended, in
// milliseconds.
function closedAt(endedAt, now, said, day) {
    if (typeof endedAt !== "number" || !Number.isFinite(endedAt)) {
        return String(said.closed ?? "");
    }
    const ended = new Date(endedAt);
    const time = [ended.getHours(), ended.getMinutes()].map(part => String(part).padStart(2,
        "0")).join(":");
    const today = ended.toDateString() === now.toDateString();
    return String(said[today ? "closedAt" : "closedOn"] ?? "").replace("{time}", time)
        .replace("{day}", today ? "" : day(ended));
}

// The glyph and hue a session's row wears in For You, from Agents' row
// `made`: its mark and hue as it'd stand not waiting (the agents daemon's
// held mark and markHue), since every session here waits and the person
// with a clock is a to-do's here, so they aren't all alert and a nearing
// limit still warns. With none served, the row's own.
function unwaited(made) {
    return {
        glyph: made?.mark ?? made?.glyph,
        hue: made?.markHue ?? made?.hue
    };
}

// What the foot names a session by when it says what became of an act on
// it: the agents daemon's name for its row, else its quest's title as its
// record says it, and its address only when neither is known, since an
// address means nothing to someone who never saw the session (WCJQSG, 19
// November: no ids on what reaches Joel).
function footName(served, title, address) {
    return [served, title, address].map(one => String(one ?? "").trim())
        .find(one => one !== "") ?? "";
}

// footName of a session as the shell hands it over. One with no record
// yet, as a start in pi's host is, has no presence to title it by, so its
// quest's name, as the launcher gave it, stands as `name`; `titleOf` is
// the title a presence names (Sessions.named).
function footNameOf(served, session, titleOf) {
    return footName(served, session.presence ? titleOf(session.presence) : session.name,
        session.address);
}

// What an address is made of, the only kind reasonOf looks for in a reason.
const ADDRESS = /^[A-Z0-9]+$/u;

// The node's reason for refusing an act on a session, without the address
// it opens with (mycelium's steer_here says "<ADDRESS> isn't parked ..."),
// since the row or the name beside it already stands for the session, and
// the address anywhere after said as `name`, when there is one, so no id
// reaches the foot.
function reasonOf(why, address, name) {
    const upper = String(address ?? "").toUpperCase();
    const given = String(why ?? "");
    if (upper === "") {
        return given;
    }
    const rest = given.toUpperCase().startsWith(`${upper} `) ? given.slice(upper.length + 1)
        : given;
    const called = String(name ?? "").trim();
    // An address is letters and figures alone, so it needs no escaping.
    return called === "" || !ADDRESS.test(upper) ? rest
        : rest.replace(new RegExp(`\\b${upper}\\b`, "gu"), () => called);
}

// An act's own name as the vocabulary says it ("Wake", "Pause"), by
// `nameOf`, Words.name: none for words talked to a session, and none for an
// act the vocabulary doesn't name, as For You's "fill" or a look's
// "answer", whose id is all Words.name would give.
function actName(act, nameOf) {
    const given = String(act ?? "");
    if (given === "") {
        return "";
    }
    const id = `${AGENT}${given}`;
    const name = String(nameOf(id) ?? "");
    return name === id ? "" : name;
}

// What the foot says of an act a session's node refused or never answered,
// as `[name, words]`. The act is named by actName ("Wake", "Talk"), or none
// for words talked to it or an act the vocabulary doesn't name, and the
// node's reason by reasonOf. With no reason, the node never answered, so
// nothing is said refused or untold, since it may yet have been done.
// `said` is the vocabulary's agents:machine words; `refused` holds the
// act's name, the node's `why`, the session's `address` and its `name`.
// Each value goes in as it is, never read as a replacement's pattern.
function refusal(said, refused) {
    const { act, why, address, name } = refused;
    const upper = String(address ?? "").toUpperCase();
    const called = footName(name, "", upper);
    const reason = reasonOf(why, upper, called === upper ? "" : called);
    if (reason === "") {
        const words = act === "" ? String(said.unanswered ?? "")
            : String(said.unansweredTo ?? "").replace("{act}", () => act);
        return [called, words];
    }
    const words = act === "" ? String(said.untold ?? "")
        : String(said.refused ?? "").replace("{act}", () => act);
    return [called, words.replace("{why}", () => reason)];
}

// Why a row drawn as woken is taken back at once, for a steer its node
// refused or never answered, or null where it's left be. Only a Wake takes
// it back: another act refused meanwhile, a Pause or words talked to it,
// is said as itself (refusal) and leaves the Wake's row as it was to
// become. A Wake with no answer is taken back too, as "", which the
// launcher says as not woken yet, with no answer (becameNot): no answer is
// still coming, but the Wake may still land. From Agents, no answer means
// the agents daemon's line was gone or went while it waited, and it may
// already have sent the steer, so nobody is blamed. Held for Becoming's
// 30 s, the row read woken all that time (#5878's follow-up). `refused` is as refusal
// takes it, the act by its id ("wake"), and `refused.held` whether the row
// is becoming woken.
function takesBack(refused) {
    const { act, why, address, name, held } = refused;
    if (act !== "wake" || !held) {
        return null;
    }
    return reasonOf(why, address, name);
}

// What Agents does with a steer its node refused or never answered, so
// the choice is made here, where its cases reach it. A row takesBack takes
// back goes straight to the launcher's becameNot, "" included, which a
// provider's refused would drop; anything else is said on the foot by
// refusal. `launcher` holds becameNot and tellOnOpen, `said` the
// agents:machine words, `alert` the foot's colour, and `refused` is as
// takesBack takes it, with `actName` the act's name and `called` what the
// foot names the session by.
function unsteered(launcher, said, refused, alert) {
    const back = takesBack(refused);
    if (back !== null) {
        launcher?.becameNot?.(`agent:${refused.address}`, back);
        return;
    }
    const [name, words] = refusal(said, { act: refused.actName, why: refused.why,
        address: refused.address, name: refused.called });
    launcher?.tellOnOpen?.([[name, words, alert]]);
}

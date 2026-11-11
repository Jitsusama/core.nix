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

// How a running session can stand, in the one order every list of
// sessions keeps: what only Joel can move first, then what's under way,
// then what waits for work, then what he put down, then what gave its
// memory back until it's wanted (the harness's park-workflow). One idle
// is listening, never waiting, which is only ever said of something
// waiting on Joel.
const STATES = ["needs", "working", "inLine", "listening", "paused", "parked"];

// How a session stands, from its presence, whether its want is Joel's
// (`asks`) and whether it waits in the queue (`queued`); "off" with none.
// A want not his reads as what it does; one parked stays parked, its want
// For You's (Sessions.wanting), since Wake is its only act.
function state(presence, asks, queued) {
    if (!presence) {
        return "off";
    }
    if (presence.state === "parked") {
        return "parked";
    }
    if (asks) {
        return "needs";
    }
    if (presence.state === "paused") {
        return "paused";
    }
    if (queued) {
        return "inLine";
    }
    return presence.busy || presence.state === "busy" ? "working" : "listening";
}

// Where a state stands in that order; one not running after them all.
function rank(standing) {
    const at = STATES.indexOf(standing);
    return at < 0 ? STATES.length : at;
}

// The acts a running session offers, by their vocabulary's ids, in the
// order every kind keeps (docs/design.md, the order of actions): what
// leaves it as it is before what changes it, and what ends it last. Every
// session on the mesh reads its steer inbox, so each can be talked to,
// paused, resumed and stopped; one with no window offers no Go To, and
// only one under way can be stopped. One parked has no process to pause
// or stop and no window known to go to: it can be woken, or talked to,
// which wakes it too. Only one here can be closed, last, since the
// harness ends a session on a close Joel steers it from this machine and
// drops one a node relays (harness #1023).
function acts(standing, windowed, here) {
    if (standing === "parked") {
        return ["wake", "talk"];
    }
    const ids = windowed ? ["switch"] : [];
    ids.push("talk");
    ids.push(standing === "paused" ? "resume" : "pause");
    if (standing === "working" || standing === "inLine") {
        ids.push("stop");
    }
    if (here) {
        ids.push("close");
    }
    return ids;
}

// What Close's ask says stays, so ending a session never reads as losing
// its work (VKCF3Z, 2026-11-06, in to-dos since 2026-11-08: "Close
// 8IDU4E? Its worktree and 1 to-do stay."): its worktree, or its folder
// where it works outside one, and its quest's to-dos, by their count, in the agents'
// words (`said`).
function leaves(inTree, needs, said) {
    const place = inTree ? said.tree : said.folder;
    if (needs === 0) {
        return String(said.stays ?? "").replace("{kept}", place);
    }
    const items = String((needs === 1 ? said.toDo : said.toDos) ?? "")
        .replace("{count}", needs);
    return String(said.stay ?? "").replace("{kept}", `${place} ${said.and} ${items}`);
}

// The theme's role its glyph is drawn in: a program's, as a live one
// is, and muted for one paused, parked or closed, which never looks live
// (#2052; VKCF3Z, 2026-11-07), colour going on glyphs while its name
// stays in text and the far-end word says how it stands.
function hue(stands, warns) {
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

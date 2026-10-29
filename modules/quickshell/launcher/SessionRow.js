.pragma library
// A pi session as the launcher shows it, said once, and named apart from
// Session.qml, which is the desktop session: how it stands, the acts it
// offers in their order, and what its drawer says, wherever the launcher
// shows it (Agents, an orchestrator's row, a quest's Go To Its Session).
// Joel, 2026-10-28: "we need to not have different versions of these sort
// of elements. they need to be treated the same everywhere." The providers
// turn what this says into rows, each through its own `thing`.

// What a session is, as the launcher's components name their kinds.
const KIND = "session";

// How a session stands, from its presence: waiting on Joel, paused,
// working, listening for what's next, or not running at all.
function stands(presence) {
    if (!presence) {
        return "off";
    }
    if (presence.state === "paused") {
        return "paused";
    }
    if (presence.wants || presence.state === "waiting") {
        return "waiting";
    }
    return presence.busy || presence.state === "busy" ? "busy" : "listening";
}

// The acts a running session offers, by their vocabulary's ids, in the
// order every kind keeps (docs/design.md, the order of actions): what
// leaves it as it is before what changes it, and what ends it last. Every
// session on the mesh reads its steer inbox, so each can be talked to,
// paused, resumed and stopped; one with no window offers no Switch.
function acts(standing, windowed) {
    const ids = windowed ? ["switch"] : [];
    ids.push("talk");
    ids.push(standing === "paused" ? "resume" : "pause");
    if (standing === "busy") {
        ids.push("stop");
    }
    return ids;
}

// What its drawer says: what it works on, its step, what it waits on Joel
// for, what it's spent today and since when it's run, each as a name and a
// figure, its title over them. Nothing it hasn't said is guessed.
function drawer(title, presence, said) {
    const known = {
        works: presence?.worksOn,
        step: presence?.step,
        wants: presence?.wants?.said,
        spent: said.spent,
        started: said.started,
        address: said.address
    };
    const facts = Object.keys(known).filter(id => (known[id] ?? "") !== "")
        .map(id => ({ label: said.named(id), value: known[id] }));
    return { title, facts, body: "" };
}

// The Work Network's own drawer, on its row in Network: whose link it is
// once in the eyebrow, a lead saying how work stands and for how long,
// the facts that say why, and warp-cli's own words for it last, in the
// fold, for whoever looks further (VKCF3Z, 2026-11-12: "Every row has
// one", Joel's rule of 2026-11-11 22:00). It reads the link as Warp.js
// tells it and how long it has stood as the warp daemon saw it begin
// (WarpState.qml), so it asks nothing. Warp.qml names each fact.
.pragma library
.import "Warp.js" as Warp
.import "Ago.js" as Ago

// The parts the drawer says, in this order, by the ids Warp.js gives
// them: how it stands, how long a round trip takes and what waits on
// it, then how it's set and who for, and what carries it.
const FACTS = ["state", "latency", "waiting", "settings:mode", "settings:organisation",
    "through"];

// The part of `id` among the parts, or among those a step in.
function found(parts, id) {
    for (const each of parts ?? []) {
        if (each.id === id) {
            return each;
        }
        const inside = found(each.parts, id);
        if (inside !== null) {
            return inside;
        }
    }
    return null;
}

// The lead, a sentence: how the link stands, for how long where the
// daemon saw it begin, and its round trip where one was read, "Connected
// to work for 3 h, with a 38 ms round trip." A state first heard when the
// daemon started may have stood for hours, so it says no time rather than
// one too short. Nothing where WARP said nothing it knows, or none heard.
// The sentence is the vocabulary's `tells`, its parts warp's `said`.
function lead(stood, latency, now, words) {
    const stands = stood?.said ?? {};
    const key = sentence(stands.status, String(stands.reason ?? ""));
    if (key === "") {
        return "";
    }
    const since = stood.since_ms;
    const lasted = Number.isFinite(since)
        ? Warp.worded(words.said, "leadFor", { lasted: Ago.span(since, now) }) : "";
    const trip = latency && stands.status === "Connected"
        ? Warp.worded(words.said, "leadTrip", { rtt: latency.side }) : "";
    return Warp.worded(words.tells, key, { lasted, trip });
}

// Which of the lead's sentences says WARP's status, as warp-cli words it,
// or none for one it doesn't know.
function sentence(status, reason) {
    if (status === "Connected") {
        return reason === "NetworkDegraded" ? "degraded" : "connected";
    }
    if (status === "Connecting") {
        return "connecting";
    }
    return status === "Disconnected" ? "disconnected" : "";
}

// The drawer, from what was read: the parts Warp.js told, the record it
// told them from and the daemon's held state, at `now`, in warp's `said`
// and `tells`. It's `{ eyebrow, body, facts }`, each fact `{ id, value }`
// and those that are warp-cli's own words marked as such, so Warp.qml
// names them and Drawer.qml sets them as names.
function drawer(read, words) {
    const { parts, record, stood, now } = read;
    const facts = FACTS.map((id) => found(parts, id))
        .filter((each) => each !== null && each.side !== "")
        .map((each) => ({ id: each.id, value: each.side }));
    const status = record?.status ?? {};
    const raw = [["status", status.state], ["reason", status.reason]]
        .filter(([, value]) => typeof value === "string" && value !== "")
        .map(([id, value]) => ({ id, value, identifies: true }));
    return {
        eyebrow: Warp.worded(words.said, "eyebrow"),
        body: lead(stood, found(parts, "latency"), now, words),
        facts: facts.concat(raw),
    };
}

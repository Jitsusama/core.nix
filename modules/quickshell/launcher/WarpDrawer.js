// The Work Network's own drawer, on its row in Network: whose link it is
// once in the eyebrow, a lead saying how work stands and for how long,
// the facts that say why, and warp-cli's own words for it in its nerdy
// bits, for whoever looks further (VKCF3Z, 2026-11-12: "Every row has
// one", Joel's rule of 2026-11-11 22:00; and f8984db0: they're the
// daemon's raw fields, not words for Joel). It reads the link as Warp.js
// tells it and how long it has stood as the warp daemon saw it begin
// (WarpState.qml), so it asks nothing. Warp.qml names each fact.
.pragma library
.import "Warp.js" as Warp
.import "Ago.js" as Ago

// The parts the drawer says, in this order, by the ids Warp.js gives
// them: how it stands, how long a round trip takes and what waits on
// it, and what carries it.
const FACTS = ["state", "latency", "waiting", "through"];

// The parts that are warp-cli's own words, set in the nerdy bits: how
// it's set and who for. WARP's reason joins them from its record; its
// status doesn't, since State already says it, and one reading is never
// two rows (VKCF3Z, f8984db0).
const NERD = ["settings:mode", "settings:organisation"];

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
// to work for 3 h; a round trip takes 38 ms." No article stands before
// the figure, since "a" or "an" is wrong for 8, 11, 18 and the eighties
// (VKCF3Z, f8984db0). A state first heard when the
// daemon started may have stood for hours, so it says no time rather than
// one too short. Nothing where WARP said nothing it knows, or none heard.
// While the warp daemon judges the way slow (modules/warp/src/slow.rs)
// and WARP is otherwise well, it says for how long it's been slow and
// what waits, "Connected to work, slow for 4 min, with 120 ms waiting."
// The sentence is the vocabulary's `tells`, its parts warp's `said`.
function lead(stood, latency, now, words) {
    const stands = stood?.said ?? {};
    const key = sentence(stands.status, String(stands.reason ?? ""));
    if (key === "") {
        return "";
    }
    const slow = stood.slow;
    if (key === "connected" && Number.isFinite(slow?.waiting_ms)) {
        return Warp.worded(words.tells, "slow", {
            lasted: Warp.worded(words.said, "leadFor", { lasted: Ago.span(slow.since_ms, now) }),
            waits: Warp.worded(words.said, "leadWaits", {
                waiting: Warp.ms(slow.waiting_ms, {}, words.said)
            })
        });
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
// and `tells`. It's `{ eyebrow, body, facts, nerd }`, each fact and bit
// `{ id, value }`, which Warp.qml names.
function drawer(read, words) {
    const { parts, record, stood, now } = read;
    const reason = record?.status?.reason;
    const raw = typeof reason === "string" && reason !== ""
        ? [{ id: "reason", value: reason }] : [];
    return {
        eyebrow: Warp.worded(words.said, "eyebrow"),
        body: lead(stood, found(parts, "latency"), now, words),
        facts: told(parts, FACTS),
        nerd: told(parts, NERD).concat(raw),
    };
}

// The parts of `ids` among the parts, in that order, as `{ id, value }`,
// leaving out any not told or told as nothing.
function told(parts, ids) {
    return ids.map((id) => found(parts, id))
        .filter((each) => each !== null && each.side !== "")
        .map((each) => ({ id: each.id, value: each.side }));
}

// What the Work Network says waits on the way through it, from what the
// network's shaper heard (modules/shaper), which warp's daemon passes on
// beside WARP's record as `waiting` (modules/warp/src/waiting.rs): the
// round trip of a question asked through the VPN, past the least of the
// last five minutes, the way at rest. That's the link's own queue and the
// VPN's together, not WARP's alone, so it's Waiting rather than a queue,
// which would read as a count (B8RBNW, 2026-11-12). A part as Warp.js
// makes one, in warp's words, placed after the Round Trip it explains.
// Upload follows it: the rate the shaper limits the upload to, and why,
// in the shaper's own words, passed on as `limit`.
.pragma library
.import "Warp.js" as Warp
.import "HomeNetwork.js" as HomeNetwork

// Under this, the wait is within what the shaper takes for the way's own
// jitter (its VPN_MARGIN_MS), so nothing is said to wait.
const MARGIN_MS = 5;

// Waiting, from `waiting`, or null where warp's daemon read no shaper, as
// on a machine without one: how long past the way at rest, "75 ms", noted
// "103 ms through it · 28 ms at rest"; none under the margin; and unknown
// while the shaper isn't running or its last question went unanswered,
// never none, since nothing heard isn't nothing waiting.
function part(waiting, settings, said) {
    if (!waiting || typeof waiting !== "object") {
        return null;
    }
    const unknown = (why) =>
        Warp.part("waiting", Warp.worded(said, "unknown"), Warp.worded(said, why), "");
    if (waiting.running !== true) {
        return unknown("shaperOff");
    }
    const through = Math.round(waiting.through_ms);
    const rest = Math.round(waiting.at_rest_ms);
    if (!(Number.isFinite(waiting.through_ms) && Number.isFinite(waiting.at_rest_ms))) {
        return unknown("unheard");
    }
    const wait = Math.max(0, through - rest);
    const note = Warp.worded(said, "waiting", {
        through: Warp.ms(through, settings, said),
        rest: Warp.ms(rest, settings, said),
    });
    return wait < MARGIN_MS
        ? Warp.part("waiting", Warp.worded(said, "nothingWaiting"), note, "0")
        : Warp.part("waiting", Warp.ms(wait, settings, said), note, String(wait));
}

// The shaper's kbit are thousands of bits, so a thousand are a Mb.
const KBIT_PER_MBIT = 1000;

// Upload, from `limit`, or null where warp's daemon heard no shaper: the
// rate the shaper limits the upload to, "7.42 Mb/s", copied as Mb/s, or
// unlimited where it sets none, noted with why in its words
// (modules/shaper/src/why.rs), or with nothing where it gave none.
function upload(limit, settings, said) {
    if (!limit || typeof limit !== "object") {
        return null;
    }
    const why = typeof limit.why === "string" ? limit.why : "";
    if (!Number.isFinite(limit.kbit)) {
        return Warp.part("upload", Warp.worded(said, "unlimited"), why, "");
    }
    const mbit = limit.kbit / KBIT_PER_MBIT;
    return Warp.part("upload", HomeNetwork.rate(mbit, settings), why, String(mbit));
}

// How the row says the way through is slow, in place of how WARP stands,
// while the warp daemon judges it so (modules/warp/src/slow.rs) and WARP
// is up and well, "slow · 120 ms waiting"; null otherwise, as while it's
// degraded or down, which says more. `state` is warp:state (WarpState).
function slowed(state, settings, said) {
    const waited = state?.slow?.waiting_ms;
    const { status, reason } = state?.said ?? {};
    if (!Number.isFinite(waited) || status !== "Connected" || reason === "NetworkDegraded") {
        return null;
    }
    return Warp.worded(said, "slow", { waiting: Warp.ms(waited, settings, said) });
}

// The link's parts with Waiting and Upload among them, from the link's
// record as warp's daemon says it (`record.waiting`, `record.limit`),
// after the Round Trip, or before Through, always last, where there's no
// tunnel to time.
function among(parts, record, settings, said) {
    const found = [part(record?.waiting, settings, said), upload(record?.limit, settings, said)]
        .filter((each) => each !== null);
    if (found.length === 0) {
        return parts;
    }
    const latency = parts.findIndex((each) => each.id === "latency");
    const through = parts.findIndex((each) => each.id === "through");
    const at = latency >= 0 ? latency + 1 : through >= 0 ? through : parts.length;
    return parts.slice(0, at).concat(found, parts.slice(at));
}

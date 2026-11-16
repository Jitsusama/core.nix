.pragma library

// How a place stands by its part of the system, as the system daemon
// judges it (modules/system/src/record.rs): the Network place by the
// network part, so home says what's worth a look on the network without
// stepping in (VKCF3Z's ruling, 2026-11-16 [a1ef0c0a]). Every word is the
// daemon's own, worded for a side there; this only says which to show.

// The health words the daemon gives. Any other is read as not read, so a
// part judged in a word the shell doesn't know never passes for good.
const HEALTHS = ["good", "watch", "bad", "not read"];

// The role each health's mark wears: watch the warning's, bad the one
// the launcher already gives what's failing. Good and not read keep the
// place's own, so only what's worth a look draws the eye.
const MARKS = { watch: "warning", bad: "alert" };

// How the part's place stands, or null to say it as it does without the
// daemon: while it isn't answering, so no place shouts at once before
// the daemon runs, and while the part is good, since a place that groups
// others says nothing while all is well (VKCF3Z, dae7fb67).
//
// `record` is the part's record as the daemon told it, or null; `heard`
// whether the daemon is answering; `notRead` the launcher's words for a
// part it couldn't read, which can't look like good, since a blank only
// ever means none (VKCF3Z, [a1ef0c0a]).
//
// Otherwise `{ sideItems, hue, warns, alerts, state }`: the side's items
// in the daemon's order, the first kept longest, as Choice gives a side
// way by them; the mark's role, or "" for the place's own; whether the
// side wears the warning's colour or the alert's, as the mark does; and
// the whole sentence the foot says while the cursor's on it.
function standing(record, heard, notRead) {
    if (!heard || record === null || typeof record !== "object") {
        return null;
    }
    const health = HEALTHS.includes(record.health) ? record.health : "not read";
    if (health === "good") {
        return null;
    }
    const says = typeof record.says === "string" ? record.says : "";
    if (health === "not read") {
        return {
            sideItems: [{ said: notRead, keep: 1, hue: "muted" }],
            hue: "",
            warns: false,
            alerts: false,
            state: says
        };
    }
    const side = Array.isArray(record.side) ? record.side : [];
    const items = side.filter(said => typeof said === "string" && said !== "")
        .map((said, at) => ({ said, keep: side.length - at }));
    // Before the daemon words its side, a part worth a look still says so
    // by its mark and its foot, rather than by a sentence worded for
    // neither.
    return {
        sideItems: items,
        hue: MARKS[health],
        warns: health === "watch",
        alerts: health === "bad",
        state: says
    };
}

// What one of For You's to-dos says in its note (launcher/Now.qml's item),
// out of what its owner wrote of it through mesh_wait (Waiting.js reads
// which stand), so its own words for when and how long it has waited are
// shown on every item (docs/design.md, As deep as the deepest; VKCF3Z's
// grading, 2026-11-07: raisedAt was only sorted by, and when shown only
// while an item wasn't ready). Its drawer is For You's (Now.qml's
// drawerOf). Kept apart from the view, so the `needs-you-is-true` check
// runs it in Qt's own engine (tests/waiting-on-you.qml). Every word comes
// from the vocabulary, passed in as `said`; what the view works out
// itself, a time as the clock says it, comes in as `told`.
.pragma library
.import "Ago.js" as Ago

// How long an item has waited since its owner raised it, as the launcher
// says a span (Ago.js): "12 min", "15 h", "3 d"; "" for one that says no
// time.
function waited(raisedAt, now) {
    const at = Date.parse(raisedAt ?? "");
    return Number.isNaN(at) ? "" : Ago.span(at, now);
}

// A phrase of the vocabulary's, its blanks filled.
function fill(words, blanks) {
    let filled = words ?? "";
    for (const key of Object.keys(blanks)) {
        filled = filled.replace(`{${key}}`, blanks[key]);
    }
    return filled;
}

// mesh_wait's word for an item Joel can do now, which one ready already
// says by its hue, so it's no reason of its own (VKCF3Z, 2026-11-08).
const NOW = "now";

// Its owner's own words for when, or "".
function whenOf(each) {
    return typeof each.when === "string" ? each.when.trim() : "";
}

// Its note (VKCF3Z, 2026-11-07): its owner's words for when, unless its
// side already says them or they're "now" for one ready; that nobody's
// watching it, when only Joel can say it's done and its session has gone;
// and how long it has waited, last, so it's the first to give way as the
// note is cut from its end. When it's for and by, and the machine it's
// done on, are its drawer's facts, said there alone (Y2YBTS, 2026-11-08).
// One that says none of it, from a file older than mesh_wait's, says why
// it waits instead. `told` is { watched, side, ready, now }: whether
// anything will see it done, the side it shows, whether Joel can do it
// now, and the time now.
function note(each, said, told) {
    const when = whenOf(each);
    const parts = [
        when !== "" && when !== told.side && !(told.ready && when === NOW) ? when : "",
        told.watched ? "" : said.nobody
    ].filter(part => part !== "");
    const words = parts.length === 0 && each.v !== 2 && typeof each.why === "string"
        ? [each.why] : parts;
    const span = waited(each.raisedAt, told.now);
    return words.concat(span === "" ? [] : [fill(said.waitedFor, { span })]).join(" \u00b7 ");
}

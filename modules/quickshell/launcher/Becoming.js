// What an act's rows will be once the daemon it asks has done it, laid over
// what the rows say now, so the launcher draws the change on the key's own
// frame rather than when the daemon says so (Joel, 2026-11-07: "it should
// show right away. the launcher needs to be as live as possible for good
// ux"). An act that asks a daemon, as Wi-Fi's Disconnect asks wifid, says
// the row it becomes; that's held by the row's key until the row its source
// makes reads the same, and taken back if the source refuses, or hasn't
// agreed in time. Each held is { key, row, at }, row null for one that
// goes, as a recording stopped does. An act whose outcome its source can't
// foretell, as a session woken may come back listening or at work, says
// how it knows it's done instead, `done(row)` given the row its source
// makes now or null; and one slower than a daemon, as a pi starting, how
// long it waits, `within` ms.
.pragma library

// The rows as they'll be: each held row laid over the one with its key, the
// fields the launcher added to it kept, and one that goes left out. A row
// that already reads as it was to become is agreed, its hold done with.
// `reads(held, row)` says how a row reads, as its source compares them.
function laid(rows, holds, reads) {
    const agreed = [];
    const shown = [];
    for (const row of rows) {
        const one = holds[row.key ?? ""];
        if (!one) {
            shown.push(row);
        } else if (one.row !== null) {
            const becomes = Object.assign({}, row, one.row);
            if (typeof one.done === "function" ? one.done(row)
                : reads(one, becomes) === reads(one, row)) {
                agreed.push(one.key);
                shown.push(row);
            } else {
                shown.push(Object.assign(becomes, { becoming: true }));
            }
        }
    }
    return { rows: shown, agreed };
}

// Whether the row its source makes now, or null where it makes none, is what
// a hold was to become.
function agrees(one, now, reads) {
    if (typeof one.done === "function") {
        return one.done(now);
    }
    if (one.row === null || now === null) {
        return one.row === now;
    }
    return reads(one, Object.assign({}, now, one.row)) === reads(one, now);
}

// The holds with these keys let go of.
function without(holds, keys) {
    const kept = Object.assign({}, holds);
    for (const key of keys) {
        delete kept[key];
    }
    return kept;
}

// The keys of the holds no source has agreed to within `within` ms of `now`,
// or a hold's own.
function late(holds, now, within) {
    return Object.values(holds).filter(one => one.at + (one.within ?? within) <= now)
        .map(one => one.key);
}

// When the next hold comes due, in ms from `now`, or -1 for none held.
function nextDue(holds, now, within) {
    const ats = Object.values(holds).map(one => one.at + (one.within ?? within) - now);
    return ats.length === 0 ? -1 : Math.max(0, Math.min(...ats));
}

// What `act` did, as words.json's `did` says it, else `done`: a daemon's
// own word for an act is never said to Joel (PN2VZC, 2026-11-12: "Optimize
// for Conferencing and Gaming not sqm yet", and Power's "not
// charge-limit yet").
function did(words, act, done) {
    const known = words ?? {};
    // Its own words alone, as Qt's engine has no Object.hasOwn.
    return Object.keys(known).includes(act) ? known[act] : done;
}

// What the foot says of a hold taken back, in the order the foot's keys are
// given, Return's end first: why, muted, then the failed mark beside what
// wasn't done, in the alert (VKCF3Z, 2026-11-07 [1150c969]), as "Home-5G not
// disconnected · not connected". Each says as one text, so its words keep
// the foot's own spaces between them.
function told(said, colours) {
    const whole = text => text.replace(/ /g, "\u00a0");
    return [[whole(said.why), "", colours.why], [said.mark, whole(said.what), colours.what,
        colours.what]];
}

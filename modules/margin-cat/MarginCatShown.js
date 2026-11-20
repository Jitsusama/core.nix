// How the margin cat's walks and plays reached the screen, as niri said it showed each frame:
// which of niri's answers a walk counts, when it has heard them all, and what the journal says
// of them once it sits (MarginCatScreen.qml). Kept apart from MarginCat.js, which works out
// where it goes, since this only reads back how it went.
.pragma library

// The longest a walk that's done waits, in ms, to hear of the frames still on their way to the
// screen as it sat, before it tells of the walk without them. A frame that was never asked of
// is never answered, so this ends the wait for one; a loaded builder's nested niri took up to
// 160 ms between two frames (2026-11-19).
const ANSWERS = 2000;

// Whether niri's answer for `frame` counts toward a walk's frames, and whether it ends the wait
// to tell of it. A frame begun before it set off, its `from`, is the walk before's, still on
// its way when that one was told so this one could set off; it neither counts nor ends the
// wait. While it walks, `last` is null and every later answer counts. Once it's done, `last`
// is the last frame begun as it sat: niri answers the frames in the order it showed them, so its
// answer for `last`, or for any frame after, says every one before is in, and one after isn't
// the walk's. Told at once instead, a walk on a busy screen said too few: four of its 135
// frames were still on their way on a builder, 62 ms apart (2026-11-19).
function heard(frame, from, last) {
    if (frame <= from) {
        return { counts: false, ends: false };
    }
    if (last === null) {
        return { counts: true, ends: false };
    }
    return { counts: frame <= last, ends: frame >= last };
}

// How a walk reached the screen, as the journal says it once it sits:
// `shown`, when niri presented each of its frames, in monotonic ms, so
// the gaps between them say whether it moved at the screen's rate;
// `drawing`, how long each took the shell to sync and render, its own
// cost of a frame, said as the typical frame's and the slowest's, since
// a busy machine stretches a frame or two by what else it runs;
// `unseen`, how many niri never showed (Joel,
// 2026-11-12: "we need proper smooth animations with character to them");
// and as `counted`, its `begun`, how many frames the shell began
// meanwhile, so a walk whose frames went unasked of says so rather than
// reading slower than it ran, and `what` it was: a walk, or a hop, said
// the same way. Its `resumed`, each frame in `shown` that came first after
// it was held under the launcher, leaves the hold out of its gaps, and the
// journal says how many times it was held.
function walked(shown, drawing, unseen, counted) {
    const begun = counted?.begun ?? 0;
    const resumed = counted?.resumed ?? [];
    const gaps = shown.slice(1).map((at, i) => at - shown[i])
        .filter((_, i) => !resumed.includes(i + 1)).sort((a, b) => a - b);
    const costs = drawing.slice().sort((a, b) => a - b);
    const tenths = ms => Math.round((ms ?? 0) * 10) / 10;
    const said = {
        frames: shown.length,
        median: tenths(gaps[(gaps.length - 1) >> 1]),
        p99: tenths(gaps[Math.max(0, Math.ceil(gaps.length * 0.99) - 1)]),
        slowest: tenths(gaps[gaps.length - 1]),
        typical: tenths(costs[(costs.length - 1) >> 1]),
        drawn: tenths(drawing.reduce((most, ms) => Math.max(most, ms), 0)),
        unseen,
        begun,
        stepped: counted?.stepped ?? 0,
        held: resumed.length
    };
    const line = `cat: a ${counted?.what ?? "walk"} presented ${said.frames} of its ${begun} frames `
        + "a median "        + `${said.median} ms `
        + `apart, the p99 ${said.p99} ms, the slowest ${said.slowest} ms, each drawn in `
        + `${said.typical} ms typically and within ${said.drawn} ms, ${unseen} never shown, `
        + `stepped ${said.stepped} times`
        + (said.held > 0 ? `, held under the launcher ${heldTimes(said.held)}` : "");
    return { said, line };
}

function heldTimes(times) {
    return times === 1 ? "once" : `${times} times`;
}

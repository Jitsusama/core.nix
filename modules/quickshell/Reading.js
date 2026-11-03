// How a name too long for its row slides to show its end (Choice.qml): a
// reading motion, steady with soft ends, not an arrival. It holds one
// reading pace from end to end but for a short ramp at each, where it
// gathers speed from rest and settles back to it, so it never starts or
// stops hard and the eye follows one speed (Joel, 2026-10-29: the slide
// was too slow and started and stopped abruptly; VKCF3Z's [e804ce8d]).
// Tested as it is by tests/reading.qml.
.pragma library

// What a cut leaves dangling at its end, a separator or a pause, which
// goes before the ellipsis does.
const DANGLING = /[\s\u00b7,;:]+$/u;

// The longest each ramp lasts, in seconds. A slide the pace alone would
// take less than two ramps for ramps for half that time, so the pace
// still holds between them.
const RAMP = 0.2;

// The slide over `over` pixels at `pace` pixels a second: how long it
// lasts, in seconds, and the easing curve QML's BezierSpline takes, as
// three pieces: speeding up evenly, the pace held, and slowing evenly.
// Speeding up evenly from rest to the pace covers half the ground the
// pace would in that time, so the slide lasts a ramp longer than the
// pace alone would take, and the pieces meet at the same speed.
function slide(over, pace) {
    if (over <= 0 || pace <= 0) {
        return { seconds: 0, curve: [1 / 3, 1 / 3, 2 / 3, 2 / 3, 1, 1] };
    }
    const steady = over / pace;
    const ramp = Math.min(RAMP, steady / 2);
    const seconds = steady + ramp;
    // The share of the time each ramp takes, and of the ground.
    const time = ramp / seconds;
    const ground = pace * ramp / 2 / over;
    const third = (from, to, part) => from + (to - from) * part;
    return {
        seconds,
        curve: [
            time / 3, 0, 2 * time / 3, ground / 3, time, ground,
            third(time, 1 - time, 1 / 3), third(ground, 1 - ground, 1 / 3),
            third(time, 1 - time, 2 / 3), third(ground, 1 - ground, 2 / 3),
            1 - time, 1 - ground,
            1 - 2 * time / 3, 1 - ground / 3, 1 - time / 3, 1, 1, 1
        ]
    };
}

// Words too long for their room, given way at the last whole word that
// fits, an ellipsis after it, so what's said is never cut mid-word
// (docs/design.md, Collapsing; VKCF3Z, 2026-11-02: "Open-Meteo didn't
// a\u2026"). `width` measures a text in the face it's drawn in. Where not
// even the first word fits, nothing is said, rather than a piece of one.
function wholeWords(words, room, width) {
    if (width(words) <= room) {
        return words;
    }
    const parts = words.split(" ");
    for (let count = parts.length - 1; count > 0; count -= 1) {
        const cut = `${parts.slice(0, count).join(" ").replace(DANGLING, "")}\u2026`;
        if (width(cut) <= room) {
            return cut;
        }
    }
    return "";
}

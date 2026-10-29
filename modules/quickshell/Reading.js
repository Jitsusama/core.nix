// How a name too long for its row slides to show its end (Choice.qml): a
// reading motion, steady with soft ends, not an arrival. It holds one
// reading pace from end to end but for a short ramp at each, where it
// gathers speed from rest and settles back to it, so it never starts or
// stops hard and the eye follows one speed (Joel, 2026-10-29: the slide
// was too slow and started and stopped abruptly; VKCF3Z's [e804ce8d]).
// Tested as it is by tests/reading.qml.
.pragma library

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

// Whether a notice arriving knocks (Notices.qml's chime), worked out apart
// from the sound so tests/notice-line.qml holds it in Qt's own engine, as
// the arrival line's own choosing is. A call being on is the one thing it
// can't know here: Notices.qml asks the calls program first, and the sound
// plays only when it says none is on.
.pragma library

// The least time between two knocks, so a burst knocks once.
const APART = 2000;

// Whether to knock now: only for one the notices daemon says chimes, a
// message to Joel himself; only with a sound set; never while notices are
// held (Quiet.qml: Do Not Disturb, or the screen shared); and not again
// within `APART` of the last, `last` being when that was, 0 for never.
function knocks(chimes, sound, held, now, last) {
    return chimes && sound !== "" && !held && now - last >= APART;
}

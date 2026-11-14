// Whether a notice arriving knocks (Notices.qml's chime), worked out apart
// from the sound so tests/notice-line.qml holds it in Qt's own engine, as
// the arrival line's own choosing is. A call being on is the one thing it
// can't know here: Notices.qml asks the calls program first, but for an
// alarm, and the sound plays only when it says none is on.
.pragma library

// The least time between two knocks, so a burst knocks once.
const APART = 2000;

// Whether to knock at `now`, `last` being when it last did, 0 for never:
// only for one the notices daemon says chimes, a message to Joel himself,
// or alarms, someone stopping his agents with his safe word; only with a
// sound set; never while notices are held (Quiet.qml: Do Not Disturb, or
// the screen shared) but for an alarm; and not again within `APART` of
// the last.
function knocks({ chimes, alarms, sound, held }, now, last) {
    return (chimes || alarms) && sound !== "" && (alarms || !held) && now - last >= APART;
}

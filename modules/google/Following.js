// Which Meet calls the google program follows, asking Meet who's in each
// (src/watch.rs's follow), for Calendar.qml, which says it: the calls open
// in windows of their own, while the launcher is open to show who's in
// them (Meetings.qml's callsWatched) or a call is being recorded, whose
// transcript names its people from them (call-transcripts' meet.rs);
// otherwise none, since each look is Meet asked once or twice more.
//
// The shell says its follow again every RENEW while it follows any, and
// the program lets one go once its FOLLOW_LASTS has passed unsaid, so a
// shell that went away leaves nothing followed.
.pragma library

const RENEW = 60_000;

// Whether call-transcripts is recording a call, from the calls it serves
// over calls.sock, by key: one is served `recording` while it is (its
// book.rs). With no call-transcripts, or one gone away, none is held, so
// none is, and calls are followed as the launcher alone says.
function recording(held) {
    return Object.values(held).some(call => call.recording === true);
}

// The codes of `calls`, each { code }, to follow while the launcher's
// `open` or call-transcripts' `held` calls say one is recorded; none while
// neither is. Every Meet call open is followed, not only the recorded
// call's, since a call with no meeting is known only as the one Meet call
// under way (meet.rs).
function follows(calls, open, held) {
    return open || recording(held) ? calls.map(call => call.code) : [];
}

// Whether the follow is said again each RENEW: only while it holds a call.
function renews(codes) {
    return codes.length > 0;
}

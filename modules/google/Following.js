// Which Meet calls the google program follows, asking Meet who's in each
// (src/watch.rs's follow), for Calendar.qml, which says it: the calls open
// in windows of their own, while the launcher is open to show who's in
// them (Meetings.qml's callsWatched) or a call is being recorded, whose
// transcript names its people from them (call-transcripts' meet.rs);
// otherwise none, since each look is Meet asked once or twice more.
//
// The shell says its follow again every RENEW while it follows any, and
// the program lets one go once LASTS has passed unsaid, so a shell that
// went away leaves nothing followed. LASTS is watch.rs's FOLLOW_LASTS, and
// the-desktop-tells-the-next-meeting fails when the two differ.
.pragma library

const RENEW = 60_000;
const LASTS = 90_000;

// Whether call-transcripts is recording a call, from the calls it serves
// over calls.sock, by key: one is served `recording` while it is (its
// book.rs). With no call-transcripts, nothing is served, so none is, and
// calls are followed as the launcher alone says.
function recording(held) {
    return Object.values(held).some(call => call.recording === true);
}

// The codes of `calls`, each { code }, to follow while the launcher's
// `open` or a call's `recorded`; none while neither is.
function followed(calls, open, recorded) {
    return open || recorded ? calls.map(call => call.code) : [];
}

// Whether the follow is said again each RENEW: only while it holds a call.
function renews(codes) {
    return codes.length > 0;
}

// What the program follows at `now`, as watch.rs's following reckons it,
// when it was last told `told`, { at, codes }, or never: those codes until
// LASTS has passed since, and none after.
function following(told, now) {
    return told !== null && now - told.at <= LASTS ? told.codes : [];
}

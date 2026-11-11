.pragma library
// Where the viewers of sessions pi's host runs open (Sessions.qml), and
// when such a session counts as back: tests/places.qml holds it to the
// cases there. A hosted session's conversation outlives its window, as its
// host keeps it, so only its viewer is ever opened for it: where its window
// sat as it moved onto the host, or where the last boot kept it.

// Where a session's `window` sits, as the windows daemon's record of it
// says, kept `at` a time in milliseconds, so its viewer opens there once
// pi's host runs it (viewersDue): its workspace, and its column where it
// has one, as a floating window has none. A session with no window, or
// one niri hasn't laid out, keeps nothing.
function placeOf(window, at) {
    if (!window || window.workspace === null || window.workspace === undefined) {
        return null;
    }
    return { workspace: window.workspace, column: window.column ?? null, at };
}

// How long a kept place waits for its session to run hosted: a day, past
// which its session most likely never moved, busy past the cutover's
// deadline or refusing it (2QZBX9, [906a6c10]).
const KEPT_FOR = 24 * 60 * 60 * 1000;

// Of the places `kept`, by pi's session id, the viewers to open `now`
// among `sessions` as Sessions.qml lists them, each with where it goes
// (`into`, viewerPlace among niri's `workspaces` by id, in `screen`), and the places
// still to keep: one whose session runs hosted with words to view it by
// opens, unless its viewer is open already, and is kept no more either
// way; one with nowhere to open yet waits; one kept past KEPT_FOR lapses.
// Nothing opens unless Joel is `away`, and nothing kept before `since`,
// when this shell started, both said in `screen` too: a place kept by a shell before it belongs to
// a move that has ended, and a shell restarted by a switch opened two
// such viewers on a new workspace while Joel was there (OSPHT4
// [0762284c], 13:41:38 on 11 November). One parked waits too, since
// viewing it is what wakes it, which woke a parked QCXZEB then.
function viewersDue(kept, sessions, now, screen) {
    const { workspaces, since, away } = screen ?? {};
    const open = [];
    const still = {};
    for (const id of Object.keys(kept).filter(one => now - kept[one].at <= KEPT_FOR)) {
        const place = kept[id];
        const session = sessions.find(one => one.presence?.session === id
            && one.presence?.hosted);
        const viewable = session !== undefined && Array.isArray(session.attach)
            && session.attach.length > 0;
        if (session?.window) {
            // Its viewer is open, so its place is done with.
        } else if (!viewable || away !== true || !(place.at >= since) || parkedHere(session)) {
            still[id] = place;
        } else {
            const into = viewerPlace(place, workspaces ?? {});
            if (into === null) {
                still[id] = place;
            } else {
                open.push({ session, into });
            }
        }
    }
    return { open, kept: still };
}

// Whether a hosted `session`, as Sessions.qml lists it, is parked in its
// host, as the mesh says in its presence (the harness's park.ts): viewing
// it would wake it.
function parkedHere(session) {
    const parked = session?.presence?.parked;
    return parked !== undefined && parked !== null && parked !== false;
}

// Where a moved session's viewer opens, always a place niri is told of,
// so it opens there behind whatever has the focus and with no window
// flashing where Joel is (opens-next-window-on.patch): its own workspace
// and column while that workspace is there, and nowhere yet while Joel is
// on it, since a window appearing beside his would catch his eye, so it
// waits for him to leave. Joel, 10 November: six windows had taken his
// focus that day. One whose workspace has gone waits rather than taking
// another, until Go To or Reopen Them All opens it or its place lapses:
// the empty one niri keeps last became a new workspace of two viewers
// nobody asked for (OSPHT4 [0762284c], 11 November).
function viewerPlace(place, workspaces) {
    const own = workspaces[place.workspace];
    return own && !own.is_focused ? { workspace: own.id, column: place.column } : null;
}


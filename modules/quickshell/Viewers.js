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
// (`into`, viewerPlace among niri's `workspaces` by id), and the places
// still to keep: one whose session runs hosted with words to view it by
// opens, unless its viewer is open already, and is kept no more either
// way; one with nowhere to open yet waits; one kept past KEPT_FOR lapses.
function viewersDue(kept, sessions, now, workspaces) {
    const open = [];
    const still = {};
    for (const id of Object.keys(kept).filter(one => now - kept[one].at <= KEPT_FOR)) {
        const place = kept[id];
        const session = sessions.find(one => one.presence?.session === id
            && one.presence?.hosted);
        const viewable = session !== undefined && Array.isArray(session.attach)
            && session.attach.length > 0;
        if (!viewable) {
            still[id] = place;
        } else if (!session.window) {
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

// Where a moved session's viewer opens, always a place niri is told of,
// so it opens there behind whatever has the focus and with no window
// flashing where Joel is (opens-next-window-on.patch): its own workspace
// and column while that workspace is there; the empty workspace niri keeps
// last on Joel's screen once it has gone, as niri takes a workspace away
// with its last window, which a session alone on its workspace leaves it;
// and nowhere yet while Joel is on the workspace it would open on, since
// a window appearing beside his would catch his eye, so it waits for him
// to leave. Joel, 10 November: six windows had taken his focus that day.
function viewerPlace(place, workspaces) {
    const all = Object.values(workspaces);
    const own = workspaces[place.workspace];
    if (own) {
        return own.is_focused ? null : { workspace: own.id, column: place.column };
    }
    const focused = all.find(one => one.is_focused);
    // With no focused workspace known, there's no screen of Joel's to prefer, so the spare
    // can be an empty workspace on any screen; it's still never one he is on.
    const [spare] = all
        .filter(one => one.active_window_id === null && !one.is_focused
            && (focused === undefined || one.output === focused.output))
        .sort((one, other) => other.idx - one.idx);
    return spare ? { workspace: spare.id, column: null } : null;
}

// Which of `sessions`, as Sessions.qml lists those running and parked, are
// back here, as Places.lastBoot's `here` takes them, { id, quest }: each
// one here, but one pi's host runs only once its viewer is open, or at once
// when the boot being restored kept no place of its window (`arranged`, by
// id), since it had none to bring back. Its host brings the conversation
// back before Joel asks, so counting it back by that alone offered no
// viewer (2026-11-11, 430 and 457 after the 11:06 boot).
function backHere(sessions, arranged) {
    return sessions.filter(one => one.here && (!one.presence?.hosted || one.window
        || !arranged?.[one.presence.session]))
        .map(one => ({ id: one.presence.session,
            quest: one.presence.questInfo?.id ?? one.presence.quest ?? "" }));
}

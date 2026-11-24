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

// How long after Joel asks Reopen Them All a sleeper it wakes, or a viewer
// it kept for him to leave its workspace, still opens its window: long
// enough for the wakes it paces out to land, so an ask never opens a
// window hours later, when something else wakes it (four restarted at
// 19:48 on 16 November opened windows kept from earlier).
const ASKED_FOR = 15 * 60 * 1000;

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
// viewing it is what wakes it, which woke a parked QCXZEB then. One
// Reopen Them All kept while Joel was on its workspace (`asked`), or with
// the words its viewer waits for its host by (`waiting`), opens once he
// isn't there, him away or not, since he asked for it: by its session's
// words once its host serves it, else by its own, without its session.
// Kept for its host alone, it opened by itself nearly two hours later on
// 20 November, so it's let go ASKED_FOR after the ask, or once its
// session is back as a plain pi, whose window it would never be. While
// Joel stays on its workspace (`there`) that time doesn't run, since he
// may work there for hours and the window is still his to come back to;
// left, it opens at once, or ASKED_FOR runs from then if it can't yet.
function viewersDue(kept, sessions, now, screen) {
    const { workspaces, since, away } = screen ?? {};
    // Whether a session's window is open: as niri lists it where the
    // shell trusts that list (Sessions.qml's listedNow), so one the
    // windows daemon's index alone still names, as in a gap of its, is
    // no viewer open.
    const listed = screen?.listed ?? (window => Boolean(window));
    const open = [];
    const still = {};
    for (const id of Object.keys(kept)) {
        const place = kept[id];
        const session = sessions.find(one => one.presence?.session === id
            && one.presence?.hosted);
        const viewable = session !== undefined && Array.isArray(session.attach)
            && session.attach.length > 0;
        if (place.slept === true) {
            // A sleeper is any session of its id, held or hosted. Only a
            // place Joel asked for, by Reopen Them All or by waking it
            // himself (Sessions.qml's wake), opens a window as it wakes,
            // and only within ASKED_FOR of the ask, or never when he
            // asked can't be read; one kept as it went to sleep, which
            // nobody asked to see, is let go once it wakes, with no window
            // (Joel, 16 November: a window opens only when asked for).
            const sleeper = sessions.find(one => one.presence?.session === id);
            const asked = place.asked === true;
            const stale = asked && !(now - place.at <= ASKED_FOR);
            const woke = stale ? { kept: null, into: null }
                : sleeperDue(place, sleeper, now, { workspaces: workspaces ?? {}, listed });
            if (woke.into !== null && asked && (sleeper.held || viewable)) {
                open.push({ session: sleeper, into: woke.into });
            } else if (woke.into !== null && asked) {
                still[id] = place;
            } else if (woke.into === null && woke.kept !== null) {
                still[id] = woke.kept;
            }
        } else if (now - place.at > KEPT_FOR) {
            // Lapsed, its session most likely never moved.
        } else if (listed(session?.window)) {
            // Its viewer is open, so its place is done with.
        } else if (place.asked === true || waitingWords({ view: place.waiting }) !== null) {
            const words = waitingWords({ view: place.waiting }) !== null;
            const ours = place.at >= since;
            const joelThere = workspaces?.[place.workspace]?.is_focused === true;
            // A place an earlier shell kept lapses from its ask, Joel there
            // or not, since this shell never saw him leave.
            const lapsed = !(now - place.at <= ASKED_FOR) && (place.there !== true || !ours);
            const into = viewerPlace(place, workspaces ?? {});
            if (lapsed || sessions.some(one => one.presence?.session === id
                && !one.presence?.hosted)) {
                // Asked too long ago, or back as a plain pi: let go.
            } else if (!ours || joelThere) {
                still[id] = place;
            } else if (into !== null && (viewable || words)
                && (!parkedHere(session) || place.asked === true)) {
                // One Joel asked for is viewed parked too, which wakes it,
                // as the walk does where he isn't.
                open.push(viewable ? { session, into } : { id, place, into });
            } else if (place.there === true) {
                // Left, it can't open yet: ASKED_FOR runs from now. An
                // undefined mark is dropped as the place is kept.
                still[id] = Object.assign({}, place, { at: now, there: undefined });
            } else {
                still[id] = place;
            }
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

// Of a place kept as its session went to sleep, in pi's host or in its
// mycelium holder (`slept`, as Sessions.qml's keepPlaceAsleep keeps it),
// what's still to keep and where its window would open now, if anywhere,
// which viewersDue opens only for a place Joel asked for (`asked`).
// Its window closed so a sleeper costs next to nothing, and anything that
// needs it wakes it where it sleeps (Joel,
// 14 November: sessions put to sleep have no windows, and are resumed
// whenever needed, at once). So its place is due once it has been seen
// asleep and is awake again, while Joel is here too and from a place kept
// before this shell started, since it's waking that makes it due and never
// a switch; but never on the workspace he's on (viewerPlace), and never
// while it sleeps. Until it has been seen asleep it waits, its window
// still open as its place is kept just before it closes, or gone before
// the host has parked it, and lapses as any place does; once seen it's
// kept however long it sleeps. One whose viewer is open again, by Go To
// or by waking, is let go. `screen` holds niri's `workspaces` and
// `listed`, whether a window is open, as viewersDue's.
function sleeperDue(place, session, now, screen) {
    const { workspaces, listed } = screen;
    const kept = { kept: place, into: null };
    if (session === undefined) {
        return place.asleep === true || now - place.at <= KEPT_FOR ? kept
            : { kept: null, into: null };
    }
    if (parkedHere(session)) {
        return { kept: Object.assign({}, place, { asleep: true }), into: null };
    }
    if (place.asleep !== true) {
        return now - place.at > KEPT_FOR ? { kept: null, into: null } : kept;
    }
    if (listed(session.window)) {
        return { kept: null, into: null };
    }
    const into = viewerPlace(place, workspaces);
    return into === null ? kept : { kept: null, into };
}

// Whether a `session`, as Sessions.qml lists it, is parked: in pi's host,
// as the mesh says in its presence (the harness's park.ts), where viewing
// it would wake it, or by its holder, as its record's state says.
function parkedHere(session) {
    const parked = session?.presence?.parked;
    return parked !== undefined && parked !== null && parked !== false
        || session?.presence?.state === "parked";
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

// The slashes a folder's path may end in, which name no folder.
const TRAILING_SLASHES = /\/+$/;

// The title a viewer's window opens with: the one pi's host gives it once
// its conversation shows, as the session's record says it (`said`), else
// "π - ", its quest's name and " - " its folder, or its folder alone
// where its name is its folder, as the harness names one with no quest
// (windowTitle in its presence.ts). kitty names a window for the command
// it runs until that names it, and a viewer waiting for its host read
// "mycelium" for over an hour on 20 November, a window Joel couldn't
// tell for a session.
function viewerTitle(said, title, cwd) {
    if (typeof said === "string" && said.startsWith("\u03c0 - ")) {
        return said;
    }
    const trimmed = typeof cwd === "string" ? cwd.replace(TRAILING_SLASHES, "") : "";
    const folder = trimmed.slice(trimmed.lastIndexOf("/") + 1);
    const name = typeof title === "string" && title !== folder ? title : "";
    return ["\u03c0", name, folder].filter(part => part !== "").join(" - ");
}

// `words`, run once the window is titled `title`. It's set within what
// the realm's holder runs, which keeps the title its program set last, so
// a window that joins later is titled too, and it stands until the viewer
// titles the window itself. Control characters become spaces, so the
// title can't end the sequence early and say anything else to the
// terminal.
function titled(title, words) {
    const clean = Array.from(String(title), c => (controlCharacter(c) ? " " : c)).join("");
    return ["sh", "-c", 'printf "\\033]2;%s\\007" "$0"; exec "$@"', clean].concat(words);
}

// Whether `c` is a C0 or C1 control character, or DEL, any of which a
// terminal could read as the start or end of a sequence.
function controlCharacter(c) {
    const code = c.charCodeAt(0);
    return code < 0x20 || (code >= 0x7f && code < 0xa0);
}

// The words a hosted session's viewer runs while its host isn't serving
// it yet, as the harness's restore entry says them (`view`): its join,
// whose attach fails while the host the roster names is away, and which
// looks again, saying so in the window, until the host shows the
// conversation. None where the entry says none whole, as one an older
// harness wrote doesn't.
function waitingWords(one) {
    const words = one?.view;
    return Array.isArray(words) && words.length > 0
        && words.every(word => typeof word === "string") ? words.slice() : null;
}

// What the launcher's foot says a row asked for by itself by, as the
// shell tells it (Sessions.qml's unhosted): the row's name as the daemon
// words it, else the session's id, since an empty name, as a row the
// daemon words without one has, is no name to say it by.
function rowSaid(id, row) {
    return { address: String(row?.name || id), machine: "" };
}


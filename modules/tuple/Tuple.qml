pragma Singleton
pragma ComponentBehavior: Bound
// Tuple's calls, as the shell sees them, for the margin's piece (Call.qml)
// and the launcher's rows (Calls.qml). The call Joel's in is tuple-call's
// one record, held over the daemon interface (Daemon.qml, call/): how it
// stands and since when, who's in it by name, and for Joel and each of
// them, whether they're muted, sharing their screen or showing their
// camera; who's calling while a ring lasts, the calls missed, Joel's
// room's link, and Recent, the calls ended and missed lately. Joining,
// leaving, muting, sharing and opening the room are asks of the daemon,
// which does each through Tuple's own command.
//
// Tuple on Linux doesn't yet tell Joel when someone joins or leaves, or
// that everyone else has left, so the shell does, as notices of Tuple's,
// from how the record changes. Who came and went matters only as it
// happens, since the margin says who's there, so those ask not to be
// kept; one that offers something is kept, so the keys reach it too, and
// withdrawn once it can't be done; and a call that rang and wasn't
// answered is kept, as a missed call. Only what happened since the shell
// started is news.
//
// A call's link on the clipboard, as one pasted in a chat arrives, is kept
// too, so the launcher can offer to join it; nothing else copied is kept.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Tuple.js" as Log

Singleton {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR") ?? ""

    // The record of a call nobody's in, as the daemon says one, until it
    // says otherwise.
    readonly property var none: ({
        state: "none",
        since: null,
        muted: null,
        sharing: false,
        camera: false,
        people: [],
        ringing: null,
        missed: [],
        room: null,
        recent: []
    })

    // The call as tuple-call last said it.
    property var call: none
    // Whether Tuple's stream is followed, which it is only while Tuple runs
    // and someone's signed in.
    readonly property bool answering: calls.sources.tuple === "answering"

    readonly property bool calling: call.state === "connecting" || call.state === "connected"
    // Whether it's still connecting, or is again.
    readonly property bool connecting: call.state === "connecting"
    // When the call connected, in milliseconds since 1970, or when the
    // shell heard of it, until it does.
    property real heardAt: 0
    readonly property real since: calling ? call.since ?? heardAt : 0
    // Who's in it besides Joel, each by the id Tuple knows them by.
    readonly property var others: call.people.map(one => one.id)
    // Whose screen is shown in the call, by the first sharing it.
    readonly property var showing: call.people.filter(one => one.sharing).map(one => one.id)
    // Whose cameras are on, besides Joel's.
    readonly property var watching: call.people.filter(one => one.camera).map(one => one.id)
    // Whether Joel's screen and camera are in the call, as Tuple says.
    readonly property bool sending: call.sharing
    readonly property bool filming: call.camera
    // Whether Tuple keeps Joel's voice from the call, once it's connected:
    // true, false, or null, unknown.
    readonly property var muted: call.muted
    // Who's calling Joel, by the id Tuple knows them by, while a ring
    // lasts, or "".
    readonly property string ringing: call.ringing === null ? "" : call.ringing.id || call.ringing.who
    // The names the record gives, by id: those in the call, calling, and
    // missed.
    readonly property var names: {
        const named = {};
        for (const one of call.missed.concat(call.ringing === null ? [] : [call.ringing], call.people))
            if (one.name !== "")
                named[one.id || one.who] = one.name;
        return named;
    }
    // Joel's room's link, once Tuple's said it.
    readonly property string room: call.room ?? ""
    // The calls that ended and were missed lately, newest first, as
    // tuple-call keeps them across restarts, each { people: [{ id, name,
    // email }], since, ended, missed }: when it connected or began to
    // ring, and when it or the ring ended. The launcher lists them as
    // Tuple's Recent (Calls.qml).
    readonly property var recent: Log.newestFirst(call.recent)
    // The call link last copied, until something else is. A copy with no
    // text, such as a screenshot, or one marked as a secret, has none, so
    // a link copied before it isn't offered after it's gone. Its lines are
    // run together, as they were before the shell shared one watcher of
    // the clipboard (Copied.qml).
    readonly property string copied: Log.link(Copied.text.split("\n").join(" "))

    // When the shell started, before which nothing is news.
    readonly property real started: Date.now()

    // Whether a screen or window is being cast to a call now, as niri says.
    readonly property bool casting: Object.values(Niri.casts)
        .some(cast => cast.kind === "PipeWire" && cast.is_active)
    // A link copied that isn't Joel's own room, which he opens instead.
    readonly property string joinable: calling || copied === room ? "" : copied

    // Whether Tuple has a window to show, as a call it's in has.
    readonly property bool windowed: Object.values(Niri.windows)
        .some(window => window.app_id === "Tuple")

    // What can be done about Tuple's calls now, most wanted first, each
    // there only when it can be done, as the launcher's rows say them:
    // found anywhere by name and as Tuple's own acts in Programs › Tuple
    // (Calls.qml). A ringing call's window, where it's answered, comes
    // first; in a call, its window while it has one, whether Joel's Muted
    // there, sharing while nothing is shared, and leaving; out of one,
    // opening Joel's room, or joining the call whose link was just copied;
    // and copying his room's link once it's known.
    readonly property var controls: {
        // Named and found as the launcher's vocabulary says (Words.qml),
        // never by the program's name, which only finds them.
        const said = (key, state, fill) => ({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state, fill),
            // Its words' state, so the : line names the command for it.
            wordState: state
        });
        const rows = [];
        if (ringing !== "" && !calling)
            rows.push(Object.assign(said("tuple:ringing", "", { who: name(ringing) || "someone" }), {
                glyph: Theme.glyph.call,
                hue: "person",
                note: "answer it in Tuple's window",
                run: () => root.show()
            }));
        if (calling) {
            if (windowed)
                rows.push(Object.assign(said("tuple:show"), {
                    glyph: Theme.glyph.call,
                    hue: "person",
                    run: () => root.show()
                }));
            // Muted is a switch, one row whose name never changes (VKCF3Z's
            // design, RSCH-20261028-2NK1E5, rule 8): it says yes or no,
            // and Return flips it and keeps the launcher.
            const quiet = muted === true;
            rows.push(Object.assign(said("tuple:mute", quiet ? "muted" : ""), {
                glyph: quiet ? Theme.glyph.microphone : Theme.glyph.muted,
                hue: "action",
                side: quiet ? "yes" : "no",
                stays: true,
                later: true,
                run: () => root.mute(!quiet)
            }));
            if (!casting)
                rows.push(Object.assign(said("tuple:share"), {
                    glyph: Theme.glyph.sharing,
                    hue: "action",
                    later: true,
                    run: () => root.share()
                }));
            rows.push(Object.assign(said("tuple:leave"), {
                glyph: Theme.glyph.hangUp,
                hue: "action",
                later: true,
                run: () => root.leave()
            }));
        } else {
            rows.push(Object.assign(said("tuple:open"), {
                glyph: Theme.glyph.openRoom,
                hue: "action",
                note: "and copy its link to hand on",
                later: true,
                run: () => root.open()
            }));
            const link = joinable;
            if (link !== "")
                rows.push(Object.assign(said("tuple:join"), {
                    glyph: Theme.glyph.joinCall,
                    hue: "action",
                    note: link.replace(/^https:\/\//, ""),
                    later: true,
                    run: () => root.join(link)
                }));
        }
        if (room !== "")
            rows.push(Object.assign(said("tuple:link"), {
                glyph: Theme.glyph.link,
                hue: "action",
                note: room.replace(/^https:\/\//, ""),
                run: () => root.copyRoom()
            }));
        return rows;
    }

    // How Tuple stands, in a few words: who's calling, or whether Joel's
    // in a call and heard there, or "" while he's in none.
    readonly property string standing: ringing !== "" && !calling
        ? `${name(ringing) || "someone"} calling`
        : calling ? (muted === true ? "in a call, muted" : "in a call") : ""

    // Someone's name, by the id Tuple knows them by, or "" when Tuple
    // doesn't say.
    function name(who) {
        return names[`${who}`] ?? "";
    }

    // The people in the call, as a sentence says them.
    readonly property string company: Log.company(others.map(who => name(who)))

    // Opens Joel's room and copies its link, to hand to whoever's joining.
    function open() {
        calls.ask({ ask: "open" }, answer => {
            if (typeof answer?.link === "string" && answer.link !== "")
                Niri.spawn(["wl-copy", answer.link]);
        });
    }

    function join(url) {
        calls.ask({ ask: "join", link: url });
    }

    function leave() {
        calls.ask({ ask: "leave" });
    }

    // Tuple asks the portal, which asks Joel what to share.
    function share() {
        calls.ask({ ask: "share" });
    }

    // Once Tuple says it's done, the record says so.
    function mute(quiet) {
        calls.ask({ ask: "mute", muted: quiet });
    }

    function copyRoom() {
        if (room !== "")
            Niri.spawn(["wl-copy", room]);
    }

    // Brings forward the window Tuple opened last, where a ringing call is
    // answered.
    function show() {
        const tuples = Object.values(Niri.windows).filter(window => window.app_id === "Tuple");
        if (tuples.length > 0)
            Niri.act({ FocusWindow: { id: Math.max(...tuples.map(window => window.id)) } });
    }

    // The daemon holding the call.
    Daemon {
        id: calls

        socket: `${root.runtime}/daemons/tuple-call.sock`
        kind: "call"
        onHeard: root.took(calls.held["call:current"] ?? root.none)
    }

    // How long after a call connects its people are still arriving, as
    // Tuple adds those already in it one by one.
    readonly property int settling: 5 * 1000

    // The call as it's said now, told as notices where it's news.
    function took(now) {
        const before = call;
        const was = before.state === "connecting" || before.state === "connected";
        const is = now.state === "connecting" || now.state === "connected";
        if (is && !was)
            heardAt = now.heard ?? Date.now();
        call = now;
        if (was && !is)
            withdraw("leave");
        if (before.ringing !== null && now.ringing === null)
            withdraw("show");
        const ring = now.ringing;
        if (ring !== null && ring.since >= started
            && (before.ringing === null || before.ringing.who !== ring.who
                || before.ringing.since !== ring.since))
            notify(`${ring.name || "Someone"} is calling`, "Answer in Tuple's window", [
                ["show", "Go To the Call"]
            ], true);
        const told = before.missed.map(missed => `${missed.who}@${missed.at}`);
        for (const missed of now.missed)
            if (missed.at >= started && !told.includes(`${missed.who}@${missed.at}`))
                notify(`Missed a call from ${missed.name || "someone"}`, "", [], true);
        // Whoever's there as the call connects was there already, and
        // everyone goes as Joel leaves, so neither is news.
        if (!was || !is || before.since === null || now.since === null
            || Date.now() - now.since < settling)
            return;
        const named = new Map(before.people.concat(now.people).map(one => [one.id, one.name]));
        const said = id => named.get(id) || "Someone";
        const were = before.people.map(one => one.id);
        const are = now.people.map(one => one.id);
        for (const id of are.filter(id => !were.includes(id)))
            notify(`${said(id)} joined the call`, "", [], false);
        const went = were.filter(id => !are.includes(id));
        for (const id of went)
            if (are.length === 0 && id === went[went.length - 1])
                notify(`${said(id)} left the call`, "You're the only one still in it", [
                    ["leave", "Leave the Call"]
                ], true);
            else
                notify(`${said(id)} left the call`, "", [], false);
    }

    // The notices on screen that offer something, by their ids, each with
    // what it offers.
    property var offering: ({})

    // Puts away Tuple's notices that offer what can't be done any more,
    // leaving a call that's over or showing a ring that's stopped. Each is
    // closed as its program would close it, by its id, which counts it
    // read.
    function withdraw(key) {
        for (const [id, keys] of Object.entries(offering))
            if (keys.includes(key))
                closing.createObject(root, {
                    command: ["busctl", "--user", "call", "org.freedesktop.Notifications",
                        "/org/freedesktop/Notifications", "org.freedesktop.Notifications",
                        "CloseNotification", "u", id]
                });
    }

    function notify(summary, body, actions, kept) {
        notice.createObject(root, {
            offers: actions.map(([key]) => key),
            command: ["notify-send", "--app-name=Tuple", "--print-id"]
                .concat(kept ? [] : ["--transient"])
                .concat(actions.map(([key, label]) => `--action=${key}=${label}`))
                .concat([summary], body === "" ? [] : [body])
        });
    }

    // A notice of Tuple's, which notify-send waits on while it has actions,
    // printing its id as it's shown and then the action Joel picks.
    Component {
        id: notice

        Process {
            id: asking

            property var offers: []
            property string shown: ""

            running: true
            stdout: SplitParser {
                onRead: line => {
                    const said = line.trim();
                    if (asking.shown === "" && /^\d+$/.test(said)) {
                        asking.shown = said;
                        if (asking.offers.length > 0)
                            root.offering = Object.assign({}, root.offering,
                                { [said]: asking.offers });
                    } else if (said === "leave")
                        root.leave();
                    else if (said === "show")
                        root.show();
                }
            }
            onExited: {
                const still = Object.assign({}, root.offering);
                delete still[asking.shown];
                root.offering = still;
                asking.destroy();
            }
        }
    }

    Component {
        id: closing

        Process {
            id: close
            running: true
            onExited: close.destroy()
        }
    }
}

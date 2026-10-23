pragma Singleton
pragma ComponentBehavior: Bound
// Tuple's calls, as the shell sees them, for the margin's piece (Call.qml)
// and the launcher's rows (Calls.qml). Tuple's command starts, joins and
// ends a call and says nothing after, so how a call stands comes from the
// daemon's log, which Tuple writes in its data, where the XDG spec puts it:
// when a call connects and ends, who comes and goes, a call ringing, and
// whether Tuple sends Joel's voice. The log is followed as it's written,
// from its last lines when the shell starts, so a call already going
// counts. What a line means is Tuple.js's to say.
//
// Tuple on Linux doesn't yet tell Joel when someone joins or leaves, or
// that everyone else has left, so the shell does, as notices of Tuple's,
// for lines written since it started. Who came and went matters only as
// it happens, since the margin says who's there, so those ask not to be
// kept; one that offers something is kept, so the keys reach it too, and
// withdrawn once it can't be done; and a call that rang and wasn't
// answered is kept, as a missed call. Each person is named
// from Tuple's contacts, read once whenever someone new comes, since the
// list holds every one.
//
// Joel's room keeps one link, which tuple new prints as it opens the room.
// It's kept in the shell's state, to copy again without opening the room.
// A call's link on the clipboard, as one pasted in a chat arrives, is kept
// too, so the launcher can offer to join it; nothing else copied is kept.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Tuple.js" as Log

Singleton {
    id: root

    readonly property string log: `${Quickshell.env("XDG_DATA_HOME")
        || Quickshell.env("HOME") + "/.local/share"}/tuple/0/log.txt`

    // How the call stands, as Tuple.js tells it from the log.
    property var call: Log.none
    readonly property bool calling: call.calling
    // When the call connected, in milliseconds since 1970.
    readonly property real since: call.since
    // Who's in it besides Joel, each by whatever the log knows them by.
    readonly property var others: Log.others(call)
    // The names Tuple's contacts give them, by the number Tuple knows them by.
    property var names: ({})
    // Who's calling Joel, while a ring lasts, or "".
    property string ringing: ""
    // Whether Tuple keeps Joel's voice from the call, once it's said: true,
    // false, or null, unknown.
    property var muted: null
    // Joel's room's link, once tuple new has printed it.
    property string room: ""
    // The call link last copied, until something else is.
    property string copied: ""

    // When the shell started, before which nothing in the log is news.
    readonly property real started: Date.now()
    // A ring Tuple stops writing about is over after a minute, as a phone's
    // would be, whatever the log leaves unsaid.
    readonly property int ringsFor: 60 * 1000

    // Someone's name, or "" when Tuple's contacts don't list them.
    function name(who) {
        return names[Log.number(who)] ?? "";
    }

    // The people in the call, as a sentence says them.
    readonly property string company: Log.company(others.map(who => name(who)))

    // Opens Joel's room and copies its link, to hand to whoever's joining.
    function open() {
        opening.running = true;
    }

    function join(url) {
        Niri.spawn(["tuple", "call", "join", url]);
    }

    function leave() {
        Niri.spawn(["tuple", "call", "leave"]);
    }

    // Tuple asks the portal, which asks Joel what to share.
    function share() {
        Niri.spawn(["tuple", "share"]);
    }

    // Once Tuple says it's done, which it does as well when he already
    // was, that's how he stands, even when the shell had it wrong.
    property bool quieting: false

    function mute(quiet) {
        quieting = quiet;
        muting.command = ["tuple", "call", quiet ? "mute" : "unmute"];
        muting.running = true;
    }

    function copyRoom() {
        if (room !== "")
            Niri.spawn(["wl-copy", room]);
    }

    // Brings forward the window Tuple opened last, where a ringing call is
    // answered, since Tuple's command can't answer one.
    function show() {
        const tuples = Object.values(Niri.windows).filter(window => window.app_id === "Tuple");
        if (tuples.length > 0)
            Niri.act({ FocusWindow: { id: Math.max(...tuples.map(window => window.id)) } });
    }

    // What's to be said, kept until Tuple's contacts have named whoever it's
    // about.
    property var telling: []

    function tell(what, who) {
        telling = telling.concat([{ what: what, who: who }]);
        if (!learn())
            Qt.callLater(root.say);
    }

    // Everyone the shell might name, by the numbers the contacts haven't
    // been looked through for: those in the call, sharing, calling, or to
    // be told of.
    function unnamed() {
        return others.concat(call.showing, [ringing], telling.map(told => told.who))
            .map(who => Log.number(who))
            .filter(id => id !== "" && !(id in names));
    }

    // Looks through the contacts for anyone not yet named, saying whether
    // it had to.
    function learn() {
        if (unnamed().length === 0)
            return false;
        listing.running = true;
        return true;
    }

    function say() {
        if (listing.running)
            return;
        for (const { what, who } of telling) {
            const someone = name(who) || "Someone";
            if (what === "came")
                notify(`${someone} joined the call`, "", [], false);
            else if (what === "went")
                notify(`${someone} left the call`, "", [], false);
            else if (what === "alone")
                notify(`${someone} left the call`, "You're the only one still in it", [
                    ["leave", "Leave the call"]
                ], true);
            else if (what === "rings")
                notify(`${someone} is calling`, "Answer in Tuple's window", [
                    ["show", "Show the call"]
                ], true);
            else if (what === "missed")
                notify(`Missed a call from ${name(who) || "someone"}`, "", [], true);
        }
        telling = [];
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

    function heard(line) {
        const before = call;
        const now = Date.now();
        call = Log.after(call, line, now);
        if (call === before)
            return;
        if (call.muted !== before.muted && call.muted !== null)
            muted = call.muted;
        if (!call.calling && before.calling) {
            muted = null;
            withdraw("leave");
        }
        const rang = call.ringing !== "" && now - call.rang < ringsFor;
        const rung = ringing;
        ringing = rang ? call.ringing : "";
        if (rung !== "" && !rang)
            withdraw("show");
        if (rang)
            ringEnds.restart();
        learn();
        // Only what's happened since the shell started is news.
        const at = Log.written(line, now);
        if (at < started)
            return;
        if (rung !== "" && !rang && !Log.answering(line))
            tell("missed", rung);
        if (call.ringing !== "" && call.ringing !== before.ringing)
            tell("rings", call.ringing);
        // Whoever's there as Joel joins was there already, and everyone
        // goes as he leaves, so neither is news.
        if (!call.calling || !before.calling)
            return;
        const was = Log.others(before);
        const is = Log.others(call);
        if (at - call.since > settling)
            for (const who of is.filter(who => !was.includes(who)))
                tell("came", who);
        const went = was.filter(who => !is.includes(who));
        for (const who of went)
            tell(is.length === 0 && who === went[went.length - 1] ? "alone" : "went", who);
    }

    // How long after a call connects its people are still arriving, as
    // Tuple adds those already in it one by one.
    readonly property int settling: 5 * 1000

    Timer {
        id: ringEnds
        interval: Math.max(0, root.call.rang + root.ringsFor - Date.now())
        onTriggered: {
            const rung = root.ringing;
            root.ringing = "";
            root.withdraw("show");
            if (rung !== "")
                root.tell("missed", rung);
        }
    }

    Process {
        running: true
        command: ["tail", "-F", "-n", "400", root.log]
        stdout: SplitParser {
            onRead: line => root.heard(line)
        }
    }

    // Every contact, to find names in, read only when someone new comes.
    Process {
        id: listing
        command: ["tuple", "--format", "json", "contacts", "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                // One it doesn't list is kept as "", so it isn't asked again.
                const found = Object.assign({}, root.names);
                for (const id of root.unnamed())
                    found[id] = Log.named(this.text, id);
                root.names = found;
            }
        }
        onExited: Qt.callLater(root.say)
    }

    Process {
        id: muting
        onExited: exitCode => {
            if (exitCode === 0)
                root.muted = root.quieting;
        }
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

    // Each copy as one line, of which only a call's link is kept. A copy
    // with no text, such as a screenshot, or one marked as a secret, is
    // read as an empty line, so a link copied before it isn't offered
    // after it's gone. wl-paste skips a copy that has no type asked for,
    // so it asks for none and gives the command whatever there is, and
    // the command looks at the copy's types to know whether that's text.
    Process {
        running: true
        command: ["wl-paste", "--no-newline", "--watch", "sh", "-c",
            "if [ \"$CLIPBOARD_STATE\" = data ] && wl-paste --list-types"
            + " | grep -q -x -E 'text/.*|TEXT|STRING|UTF8_STRING'"
            + "; then tr '\\n' ' '; else cat > /dev/null; fi; echo"]
        stdout: SplitParser {
            onRead: line => root.copied = Log.link(line)
        }
    }

    Process {
        id: opening
        command: ["tuple", "new"]
        stdout: SplitParser {
            onRead: line => {
                const url = Log.link(line);
                if (url === "")
                    return;
                Niri.spawn(["wl-copy", url]);
                if (url !== root.room) {
                    root.room = url;
                    kept.setText(url);
                }
            }
        }
    }

    FileView {
        id: kept
        path: Quickshell.statePath("tuple-room")
        blockLoading: true
        printErrors: false
    }

    Component.onCompleted: room = Log.link(kept.text())
}

// Drives the launcher's Sound (modules/quickshell/launcher/Sound.qml) over
// the rows the sound daemon serves ready, through Quickshell itself, with a
// stand-in serving them on the daemon's socket as it does (modules/sound's
// interface.rs), and stand-in devices as the sound server would give them
// for the parts still made here. Wanted, it lists the rows served, in their
// order, each stepping into its device's parts, and home's count says them;
// shut, a row served anew makes none of its rows, and Now's Volume borrows
// a device's row as it's served; wanted again, they're made once, and a
// device's row served anew remakes it alone; and what its spots are made
// of moves only once a device comes or goes. (tests/sound.qml holds the
// parts and drawers.) It exits 0 once every case holds, and otherwise 1,
// saying which didn't.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")

    // Each case, as the test library says it.
    readonly property var expect: Expect.expect

    // A device as the sound server holds one, an object as its nodes are.
    component Node: QtObject {
        property string name
        property string nickname: ""
        property string description: name
        property bool isSink: true
        property bool isStream: false
        property int type: isSink ? PwNodeType.AudioSink : PwNodeType.AudioSource
        property var properties: ({ "device.bus": "usb" })
        property var audio: ({ volume: 0.5, muted: false })
    }

    Node {
        id: deskSpeakers
        name: "usb-desk-speakers"
    }

    Node {
        id: deskMicrophone
        name: "usb-desk-microphone"
        isSink: false
    }

    // The launcher as Sound asks it, and whether Sound's rows are wanted
    // (Provider.qml's wanted).
    QtObject {
        id: asking

        property bool open: false
        property bool wanting: true
        readonly property var held: []

        function wants(source) {
            return wanting;
        }
    }

    Launcher.Sound {
        id: sound
        launcher: asking
        serverNodes: [deskSpeakers, deskMicrophone]
    }

    // A device's row as the sound daemon's rows.rs lays one: its name, its
    // order among the rest, which way it goes and its volume in its note.
    function served(name, called, order, sink, level) {
        const key = `row:sound:${name}`;
        return {
            key: key, kind: "row",
            row: {
                key: key, thing: `sound:${name}`, place: "sound", kind: "device",
                order: order, name: called, glyph: sink ? "speaker" : "microphone",
                hue: "machine", depth: 0, side: [],
                graph: { shape: "bar", values: [level], warn: false },
                note: `volume ${Math.round(level * 100)}%`,
                acts: [{ key: "verb:more", step: "rows" }],
                held: { node: name, sink: sink }
            }
        };
    }

    // The sound daemon's socket as it serves it: each watch told its rows
    // whole, then only what changed, as `tell` and `forget` say it.
    property var records: ({})
    property var watchers: []

    function tell(record) {
        records[record.key] = record;
        said({ id: "all", changed: [record], gone: [] });
    }

    function forget(key) {
        delete records[key];
        said({ id: "all", changed: [], gone: [key] });
    }

    function said(line) {
        for (const watcher of watchers) {
            watcher.write(`${JSON.stringify(line)}\n`);
            watcher.flush();
        }
    }

    SocketServer {
        active: true
        path: `${root.runtime}/daemons/sound.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer = { id: ask.id, unwatched: true };
                    if (ask.ask === "hello")
                        answer = { id: ask.id, version: 1, domain: "sound", state: "answering" };
                    else if (ask.ask === "watch") {
                        root.watchers.push(asked);
                        answer = { id: ask.id, records: Object.values(root.records) };
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }

    // The case waiting on what Sound hears next: once `holds`, on to
    // `step`; five seconds without, it fails saying `what`.
    property var pending: null

    function when(what, holds, step) {
        pending = { what: what, holds: holds, step: step };
        gaveUp.restart();
        heardNow();
    }

    // Each change Sound hears from the socket, it's asked again.
    function heardNow() {
        const waiting = pending;
        if (!waiting)
            return;
        try {
            if (waiting.holds()) {
                pending = null;
                gaveUp.stop();
                waiting.step();
            }
        } catch (error) {
            fail(error.message ?? String(error));
        }
    }

    Connections {
        target: sound.served

        function onHeard() {
            root.heardNow();
        }
    }

    // A case that never holds fails rather than leaving the check to wait
    // out its timeout; five seconds is far past a socket's line.
    Timer {
        id: gaveUp

        interval: 5000
        onTriggered: root.fail(`${root.pending?.what}: never held`)
    }

    function fail(why) {
        Expect.fail(why);
    }

    function names(rows) {
        return rows.map(row => row.key).join(", ");
    }

    // The daemon's two rows, served before Sound watches, as a daemon
    // started first holds them.
    Component.onCompleted: {
        records[`row:sound:${deskSpeakers.name}`] = served(deskSpeakers.name, "Desk Speakers",
            1024, true, 0.5);
        records[`row:sound:${deskMicrophone.name}`] = served(deskMicrophone.name,
            "Desk Microphone", 2048, false, 0.5);
        when("the served rows listed", () => sound.everything().length === 2, listed);
    }

    function listed() {
        const rows = sound.everything();
        expect("wanted, it lists the rows served, in their order",
            names(rows) === "sound:usb-desk-speakers, sound:usb-desk-microphone"
            && rows[0].name === "Desk Speakers" && rows[0].note === "volume 50%", names(rows));
        const parts = rows[0].targets().map(part => part.key);
        expect("a served row steps into its device's parts",
            parts.includes("sound:usb-desk-speakers:volume"), parts.join(", "));
        expect("and home's count says them", sound.counted() === "1 out \u00b7 1 in",
            sound.counted());
        root.stamp = sound.spotsStamp;
        asking.wanting = false;
        root.before = sound.tally.rows;
        tell(served(deskSpeakers.name, "Desk Speakers", 1024, true, 0.3));
        when("the shut row heard", () => sound.served.served(
            `row:sound:${deskSpeakers.name}`)?.note === "volume 30%", shut);
    }

    property string stamp: ""
    property int before: 0

    function shut() {
        expect("shut, a row served anew makes none of its rows",
            sound.tally.rows === root.before, `${sound.tally.rows - root.before} made`);
        const now = sound.rowNow("sound:usb-desk-speakers");
        expect("a device's row borrowed elsewhere is made as it's served",
            now?.note === "volume 30%" && sound.rowNow("sound:gone") === undefined, now?.note);
        const made = sound.tally.rows;
        asking.wanting = true;
        const wanted = sound.everything();
        expect("wanted again, they're made once, as they stand",
            names(wanted) === "sound:usb-desk-speakers, sound:usb-desk-microphone"
            && wanted[0].note === "volume 30%" && sound.tally.rows - made <= 2,
            `${sound.tally.rows - made} made, ${names(wanted)}`);
        root.wanted = wanted;
        root.before = sound.tally.rows;
        tell(served(deskMicrophone.name, "Desk Microphone", 2048, false, 0.7));
        when("the wanted row heard", () => sound.everything()[1]?.note === "volume 70%", moved);
    }

    property var wanted: []

    function moved() {
        const heard = sound.everything();
        expect("wanted, a device's row served anew remakes it alone",
            heard[0] === root.wanted[0] && sound.tally.rows - root.before === 1,
            `${sound.tally.rows - root.before} made`);
        expect("a volume served anew leaves what its spots are made of",
            sound.spotsStamp === root.stamp, sound.spotsStamp);
        forget(`row:sound:${deskMicrophone.name}`);
        when("the gone row heard", () => sound.everything().length === 1, gone);
    }

    function gone() {
        expect("a device going changes what its spots are made of",
            sound.spotsStamp !== root.stamp && sound.counted() === "1 out \u00b7 0 in",
            `${sound.spotsStamp}, ${sound.counted()}`);
        // The rows whose texts are made ready while shut (Launcher.qml's
        // textsMadeClosed) are those it lists once it opens, so the first
        // search makes none (vm-desktop-launcher-goes-places, 2026-11-11).
        asking.wanting = false;
        const seen = sound.looksThrough();
        asking.wanting = true;
        const shown = sound.everything();
        expect("the rows looked through shut are the ones listed once wanted",
            seen.length === 1 && shown.length === 1 && shown[0] === seen[0],
            `${seen.length} looked through, ${shown.length} listed`);
        Qt.exit(0);
    }
}

// Drives the launcher's Sound (modules/quickshell/launcher/Sound.qml) over
// the rows the sound daemon serves ready, through Quickshell itself, with a
// stand-in serving them on the daemon's socket as it does (modules/sound's
// interface.rs), and stand-in devices as the sound server would give them
// for the parts still made here. Wanted, it lists the rows served, in their
// order, each stepping into its device's parts, and home's count says them;
// shut, a row served anew makes none of its rows, and Now's Volume borrows
// a device's row as it's served; wanted again, they're made once, and a
// device's row served anew remakes it alone; and what its spots are made
// of moves only once a device comes or goes; and a device's Port, served
// inside its row, steps into its choices, a choice's Return asking the
// daemon to move it, as its option does; and its Use, Muted and Volume,
// served, lead its parts in Sound's order, Muted saying unmute once it's
// muted, the Volume its drawer's, and each Return asking the daemon.
// (tests/sound.qml holds the parts and drawers made here.) It exits 0 once every case holds, and otherwise 1,
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

        // An act asked draws its row as it becomes, here nowhere.
        function becomesAll() {
        }

        function becameNot() {
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
    // Each act asked on the socket, as the daemon would take it.
    property var acted: []
    onActedChanged: heardNow()

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
                    } else if (ask.ask === "act") {
                        root.acted = root.acted.concat([ask]);
                        answer = { id: ask.id, done: true };
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
            parts.includes("sound:usb-desk-speakers:level"), parts.join(", "));
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
        tell(portPart());
        tell(portChoice(1, "Speakers", false));
        tell(portChoice(2, "Headphones", true));
        // Each record comes on a line of its own, so the Port can be heard
        // before its choices are.
        when("the Port served with its choices", () => sound.everything()[0]?.targets()
            .find(part => part.key === `sound:${deskSpeakers.name}:port`)?.targets()
            .length === 2, ported);
    }

    // The speakers' Port, as rows.rs's port_rows serves it inside their
    // row, and each port they can be moved to inside it.
    function portPart() {
        const parent = `row:sound:${deskSpeakers.name}`;
        return {
            key: `${parent}:port`, kind: "row",
            row: {
                key: `${parent}:port`, thing: `sound:${deskSpeakers.name}:port`,
                place: "sound", kind: "setting", order: 1536, parent: parent, depth: 1,
                name: "Port", glyph: "speaker", hue: "machine", side: [{ said: "Headphones" }],
                acts: [{ key: "verb:more", step: "rows" }], held: { node: deskSpeakers.name }
            }
        };
    }

    function portChoice(index, name, chosen) {
        const parent = `row:sound:${deskSpeakers.name}:port`;
        return {
            key: `${parent}:${index}`, kind: "row",
            row: {
                key: `${parent}:${index}`, thing: `verb:sound-port-${index}`, place: "sound",
                kind: "port", order: 1536 + index, parent: parent, depth: 2,
                name: `Use ${name}`, glyph: "speaker", hue: "action",
                side: chosen ? [{ said: "chosen" }] : [], note: "",
                acts: [{
                    key: "verb:sound-port",
                    ask: { ask: "act", act: "port", key: `device:${deskSpeakers.name}`,
                        index: index }
                }],
                held: { node: deskSpeakers.name, port: name, index: index, chosen: chosen }
            }
        };
    }

    function ported() {
        const port = sound.everything()[0].targets()
            .find(part => part.key === `sound:${deskSpeakers.name}:port`);
        expect("a served Port is a setting saying the one in use",
            port.offers === true && port.side === "Headphones", `${port.offers}, ${port.side}`);
        const choices = port.targets();
        expect("it steps into each port to move to, as they were keyed",
            names(choices) === "verb:sound-port-1, verb:sound-port-2", names(choices));
        expect("only Sound's top rows are listed", sound.everything().length === 1,
            names(sound.everything()));
        choices[0].run();
        when("the port asked", () => root.acted.length === 1, () => {
            const ask = root.acted[0];
            expect("a choice's Return asks the daemon to move it", ask.act === "port"
                && ask.key === `device:${deskSpeakers.name}` && ask.index === 1,
                JSON.stringify(ask));
            tell(usePart());
            tell(mutePart(false));
            tell(volumePart());
            tell(louderChoice());
            when("the parts served", () => partOf("volume")?.targets().length === 1,
                levelled);
        });
    }

    // One of the speakers' parts, as Sound lists it now.
    function partOf(last) {
        return sound.everything()[0].targets()
            .find(part => part.key === `sound:${deskSpeakers.name}:${last}`);
    }

    // A part as parts.rs serves it inside the speakers' row.
    function part(last, fields) {
        const parent = `row:sound:${deskSpeakers.name}`;
        const key = `${parent}:${last}`;
        return {
            key: key, kind: "row",
            row: Object.assign({
                key: key, place: "sound", parent: parent, depth: 1, glyph: "speaker",
                hue: "action", side: [], note: "", held: { node: deskSpeakers.name }
            }, fields)
        };
    }

    function asks(act, more) {
        return Object.assign({ ask: "act", act: act, key: `device:${deskSpeakers.name}` },
            more);
    }

    function usePart() {
        return part("use", {
            thing: "verb:sound-use", kind: "act", order: 1100,
            name: "Play Sound Through It",
            acts: [{ key: "verb:sound-use", ask: asks("use", {}) }],
            held: { node: deskSpeakers.name, fill: { name: "Desk Speakers" }, state: "" }
        });
    }

    function mutePart(muted) {
        return part("mute", {
            thing: `sound:${deskSpeakers.name}:mute`, kind: "switch", order: 1200,
            name: "Muted", glyph: muted ? "silent" : "speaker",
            side: [{ said: muted ? "yes" : "no" }],
            acts: [{
                key: "sound:mute", state: muted ? "muted" : "",
                ask: asks("mute", { muted: !muted })
            }]
        });
    }

    function volumePart() {
        return part("volume", {
            thing: `sound:${deskSpeakers.name}:volume`, kind: "setting", order: 1300,
            name: "Volume", hue: "machine", side: [{ said: "50%" }],
            graph: { shape: "bar", values: [0.5], warn: false }, note: "-18.1 dB",
            acts: [{ key: "verb:more", step: "rows" }]
        });
    }

    function louderChoice() {
        return part("volume:louder", {
            parent: `row:sound:${deskSpeakers.name}:volume`, depth: 2,
            thing: "verb:sound-louder", kind: "act", order: 1301, name: "Louder",
            glyph: "volume",
            acts: [{ key: "verb:sound-louder", ask: asks("volume", { volume: 0.55,
                muted: false }) }]
        });
    }

    function levelled() {
        const speakers = deskSpeakers.name;
        const parts = sound.everything()[0].targets();
        expect("its Use, Muted and Volume lead its parts, in Sound's order",
            names(parts).startsWith(`verb:sound-use, sound:${speakers}:mute, `
                + `sound:${speakers}:level, sound:${speakers}:volume`), names(parts));
        const using = parts[0];
        expect("using it names it on its line", using.fill?.name === "Desk Speakers"
            && using.wordState === "", JSON.stringify(using.fill));
        const muting = partOf("mute");
        expect("unmuted, its switch says no and mutes",
            muting.side === "no" && muting.verb === "mute", `${muting.side}, ${muting.verb}`);
        const volume = partOf("volume");
        expect("its Volume is a setting saying how it's set", volume.offers === true
            && volume.side === "50%" && names(volume.targets()) === "verb:sound-louder",
            `${volume.offers}, ${volume.side}, ${names(volume.targets())}`);
        const drawn = sound.drawerOf(deskSpeakers).facts
            .find(fact => fact.label === "Volume");
        expect("its drawer says its decibels, drawing its share as a stroke",
            drawn?.value === "-18.1 dB" && drawn?.figure?.shape === "stroke"
            && drawn.figure.values[0] === 0.5, JSON.stringify(drawn));
        volume.targets()[0].run();
        when("the volume asked", () => root.acted.length === 2, louder);
    }

    function louder() {
        const ask = root.acted[1];
        expect("Louder asks the daemon, unmuting it", ask.act === "volume"
            && ask.volume === 0.55 && ask.muted === false, JSON.stringify(ask));
        tell(mutePart(true));
        when("the mute heard", () => partOf("mute")?.side === "yes", muted);
    }

    function muted() {
        const muting = partOf("mute");
        expect("muted, it says unmute", muting.verb === "unmute", muting.verb);
        muting.run();
        when("the mute asked", () => root.acted.length === 3, () => {
            const ask = root.acted[2];
            expect("its Return asks the daemon to unmute it", ask.act === "mute"
                && ask.muted === false, JSON.stringify(ask));
            Qt.exit(0);
        });
    }
}

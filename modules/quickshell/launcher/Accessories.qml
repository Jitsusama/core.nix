// Bluetooth, a collection: the devices around, those connected first, then
// those paired, then the rest by name, each glyph saying what kind of device
// it is and the footer its battery where it says. Return connects one,
// pairing it first if it's new, or disconnects the one connected, and the
// launcher stays, saying it's connecting, until it's done or says why not.
// A device that wants a passkey typed is paired in bluetoothctl instead,
// since this pairs only as headphones and mice do, with nothing to type.
// Turning Bluetooth off and on is an action in the main search. Everything
// comes from BlueZ over D-Bus through Quickshell, so no process is asked,
// and the radio looks for devices only while the launcher is open. What a
// connected input device's kernel side says of it (how it's carried, its
// buttons, wheel or keys, its ids) comes from desk-facts, after its verbs.
//
// How a connected device's link is doing comes between its verbs and those
// facts: its Signal, how well the Link carries, the Codec its sound goes
// in, as PipeWire names it, and how many Channels are clear, the rest
// skipped as noisy, which is how crowded the air is. BlueZ says none of
// the first and last, so they're from the file the machine's link reader
// writes (modules/bluetooth-links), read as the device's list is made and
// again each second while it's shown, and never for a key; a machine
// without the reader shows none of them. "bluetooth performance",
// "bluetooth signal" or "interference" answers with each connected
// device's Signal and Channels.
import qs
import Quickshell
import Quickshell.Bluetooth
import Quickshell.Io
import Quickshell.Services.Pipewire
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Links.js" as Links

Provider {
    id: provider

    what: "Bluetooth"

    readonly property var adapter: Bluetooth.defaultAdapter
    readonly property bool on: adapter !== null && adapter.enabled

    collection: adapter === null ? null : Words.collection("bluetooth", {
        empty: on ? "Looking for devices\u2026" : "Bluetooth is off"
    })

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    // Looking for devices costs the radio's power, so it looks only while
    // the launcher is there to show them.
    Binding {
        target: provider.adapter
        property: "discovering"
        value: provider.on && provider.launcher !== null && provider.launcher.open
        when: provider.adapter !== null
    }

    readonly property var radio: thing({
        key: "bluetooth:radio",
        // An act, not a device, so it says no kind (docs/design.md).
        what: "",
        verb: Words.verb("bluetooth:radio", on ? "" : "off"),
        name: Words.name("bluetooth:radio", on ? "" : "off"),
        keywords: `bluetooth bt headphones ${Words.keywords("bluetooth:radio", on ? "" : "off")}`,
        glyph: Words.glyph("bluetooth:radio", on ? "" : "off"),
        hue: "machine",
        run: () => provider.adapter.enabled = !provider.on
    })

    things: adapter === null ? [] : [radio]

    // What it is, from the icon BlueZ names for its class.
    function kind(device) {
        const icon = device.icon;
        const kinds = {
            "audio-headphones": 0xf0970,
            "audio-headset": 0xf02ce,
            "audio-card": 0xf09a2,
            "input-mouse": 0xf037d,
            "input-tablet": 0xf037d,
            "input-keyboard": 0xf030c,
            "input-gaming": 0xf0297,
            "phone": 0xf011c,
            "computer": 0xf0322,
            "video-display": 0xf0502
        };
        return glyph(kinds[icon] ?? 0xf00af);
    }

    // Connected, then paired, then the rest, then by name; a device that
    // never said its name is left out, as nothing could find it.
    readonly property var devices: {
        if (!on)
            return [];
        const rank = device => device.connected ? 0 : device.paired ? 1 : 2;
        return adapter.devices.values.filter(device => device.deviceName !== "")
            .sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
    }

    // The device being connected, and what it said if it wouldn't be.
    property var joining: null
    property string trouble: ""

    // A new device is trusted before it pairs. Headphones start their audio
    // the moment they're paired, and with no agent to ask, BlueZ refuses an
    // untrusted device's audio, so it hangs up; trust given after pairing
    // comes too late. A pairing that fails takes the trust back.
    function join(device) {
        trouble = "";
        joining = device;
        if (device.paired) {
            device.connect();
            return;
        }
        device.trusted = true;
        device.pair();
    }

    function said(device, otherwise) {
        if (joining !== device)
            return otherwise;
        if (trouble !== "")
            return trouble;
        const doing = device.pairing ? "pairing with" : "connecting";
        return `${doing} ${device.name}\u2026`;
    }

    function made(device) {
        const joined = device.connected;
        const told = joined ? "connected" : device.paired ? "paired" : "new";
        const charge = Math.round(device.battery * 100);
        const note = device.batteryAvailable ? `${charge}% battery` : told;
        return member({
            key: `bluetooth:${device.address}`,
            verb: joined ? "disconnect" : device.paired ? "connect" : "pair",
            name: device.name,
            glyph: kind(device),
            hue: "machine",
            side: joined ? "connected" : device.paired ? "paired" : "",
            note: note,
            device: device,
            state: () => provider.said(device, note),
            stays: !joined,
            run: joined ? () => device.disconnect() : () => provider.join(device),
            verbs: () => provider.verbs(device)
        });
    }

    readonly property var all: devices.map(device => made(device))
    onAllChanged: changed()

    // What desk-facts knows of each connected input device, by its address;
    // a device it says nothing of, or no file at all, has no facts.
    property var facts: ({})

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/desk/bluetooth.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.facts = provider.parsed(text())
        onLoadFailed: provider.facts = ({})
    }

    function factRows(device) {
        const said = facts[device.address] ?? [];
        return said.map(fact => copyRow({
            key: `fact:bluetooth:${device.address}:${fact.name}`,
            name: fact.value,
            side: fact.name,
            glyph: kind(device)
        }));
    }

    // How each link is doing, as the reader last wrote it: its text and
    // when it was read, so an answer asked again by each letter of its
    // question reads it once a second at most. A check points it at a
    // stand-in.
    readonly property var memo: ({ text: "", at: 0 })
    readonly property int second: 1000

    FileView {
        id: linksFile
        path: provider.settings.links ?? "/run/bluetooth-links.json"
        blockLoading: true
        printErrors: false
    }

    // Every link the reader says, or null with no reader.
    function links(fresh) {
        const now = Date.now();
        if (fresh || now - memo.at >= second) {
            memo.text = read(linksFile);
            memo.at = now;
        }
        return Links.links(memo.text);
    }

    // PipeWire's own Bluetooth nodes, whose properties name the codec each
    // sends in, heard once something binds them.
    readonly property var bluez: Pipewire.nodes.values
        .filter(node => (node.name ?? "").startsWith("bluez_"))

    PwObjectTracker {
        objects: provider.bluez
    }

    // The codec a device's sound goes in, its output's before its input's,
    // or null for a device with no sound.
    function codec(device) {
        const nodes = bluez.filter(node => node.properties["api.bluez5.address"] === device.address
            && !!node.properties["api.bluez5.codec"]);
        const node = nodes.find(one => one.isSink) ?? nodes[0];
        return node ? Links.codec(node.properties["api.bluez5.codec"]) : null;
    }

    // A figure says something and copies nothing, wearing no glyph.
    function figure(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    function bar(value) {
        return { shape: "bar", values: [value], warn: false };
    }

    function signalRow(device, link) {
        const signal = Links.signal(link);
        return signal === null ? null : figure({
            key: `link:bluetooth:${device.address}:signal`,
            name: Words.name("bluetooth:signal"),
            side: signal.side,
            graph: bar(signal.value),
            note: signal.note
        });
    }

    function channelsRow(device, link) {
        const channels = Links.channels(link);
        return channels === null ? null : figure({
            key: `link:bluetooth:${device.address}:channels`,
            name: Words.name("bluetooth:channels"),
            side: channels.side,
            graph: { shape: "strip", values: channels.values, warn: false },
            note: channels.note
        });
    }

    // How a connected device's link is doing, as figures.
    function linkRows(device) {
        if (!device.connected)
            return [];
        const link = Links.of(links(false), device.address);
        const quality = Links.quality(link);
        const sent = codec(device);
        return [signalRow(device, link), quality === null ? null : figure({
            key: `link:bluetooth:${device.address}:quality`,
            name: Words.name("bluetooth:quality"),
            side: quality.side,
            graph: bar(quality.value),
            note: quality.note
        }), sent === null ? null : figure({
            key: `link:bluetooth:${device.address}:codec`,
            name: Words.name("bluetooth:codec"),
            side: sent.name,
            note: sent.note
        }), channelsRow(device, link)].filter(row => row !== null);
    }

    // How the links are doing, as an answer: one device's Signal and
    // Channels, or a row each of several, with theirs a step in.
    questions: Words.started("bluetooth:links")

    function answer(typed) {
        if (asks(typed) === "" || !on)
            return null;
        return linksAnswer(devices.filter(device => device.connected));
    }

    function linksAnswer(connected) {
        const all = links(false);
        if (all === null)
            return null;
        const told = connected.map(device => {
            const link = Links.of(all, device.address);
            return {
                device: device,
                signal: Links.signal(link),
                channels: Links.channels(link),
                rows: [signalRow(device, link), channelsRow(device, link)]
                    .filter(row => row !== null)
            };
        }).filter(each => each.rows.length > 0);
        if (told.length === 0)
            return null;
        const said = each => [each.signal ? each.signal.side : "",
            each.channels ? each.channels.side : ""].filter(part => part !== "").join(", ");
        const graph = each => each.signal ? bar(each.signal.value) : undefined;
        const glyph = Words.glyph("bluetooth");
        if (told.length === 1)
            return {
                headline: said(told[0]),
                copy: "",
                graph: graph(told[0]),
                glyph: glyph,
                note: `the link to ${told[0].device.name}`,
                rows: told[0].rows,
                live: provider
            };
        return {
            headline: `${told.length} connected`,
            copy: "",
            glyph: glyph,
            note: "",
            rows: told.map(each => {
                const row = copyRow({
                    key: `link:bluetooth:${each.device.address}`,
                    name: each.device.name,
                    side: said(each),
                    graph: graph(each),
                    glyph: kind(each.device),
                    hue: "machine",
                    copy: ""
                });
                return Object.assign(row, {
                    verb: "more",
                    stays: true,
                    now: false,
                    verbs: () => each.rows,
                    run: () => provider.launcher?.enter(row)
                });
            }),
            live: provider
        };
    }

    // The connected device whose list is shown, and whether one of its
    // answers is: each is read again once a second while it's there, and
    // never otherwise. A list says it changed only when the file did.
    readonly property var looking: {
        const launcher = provider.launcher;
        if (!launcher || !launcher.open || launcher.held.length === 0)
            return null;
        const shown = launcher.held[launcher.held.length - 1].row.device ?? null;
        return shown && shown.connected ? shown : null;
    }
    readonly property bool answering: launcher !== null && launcher.open
        && launcher.held.length === 0 && launcher.answer?.live === provider

    Timer {
        interval: provider.second
        repeat: true
        running: provider.looking !== null || provider.answering
        onTriggered: {
            const before = provider.memo.text;
            provider.links(true);
            if (provider.answering)
                provider.answered();
            else if (provider.memo.text !== before)
                provider.changed();
        }
    }

    function verbs(device) {
        const rows = [device.connected ? thing({
            key: "verb:bluetooth-disconnect",
            name: Words.name("verb:bluetooth-disconnect"),
            glyph: glyph(0xf00b2),
            hue: "action",
            run: () => device.disconnect()
        }) : thing({
            key: "verb:bluetooth-connect",
            verb: device.paired ? "connect" : "pair",
            name: Words.name("verb:bluetooth-connect", device.paired ? "" : "unpaired"),
            glyph: glyph(0xf00b1),
            hue: "action",
            device: device,
            stays: true,
            state: () => provider.said(device, ""),
            run: () => provider.join(device)
        })];
        if (!device.paired)
            rows.push(thing({
                key: "verb:bluetooth-passkey",
                name: Words.name("verb:bluetooth-passkey"),
                keywords: Words.keywords("verb:bluetooth-passkey"),
                glyph: glyph(0xf030c),
                hue: "action",
                later: true,
                run: () => Niri.spawn(["kitty", "bluetoothctl"])
            }));
        if (device.paired)
            rows.push(thing({
                key: "verb:bluetooth-forget",
                name: Words.name("verb:bluetooth-forget"),
                glyph: glyph(0xf16c5),
                hue: "action",
                run: () => device.forget()
            }));
        return rows.concat(linkRows(device), factRows(device));
    }

    // Done with the device being connected: the launcher closes, if it's
    // still on that device.
    function finish(device) {
        joining = null;
        const launcher = provider.launcher;
        if (!launcher)
            return;
        const at = launcher.selected ?? launcher.asking;
        if (launcher.open && at && at.device === device)
            launcher.open = false;
    }

    // How the device being connected went. BlueZ says a new device is
    // connected as soon as its link is up, before pairing is done, so only a
    // paired one is finished: paired, it's connected if it isn't yet, and
    // trusted, so it comes back by itself. A pairing that ends unpaired
    // says so and is no longer trusted.
    Connections {
        target: provider.joining

        function onPairedChanged() {
            const device = provider.joining;
            if (!device || !device.paired)
                return;
            device.trusted = true;
            if (device.connected)
                provider.finish(device);
            else
                device.connect();
        }

        function onPairingChanged() {
            const device = provider.joining;
            if (device && !device.pairing && !device.paired) {
                device.trusted = false;
                provider.trouble = "it didn't pair; it may want a passkey";
            }
        }

        function onConnectedChanged() {
            const device = provider.joining;
            if (device && device.connected && device.paired)
                provider.finish(device);
        }

        function onStateChanged() {
            const device = provider.joining;
            if (device && device.paired && device.state === BluetoothDeviceState.Disconnected
                && !device.connected)
                provider.trouble = "it didn't connect; is it on and near?";
        }
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    // With Bluetooth off, turning it on is all there is to do.
    function everything() {
        return on ? all.slice() : [radio];
    }

    function counted() {
        return !on ? "off" : `${all.filter(row => row.side !== "").length} paired`;
    }
}

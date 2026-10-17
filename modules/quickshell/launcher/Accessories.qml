// Bluetooth, a collection: the devices around, those connected first, then
// those paired, then the rest by name, each glyph saying what kind of device
// it is and the footer its battery where it says. Return connects one,
// pairing it first if it's new, or disconnects the one connected, and the
// launcher stays, saying it's connecting, until it's done or says why not.
// A device that wants a passkey typed is paired in bluetoothctl instead,
// since this pairs only as headphones and mice do, with nothing to type.
// Turning Bluetooth off and on is an action in the main search. Everything
// comes from BlueZ over D-Bus through Quickshell, so no process is asked,
// and the radio looks for devices only while the launcher is open.
import qs
import Quickshell.Bluetooth
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    readonly property var adapter: Bluetooth.defaultAdapter
    readonly property bool on: adapter !== null && adapter.enabled

    collection: adapter === null ? null : {
        noun: "devices",
        title: "Bluetooth",
        glyph: glyph(0xf00af),
        ask: "which device?",
        empty: on ? "Looking for devices\u2026" : "Bluetooth is off"
    }

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
        verb: on ? "turn off" : "turn on",
        name: on ? "Turn Bluetooth Off" : "Turn Bluetooth On",
        keywords: "bluetooth radio headphones airplane",
        glyph: glyph(on ? 0xf00b2 : 0xf00af),
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

    function verbs(device) {
        const rows = [device.connected ? thing({
            key: "verb:bluetooth-disconnect",
            name: "Disconnect it",
            glyph: glyph(0xf00b2),
            hue: "action",
            run: () => device.disconnect()
        }) : thing({
            key: "verb:bluetooth-connect",
            verb: device.paired ? "connect" : "pair",
            name: device.paired ? "Connect it" : "Pair and connect it",
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
                name: "Pair it in bluetoothctl",
                glyph: glyph(0xf030c),
                hue: "action",
                later: true,
                run: () => Niri.spawn(["kitty", "bluetoothctl"])
            }));
        if (device.paired)
            rows.push(thing({
                key: "verb:bluetooth-forget",
                name: "Forget it",
                glyph: glyph(0xf16c5),
                hue: "action",
                run: () => device.forget()
            }));
        return rows;
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

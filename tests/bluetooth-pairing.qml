// Drives the launcher's Bluetooth (modules/quickshell/launcher/Accessories.qml)
// through pairing and connecting, with a stand-in for each device: BlueZ
// says a new device is connected as soon as its link is up, before pairing
// is done, so connecting alone doesn't finish it; it's trusted before it
// pairs, since headphones start their audio the moment they're paired and
// BlueZ refuses an untrusted device's with no agent to ask; a pairing that
// fails says so and takes the trust back; and a device paired before is
// done once it connects. After a device's verbs come its parts, each named
// first: its Battery as a bar that warns low; how its link is doing, from
// the file the check puts where bluetooth-links would write, its Signal and
// Link quality as sparks of their last reads, its Channels as a strip and
// its Connection; a connected mouse's Input, from the bluetooth.json the
// check puts where desk-facts would, holding its other facts as parts in
// the file's order; whether it Reconnects by itself and Wakes the machine,
// each a setting whose choices set it; and its Address, which Return
// copies. The adapter holds its Radio, Visible to others and Pairable as
// settings, its Traffic as a spark, and its Name, Address, Version and Chip.
// How the links are doing is answered too; with no such file, neither the
// links nor the answer say a thing. It exits 0 once every case holds, and
// otherwise 1, saying which case didn't.
import qs.launcher
import Quickshell
import QtQuick

ShellRoot {
    id: root

    component Device: QtObject {
        property string name: "Buds"
        property string address: ""
        property bool paired: false
        property bool connected: false
        property bool pairing: false
        property bool trusted: false
        property bool wakeAllowed: false
        property bool batteryAvailable: false
        property real battery: 0
        property string icon: ""
        property int state: 0
        property int asked: 0

        function pair() {
            pairing = true;
        }

        function connect() {
            asked++;
        }
    }

    Device { id: linkedFirst }
    Device { id: refused }
    Device { id: known; paired: true; trusted: true }
    Device {
        id: mouse
        name: "Mouse"
        address: "C3:9C:E3:2A:F3:6E"
        icon: "input-mouse"
        paired: true
        connected: true
        trusted: true
    }
    Device {
        id: headphones
        name: "Headphones"
        address: "00:1B:66:AA:BB:CC"
        icon: "audio-headphones"
        paired: true
        connected: true
        trusted: true
        batteryAvailable: true
        battery: 0.15
    }

    // The adapter, as Quickshell's would say it; hci9, so no machine's own
    // chip is found for it.
    QtObject {
        id: radio
        property string adapterId: "hci9"
        property string name: "studio"
        property bool enabled: true
        property bool discoverable: false
        property bool pairable: true
        property int discoverableTimeout: 180
    }

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    Accessories {
        id: bluetooth
        settings: ({ links: `${Quickshell.env("XDG_RUNTIME_DIR")}/bluetooth-links.json` })
    }

    // A machine without the link reader.
    Accessories {
        id: unread
        settings: ({ links: `${Quickshell.env("XDG_RUNTIME_DIR")}/nothing.json` })
    }

    // What each row of a list says, by its name and what's at its right.
    function said(rows) {
        return rows.map(row => `${row.name}=${row.side}`).join(",");
    }

    // A device's parts after its verbs, how the links are doing among them,
    // and as an answer.
    function linksHold() {
        const heard = bluetooth.verbs(headphones);
        const at = headphones.address;
        expect("parts after verbs", heard.map(row => row.key).join(",")
            === ["verb:bluetooth-disconnect", "verb:bluetooth-forget", `bluetooth:${at}:battery`]
                .concat(["signal", "quality", "channels", "held"]
                    .map(part => `link:bluetooth:${at}:${part}`))
                .concat([`bluetooth:${at}:trusted`, `bluetooth:${at}:address`]).join(","),
            heard.map(row => row.key).join(","));
        const parts = heard.slice(2);
        expect("a classic link's parts", said(parts) === ["Battery=15%", "Signal=in range",
            "Link quality=90%", "Channels=61 of 79 clear",
            "Connection=Classic, central, encrypted", "Reconnects by itself=yes",
            `Address=${at}`].join(","), said(parts));
        const figures = parts.slice(0, 5);
        expect("figures wear no glyph, and copy what they show", figures.every(row =>
            row.glyph === "" && row.verb === "copy"), "a figure wears a glyph or copies nothing");
        expect("the battery warns low", parts[0].graph.shape === "bar" && parts[0].graph.warn
            && parts[0].note === "running low", JSON.stringify(parts[0].graph));
        expect("the signal's note says the power", parts[1].note
            === "where its radio hears best, sending at 4 dBm", parts[1].note);
        expect("the signal's reads drawn as a spark, a missed one left out",
            JSON.stringify(parts[1].graph) === JSON.stringify({
                shape: "spark", values: [0.5, 1], warn: false
            }), JSON.stringify(parts[1].graph));
        expect("the quality's reads drawn as a spark", parts[2].graph.shape === "spark"
            && parts[2].graph.values.length === 2 && parts[2].graph.values[0] === 0.8,
            JSON.stringify(parts[2].graph));
        const strip = parts[3].graph;
        expect("the channels drawn as a strip", strip.shape === "strip" && strip.values.length
            === 20 && strip.values[6] === 0 && strip.values[0] === 1 && strip.warn === false,
            JSON.stringify(strip));
        expect("the skipped said", parts[3].note === "the link skips the other 18 as noisy",
            parts[3].note);
        expect("how it's held said", parts[4].note
            === "the device started it, with Secure Connections\u2019 keys", parts[4].note);
        expect("the address copied", parts[6].verb === "copy" && parts[6].glyph === "",
            parts[6].verb);

        const trust = parts[5];
        expect("a setting holds its choices", trust.verb === "more" && trust.glyph !== ""
            && said(trust.verbs()) === "Let it reconnect by itself=in use,"
                + "Only connect it when asked=", said(trust.verbs()));
        trust.verbs()[1].run();
        expect("a choice sets it", !headphones.trusted, "it's still trusted");
        headphones.trusted = true;

        const pointed = bluetooth.verbs(mouse).filter(row => row.key.startsWith("link:"));
        expect("a Low Energy link", said(pointed)
            === "Signal=-58 dBm,Connection=Low Energy, peripheral, not encrypted",
            said(pointed));
        expect("one read is a spark of one", pointed[0].graph.values.length === 1,
            JSON.stringify(pointed[0].graph));
        const wake = bluetooth.verbs(mouse).find(row => row.key.endsWith(":wake"));
        expect("an input device may wake the machine", wake && wake.side === "no",
            "there's no Wakes the machine");
        wake.verbs()[0].run();
        expect("waking let", mouse.wakeAllowed, "it may not wake the machine");
        expect("headphones don't wake it", heard.every(row => !row.key.endsWith(":wake")),
            "headphones may wake the machine");
        expect("a device not connected has no link", bluetooth.verbs(known)
            .every(row => !row.key.startsWith("link:")), "a device away has a link");
        expect("no reader, no link", unread.verbs(headphones)
            .every(row => !row.key.startsWith("link:")), "rows came from no file");

        const one = bluetooth.linksAnswer([headphones]);
        expect("one link answered", one.headline === "in range, 61 of 79 clear"
            && said(one.rows) === "Signal=in range,Channels=61 of 79 clear"
            && one.note === "the link to Headphones" && one.graph.shape === "spark",
            JSON.stringify(one.headline));
        const both = bluetooth.linksAnswer([headphones, mouse]);
        expect("two links answered", both.headline === "2 connected"
            && said(both.rows) === "Headphones=in range, 61 of 79 clear,Mouse=-58 dBm",
            said(both.rows));
        expect("each a step in", said(both.rows[0].verbs())
            === "Signal=in range,Channels=61 of 79 clear", said(both.rows[0].verbs()));
        expect("nothing connected, nothing answered", bluetooth.linksAnswer([]) === null,
            "it answered with nothing connected");
        expect("no reader, no answer", unread.linksAnswer([headphones]) === null,
            "it answered with no file");
    }

    // The adapter's parts: its settings, its traffic, and what it is.
    function adapterHolds() {
        const parts = bluetooth.adapterParts(radio);
        expect("the adapter's parts", said(parts) === ["Radio=on", "Visible to others=no",
            "Pairable=yes", "Traffic=12 kb/s", "Name=studio", "Address=02:00:00:00:00:01",
            "Version=Bluetooth 6.0", "Model=Intel"].join(","), said(parts));
        expect("visible for a while", parts[1].note === "for 3 min once made visible",
            parts[1].note);
        expect("its traffic drawn as a spark", JSON.stringify(parts[3].graph)
            === JSON.stringify({ shape: "spark", values: [0, 0.5, 1], warn: false })
            && parts[3].note === "9.6 kb/s in, 2.4 kb/s out", JSON.stringify(parts[3]));
        expect("what it is copied", parts.slice(4).every(row => row.verb === "copy"
            && row.glyph === ""), "a figure of the adapter doesn't copy");
        const choices = parts[0].verbs();
        expect("the radio's choices", said(choices)
            === "Turn the radio on=in use,Turn the radio off=", said(choices));
        parts[1].verbs()[0].run();
        expect("made visible", radio.discoverable, "it's still hidden");
        choices[1].run();
        expect("the radio off", !radio.enabled, "the radio's still on");
        expect("off, it says only its radio and what it is", said(bluetooth.adapterParts(radio))
            .startsWith("Radio=off,Name=studio,"), said(bluetooth.adapterParts(radio)));
        expect("no reader, only what Quickshell says", said(unread.adapterParts(radio))
            === "Radio=off,Name=studio", said(unread.adapterParts(radio)));
    }

    Component.onCompleted: {
        try {
            bluetooth.join(linkedFirst);
            expect("pairing starts", linkedFirst.pairing, "it wasn't asked to pair");
            expect("trusted before paired", linkedFirst.trusted,
                "it wasn't trusted before pairing, so its audio would be refused");
            linkedFirst.connected = true;
            expect("linked before paired", bluetooth.joining === linkedFirst,
                "it stopped following the device once its link came up");
            linkedFirst.paired = true;
            linkedFirst.pairing = false;
            expect("paired", linkedFirst.trusted, "it wasn't trusted");
            expect("paired and linked", bluetooth.joining === null, "it's still connecting");
            expect("no second connect", linkedFirst.asked === 0, "it connected again");

            bluetooth.join(refused);
            refused.connected = true;
            refused.pairing = false;
            refused.connected = false;
            expect("refused", bluetooth.trouble !== "", "it didn't say why");
            expect("refused, untrusted", !refused.trusted, "it's still trusted");

            bluetooth.join(known);
            expect("known connects", known.asked === 1, "it wasn't asked to connect");
            known.connected = true;
            expect("known connected", bluetooth.joining === null, "it's still connecting");
            facts.start();
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    // The facts file is read a moment after start, so this waits for it,
    // for up to five seconds.
    Timer {
        id: facts

        property int tries: 0

        interval: 50
        repeat: true
        onTriggered: {
            try {
                facts.tries++;
                if (bluetooth.facts[mouse.address] === undefined) {
                    root.expect("facts read", facts.tries < 100, "bluetooth.json was never read");
                    return;
                }
                facts.stop();
                const rows = bluetooth.verbs(mouse);
                root.expect("facts after verbs", rows[0].key === "verb:bluetooth-disconnect",
                    `the first row is ${rows[0].key}`);
                const input = rows.find(row => row.key === `bluetooth:${mouse.address}:input`);
                root.expect("how it's carried holds the rest", input && input.verb === "more"
                    && root.said([input]) === "Input=LE, HID over GATT", root.said(rows));
                const told = input.verbs();
                root.expect("facts named first, in order", root.said(told) === "Buttons=5",
                    root.said(told));
                root.expect("facts keyed and copied", told[0].key
                    === "fact:bluetooth:C3:9C:E3:2A:F3:6E:buttons" && told[0].verb === "copy"
                    && told[0].glyph === undefined, told[0].key);
                root.expect("another device has none", bluetooth.verbs(known)
                    .every(row => !row.key.endsWith(":input")), "a stranger got facts");
                root.linksHold();
                root.adapterHolds();
                Qt.callLater(() => Qt.exit(0));
            } catch (error) {
                facts.stop();
                console.warn(`FAIL ${error.message}`);
                Qt.callLater(() => Qt.exit(1));
            }
        }
    }
}

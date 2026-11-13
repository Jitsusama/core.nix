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
// each a setting whose choices set it; and its Hardware Address, which Return
// copies. The adapter holds its Radio, Visible to others and Pairable as
// settings, its Traffic as a spark, and its Name, Hardware Address, Version and
// Chip.
// A device the devices piece says is running low (the check's Facts.qml)
// says so at its side, and one that isn't says it's connected. An adapter
// the piece says stopped answering says so at its side, warning, with what
// to do as its note, and holds first the line that starts it again, which
// Return copies; once it answers it's on again.
// How the links are doing is answered too; with no such file, neither the
// links nor the answer say a thing.
//
// Nearby: Bluetooth lists only the devices paired, then Nearby, then the
// Adapter; the radio looks only while Nearby is held open, which lists the
// named devices heard, strongest first, each with its signal as a bar and
// in dBm and whether it's paired, and says how many it heard. A pairing or
// connecting that fails says what BlueZ answered, in words.json's words
// (launcher/Nearby.js). It exits 0 once every case holds, and otherwise 1,
// saying which case didn't.
import qs
import qs.launcher
import Quickshell
import QtQuick
import "launcher/Nearby.js" as Nearby

ShellRoot {
    id: root

    component Device: QtObject {
        property string name: "Buds"
        property string deviceName: name
        property string address: ""
        property int rssi: 0
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

        signal failed(string name, string message)

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

    // What the radio hears for Nearby: a speaker beside the machine, some
    // headphones paired before across the room, a beacon that gives no
    // name, and a keyboard it heard before but not since it last looked.
    Device {
        id: speaker
        name: "Speaker"
        address: "AA:00:00:00:00:01"
        icon: "audio-card"
        rssi: -48
    }
    Device {
        id: across
        name: "Studio Buds"
        address: "AA:00:00:00:00:02"
        icon: "audio-headphones"
        paired: true
        trusted: true
        rssi: -81
    }
    Device {
        id: beacon
        name: "AA-00-00-00-00-03"
        deviceName: ""
        address: "AA:00:00:00:00:03"
        rssi: -40
    }
    Device { id: unheard; name: "Keys"; address: "AA:00:00:00:00:04"; icon: "input-keyboard" }

    QtObject {
        id: air
        property bool enabled: true
        property bool discovering: false
        readonly property var devices: ({ values: [speaker, across, beacon, unheard] })
    }

    // The launcher, as far as Nearby asks it: open, and what's held.
    QtObject {
        id: theLauncher
        property bool open: true
        property var held: []
    }

    Accessories {
        id: finding
        adapter: air
        launcher: theLauncher
    }

    function nearbyHolds() {
        expect("not looking until Nearby opens", !air.discovering && finding.nearby.length === 0,
            `discovering ${air.discovering}`);
        expect("Bluetooth lists the paired, then Nearby, then the Adapter",
            finding.all.map(row => row.key).join(",") === [`bluetooth:${across.address}`,
                "bluetooth:nearby", "bluetooth:adapter"].join(","),
            finding.all.map(row => row.key).join(","));
        const place = finding.nearbyPlace;
        expect("Nearby says it looks while open", place.side === ""
            && place.note === "looks while open" && place.verb === "more"
            && place.empty === "Nothing heard yet.", `${place.side}; ${place.note}`);
        theLauncher.held = [{ row: { key: "bluetooth:nearby" } }];
        expect("looking while Nearby is open", air.discovering, "the radio isn't looking");
        const rows = finding.nearbyPlace.verbs();
        expect("the named heard, strongest first, then how many more had no name", said(rows)
            === "Speaker=\u221248 dBm \u00b7 strong,Studio Buds=\u221281 dBm \u00b7 weak \u00b7 paired,"
            + "1 more without a name=", said(rows));
        expect("how many it heard by name", finding.nearbyPlace.side === "2 heard",
            finding.nearbyPlace.side);
        const unnamed = rows[2].verbs();
        expect("the nameless stepped into, by address, to pair", rows[2].verb === "more"
            && said(unnamed) === "AA-00-00-00-00-03=\u221240 dBm \u00b7 strong"
            && unnamed[0].verb === "pair",
            said(unnamed));
        expect("a stranger is paired", rows[0].verb === "pair" && rows[1].verb === "connect",
            `${rows[0].verb}, ${rows[1].verb}`);
        expect("each heard in dBm, with a real minus, its band muted after it",
            rows[0].sideItems.map(item => `${item.said}/${item.hue ?? ""}`).join(",")
                === "\u221248 dBm/,strong/muted" && rows[0].note === "",
            JSON.stringify(rows[0].sideItems));
        expect("paired says most, the band least", rows[1].sideItems.map(item => item.keep)
            .join(",") === "2,1,3", JSON.stringify(rows[1].sideItems));
        expect("each its signal as a bar", rows[0].graph.shape === "bar"
            && Math.abs(rows[0].graph.values[0] - 52 / 70) < 0.001 && !rows[0].graph.warn,
            JSON.stringify(rows[0].graph));
        expect("each its kind's glyph, and the same thing as Bluetooth's",
            rows[1].glyph === finding.made(across).glyph
            && rows[1].key === finding.made(across).key, rows[1].glyph);
        speaker.rssi = -70;
        expect("heard again, said again", finding.nearbyPlace.verbs()[0].side
            === "\u221270 dBm \u00b7 fair", finding.nearbyPlace.verbs()[0].side);
        theLauncher.held = [];
        expect("done looking once Nearby closes", !air.discovering, "the radio's still looking");

        expect("none heard, none listed", Nearby.heard([unheard]).length === 0, "one listed");
        // Alike in signal, by name as it's read, then in the order heard.
        const alike = (name, address) => ({ name, deviceName: name, address, rssi: -60 });
        const tied = Nearby.heard([alike("Buds 10", "1"), alike("Buds 9", "2"),
            alike("Buds 9", "3"), alike("Buds 9", "4"), alike("Buds 9", "5")])
            .map(device => `${device.name}@${device.address}`).join(",");
        expect("alike in signal, A to Z as read, then as heard",
            tied === "Buds 9@2,Buds 9@3,Buds 9@4,Buds 9@5,Buds 10@1", tied);
        expect("no signal unheard", Nearby.strength(unheard) === null, "a signal unheard");
        const failures = [
            ["org.bluez.Error.AuthenticationFailed", "Authentication Failed", "passkey"],
            ["org.bluez.Error.AuthenticationRejected", "Authentication Rejected", "refused"],
            ["org.bluez.Error.AuthenticationCanceled", "Authentication Canceled", "stopped"],
            ["org.bluez.Error.AuthenticationTimeout", "Authentication Timeout", "unreached"],
            ["org.bluez.Error.ConnectionAttemptFailed", "Page Timeout", "unreached"],
            ["org.bluez.Error.Failed", "br-connection-page-timeout", "unreached"],
            ["org.bluez.Error.Failed", "br-connection-refused", "refused"],
            ["org.bluez.Error.Failed", "br-connection-profile-unavailable", "nothing"],
            ["org.bluez.Error.InProgress", "In Progress", "busy"],
            ["org.bluez.Error.NotReady", "Resource Not Ready", "unready"],
            ["org.bluez.Error.AlreadyExists", "Already Exists", ""],
            ["org.bluez.Error.Failed", "Input/output error", "other"]
        ];
        for (const [name, message, want] of failures)
            expect(`${name} ${message}`, Nearby.failure(name, message) === want,
                Nearby.failure(name, message));

        finding.join(speaker);
        speaker.failed("org.bluez.Error.AuthenticationFailed", "Authentication Failed");
        speaker.pairing = false;
        expect("what BlueZ answered said", finding.said(speaker, "")
            === "Speaker not paired \u00b7 it wants a passkey", finding.said(speaker, ""));
        expect("a failed pairing untrusted", !speaker.trusted, "it's still trusted");
        finding.join(across);
        expect("connecting said", finding.said(across, "") === "connecting Studio Buds",
            finding.said(across, ""));
        across.failed("org.bluez.Error.Failed", "Input/output error");
        expect("anything else in BlueZ's own words", finding.said(across, "")
            === "Studio Buds not connected \u00b7 input/output error", finding.said(across, ""));
        finding.join(across);
        across.failed("org.bluez.Error.NotSupported", "");
        expect("with no message, BlueZ's name for it", finding.said(across, "")
            === "Studio Buds not connected \u00b7 not supported", finding.said(across, ""));
        finding.join(speaker);
        expect("pairing said", finding.said(speaker, "") === "pairing with Speaker",
            finding.said(speaker, ""));
        speaker.pairing = false;
        expect("no answer, not paired said", finding.said(speaker, "") === "Speaker not paired",
            finding.said(speaker, ""));
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
    // The adapter's own row, as Bluetooth lists it.
    Accessories {
        id: stalled
        adapter: radio
        settings: ({ links: `${Quickshell.env("XDG_RUNTIME_DIR")}/bluetooth-links.json` })
    }

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
        // d ends a joined device by disconnecting it, as the spec's table
        // says, and leaves forgetting it, which can't be put back, to a
        // command named.
        expect("d disconnects, never forgets", heard[0].act === "ending"
            && heard[1].act === undefined, `${heard[0].act}, ${heard[1].act}`);
        const parts = heard.slice(2);
        expect("a classic link's parts", said(parts) === ["Battery=15%", "Signal=in range",
            "Link Quality=90%", "Channels=61 of 79 clear",
            "Connection=Classic, central, encrypted", "Reconnects by Itself=yes",
            `Hardware Address=${at}`].join(","), said(parts));
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
            && said(trust.verbs()) === "Let It Reconnect by Itself=chosen,"
                + "Only Connect It When Asked=", said(trust.verbs()));
        // Each its own word for the foot, which its name's first wouldn't say.
        expect("a choice says what it does", trust.verbs().map(row => row.verb).join(",")
            === "reconnect,ask", trust.verbs().map(row => row.verb).join(","));
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
        expect("one link answered", one.headline === "in range \u00b7 61 of 79 clear"
            && said(one.rows) === "Signal=in range,Channels=61 of 79 clear"
            && one.note === "the link to Headphones" && one.graph.shape === "spark",
            JSON.stringify(one.headline));
        const both = bluetooth.linksAnswer([headphones, mouse]);
        expect("two links answered", both.headline === "2 connected"
            && said(both.rows) === "Headphones=in range \u00b7 61 of 79 clear,Mouse=-58 dBm",
            said(both.rows));
        expect("each a step in", said(both.rows[0].verbs())
            === "Signal=in range,Channels=61 of 79 clear", said(both.rows[0].verbs()));
        expect("nothing connected, nothing answered", bluetooth.linksAnswer([]) === null,
            "it answered with nothing connected");
        expect("no reader, no answer", unread.linksAnswer([headphones]) === null,
            "it answered with no file");
    }

    // A device running low says so at its side, in the warning colour, as
    // Now lists it too.
    function lowHolds() {
        const low = bluetooth.made(headphones);
        expect("one running low says so at its side", low.side === "15% \u00b7 running low"
            && low.note === "connected", `${low.side}; ${low.note}`);
        expect("one running low warns", low.warns === true, String(low.warns));
        const fine = bluetooth.made(mouse);
        expect("one not low says it's connected", fine.side === "connected", fine.side);
        expect("one not low doesn't warn", fine.warns === false, String(fine.warns));
    }

    // The adapter whose controller stopped answering, as the devices piece
    // would say it, and then answering again.
    function stoppedHolds() {
        const before = Facts.all;
        const line = "echo 0000:00:14.7 | sudo tee /sys/bus/pci/drivers/btintel_pcie/unbind; "
            + "echo 0000:00:14.7 | sudo tee /sys/bus/pci/drivers/btintel_pcie/bind";
        Facts.all = before.concat([{ entry: { mark: "bluetooth:adapter" }, piece: {
            place: "bluetooth", line: line,
            context: "type the line in a terminal to start it again" } }]);
        const row = stalled.adapterRow;
        expect("one stopped says so at its side, warning", row.side === "not answering"
            && row.warns === true, `${row.side}; ${row.warns}`);
        expect("and what to do as its note",
            row.note === "type the line in a terminal to start it again", row.note);
        const parts = stalled.adapterParts(radio);
        expect("the line that starts it again first", parts[0].key === "bluetooth:adapter:restart"
            && said([parts[0]]) === "Start It Again=", said(parts));
        expect("which Return copies", parts[0].verb === "copy" && parts[0].copies() === line,
            parts[0].verb);
        Facts.all = before;
        const back = stalled.adapterRow;
        expect("once it answers it's as its radio is", back.side === (radio.enabled ? "on" : "off")
            && back.warns === false && back.note === "", `${back.side}; ${back.warns}`);
        expect("with no line in its parts", stalled.adapterParts(radio)
            .every(part => part.key !== "bluetooth:adapter:restart"), said(parts));
    }

    // The adapter's parts: its settings, its traffic, and what it is.
    function adapterHolds() {
        const parts = bluetooth.adapterParts(radio);
        expect("the adapter's parts", said(parts) === ["Radio=on", "Visible to Others=no",
            "Pairable=yes", "Traffic=12 kb/s", "Name=studio",
            "Hardware Address=02:00:00:00:00:01",
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
            === "Turn the Radio On=chosen,Turn the Radio Off=", said(choices));
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
                root.expect("how it's carried holds the rest", input && input.verb === "copy"
                    && !!input.targets
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
                root.stoppedHolds();
                root.lowHolds();
                root.nearbyHolds();
                Qt.callLater(() => Qt.exit(0));
            } catch (error) {
                facts.stop();
                console.warn(`FAIL ${error.message}`);
                Qt.callLater(() => Qt.exit(1));
            }
        }
    }
}
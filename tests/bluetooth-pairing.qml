// Drives the launcher's Bluetooth (modules/quickshell/launcher/Accessories.qml)
// through pairing and connecting, with a stand-in for each device: BlueZ
// says a new device is connected as soon as its link is up, before pairing
// is done, so connecting alone doesn't finish it; it's trusted before it
// pairs, since headphones start their audio the moment they're paired and
// BlueZ refuses an untrusted device's with no agent to ask; a pairing that
// fails says so and takes the trust back; and a device paired before is
// done once it connects. A connected mouse's facts, from the bluetooth.json
// the check puts where desk-facts would, follow its verbs as copy rows, in
// the file's order. How a connected device's link is doing, from the file
// the check puts where bluetooth-links would write, comes between the two
// as figures, and answers how the links are doing; with no such file,
// neither says a thing. It exits 0 once every case holds, and otherwise 1,
// saying which case didn't.
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
    Device { id: mouse; name: "Mouse"; address: "C3:9C:E3:2A:F3:6E"; paired: true; connected: true }
    Device {
        id: headphones
        name: "Headphones"
        address: "00:1B:66:AA:BB:CC"
        paired: true
        connected: true
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

    // How the links are doing, beside each connected device's verbs and as
    // an answer.
    function linksHold() {
        const heard = bluetooth.verbs(headphones);
        expect("links after verbs, before facts", heard.map(row => row.key).join(",")
            === ["verb:bluetooth-disconnect", "verb:bluetooth-forget"]
                .concat(["signal", "quality", "channels"]
                    .map(part => `link:bluetooth:${headphones.address}:${part}`)).join(","),
            heard.map(row => row.key).join(","));
        const figures = heard.slice(2);
        expect("a classic link's figures", said(figures)
            === "Signal=in range,Link=90%,Channels=61 of 79 clear", said(figures));
        expect("figures wear no glyph", figures.every(row => row.glyph === "" && row.copied
            === undefined && row.verb === "close"), "a figure wears a glyph or copies");
        expect("the signal's note says the power", figures[0].note
            === "where its radio hears best, sending at 4 dBm", figures[0].note);
        const strip = figures[2].graph;
        expect("the channels drawn as a strip", strip.shape === "strip" && strip.values.length
            === 20 && strip.values[6] === 0 && strip.values[0] === 1 && strip.warn === false,
            JSON.stringify(strip));
        expect("the skipped said", figures[2].note === "the link skips the other 18 as noisy",
            figures[2].note);

        const pointed = bluetooth.verbs(mouse).filter(row => row.key.startsWith("link:"));
        expect("a Low Energy link's signal", said(pointed) === "Signal=-58 dBm", said(pointed));
        expect("a device not connected has no link", bluetooth.verbs(known)
            .every(row => !row.key.startsWith("link:")), "a device away has a link");
        expect("no reader, no link", unread.verbs(headphones)
            .every(row => !row.key.startsWith("link:")), "rows came from no file");

        const one = bluetooth.linksAnswer([headphones]);
        expect("one link answered", one.headline === "in range, 61 of 79 clear"
            && said(one.rows) === "Signal=in range,Channels=61 of 79 clear"
            && one.note === "the link to Headphones", JSON.stringify(one.headline));
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
                const told = rows.filter(row => row.key.startsWith("fact:"));
                root.expect("facts after verbs", rows[0].key === "verb:bluetooth-disconnect",
                    `the first row is ${rows[0].key}`);
                root.expect("facts in order",
                    told.map(row => `${row.side}=${row.name}`).join(",")
                        === "connection=LE, HID over GATT,buttons=5",
                    told.map(row => `${row.side}=${row.name}`).join(","));
                root.expect("facts keyed",
                    told[1].key === "fact:bluetooth:C3:9C:E3:2A:F3:6E:buttons", told[1].key);
                root.expect("another device has none", bluetooth.verbs(known)
                    .every(row => !row.key.startsWith("fact:")), "a stranger got facts");
                root.linksHold();
                Qt.callLater(() => Qt.exit(0));
            } catch (error) {
                facts.stop();
                console.warn(`FAIL ${error.message}`);
                Qt.callLater(() => Qt.exit(1));
            }
        }
    }
}

// Drives the launcher's Bluetooth (modules/quickshell/launcher/Accessories.qml)
// through pairing and connecting, with a stand-in for each device: BlueZ
// says a new device is connected as soon as its link is up, before pairing
// is done, so connecting alone doesn't finish it; it's trusted before it
// pairs, since headphones start their audio the moment they're paired and
// BlueZ refuses an untrusted device's with no agent to ask; a pairing that
// fails says so and takes the trust back; and a device paired before is
// done once it connects. It exits 0 once every
// case holds, and otherwise 1, saying which case didn't.
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

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    Component.onCompleted: {
        try {
            const made = Qt.createComponent("launcher/Accessories.qml");
            if (made.status !== Component.Ready)
                throw new Error(`loading: ${made.errorString()}`);
            const bluetooth = made.createObject(root, { settings: {} });

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
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }
}

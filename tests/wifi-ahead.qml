// Has the launcher's Wi-Fi (launcher/Wireless.qml) hold the network joined
// ahead while the launcher's open, through Quickshell itself, the launcher
// and Wifi's card stood in, with the real ReadyRows.qml making its rows: so
// wifid serves its drawer and the rows a step into it shows before Joel
// steps onto Wi-Fi or into it, and that key draws them in its own frame,
// rather than a round trip on once its row's selected ("→ Network" took
// 112 ms on 2026-11-13, wifid telling it 46 ms after the key's work). Its
// drawer's figures move at every read of the link, so a change to its
// drawer alone tells the launcher nothing while nobody looks at it, and
// the row the launcher holds draws the drawer as wifid serves it now. It
// exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // What Wireless.qml reads of the launcher to know what's ahead: whether
    // it's open and the row selected.
    QtObject {
        id: standIn

        property bool open: false
        property var held: []
        property string typed: ""
        property var matches: []
        property var selected: null
        property var terminalSees: ({ walked: [] })
        property var sources: [wireless]
        function wants(source) {
            return open;
        }
    }

    Launcher.Wireless {
        id: wireless

        launcher: standIn
        settings: ({ name: "wifi" })
    }

    // How many times Wi-Fi told the launcher it changed.
    property int told: 0
    Connections {
        target: wireless
        function onChanged() {
            root.told += 1;
        }
    }

    // Lantern's record as wifid serves it, joined, trusted or not, its
    // drawer saying `said` where it says anything.
    function lantern(said, trusted) {
        const key = "row:wifi:network:Lantern";
        const row = {
            key, thing: "wifi:Lantern", place: "wifi", kind: "network", name: "Lantern",
            glyph: "lockedSignal3", hue: "machine", side: [{ said: "3 of 4 bars" }],
            held: { network: "Lantern", security: "personal", strength: 3, connected: true,
                known: true, trusted },
            acts: [], order: 1024
        };
        if (said !== "")
            row.drawer = { title: "Lantern", facts: [], body: said };
        return { key, kind: "row", row };
    }

    // What the row the launcher holds draws in its drawer.
    function drawn(row) {
        return (typeof row.drawer === "function" ? row.drawer() : row.drawer)?.body ?? "";
    }

    // Each change is told as wifid tells it, and how many times Wi-Fi said
    // it changed meanwhile is given back.
    function tell(record) {
        const before = told;
        wireless.served.link.took([record], []);
        return told - before;
    }

    // The cases run once ReadyRows' daemon is loaded, which its LazyLoader
    // may do after the shell starts.
    Until {
        id: waits
    }
    Component.onCompleted: waits.until("ReadyRows' daemon loaded", () => !!wireless.served.link,
        () => {
            wireless.served.link.wire = { write: line => {}, flush: () => {} };
            Wifi.cards = [{ name: "wlan0", network: "Lantern" }];
            ahead();
            quiet();
            Qt.exit(0);
        })

    // The network joined is ahead only while the launcher's open and the
    // radio's on, so the link is read no longer than it could be drawn.
    function ahead() {
        const joined = ["row:wifi:network:Lantern"];
        Expect.same("closed, nothing is ahead", wireless.ahead, []);
        standIn.open = true;
        Expect.same("open at home, the network joined is ahead", wireless.ahead, joined);
        Wifi.radio = false;
        Expect.same("open with the radio off, nothing is", wireless.ahead, []);
        Wifi.radio = true;
        standIn.open = false;
        Expect.same("closed again, nothing is", wireless.ahead, []);
    }

    // A change to the joined network's drawer alone, nobody looking at it,
    // tells the launcher nothing, the row it holds drawing the drawer as
    // served now; selected, a drawer coming or going, or a change past it,
    // is told.
    function quiet() {
        standIn.open = true;
        tell(lantern("-50 dBm", false));
        const held = wireless.all.find(row => row.network === "Lantern");
        Expect.same("its drawer alone changing, nobody looking, tells nothing",
            tell(lantern("-51 dBm", false)), 0);
        Expect.same("and the row held draws the drawer as it's served now", drawn(held),
            "-51 dBm");
        standIn.selected = held;
        Expect.expect("selected, its drawer changing is told", tell(lantern("-52 dBm", false)) > 0,
            "");
        standIn.selected = null;
        Expect.expect("a change past its drawer is told", tell(lantern("-52 dBm", true)) > 0, "");
        Expect.expect("its drawer going is told", tell(lantern("", true)) > 0, "");
        Expect.expect("and coming", tell(lantern("-53 dBm", true)) > 0, "");
    }
}

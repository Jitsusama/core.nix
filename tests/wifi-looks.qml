// Has the launcher's Wi-Fi (launcher/Wireless.qml) look for networks only
// while its rows are shown, through Quickshell itself, the launcher and
// Wifi's card stood in: Wi-Fi or something in it held, or a find from the
// root showing a row of its, with the launcher open; or a terminal's show
// walked into it. Open on another place, at home, or on a find of another
// place's rows, it asks wifid for no look, since each takes the card off
// its own channel for seconds (2026-11-14). What's sent goes to a stand-in
// wire. It exits 0 once every case holds, and otherwise 1, saying which
// didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // What Wireless.qml reads of the launcher to know where it is: whether
    // it's open, the steps held, what's typed, the rows found, and what a
    // terminal walked.
    QtObject {
        id: standIn

        property bool open: false
        property var held: []
        property string typed: ""
        property var matches: []
        property var terminalSees: ({ walked: [] })
        property var sources: [wireless]
        function wants(source) {
            return open;
        }
    }

    // Another place, as Agents, whose rows are held or found.
    QtObject {
        id: elsewhere
    }

    Launcher.Wireless {
        id: wireless

        launcher: standIn
        settings: ({ name: "wifi" })
    }

    // Where the launcher is: open or not, the places its steps came from,
    // what's typed, and the places of the rows it found.
    function at(open, held, typed, found) {
        standIn.held = held.map(from => ({ row: { from } }));
        standIn.typed = typed;
        standIn.matches = found.map(from => ({ from }));
        standIn.open = open;
    }

    // The looks asked of wifid, each line as the shell writes it.
    property var sent: []
    function looks() {
        return sent.filter(said => said.act === "scan").length;
    }

    Until {
        id: waits
    }

    // No look can be waited for, so the launcher is left open on another
    // place for a while: a Timer's start is posted to the event loop, and
    // asked within a turn or two of it.
    Timer {
        id: awhile

        interval: 500
        onTriggered: {
            try {
                root.elsewhereAsksNone();
            } catch (error) {
                Expect.fail(`${error} at ${error.stack}`);
            }
        }
    }

    // The wire is laid once ReadyRows' daemon is loaded, which its
    // LazyLoader may do after the shell starts; then the launcher opens on
    // another place.
    Component.onCompleted: waits.until("ReadyRows' daemon loaded", () => !!wireless.served.link,
        () => {
            wireless.served.link.wire = { write: line => root.sent.push(JSON.parse(line)),
                flush: () => {} };
            root.at(true, [elsewhere], "", [elsewhere]);
            awhile.start();
        })

    // Open on another place, the cards are asked for no look; stepped into
    // Wi-Fi, they're asked for one at once, well before the 10 s between.
    function elsewhereAsksNone() {
        Expect.same("open on another place, no look is asked", looks(), 0);
        at(true, [wireless], "", []);
        const asked = Date.now();
        waits.until("a look asked as Wi-Fi is stepped into", () => looks() > 0, () => {
            Expect.expect("stepped into Wi-Fi, a look is asked at once",
                Date.now() - asked < 2000, `${Date.now() - asked} ms`);
            where();
            Qt.exit(0);
        });
    }

    // Where the cards look, and where they don't.
    function where() {
        const looking = [
            ["open at home, nothing typed", [true, [], "", []], false],
            ["on another place", [true, [elsewhere], "", [elsewhere]], false],
            ["on a find of another place's rows", [true, [], "ag", [elsewhere]], false],
            ["on a find showing a row of Wi-Fi's", [true, [], "lan", [elsewhere, wireless]], true],
            ["in Wi-Fi", [true, [wireless], "", []], true],
            ["in a network in Wi-Fi", [true, [wireless, wireless], "", []], true],
            ["in Wi-Fi under another place", [true, [elsewhere, wireless], "", []], true],
            ["in Wi-Fi, the launcher closed", [false, [wireless], "", []], false]
        ];
        for (const [said, [open, held, typed, found], looks] of looking) {
            at(open, held, typed, found);
            Expect.same(`${said}, the cards ${looks ? "look" : "don't look"}`,
                wireless.looksForNetworks, looks);
        }
        standIn.terminalSees = { walked: [{ from: wireless }] };
        Expect.same("closed, a terminal's show walked into Wi-Fi, the cards look",
            wireless.looksForNetworks, true);
        standIn.terminalSees = { walked: [] };
        at(true, [wireless], "", []);
        Wifi.radio = false;
        Expect.same("in Wi-Fi with the radio off, the cards don't look",
            wireless.looksForNetworks, false);
        Wifi.radio = true;
    }
}

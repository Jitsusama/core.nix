// Drives the real ReadyRows.qml, through Wi-Fi's place (launcher/Wireless.qml), to each line
// its journal says of a daemon's refusal: an act that shuts the launcher refused, one withheld,
// what an act asks besides refused, words typed in a row that the daemon didn't answer, a
// search of the whole place it can't make, a search finding a row not served yet, an act
// of Wi-Fi's own refused, and a copy whose words are asked at Return, refused. Each
// refusal names a network in the daemon's own words, as "couldn't join Harbour House" would,
// and the check around it reads the shell's output, with reports off, for each line saying how
// many letters rather than the words (Y2YBTS, 2026-11-20, 41ca61c4). What's told on screen
// keeps the words whole, as Joel reads them there. What's sent goes to a stand-in wire, answered
// by its hear, so every case runs in one turn with no daemon. It exits 0 once every case holds,
// and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string key: "row:wifi:network:Harbour House"

    // What the launcher is told of a refusal, to say as Joel next opens it.
    QtObject {
        id: standIn

        property bool open: true
        // What Wireless.qml reads of the launcher besides, as Launcher.qml has them, so its
        // bindings run as in the shell rather than throwing on what's missing.
        property var held: []
        property var selected: null
        property var terminalSees: ({ walked: [] })
        property var sources: [wireless]
        property var toldWhys: []
        function wants(source) {
            return open;
        }
        function toldNot(name, act, why) {
            toldWhys = toldWhys.concat([why]);
        }
        function becomesAll(provider, holds, how) {
        }
        function becameNot(key, why) {
        }
    }

    Launcher.Wireless {
        id: wireless

        launcher: standIn
        settings: ({ name: "wifi" })
    }

    // What the shell sends wifid, each line as it's written.
    property var sent: []
    readonly property var wire: ({
            write: line => root.sent.push(JSON.parse(line)),
            flush: () => {}
        })

    // The daemon answering the last line sent that `asks` with `error`.
    function refuse(asks, error) {
        const asked = sent.filter(asks);
        Expect.expect("the ask was sent", asked.length > 0, JSON.stringify(sent));
        wireless.served.link.hear(JSON.stringify({ id: asked[asked.length - 1].id, error }));
    }

    // The cases run once ReadyRows' daemon is loaded, which its LazyLoader may do after the
    // shell starts, and nothing here waits forever: a daemon that never loads fails, by name.
    property bool begun: false
    Component.onCompleted: begin()
    Timer {
        running: !root.begun
        interval: 30000
        onTriggered: Expect.fail("never came: ReadyRows' daemon loaded")
    }
    Connections {
        target: wireless.served
        function onLinkChanged() {
            root.begin();
        }
    }

    function begin() {
        if (begun || !wireless.served.link)
            return;
        begun = true;
        try {
            wireless.served.link.wire = root.wire;
            wireless.served.link.took([], []);
            shutRefused();
            withheld();
            alsoRefused();
            typedRefused();
            searchRefused();
            foundUnserved();
            wifiRefused();
            copyRefused();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error} at ${error.stack}`);
        }
    }

    // An act that shuts the launcher, refused in the daemon's words, which the screen tells.
    function shutRefused() {
        const forget = { key: "verb:wifi-forget",
            ask: { ask: "act", act: "forget", network: "Harbour House" } };
        wireless.served.shut(root.key, forget);
        refuse(said => said.act === "forget",
            { code: "refused", said: "couldn't forget Harbour House" });
        Expect.same("a shut act's refusal is told on screen in the daemon's words",
            standIn.toldWhys.slice(-1), ["couldn't forget Harbour House"]);
    }

    // An act its place won't send as it's asked, withheld in the place's words, its key naming
    // the network as a calendar's act names its meeting, so its line must leave that out too.
    function withheld() {
        wireless.served.whyWithheld = key => `${key.split(":").pop()} waits on your own key press`;
        wireless.served.withheld(root.key, { key: "verb:wifi-join:Harbour House",
            ask: { ask: "act", act: "join", network: "Harbour House" } });
        Expect.same("a withheld act is told on screen in its place's words",
            standIn.toldWhys.slice(-1), ["Harbour House waits on your own key press"]);
    }

    // What an act the shell does asks besides, refused, which only the journal says.
    function alsoRefused() {
        wireless.served.alsoAsked({ key: "verb:copy", copy: "Harbour House",
            also: { ask: "act", act: "read", network: "Harbour House" } });
        refuse(said => said.act === "read",
            { code: "gone", said: "Harbour House's notice is gone" });
    }

    // Words typed in a network's row, which the daemon didn't answer, in its words.
    function typedRefused() {
        wireless.served.typedIn(root.key, "door");
        refuse(said => said.ask === "search",
            { code: "busy", said: "Harbour House is busy", message: "Harbour House holds" });
    }

    // Words searched for over the whole place, which the daemon can't search, in its words.
    function searchRefused() {
        wireless.served.search("door", 0);
        refuse(said => said.ask === "search" && !said.within,
            { code: "unread", said: "couldn't search Harbour House" });
    }

    // A search's answer naming a row the daemon doesn't serve yet, its key a daemon's.
    function foundUnserved() {
        Expect.same("a row not served yet isn't made",
            wireless.served.foundRows({ keys: ["row:wifi:network:Harbour Annex"], rows: {} }),
            []);
    }

    // An act of Wi-Fi's own holding no row, refused with a code but no reason of its own.
    function wifiRefused() {
        wireless.actOn({ ask: "act", act: "forget", network: "Harbour House" }, []);
        refuse(said => said.act === "forget",
            { code: "gone", said: "Harbour House went" });
    }

    // A copy whose words its daemon gives only at Return, refused in its words, its key naming
    // the network, so its line says the act and the daemon's code alone.
    function copyRefused() {
        wireless.served.copyAsked(root.key, { key: "verb:wifi-copy:Harbour House",
            copy: { ask: { ask: "copy", network: "Harbour House" } } });
        refuse(said => said.ask === "copy",
            { code: "locked", said: "couldn't copy Harbour House's password" });
        Expect.same("a copy refused is told on screen in the daemon's words",
            standIn.toldWhys.slice(-1), ["couldn't copy Harbour House's password"]);
    }
}

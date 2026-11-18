// Drives Address's answer through Quickshell itself, over routing tables of
// the case's own in net/ beside it, to show an answer waits on no route
// file. A step into Network asks Address and Ethernet, which each read both
// files where they waited on them, and each wait took a median 6 to 14 ms
// and up to 133 on Joel's laptop under load (2026-11-17), of a step that
// has 50 ms for all of it. So an answer says the routes as last loaded, and
// they're loaded again in the background as the launcher opens, as the
// links change and when they're old: a route changed is said once that load
// lands, and not before. Which connection is the way out is how the case
// tells which tables were read. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string net: `${Quickshell.shellDir}/net`

    // The launcher, as much of it as Address reads: whether it's open.
    QtObject {
        id: shown

        property bool open: false
        property var held: []
        property var answer: null
    }

    Launcher.Networks {
        id: networks

        kernel: root.net
        launcher: shown
        settings: ({ lookups: false })
    }

    // The routes, IPv4's alone, since IPv6's are read the same way.
    FileView {
        id: route
        path: `${root.net}/route`
        blockWrites: true
        printErrors: false
    }

    // Wi-Fi with an address, and a cable with one, as the machine daemon
    // would tell them (Interfaces.js).
    readonly property var links: [{
        name: "wlan0",
        kind: "wifi",
        state: "up",
        mtu: 1500,
        mac: "3c:22:fb:12:34:56",
        own: "",
        assigned: "random",
        four: [{ address: "192.168.4.37", prefix: 22 }],
        six: []
    }, {
        name: "enp0s1",
        kind: "wired",
        state: "up",
        mtu: 1500,
        mac: "00:1b:21:aa:bb:cc",
        own: "",
        assigned: "own",
        four: [{ address: "192.168.4.52", prefix: 22 }],
        six: []
    }]

    readonly property string header:
        "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n"

    // The routing table with a default route by each of `ways`, the first
    // costing least, through 192.168.4.1, each used `used` times, as the
    // kernel counts with every connection made.
    function table(ways, used = 0) {
        return header + ways.map((name, i) =>
            `${name}\t00000000\t0104A8C0\t0003\t${used}\t${used}\t${100 + 500 * i}`
            + "\t00000000\t0\t0\t0\n").join("");
    }

    function written(text) {
        route.setText(text);
        route.waitForJob();
    }

    // The connection Address says the machine's traffic leaves by, or "".
    function wayOut() {
        const rows = networks.answer("address")?.rows ?? [];
        const out = rows.find(row => row.note === "where the machine's traffic leaves");
        return out ? String(out.key).replace(/^networks:/, "") : "";
    }

    Until {
        id: waits
    }

    // Holds once Address's way out is `name`, failing past two seconds,
    // well inside the five after which old routes are loaded again
    // anyway, so a load that lands is the one the case set off.
    function soon(name) {
        const began = Date.now();
        return () => {
            if (Date.now() - began > 2000)
                throw new Error(`the way out still ${wayOut() || "none"}, not ${name}`);
            return wayOut() === name;
        };
    }

    // The way out Address said each time it told the launcher it had
    // something new, asked again as the launcher would on the tell.
    property var told: []

    Connections {
        target: networks

        function onAnswered() {
            root.told.push(root.wayOut());
        }
    }

    // Holds once `ms` have passed, for a load the case can't see to land:
    // one reads a file in well under a millisecond here.
    function after(ms) {
        const began = Date.now();
        return () => Date.now() - began >= ms;
    }

    // Holds once a tell said `name`, failing past two seconds.
    function toldSoon(name) {
        const began = Date.now();
        return () => {
            if (Date.now() - began > 2000)
                throw new Error(`no tell said ${name}: ${JSON.stringify(told)}`);
            return told.includes(name);
        };
    }

    Component.onCompleted: Qt.callLater(root.begun)

    function begun() {
        try {
            // The provider's own load, as it was made, found no table yet;
            // the links arriving load the routes again, and the first
            // answer, its routes old, asks again too, so Wi-Fi is said
            // once a load lands.
            written(table(["wlan0", "enp0s1"]));
            networks.links = links;
            waits.until("Wi-Fi said as the way out once the routes are loaded",
                soon("wlan0"), root.loaded);
        } catch (error) {
            Expect.fail(error.message ?? error);
        }
    }

    // Both of those may have set off a load, and one still under way would
    // read the next table, standing in for the opening's: each lands first.
    function loaded() {
        waits.until("the first loads landed", after(200), root.settled);
    }

    // A cable plugged in, its route now costing least: an answer straight
    // after says what was loaded, since it reads nothing. Opened again, the
    // routes load in the background, and an answer while they do says them
    // as they were; once they land the launcher is told, so it asks again
    // and shows the change.
    function settled() {
        shown.open = false;
        written(table(["enp0s1", "wlan0"]));
        const out = wayOut();
        Expect.expect("an answer says the routes as last loaded, reading no file",
            out === "wlan0", out);
        told = [];
        shown.open = true;
        const loading = wayOut();
        Expect.expect("an answer while the opening's load is under way waits on none of it",
            loading === "wlan0", loading);
        waits.until("the launcher told of the cable as the way out once the opening's load lands",
            toldSoon("enp0s1"), root.opened);
    }

    // Each route used again, the gateways and their costs the same: a load
    // of that tells the launcher nothing, as the kernel's counts change with
    // every connection made.
    function opened() {
        told = [];
        written(table(["enp0s1", "wlan0"], 7));
        networks.reroute();
        waits.until("the load of the same routes landed", after(200), root.used);
    }

    // The cable's route gone with its link, the launcher left open and the
    // routes loaded as it opened, not yet old: the links changing loads the
    // routes again.
    function used() {
        Expect.expect("a load that changes no gateway or cost tells nothing",
            told.length === 0, JSON.stringify(told));
        written(table(["wlan0"]));
        networks.links = links.slice(0, 1);
        waits.until("a link changing loads the routes again", soon("wlan0"), () => Qt.exit(0));
    }
}

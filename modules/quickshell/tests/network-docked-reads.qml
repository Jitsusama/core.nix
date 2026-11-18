// Drives Ethernet's answer through Quickshell itself, over a card's folder,
// a resolv.conf and an uptime of the case's own beside it, to show an answer
// waits on none of their files. Docked with one card, a step into Network
// asked Ethernet, which read the cable's twenty files twice, resolv.conf
// and the uptime, each a wait of a median 6 to 14 ms and up to 133 on
// Joel's laptop under load (2026-11-17), of a step that has 50 ms for all
// of it; and read them all again each second while a cable was shown. So an
// answer says each as last loaded, and they're loaded again in the
// background as the launcher opens and each second while a cable's shown:
// a change is said once that load lands, and not before. It exits 0 once
// every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string dir: Quickshell.shellDir
    readonly property string card: `${dir}/sys/enp0s1`

    // The launcher, as much of it as Network reads: whether it's open and
    // what's held.
    QtObject {
        id: shown

        property bool open: false
        property var held: []
        property var answer: null
    }

    // Pointed at the uptime only once there's one, so the case sees a
    // lease before it lands.
    Launcher.Networks {
        id: networks

        kernel: `${root.dir}/net`
        sysfs: `${root.dir}/sys`
        resolvFile: `${root.dir}/etc/resolv.conf`
        resolvedFile: `${root.dir}/etc/resolved.conf`
        uptimeFile: ""
        // No networkd, so the case's leases stand.
        networkdSocket: `${root.dir}/runtime/no-networkd`
        launcher: shown
        settings: ({ lookups: false })
    }

    FileView {
        id: speed
        path: `${root.card}/speed`
        blockWrites: true
        printErrors: false
    }
    FileView {
        id: carrier
        path: `${root.card}/carrier`
        blockWrites: true
        printErrors: false
    }
    FileView {
        id: duplex
        path: `${root.card}/duplex`
        blockWrites: true
        printErrors: false
    }
    FileView {
        id: resolv
        path: `${root.dir}/etc/resolv.conf`
        blockWrites: true
        printErrors: false
    }
    FileView {
        id: up
        path: `${root.dir}/uptime`
        blockWrites: true
        printErrors: false
    }
    FileView {
        id: route
        path: `${root.dir}/net/route`
        blockWrites: true
        printErrors: false
    }

    function written(file, text) {
        file.setText(text);
        file.waitForJob();
    }

    // One cable with an address, as the machine daemon would tell it
    // (Interfaces.js), the way out by its route.
    readonly property var links: [{
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

    // Its lease, ending 7,230 s after the uptime the case first writes.
    readonly property var leases: ({
        enp0s1: { server: "192.168.4.1", ends: 8230, lasts: null, got: null, renews: null,
            rebinds: null, nameServers: [], timeServers: [], domain: null, hostname: null }
    })

    readonly property string table:
        "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n"
        + "enp0s1\t00000000\t0104A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0\n"

    // What Ethernet says now: its headline, its name servers and its
    // lease, as "headline | servers | lease".
    function said() {
        const ethernet = networks.answer("ethernet");
        const side = key => (ethernet?.rows ?? []).find(row => row.key === `networks:enp0s1:${key}`)
            ?.side ?? "";
        return `${ethernet?.headline ?? ""} | ${side("dns")} | ${side("lease")}`;
    }

    // What Ethernet said each time Network told the launcher it had
    // something new, asked again as the launcher would on the tell.
    property var told: []

    Connections {
        target: networks

        function onAnswered() {
            root.told.push(root.said());
        }
    }

    Until {
        id: waits
    }

    // Holds once `check` does, failing past `ms` as `what` never held,
    // with what was said, well inside the five seconds after which old
    // facts are loaded again anyway, so a load that lands is the one the
    // case set off.
    function within(ms, check) {
        const began = Date.now();
        return () => {
            if (Date.now() - began > ms)
                throw new Error(`${waits.what} never held in ${ms} ms: ${said()}; `
                    + `told ${JSON.stringify(told)}`);
            return check();
        };
    }

    Component.onCompleted: Qt.callLater(root.begun)

    // The card linked at 1 Gb/s, full duplex, its lease from networkd and
    // a name server in resolv.conf, the uptime not yet written: before its
    // files land, Ethernet says the link as the daemon told it, and its
    // drawer isn't made, so neither grows; it says it's still reading, so a
    // terminal's show asks again; and the lease says its server until the
    // time since boot lands.
    function begun() {
        try {
            written(carrier, "1\n");
            written(speed, "1000\n");
            written(duplex, "full\n");
            written(resolv, "nameserver 192.168.4.1\n");
            written(route, table);
            networks.leases = leases;
            networks.links = links;
            const first = said();
            Expect.expect("before its files land, a cable says its link as the daemon told it",
                first.startsWith("up | "), first);
            Expect.expect("before its files land, a cable says it's still reading",
                networks.stillReading, String(networks.stillReading));
            // Made now, rather than in the turn of their own they wait for.
            networks.makeGlances();
            Expect.expect("before its files land, a cable's drawer isn't made",
                networks.glances.enp0s1 === undefined, JSON.stringify(networks.glances));
            waits.until("the cable's files and the name servers land",
                within(2000, () => said().startsWith("1 Gb/s | 192.168.4.1 | ")), root.landed);
        } catch (error) {
            Expect.fail(error.message ?? error);
        }
    }

    function landed() {
        Expect.expect("once they land, it's read",
            !networks.stillReading, String(networks.stillReading));
        Expect.expect("before the time since boot lands, a lease says its server",
            said().endsWith(" | 192.168.4.1"), said());
        written(up, "1000.00 4000.00\n");
        networks.uptimeFile = `${root.dir}/uptime`;
        waits.until("the lease's time left from the uptime loaded",
            within(2000, () => said().endsWith(" | ends in 2 h")), root.glanced);
    }

    function glanced() {
        waits.until("the cable's drawer made whole once its files land", within(2000, () => {
            const facts = (networks.glances.enp0s1 ?? []).map(f => `${f.label}=${f.value}`);
            return facts.includes("Speed=1 Gb/s") && facts.includes("Duplex=full");
        }), root.changed);
    }

    // The card renegotiated to 100 Mb/s, the name server changed and the
    // machine up two hours on, the launcher shut: an answer straight after
    // says them as last loaded, since it reads nothing. Opened, they load
    // in the background, and once they land the launcher is told.
    function changed() {
        written(speed, "100\n");
        written(resolv, "nameserver 9.9.9.9\n");
        written(up, "8000.00 4000.00\n");
        const now = said();
        Expect.expect("an answer says the cable, name servers and uptime as last loaded",
            now === "1 Gb/s | 192.168.4.1 | ends in 2 h", now);
        told = [];
        shown.open = true;
        waits.until("the launcher told of the change once the opening's loads land",
            within(2000, () => told.includes("100 Mb/s | 9.9.9.9 | ends in 3 min")), root.opened);
    }

    // A cable shown, held as a step into it: its link loads again each
    // second, unasked, and the launcher is told when it changes.
    function opened() {
        shown.held = [{ row: { key: "networks:enp0s1" } }];
        told = [];
        written(speed, "2500\n");
        waits.until("while a cable's shown, its link loads again each second",
            within(2500, () => told.some(each => each.startsWith("2.5 Gb/s | "))), root.unplugged);
    }

    // The card pulled out: what its files said is forgotten.
    function unplugged() {
        shown.held = [];
        networks.links = [];
        Expect.expect("a card gone is forgotten",
            networks.cables.enp0s1 === undefined, JSON.stringify(networks.cables));
        Qt.exit(0);
    }
}

// Drives Ethernet's answer through Quickshell itself, over cards' folders,
// a resolv.conf and an uptime of the case's own beside it, to show an answer
// waits on none of their files. Docked with one card, a step into Network
// asked Ethernet, which read the cable's twenty files twice, resolv.conf
// and the uptime, each a wait of a median 6 to 14 ms and up to 133 on
// Joel's laptop under load (2026-11-17), of a step that has 50 ms for all
// of it; and read them all again each second while a cable was shown. So an
// answer says each as last loaded, and they're loaded again in the
// background as the launcher opens, as a terminal asks after a gap, and
// each second while a cable's shown: a change is said once that load lands,
// and not before. A second card's load is given up as the watchdog gives
// up one a busy driver holds up, since FileView refuses a pipe at once
// and nothing else in a sandbox hangs a read. It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect
import "Written.js" as Written

ShellRoot {
    id: root

    readonly property string dir: Quickshell.shellDir
    readonly property string card: `${dir}/sys/enp0s1`
    readonly property string stuck: `${dir}/sys/enp0s2`

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

    // A file the case writes, loaded by nothing of its own.
    component Case: FileView {
        blockWrites: true
        printErrors: false
        preload: false
    }

    Case {
        id: speed
        path: `${root.card}/speed`
    }
    Case {
        id: carrier
        path: `${root.card}/carrier`
    }
    Case {
        id: duplex
        path: `${root.card}/duplex`
    }
    Case {
        id: errorsIn
        path: `${root.card}/statistics/rx_errors`
    }
    Case {
        id: errorsOut
        path: `${root.card}/statistics/tx_errors`
    }
    Case {
        id: stuckCarrier
        path: `${root.stuck}/carrier`
    }
    Case {
        id: resolv
        path: `${root.dir}/etc/resolv.conf`
    }
    Case {
        id: up
        path: `${root.dir}/uptime`
    }
    Case {
        id: route
        path: `${root.dir}/net/route`
    }

    // A cable as the machine daemon would tell it (Interfaces.js), with an
    // address, the way out by its route.
    function cable(name, state, address) {
        return {
            name: name,
            kind: "wired",
            state: state,
            mtu: 1500,
            mac: "00:1b:21:aa:bb:cc",
            own: "",
            assigned: "own",
            four: [{ address: address, prefix: 22 }],
            six: []
        };
    }
    readonly property var docked: cable("enp0s1", "up", "192.168.4.52")
    readonly property var second: cable("enp0s2", "up", "192.168.4.53")

    // Its lease, ending 7,230 s after the uptime the case first writes.
    readonly property var leases: ({
        enp0s1: { server: "192.168.4.1", ends: 8230, lasts: null, got: null, renews: null,
            rebinds: null, nameServers: [], timeServers: [], domain: null, hostname: null }
    })

    readonly property string table:
        "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n"
        + "enp0s1\t00000000\t0104A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0\n"

    // What Ethernet says now: its headline, and its speed's, name servers'
    // and lease's rows, as "headline | speed | servers | lease".
    function said() {
        const ethernet = networks.answer("ethernet");
        const side = key => (ethernet?.rows ?? []).find(row => row.key === `networks:enp0s1:${key}`)
            ?.side ?? "";
        return `${ethernet?.headline ?? ""} | ${side("speed")} | ${side("dns")} | ${side("lease")}`;
    }

    // What the cable's errors a step in say came in broken, or "" with
    // none said.
    function brokenIn() {
        return networks.errorParts("enp0s1").find(row => row.key.endsWith(":errors-in"))?.side
            ?? "";
    }

    // What Ethernet said each time Network told the launcher it had
    // something new, asked again as the launcher would on the tell.
    property var told: []

    Connections {
        target: networks

        function onAnswered() {
            root.told.push(root.said());
        }

        function onCablesChanged() {
            if (networks.cables.enp0s1 !== undefined)
                root.kept.push(networks.cableStates.enp0s1);
        }
    }

    // The daemon's word each time the first card's cable was kept, as it
    // stood when its files began to load.
    property var kept: []

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

    // Whether nothing is loading for a cable.
    function swept() {
        return networks.cableLoading === null && networks.cablesAsked.length === 0;
    }

    Component.onCompleted: Qt.callLater(root.begun)

    // The card linked at 1 Gb/s, full duplex, three frames in broken, its
    // lease from networkd and a name server in resolv.conf, the uptime not
    // yet written: before its files land, Ethernet says the link as the
    // daemon told it, with no row of the cable's, and its drawer isn't
    // made, so none grows; it says it's still reading, so a terminal's
    // show asks again; and the lease says its server until the time since
    // boot lands.
    function begun() {
        try {
            Written.written(carrier, "1\n");
            Written.written(speed, "1000\n");
            Written.written(duplex, "full\n");
            Written.written(errorsIn, "3\n");
            Written.written(errorsOut, "0\n");
            Written.written(stuckCarrier, "1\n");
            Written.written(resolv, "nameserver 192.168.4.1\n");
            Written.written(route, table);
            networks.leases = leases;
            networks.links = [docked];
            const first = said();
            Expect.expect("before its files land, a cable says its link as the daemon told it",
                first.startsWith("up |  | "), first);
            Expect.expect("before its files land, a cable's errors aren't said",
                brokenIn() === "", brokenIn());
            Expect.expect("before its files land, a cable says it's still reading",
                networks.stillReading, String(networks.stillReading));
            // Made now, rather than in the turn of their own they wait for.
            networks.makeGlances();
            Expect.expect("before its files land, a cable's drawer isn't made",
                networks.glances.enp0s1 === undefined, JSON.stringify(networks.glances));
            waits.until("the cable's files and the name servers land",
                within(2000, () => said() === "1 Gb/s | 1 Gb/s | 192.168.4.1 | 192.168.4.1"),
                root.landed);
        } catch (error) {
            Expect.fail(error.message ?? error);
        }
    }

    function landed() {
        Expect.expect("once they land, it's read",
            !networks.stillReading, String(networks.stillReading));
        Expect.expect("once they land, a cable's errors are said", brokenIn() === "3",
            brokenIn());
        Written.written(up, "1000.00 4000.00\n");
        networks.uptimeFile = `${root.dir}/uptime`;
        waits.until("the lease's time left from the uptime loaded",
            within(2000, () => said().endsWith(" | ends in 2 h")), root.glanced);
    }

    // The drawer, made whole once the files land; and the uptime counted
    // on from when it was loaded, an hour back, by the clock.
    function glanced() {
        waits.until("the cable's drawer made whole once its files land", within(2000, () => {
            const facts = (networks.glances.enp0s1 ?? []).map(f => `${f.label}=${f.value}`);
            return facts.includes("Speed=1 Gb/s") && facts.includes("Duplex=full");
        }), () => {
            networks.upAt -= 3600 * 1000;
            Expect.expect("the uptime is counted on from its last load",
                said().endsWith(" | ends in 1 h"), said());
            root.changed();
        });
    }

    // The card renegotiated to 100 Mb/s with two frames more in broken,
    // the name server changed and the machine up two hours on, the launcher
    // shut: an answer straight after says them as last loaded, since it
    // reads nothing. Asked by a terminal after a gap, they load again, and
    // it's still reading until they've landed, so the terminal asks on
    // until it's told what they say now.
    function changed() {
        Written.written(speed, "100\n");
        Written.written(errorsIn, "5\n");
        Written.written(resolv, "nameserver 9.9.9.9\n");
        Written.written(up, "8000.00 4000.00\n");
        const now = said();
        Expect.expect("an answer says the cable, name servers and uptime as last loaded",
            now === "1 Gb/s | 1 Gb/s | 192.168.4.1 | ends in 1 h", now);
        Expect.expect("an answer says a cable's errors as last loaded", brokenIn() === "3",
            brokenIn());
        networks.lookedAt = 0;
        said();
        Expect.expect("asked after a gap, it's still reading until they land again",
            networks.stillReading, String(networks.stillReading));
        waits.until("a terminal asking on told what they say now", within(2000, () =>
            !networks.stillReading && said() === "100 Mb/s | 100 Mb/s | 9.9.9.9 | ends in 3 min"
                && brokenIn() === "5"), root.opening);
    }

    // The uptime stepped back by more than a minute, and nothing else
    // changed: opened, the launcher is told once its load lands, though
    // the uptime was counted on.
    function opening() {
        Written.written(up, "4600.00 4000.00\n");
        told = [];
        shown.open = true;
        waits.until("the launcher told of the uptime once the opening's load lands",
            within(2000, () => told.includes("100 Mb/s | 100 Mb/s | 9.9.9.9 | ends in 1 h")),
            root.steady);
    }

    // The uptime loaded again as it was counted on: nothing is told.
    function steady() {
        told = [];
        Written.written(up, `${Math.round(networks.uptime())}.00 4000.00\n`);
        networks.reloadFacts();
        waits.until("the facts loaded again", within(2000, () => networks.factsAsked.length === 0),
            () => {
            Expect.expect("an uptime load that agrees with the count tells nothing",
                told.length === 0, JSON.stringify(told));
            root.recabled();
        });
    }

    // Open, with no cable shown, so nothing loads it each second: once its
    // files load again and land, changed, the launcher is told.
    function recabled() {
        told = [];
        Written.written(speed, "2500\n");
        networks.recable();
        waits.until("the launcher told once a cable's load lands changed",
            within(2000, () => told.some(each => each.startsWith("2.5 Gb/s | "))), root.opened);
    }

    // A cable shown, held as a step into it: its link loads again each
    // second, unasked, and the launcher is told when it changes.
    function opened() {
        shown.held = [{ row: { key: "networks:enp0s1" } }];
        told = [];
        Written.written(speed, "1000\n");
        waits.until("while a cable's shown, its link loads again each second",
            within(2500, () => told.some(each => each.startsWith("1 Gb/s | "))), root.relinked);
    }

    // The daemon says the cable has lost its link as its files load,
    // newer than they are: it's said as the daemon says until they land
    // again, and the load begun before isn't kept.
    function relinked() {
        shown.held = [];
        kept = [];
        networks.recable();
        Expect.expect("the cable's load is under way as its link changes",
            networks.cableLoading?.name === "enp0s1", JSON.stringify(networks.cableLoading));
        networks.links = [Object.assign({}, docked, { state: "no link" })];
        Expect.expect("a cable the daemon says has changed is said as it says",
            said().startsWith("nothing at the other end | "), said());
        waits.until("the changed cable loaded again", within(2000, () => root.swept()
            && networks.cables.enp0s1 !== undefined), () => {
            Expect.expect("a load begun before its link changed isn't kept",
                kept.length > 0 && kept.every(state => state === "no link"),
                JSON.stringify(kept));
            root.held();
        });
    }

    // A second card plugged in, its load held up and given up: it's said
    // as the daemon tells it, the first card as loaded, and its drawer
    // made so; nothing is left reading, and asked again it's not loaded
    // again at once. Facts that never land are given up too.
    function held() {
        networks.links = [second, docked];
        networks.answer("ethernet");
        Expect.expect("a card plugged in between looks says it's still reading",
            networks.stillReading, String(networks.stillReading));
        Expect.expect("the second card's load is under way",
            networks.cableLoading?.name === "enp0s2", JSON.stringify(networks.cableLoading));
        networks.cableGivenUp();
        waits.until("the other card loaded", within(2000, root.swept), () => {
            const ethernet = networks.answer("ethernet");
            const side = name => (ethernet?.rows ?? []).find(row => row.key
                === `networks:${name}`)?.side ?? "";
            const both = `${ethernet?.headline} | ${side("enp0s1")} | ${side("enp0s2")}`;
            Expect.expect("a card held up is said as the daemon tells it, the other as loaded",
                both === "2 cards | 1 Gb/s | up", both);
            Expect.expect("a card held up isn't loaded", networks.cables.enp0s2 === undefined,
                JSON.stringify(networks.cables.enp0s2));
            Expect.expect("a card held up leaves nothing reading",
                !networks.stillReading, String(networks.stillReading));
            networks.reloadFacts();
            networks.factsGivenUp();
            Expect.expect("facts given up leave nothing reading",
                !networks.stillReading, String(networks.stillReading));
            networks.recable();
            Expect.expect("a card held up isn't loaded again at once",
                networks.cableLoading?.name !== "enp0s2"
                    && !networks.cablesAsked.includes("enp0s2"),
                JSON.stringify([networks.cableLoading, networks.cablesAsked]));
            waits.until("a card held up has its drawer made as the daemon says it",
                within(2000, () => networks.glances.enp0s2 !== undefined), root.retried);
        });
    }

    // A minute on, the card held up is asked again; held up again, it
    // waits twice as long.
    function retried() {
        const first = networks.cablesStuck.enp0s2;
        Expect.expect("a card held up waits a minute", first?.wait === 60000,
            JSON.stringify(first));
        first.at -= 61000;
        networks.recable();
        Expect.expect("a card held up is asked again once its wait is over",
            networks.cableLoading?.name === "enp0s2", JSON.stringify(networks.cableLoading));
        networks.cableGivenUp();
        Expect.expect("a card held up again waits twice as long",
            networks.cablesStuck.enp0s2?.wait === 120000,
            JSON.stringify(networks.cablesStuck.enp0s2));
        waits.until("the other card loaded again", within(2000, root.swept), root.requeued);
    }

    // Its wait over, the card's link changes as it loads, so it's asked
    // again, and then it's held up: it isn't loaded again at once,
    // though it was waiting its turn.
    function requeued() {
        networks.cablesStuck.enp0s2.at -= 121000;
        networks.recable();
        networks.links = [Object.assign({}, second, { state: "no link" }), docked];
        Expect.expect("a card changed as it loads is asked again",
            networks.cablesAsked.includes("enp0s2"), JSON.stringify(networks.cablesAsked));
        networks.cableGivenUp();
        Expect.expect("a card held up as it waits its turn isn't loaded again at once",
            networks.cableLoading?.name !== "enp0s2"
                && !networks.cablesAsked.includes("enp0s2"),
            JSON.stringify([networks.cableLoading, networks.cablesAsked]));
        waits.until("the other card loaded once more", within(2000, root.swept),
            root.recovered);
    }

    // Its link back as it was and its wait over, the card held up is
    // loaded once its files land, and held up no more.
    function recovered() {
        networks.links = [second, docked];
        networks.cablesStuck.enp0s2.at -= 61000;
        networks.recable();
        waits.until("a card held up is loaded once asked after its wait", within(2000, () =>
            root.swept() && networks.cables.enp0s2 !== undefined
                && networks.cablesStuck.enp0s2 === undefined), root.unplugged);
    }

    // The cards pulled out as the first's files load again: what they
    // said is forgotten, and the load under way isn't kept once it lands.
    function unplugged() {
        networks.links = [docked];
        networks.links = [];
        waits.until("the load under way as the cards went landed", within(2000, root.swept),
            () => {
            Expect.expect("a card gone is forgotten, though its load was under way",
                networks.cables.enp0s1 === undefined, JSON.stringify(networks.cables));
            Qt.exit(0);
        });
    }
}

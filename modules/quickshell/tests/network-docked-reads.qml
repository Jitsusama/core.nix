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
// holds, and otherwise 1, saying which didn't. Long because its steps are
// one chain, each beginning from the cards, files and holds the one before
// left, which a split would have to build again in every part.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/Interfaces.js" as Interfaces
import "launcher/Networkd.js" as Networkd
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
    // Whether held has begun giving loads up on purpose.
    property bool holdsMade: false

    Connections {
        target: networks

        function onAnswered() {
            root.told.push(root.said());
        }

        function onCablesChanged() {
            if (networks.cables.enp0s1 !== undefined)
                root.kept.push(networks.cableStates.enp0s1);
        }

        // Watched as it changes, not polled, since a fact landing before
        // the rest lasts a turn: a terminal asking on stops once it ends.
        function onStillReadingChanged() {
            const loading = { facts: networks.factsAsked.length,
                cable: networks.cableLoading?.name ?? null, asked: networks.cablesAsked };
            if (!networks.stillReading && (loading.facts > 0 || loading.cable !== null
                    || loading.asked.length > 0))
                Expect.fail(
                    `still reading ended with a load under way: ${JSON.stringify(loading)}`);
        }
    }

    // The daemon's word each time the first card's cable was kept, as it
    // stood when its files began to load.
    property var kept: []

    // A fact whose load never lands, since no file in a sandbox hangs a
    // read, asked `ticksAgo` of the facts' watch's ticks back.
    QtObject {
        id: hungFact

        property string path: "a file that hangs"
    }
    function askHung(ticksAgo) {
        const since = new Map(networks.factsSince);
        since.set(hungFact, networks.factsTicks - ticksAgo);
        networks.factsSince = since;
        networks.factsAsked = networks.factsAsked.concat([hungFact]);
        networks.stillReading = true;
    }

    // Every fact under way taken as asked `ticksAgo` ticks back, eight
    // being two seconds.
    function askedAt(ticksAgo) {
        const since = new Map();
        for (const view of networks.factsSince.keys())
            since.set(view, networks.factsTicks - ticksAgo);
        networks.factsSince = since;
    }

    // Every fact's tick, as it stands.
    function factTicks() {
        return JSON.stringify([...networks.factsSince.values()]);
    }

    // Every fact's hold, as it stands.
    function factHolds() {
        return Object.values(networks.factsStuck);
    }

    Until {
        id: waits
    }

    // Holds once `check` does, failing past `ms` as `what` never held,
    // with what was said and any load given up since it began, since one
    // given up inside a step's own wait fails that wait rather than the
    // step before. A give-up makes a hold anew, so a hold still the one
    // there as the wait began, put by the case on purpose or stamped by
    // it, is left out. `check` is tried first, so a poll that comes late can't fail a wait
    // that holds. A wait on a load gets four seconds a card, for a loaded
    // builder. Only a wait on a give-up, which no load brings, runs past
    // four.
    function within(ms, check) {
        const began = Date.now();
        const facts = Object.assign(Object.create(null), networks.factsStuck);
        const cables = Object.assign(Object.create(null), networks.cablesStuck);
        const since = (stuck, before) => Object.keys(stuck)
            .filter(name => stuck[name] !== before[name]);
        return () => {
            if (check())
                return true;
            if (Date.now() - began > ms)
                throw new Error(`${waits.what} never held in ${ms} ms: ${said()}; `
                    + `told ${JSON.stringify(told)}; given up `
                    + JSON.stringify(since(networks.factsStuck, facts)
                        .concat(since(networks.cablesStuck, cables))));
            return false;
        };
    }

    // Whether nothing is loading for a cable.
    function swept() {
        return networks.cableLoading === null && networks.cablesAsked.length === 0;
    }

    // Until held gives loads up on purpose, one given up on a slow builder
    // is named at the step it came before, by each quiet wait, relinked
    // and held itself, rather than by the step its hold would fail. One
    // given up from held on isn't named: it may fail a later step, or none.
    function noneGivenUp(name) {
        if (!root.holdsMade)
            Expect.expect(`no load before ${name} was given up`,
                Object.keys(networks.factsStuck).length === 0
                && Object.keys(networks.cablesStuck).length === 0,
                JSON.stringify([networks.factsStuck, networks.cablesStuck]));
    }

    // `step`, once every fact and cable load under way has landed, with a look counted
    // from then. An ask five seconds past the last look loads everything
    // again, and a load skips what's already under way, so a step that
    // writes a file and asks begins quiet: no load asked before its write
    // can answer for it, and no ask in its first five seconds looks again.
    function quiet(name, step) {
        return () => waits.until(`the loads before ${name} landed`,
            within(4000, () => swept() && networks.factsAsked.length === 0), () => {
            root.noneGivenUp(name);
            networks.lookedAt = Date.now();
            step();
        });
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
                within(4000, () => said() === "1 Gb/s | 1 Gb/s | 192.168.4.1 | 192.168.4.1"),
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
            within(4000, () => said().endsWith(" | ends in 2 h")), root.glanced);
    }

    // The drawer, made whole once the files land; and the uptime counted
    // on from when it was loaded, an hour back, by the clock.
    function glanced() {
        waits.until("the cable's drawer made whole once its files land", within(4000, () => {
            const facts = (networks.glances.enp0s1 ?? []).map(f => `${f.label}=${f.value}`);
            return facts.includes("Speed=1 Gb/s") && facts.includes("Duplex=full");
        }), root.quiet("the uptime's count", () => {
            // Quiet first, so no uptime load asked before lands on the hour.
            networks.upAt -= 3600 * 1000;
            Expect.expect("the uptime is counted on from its last load",
                said().endsWith(" | ends in 1 h"), said());
            root.changed();
        }));
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
        waits.until("a terminal asking on told what they say now", within(4000, () =>
            !networks.stillReading && said() === "100 Mb/s | 100 Mb/s | 9.9.9.9 | ends in 3 min"
                && brokenIn() === "5"), root.quiet("opening", root.opening));
    }

    // The uptime stepped back by more than a minute, and nothing else
    // changed: opened, the launcher is told once its load lands, though
    // the uptime was counted on.
    function opening() {
        Written.written(up, "4600.00 4000.00\n");
        told = [];
        shown.open = true;
        waits.until("the launcher told of the uptime once the opening's load lands",
            within(4000, () => told.includes("100 Mb/s | 100 Mb/s | 9.9.9.9 | ends in 1 h")),
            root.quiet("steady", root.steady));
    }

    // The uptime loaded again as it was counted on: nothing is told.
    function steady() {
        told = [];
        // Two minutes counted on from the last load, so a load that agrees
        // with the count is two minutes past it, as one checked against the
        // last load alone would tell.
        networks.upAt -= 120000;
        Written.written(up, `${Math.round(networks.uptime())}.00 4000.00\n`);
        const before = networks.upAt;
        networks.reloadFacts();
        // The uptime's stamp moves only as its load lands, never as it's
        // given up.
        waits.until("the facts loaded again", within(4000, () =>
            networks.factsAsked.length === 0 && networks.upAt > before),
            () => {
            Expect.expect("an uptime load that agrees with the count tells nothing",
                told.length === 0, JSON.stringify(told));
            root.quiet("ahead", root.ahead)();
        });
    }

    // The uptime loaded about two minutes ahead of the count, as a clock
    // stepped back leaves it, the launcher open: it's told once the load
    // lands, as it is of one behind. The uptime written leaves the lease's
    // end half a minute past a whole minute, 90 to 150 s ahead of the
    // count, so the time left it's told with reads the same for thirty
    // seconds, however long the steps before took.
    function ahead() {
        told = [];
        const minutes = Math.floor((8230 - Math.round(networks.uptime()) - 120) / 60);
        Written.written(up, `${8230 - minutes * 60 - 30}.00 4000.00\n`);
        networks.reloadFacts();
        const row = `100 Mb/s | 100 Mb/s | 9.9.9.9 | ends in ${minutes} min`;
        waits.until("the launcher told of an uptime ahead of the count",
            within(4000, () => told.includes(row)), root.quiet("served", root.served));
    }

    // A name server changed: shut, once its file lands again, the drawer
    // is made again; open, the launcher is told too.
    function served() {
        shown.open = false;
        const shut = Date.now();
        // A drawer's make asked before is over a second on, so only the
        // name server's own can make it again.
        waits.until("a drawer's make asked before is over",
            within(2500, () => Date.now() - shut >= 1500), () => {
            Written.written(resolv, "nameserver 1.1.1.1\n");
            networks.reloadFacts();
            waits.until("shut, a name server changed makes its drawer again",
                within(4000, () =>
                    JSON.stringify(networks.glances.enp0s1).includes("1.1.1.1")),
                root.quiet("toldServers", root.toldServers));
        });
    }
    function toldServers() {
        told = [];
        Written.written(resolv, "nameserver 8.8.8.8\n");
        shown.open = true;
        networks.reloadFacts();
        waits.until("open, the launcher is told of a name server changed",
            within(4000, () => told.some(each => each.includes(" | 8.8.8.8 | "))),
            root.quiet("recabled", root.recabled));
    }

    // Open, with no cable shown, so nothing loads it each second: once its
    // files load again and land, changed, the launcher is told.
    function recabled() {
        told = [];
        Written.written(speed, "2500\n");
        networks.recable();
        waits.until("the launcher told once a cable's load lands changed",
            within(4000, () => told.some(each => each.startsWith("2.5 Gb/s | "))),
            root.quiet("opened", root.opened));
    }

    // A cable shown, held as a step into it: its link loads again each
    // second, unasked, so the launcher hears a change a second and a sweep
    // on. Begun quiet, so the load it waits on is the tick's alone. The tick
    // tells the launcher each second whatever it loads, so that a load
    // tells it is recabled's to show.
    function opened() {
        shown.held = [{ row: { key: "networks:enp0s1" } }];
        told = [];
        Written.written(speed, "1000\n");
        waits.until("while a cable's shown, its link loads again each second",
            within(5000, () => told.some(each => each.startsWith("1 Gb/s | "))), root.relinked);
    }

    // The daemon says the cable has lost its link as its files load,
    // newer than they are: it's said as the daemon says until they land
    // again, and the load begun before isn't kept.
    function relinked() {
        root.noneGivenUp("relinked");
        shown.held = [];
        kept = [];
        networks.recable();
        Expect.expect("the cable's load is under way as its link changes",
            networks.cableLoading?.name === "enp0s1", JSON.stringify(networks.cableLoading));
        networks.links = [Object.assign({}, docked, { state: "no link" })];
        Expect.expect("a cable the daemon says has changed is said as it says",
            said().startsWith("nothing at the other end | "), said());
        // The load let go still reads its files before the next is asked,
        // so this is two sweeps.
        waits.until("the changed cable loaded again", within(8000, () => root.swept()
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
        root.noneGivenUp("held");
        root.holdsMade = true;
        networks.links = [second, docked];
        networks.answer("ethernet");
        Expect.expect("a card plugged in between looks says it's still reading",
            networks.stillReading, String(networks.stillReading));
        Expect.expect("the second card's load is under way",
            networks.cableLoading?.name === "enp0s2", JSON.stringify(networks.cableLoading));
        networks.cableFile();
        const reading = networks.cableProbe;
        networks.cableGivenUp();
        Expect.expect("a card given up keeps the view still reading its file",
            networks.cablesHung.enp0s2 === reading && networks.cableProbe !== reading,
            String(networks.cablesHung.enp0s2));
        waits.until("the other card loaded, and the read given up landed", within(4000, () =>
            root.swept() && networks.cablesHung.enp0s2 === undefined), () => {
            Expect.expect("a read given up landing late keeps its card's hold",
                networks.cablesStuck.enp0s2 !== undefined, JSON.stringify(networks.cablesStuck));
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
            root.factsHeld();
            networks.recable();
            Expect.expect("a card held up isn't loaded again at once",
                networks.cableLoading?.name !== "enp0s2"
                    && !networks.cablesAsked.includes("enp0s2"),
                JSON.stringify([networks.cableLoading, networks.cablesAsked]));
            waits.until("a card held up has its drawer made as the daemon says it",
                within(4000, () => networks.glances.enp0s2 !== undefined), root.watched);
        });
    }

    // The facts asked, and asked again while they load, then given up as
    // they would be two seconds on, before any lands: they're held off a
    // minute, asked again once it's over, and held twice as long if given
    // up again.
    function factsHeld() {
        networks.reloadFacts();
        const asked = networks.factsAsked.length;
        Expect.expect("a fact is asked on the watch's tick", asked > 0
            && networks.factsAsked.every(view =>
                networks.factsSince.get(view) === networks.factsTicks),
            root.factTicks());
        root.askedAt(5);
        networks.reloadFacts();
        Expect.expect("a fact already loading isn't asked again",
            networks.factsAsked.length === asked && networks.factsAsked.every(view =>
                networks.factsSince.get(view) === networks.factsTicks - 5),
            root.factTicks());
        networks.factsTicked();
        networks.factsTicked();
        Expect.expect("a fact loading seven ticks isn't given up",
            networks.factsAsked.length === asked, JSON.stringify(networks.factsStuck));
        networks.factsTicked();
        Expect.expect("facts given up leave nothing reading",
            !networks.stillReading, String(networks.stillReading));
        Expect.expect("a fact held up waits a minute", root.factHolds().length === asked
            && root.factHolds().every(held => held.wait === 60000),
            JSON.stringify(networks.factsStuck));
        networks.reloadFacts();
        Expect.expect("a fact held up isn't asked again at once",
            networks.factsAsked.length === 0 && !networks.stillReading,
            JSON.stringify(networks.factsAsked.map(view => String(view.path))));
        for (const held of root.factHolds())
            held.at -= 61000;
        networks.reloadFacts();
        Expect.expect("a fact held up is asked again once its wait is over",
            networks.factsAsked.length === asked,
            JSON.stringify(networks.factsAsked.map(view => String(view.path))));
        root.askedAt(8);
        networks.factsGivenUp();
        Expect.expect("a fact held up again waits twice as long",
            root.factHolds().every(held => held.wait === 120000),
            JSON.stringify(networks.factsStuck));
        root.askHung(8);
        networks.factsGivenUp();
        networks.factLanded(hungFact);
        Expect.expect("a fact landing after it's given up stays held",
            networks.factsStuck[hungFact.path] !== undefined, JSON.stringify(networks.factsStuck));
        root.askHung(0);
        networks.factLanded(hungFact);
        Expect.expect("a fact landing in time is let go",
            networks.factsStuck[hungFact.path] === undefined, JSON.stringify(networks.factsStuck));
    }

    // Facts held up are asked again once they land; one two seconds past
    // its own ask is given up alone, beside facts just asked; and one
    // that never lands is given up by the watch, leaving nothing reading.
    function watched() {
        for (const held of root.factHolds())
            held.at -= 121000;
        networks.reloadFacts();
        Expect.expect("facts held up are asked again once their wait is over",
            networks.factsAsked.length > 0,
            JSON.stringify(networks.factsAsked.map(view => String(view.path))));
        const fresh = networks.factsAsked.slice();
        root.askHung(8);
        networks.factsGivenUp();
        Expect.expect("a fact two seconds past its own ask is given up alone",
            networks.factsStuck[hungFact.path] !== undefined
                && fresh.every(view => networks.factsAsked.includes(view)),
            JSON.stringify(networks.factsStuck));
        waits.until("the facts asked land",
            within(4000, () => networks.factsAsked.length === 0), () => {
            Expect.expect("facts asked again, landing in time, are let go",
                Object.keys(networks.factsStuck).join() === hungFact.path,
                JSON.stringify(networks.factsStuck));
            networks.factsStuck = Object.create(null);
            root.askHung(0);
            const hung = Date.now();
            waits.until("a second and a half on",
                within(2500, () => Date.now() - hung >= 1500), () => {
                // Qt's timers only ever fire late, so a fact given up this
                // soon was given up early, whatever the machine's load.
                Expect.expect("the watch keeps a fact a second and a half",
                    networks.factsAsked.includes(hungFact)
                        && networks.factsStuck[hungFact.path] === undefined,
                    JSON.stringify(networks.factsStuck));
                waits.until("the watch gives up a fact that never lands",
                    within(9000, () => networks.factsStuck[hungFact.path] !== undefined
                        && !networks.stillReading), () => {
                    networks.factsStuck = Object.create(null);
                    root.quiet("retried", root.retried)();
                });
            });
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
        waits.until("the other card loaded again", within(4000, root.swept), root.requeued);
    }

    // Its wait over, long held up, the card's link changes as it loads,
    // so it's asked again, and then it's held up: it isn't loaded again
    // at once, though it was waiting its turn, and it waits an hour, no
    // longer.
    function requeued() {
        networks.cablesStuck.enp0s2.wait = 2400000;
        networks.cablesStuck.enp0s2.at -= 2401000;
        networks.recable();
        networks.links = [Object.assign({}, second, { state: "no link" }), docked];
        Expect.expect("a card changed as it loads is asked again",
            networks.cablesAsked.includes("enp0s2"), JSON.stringify(networks.cablesAsked));
        networks.cableGivenUp();
        Expect.expect("a card held up as it waits its turn isn't loaded again at once",
            networks.cableLoading?.name !== "enp0s2"
                && !networks.cablesAsked.includes("enp0s2"),
            JSON.stringify([networks.cableLoading, networks.cablesAsked]));
        Expect.expect("a card held up waits an hour at most",
            networks.cablesStuck.enp0s2?.wait === 3600000,
            JSON.stringify(networks.cablesStuck.enp0s2));
        waits.until("the other card loaded once more", within(4000, root.swept),
            root.recovered);
    }

    // Its link back as it was, the card held up stays held, so a link
    // that flaps takes no more reads of a hung file; the clock stepped
    // back, it's asked again, loaded once its files land, and held up no
    // more.
    function recovered() {
        networks.links = [second, docked];
        Expect.expect("a card held up stays held though its link changes",
            networks.cableLoading?.name !== "enp0s2"
                && !networks.cablesAsked.includes("enp0s2"),
            JSON.stringify([networks.cableLoading, networks.cablesAsked]));
        networks.cablesStuck.enp0s2.at = Date.now() + 3600000;
        networks.recable();
        waits.until("a card held up is loaded once the clock steps back", within(8000, () =>
            root.swept() && networks.cables.enp0s2 !== undefined
                && networks.cablesStuck.enp0s2 === undefined), root.replugged);
    }

    // Held up once more, then unplugged and plugged in again, as a hung
    // driver's card may come and go: it stays held, so it takes no more
    // threads.
    function replugged() {
        networks.recable();
        Expect.expect("the second card's load is under way again",
            networks.cableLoading?.name === "enp0s2", JSON.stringify(networks.cableLoading));
        networks.cableGivenUp();
        networks.links = [docked];
        networks.links = [second, docked];
        networks.recable();
        waits.until("the other card loaded after the replug", within(4000, root.swept), () => {
            Expect.expect("a card held up stays held though it's plugged in again",
                networks.cables.enp0s2 === undefined && networks.cablesStuck.enp0s2 !== undefined,
                JSON.stringify([networks.cables.enp0s2, networks.cablesStuck.enp0s2]));
            root.unplugged();
        });
    }

    // Facts asked again are still reading until they land, though no card
    // is asked with them.
    function factsAlone() {
        networks.links = [];
        networks.cableLoading = null;
        networks.cablesAsked = [];
        networks.factsAsked = [];
        networks.factsStuck = Object.create(null);
        networks.stillReading = false;
        networks.reloadFacts();
        Expect.expect("facts asked with no card's cable are still reading",
            networks.stillReading && networks.factsAsked.length === 3,
            JSON.stringify(networks.factsAsked.length));
    }

    // A card asked again is still reading until it lands, though no fact
    // is asked with it.
    function recabledAlone() {
        networks.links = [docked];
        networks.cableLoading = null;
        networks.cablesAsked = [];
        networks.factsAsked = [];
        networks.stillReading = false;
        networks.recable();
        // The loads factsAlone and the links' change began land as orphans,
        // which cableFile's inFlight and factLanded's late landing absorb.
        Expect.expect("a card asked alone is still reading", networks.stillReading
            && networks.factsAsked.length === 0,
            JSON.stringify([networks.stillReading, networks.factsAsked.length,
                networks.cableLoading?.name]));
    }

    // A card with no cable kept, its hold over, is asked for by the answer
    // itself, though nothing else changed and the last look is fresh.
    function askedByAnswer() {
        const kept = Object.assign(Object.create(null), networks.cables);
        delete kept.enp0s1;
        networks.cables = kept;
        const stuck = Object.assign(Object.create(null), networks.cablesStuck);
        stuck.enp0s1 = { at: Date.now() - 120000, wait: 60000 };
        networks.cablesStuck = stuck;
        networks.lookedAt = Date.now();
        networks.answer("ethernet");
        Expect.expect("a card whose hold is over is asked for by the answer",
            networks.stillReading && (networks.cableLoading?.name === "enp0s1"
                || networks.cablesAsked.includes("enp0s1")),
            JSON.stringify([networks.cableLoading, networks.cablesAsked]));
        waits.until("the card asked for loaded", within(4000, root.swept), () => Qt.exit(0));
    }

    // A card named as one of an object's own members has a lease and
    // neighbours of its own, and a card named as another has none.
    function oddlyNamed() {
        const leases = Networkd.leases({ Interfaces: [
            { Name: "__proto__", DHCPv4Client: { Lease: { LeaseTimestampUSec: 1000000 } } },
        ] });
        Expect.expect("a card named __proto__ has its own lease",
            leases.__proto__?.got === 1 && leases.toString === undefined,
            JSON.stringify(Object.keys(leases)));
        const heard = Networkd.neighbours({ Neighbors: [
            { InterfaceName: "__proto__", Neighbors: [] },
        ] });
        Expect.expect("a card named __proto__ has its own neighbours",
            Array.isArray(heard.__proto__) && heard.constructor === undefined,
            JSON.stringify(Object.keys(heard)));
        Expect.expect("a card named constructor has no gateway of an object's",
            Interfaces.gateways("").constructor === undefined
                && Interfaces.gateways6("").constructor === undefined,
            "a gateway");
    }

    // The probe is never set to the path it holds, which would load
    // nothing, since no two asks in a row share a path: a card's files
    // are each named once, and a card starts again only past its last.
    function filesNamedOnce() {
        Expect.expect("Interfaces.cable names each file once, and more than one",
            new Set(networks.cableFiles).size === networks.cableFiles.length
                && networks.cableFiles.length > 1, JSON.stringify(networks.cableFiles));
    }

    // The cards pulled out as the first's files load again: what they
    // said is forgotten, and the load under way isn't kept once it lands.
    function unplugged() {
        networks.links = [docked];
        networks.links = [];
        waits.until("the load under way as the cards went landed", within(4000, root.swept),
            () => {
            Expect.expect("a card gone is forgotten, though its load was under way",
                networks.cables.enp0s1 === undefined, JSON.stringify(networks.cables));
            root.watchedCable();
        });
    }

    // Whether hungReads' view, its read landed late, is gone.
    property bool hungGone: false

    // A file in flight that never lands is given up by the cable's own
    // watch, about two seconds on, its view kept and a new one made: its
    // hold is stamped as it's given up, so a loop held up late can't make
    // the watch look early, and a watch past three and a half seconds is
    // too slow, leaving a builder held up a second and a half. The sandbox
    // can't hang a read, so the view stands idle as one hung would.
    function watchedCable() {
        const reading = networks.cableProbe;
        networks.cableLoading = { name: "enp0s8", at: 0, texts: {}, state: "", stale: true,
            inFlight: true };
        networks.cableWatching = true;
        const began = Date.now();
        waits.until("the watch gives up a cable file that never lands",
            within(6000, () => networks.cablesStuck.enp0s8 !== undefined), () => {
            const after = networks.cablesStuck.enp0s8.at - began;
            Expect.expect("the watch gives up a cable file about two seconds on",
                after >= 1500 && after <= 3500, `${after} ms`);
            Expect.expect("the watch keeps the view in flight and makes another",
                networks.cablesHung.enp0s8 === reading && networks.cableProbe !== reading,
                JSON.stringify(Object.keys(networks.cablesHung)));
            // Standing idle, the view would never land; it goes, as if it had.
            networks.probeLanded(reading, "");
            root.hungReads();
            root.oddlyNamed();
            root.filesNamedOnce();
            root.factsAlone();
            root.recabledAlone();
            waits.until("a hung read's view, landed, is destroyed", within(1000, () =>
                root.hungGone), () => waits.until("the card asked alone loaded",
                within(4000, root.swept), root.askedByAnswer));
        });
    }

    // A card whose read given up still hangs isn't asked again, though its
    // wait is over; once that read lands, late, its view goes, its hold
    // stays, and the card under way isn't credited with it. A file asked
    // is watched, asked once, and its watch stops as it lands.
    function hungReads() {
        const reading = Qt.createQmlObject("import QtQuick; QtObject {"
            + " Component.onDestruction: root.hungGone = true }", root);
        const hung = Object.create(null);
        hung.enp0s9 = reading;
        networks.cablesHung = hung;
        const stuck = Object.assign(Object.create(null), networks.cablesStuck);
        stuck.enp0s9 = { at: Date.now() - 120000, wait: 60000 };
        networks.cablesStuck = stuck;
        Expect.expect("a card whose read hangs isn't asked again, though its wait is over",
            networks.stuck("enp0s9"), JSON.stringify(networks.cablesStuck.enp0s9));
        networks.cableLoading = { name: "enp0s1", at: 0, texts: {}, state: "", stale: true,
            inFlight: true };
        networks.probeLanded(reading, "1");
        Expect.expect("a hung read landing lets its view go, keeps its hold, and isn't taken"
            + " for the file under way", networks.cablesHung.enp0s9 === undefined
            && networks.cablesStuck.enp0s9 !== undefined && networks.cableLoading.at === 0,
            JSON.stringify(networks.cableLoading));
        Expect.expect("a card whose hung read has landed is asked again once its wait is over",
            !networks.stuck("enp0s9"), JSON.stringify(networks.cablesStuck.enp0s9));
        networks.cableLoading = { name: "enp0s1", at: 0, texts: {}, state: "", stale: true,
            inFlight: false };
        networks.cableFile();
        const asked = networks.cableProbe.path;
        Expect.expect("a cable file asked is watched",
            networks.cableWatching && networks.cableLoading.inFlight, asked);
        networks.cableProbe.path = "";
        networks.cableFile();
        Expect.expect("a cable file in flight isn't asked again",
            networks.cableProbe.path === "", String(networks.cableProbe.path));
        networks.cableLanded("1");
        Expect.expect("a cable file landed stops its watch, the next asked a turn on",
            !networks.cableWatching && networks.cableLoading.at === 1,
            JSON.stringify(networks.cableLoading));
    }
}

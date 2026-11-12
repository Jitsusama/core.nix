// Drives the launcher's Network answers through Quickshell itself, so every
// row Address and Traffic show, and each answer as a row of its own, has a
// drawer saying more than its line (docs/design.md, Every Row Says a
// Little More), its eyebrow saying where and what: Address over two
// stand-in connections, as the machine daemon would tell them, with no
// lookups, and Traffic over the sandbox's own /proc/net/dev, two reads of
// it sparked; Ethernet holding its place with no card; and Wi-Fi's own
// row's drawer (WirelessPlate.js). A row is drawn a drawer the way the
// launcher draws one (Launcher.qml's detailed): a thing, or a figure with
// a history. It exits 0 once every case holds, and otherwise 1, saying
// which case didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "launcher/WirelessPlate.js" as Plate
import "Expect.js" as Expect

ShellRoot {
    id: root

    Launcher.Networks {
        id: networks

        settings: ({ lookups: false })
    }

    Launcher.Traffic {
        id: traffic
    }

    // Two connections as the machine daemon tells them, made sense of
    // (Interfaces.js): Wi-Fi with an address, and a cable that's up
    // without one.
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
        mtu: 9000,
        mac: "00:1b:21:aa:bb:cc",
        own: "",
        assigned: "own",
        four: [],
        six: []
    }]

    // Once the providers are made, so their bindings hold the stand-ins.
    Component.onCompleted: Qt.callLater(root.asked)

    // Address's drawers asked for once, as the launcher's binding asks,
    // and read the turn after, once their facts are made (keptGlance).
    function asked() {
        networks.links = links;
        const address = networks.answer("address");
        [address].concat(address?.rows ?? []).forEach(drawerOf);
        Qt.callLater(root.held);
    }

    function held() {
        try {
            run();
        } catch (error) {
            Expect.fail(error.message ?? error);
            return;
        }
        Qt.callLater(Qt.exit, 0);
    }

    // A row's drawer as the launcher would draw it, or null where it draws
    // none: a thing's or a sparked figure's, from its own maker.
    function drawerOf(row) {
        const drawn = row.thing === true || row.graph?.shape === "spark";
        return drawn && typeof row.drawer === "function" ? row.drawer() : null;
    }

    // Each fact as "Label=value", to compare and to say.
    function facts(drawer) {
        return (drawer?.facts ?? []).map(fact => `${fact.label}=${fact.value}`);
    }

    // Every row has a drawer with facts, none of them its line again, each
    // named by the vocabulary rather than by an id.
    function sayMore(where, rows) {
        for (const row of rows) {
            const drawer = drawerOf(row);
            const said = facts(drawer);
            Expect.expect(`${where}'s ${row.name} has a drawer`, drawer !== null && said.length > 0,
                JSON.stringify(drawer));
            Expect.expect(`${where}'s ${row.name} says more than its line`,
                drawer.facts.every(fact => fact.value !== row.side && fact.value !== ""),
                `${said.join("; ")} beside ${row.side}`);
            Expect.expect(`${where}'s ${row.name} names its facts`,
                drawer.facts.every(fact => fact.label !== "" && !String(fact.label).includes(":")),
                said.join("; "));
        }
    }

    function run() {
        const address = networks.answer("address");
        Expect.expect("address answers", address !== null && address.rows.length === 2,
            JSON.stringify(address));
        const way = drawerOf(address);
        Expect.expect("the address's drawer names the connection it's on",
            way !== null && way.title === "Wi-Fi", JSON.stringify(way));
        Expect.expect("the address's drawer says it's the machine's way out",
            way.eyebrow === "on this machine \u00b7 way out", way.eyebrow);
        Expect.expect("a connection's drawer says it's the machine's connection",
            drawerOf(address.rows[0])?.eyebrow === "on this machine \u00b7 connection",
            JSON.stringify(drawerOf(address.rows[0])));
        Expect.expect("the address's drawer says it whole, then the packet size, state and hardware",
            facts(way).join("; ") === "Address=192.168.4.37/22; MTU=1,500 bytes; State=up; "
                + "Hardware Address=3c:22:fb:12:34:56", facts(way).join("; "));
        Expect.expect("the hardware address identifies it",
            way.facts.filter(fact => fact.identifies).map(fact => fact.label).join(", ")
                === "Hardware Address", JSON.stringify(way.facts));
        sayMore("Address", address.rows);
        // The sandbox has no such card, so its link reads as turned off,
        // as a cable's carrier is read as it is now, not as the daemon said.
        const cable = drawerOf(address.rows[1]);
        Expect.expect("a cable's drawer says its packet size, its link now and its hardware",
            facts(cable).join("; ") === "MTU=9,000 bytes; State=turned off; "
                + "Hardware Address=00:1b:21:aa:bb:cc", facts(cable).join("; "));

        // With no Ethernet card, as once its adapter is pulled out, its
        // row holds its place saying so, never gone, so a show finds it.
        networks.links = links.filter(link => link.kind !== "wired");
        const unplugged = networks.answer("ethernet");
        Expect.expect("with no card, Ethernet holds its place saying it's not plugged in",
            unplugged !== null && unplugged.parted === true && unplugged.gone !== true
                && unplugged.note === "not plugged in" && unplugged.headline === "",
            JSON.stringify(unplugged));
        networks.links = links;

        // Two reads a second apart, sparked, as the traffic shown each
        // second keeps them.
        Expect.expect("the machine's own traffic is named Traffic",
            Words.name("traffic") === "Traffic", Words.name("traffic"));
        traffic.series = { moved: [1000, 3000], in: [1000, 2000], out: [0, 1000],
            at: [0, 1000] };
        const speed = traffic.answer("speed");
        Expect.expect("speed answers", speed !== null && speed.rows.length >= 3, JSON.stringify(speed));
        sayMore("Traffic", [speed].map(said => Object.assign({}, said, { name: "Traffic",
            side: said.headline })).concat(speed.rows));
        Expect.expect("Traffic's drawers say they're the machine's network card's",
            drawerOf(speed.rows[0])?.eyebrow === "on this machine \u00b7 network card",
            JSON.stringify(drawerOf(speed.rows[0])));
        const inward = facts(drawerOf(speed.rows[0]));
        Expect.expect("In's drawer says its range, its reads and what came in since startup",
            inward.join("; ") === "Range=8 kb/s to 16 kb/s; Reads=2 over 1 s; "
                + "Since Startup=0 B", inward.join("; "));
        wireless();
    }

    // Wi-Fi's own row's drawer: the network joined, its signal and how long
    // first, the rest in wifid's order; before it's read, how its row
    // stands; on none, what's in reach; off, that it's off.
    function wireless() {
        const words = { name: Words.name("wifi"), said: Words.of("wifi").said ?? {},
            leading: [Words.name("wifi:signal"), Words.name("wifi:connected")] };
        const heard = [{ name: "Harbour", connected: true }, { name: "Lantern" }];
        const drawn = { eyebrow: "on 5 GHz \u00b7 network", title: "Harbour", facts: [
            { label: "Speed", value: "866 Mb/s" },
            { label: "Signal", value: "-52 dBm" },
            { label: "Protection", value: "strong" },
            { label: "Connected For", value: "3 h" }] };
        const joined = Plate.plate({ on: true, networks: heard,
            joined: { name: "Harbour", side: "connected", drawn: drawn } }, words);
        Expect.expect("Wi-Fi's row says it's the machine's wireless",
            joined.eyebrow === "on this machine \u00b7 wireless", joined.eyebrow);
        Expect.expect("Wi-Fi's row leads with the network joined, its signal and how long",
            joined.title === "Harbour" && facts(joined).join("; ") === "Signal=-52 dBm; "
                + "Connected For=3 h; Speed=866 Mb/s; Protection=strong", facts(joined).join("; "));
        const unread = Plate.plate({ on: true, networks: heard,
            joined: { name: "Harbour", side: "connected", drawn: null } }, words);
        Expect.expect("before its network's drawer is read, Wi-Fi's row says how it stands",
            unread.title === "Harbour" && unread.body === "connected", JSON.stringify(unread));
        const none = Plate.plate({ on: true, networks: heard.slice(1), joined: null }, words);
        Expect.expect("on no network, Wi-Fi's row names what's in reach",
            none.title === "Wi-Fi" && none.body === "not on a network: Lantern",
            JSON.stringify(none));
        const off = Plate.plate({ on: false, networks: [], joined: null }, words);
        Expect.expect("off, Wi-Fi's row says so", off.title === "Wi-Fi" && off.body === "turned off",
            JSON.stringify(off));
    }
}

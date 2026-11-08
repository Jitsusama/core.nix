// What the launcher's Home Network place says of the home network, run in
// Qt's own JavaScript engine, the one the shell runs it in, over the
// record the eero daemon's parsers are tested against
// (modules/eero/home-network.json). The `the-home-network-says-each-figure`
// check copies HomeNetwork.js, HomeNetworkDrawers.js, what they import and
// the record beside this file, and runs `qml home-network.qml` in UTC. Each
// case names what's asked and what has to come back: the rows' order,
// what each row says, and every figure the eero contract places in the
// internet's, a node's and a device's drawer
// (modules/eero/contract/README.md).
import QtQuick
import "HomeNetwork.js" as Home
import "HomeNetworkDrawers.js" as Drawers

QtObject {
    // A moment a few minutes after eero last answered.
    readonly property real now: Date.parse("2026-11-07T23:00:00Z")

    function read(path) {
        const request = new XMLHttpRequest();
        request.open("GET", path, false);
        request.send();
        return JSON.parse(request.responseText);
    }

    // A drawer's facts by their part's id, or a radio's by its band.
    function byId(drawer) {
        const out = {};
        for (const fact of drawer.facts)
            if (fact.heading === undefined)
                out[fact.id === "port" ? `port ${fact.fill.n}` : fact.id ?? fact.label] = fact;
        return out;
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, got, wanted) => {
            asked++;
            const said = JSON.stringify(got);
            if (said !== JSON.stringify(wanted))
                failures.push(`${what} gave ${said}, not ${JSON.stringify(wanted)}`);
        };
        const model = read("home-network.json");
        const node = id => model.nodes.find(one => one.id === id);
        const device = name => model.devices.find(one => one.name === name);

        // The figures, each in its own unit, said one way.
        expect("a rate", Home.mbps(1200.9), "1,200.9");
        expect("a whole rate", Home.mbps(48.0), "48");
        expect("a speed in Mb/s", Home.speed(11.452299), "11 Mb/s");
        expect("a slow speed", Home.speed(0.0239604), "24 kb/s");
        expect("a crawl", Home.speed(0.0000982), "98 b/s");
        expect("a link speed", Home.speed(1000), "1 Gb/s");
        expect("bytes", Home.bytes(153124674), "153 MB");
        expect("few bytes", Home.bytes(53741), "54 kB");
        expect("bytes under ten", Home.bytes(2400000), "2.4 MB");
        expect("no bytes", Home.bytes(0), "0 B");
        expect("an unknown size", Home.bytes(null), "");
        expect("the upper 5 GHz radio", Home.radioBand("5.8GHz"), "5 GHz high");
        expect("the lower 5 GHz radio", Home.radioBand("5.2GHz"), "5 GHz low");
        expect("a radio eero hasn't named before", Home.radioBand("3.5GHz"), "3.5 GHz");
        expect("eero's word", Home.worded("laptop_computer"), "laptop computer");
        expect("a time today", Home.when("2026-11-07T07:22:42.035Z", now), "07:22");
        expect("a day this year", Home.when("2026-11-05T10:52:15Z", now), "5 Nov 10:52");
        expect("a year before", Home.when("2020-11-13T21:47:15Z", now), "13 Nov 2020");
        expect("a moment ago", Home.ago("2026-11-07T22:59:48Z", now), "12 s ago");
        expect("minutes ago", Home.ago("2026-11-07T22:54:06.658Z", now), "5 min ago");

        // The gateway node first, then each node after the one it hangs
        // from, siblings by location, each with its devices, strongest
        // first.
        const groups = Home.grouped(model);
        expect("the nodes' order", groups.map(group => group.node.location),
            ["Hall", "Den", "Study"]);
        expect("the gateway node's devices", groups[0].devices.map(one => one.name),
            ["generic 3", "phone 1"]);
        expect("a node's devices", groups[1].devices.map(one => one.name),
            ["generic 2", "laptop computer 2", "printer 1", "tablet 1"]);
        const astray = Object.assign({}, model, {
            devices: model.devices.concat([Object.assign({}, device("phone 1"),
                { mac: "02:00:00:00:00:ff", node: "9999" })])
        });
        expect("a device on no node listed", Home.grouped(astray).slice(-1)[0].node, null);
        const orphan = Object.assign({}, node("2002"), {
            id: "2009",
            location: "Attic",
            backhaul: { kind: "wi-fi", upstream: "Shed", radio: "5.8GHz", bars: 2 }
        });
        expect("a node hanging from one not listed comes last",
            Home.nodesInOrder(model.nodes.concat([orphan])).map(one => one.location),
            ["Hall", "Den", "Study", "Attic"]);

        // The head: the internet, how it stands and everything at a glance.
        expect("the internet's row", Home.internetRow(model, now), {
            name: "Internet",
            side: "connected",
            note: "Example ISP \u00b7 369.9 Mb/s down, 11.9 Mb/s up on 5 Nov \u00b7 "
                + "conferencing and gaming on \u00b7 no outages \u00b7 3 nodes online \u00b7 "
                + "24 devices"
        });
        expect("the home network's state where it isn't connected",
            Home.internetRow(Object.assign({}, model, { internet: Object.assign({},
                model.internet, { mesh: "degraded" }) }), now).note.split(" \u00b7 ")[0],
            "home network degraded");
        expect("the internet unreachable", Home.internetRow(Object.assign({}, model,
            { cloud: { state: "unreachable", since: model.read } }), now).side,
            "eero unreachable");
        expect("the place's holding", Home.holding(model), "3 nodes \u00b7 15 devices");

        // A node: its devices, its backhaul, and how it is.
        expect("the gateway node's row", Home.nodeRow(node("2003")), {
            name: "Hall",
            side: "2 devices",
            note: "gateway node \u00b7 Ethernet \u00b7 eero Pro 6 \u00b7 v7.17.4-5"
        });
        expect("a node over Wi-Fi", Home.nodeRow(node("2002")), {
            name: "Study",
            side: "9 devices",
            note: "Wi-Fi from Hall over 5 GHz high, 5 of 5 bars \u00b7 eero Pro 6 \u00b7 v7.17.4-5"
        });
        expect("a node offline", Home.nodeRow(Object.assign({}, node("2001"),
            { state: "OFFLINE" })).note.split(" \u00b7 ")[0], "offline");

        // A device: its signal, band and channel, rate each way, speed now.
        expect("a quiet device", Home.deviceRow(device("generic 2")), {
            name: "generic 2",
            side: "-37 dBm",
            note: "5 GHz, channel 52 \u00b7 1,200.9 Mb/s down, 1,080.6 Mb/s up \u00b7 idle \u00b7 "
                + "10.0.0.45",
            mark: "wireless"
        });
        expect("a busy computer", Home.deviceRow(device("unknown computer 1")).note,
            "5 GHz, channel 149 \u00b7 1,200.9 Mb/s down, 1,200.9 Mb/s up \u00b7 "
            + "11 Mb/s down, 9.9 Mb/s up \u00b7 10.0.0.20");
        expect("a computer's mark", Home.deviceRow(device("unknown computer 1")).mark, "computer");
        expect("a device rated one way", Home.deviceRow(device("printer 1")).note,
            "2.4 GHz, channel 6 \u00b7 72.2 Mb/s down \u00b7 idle \u00b7 10.0.0.31");
        expect("a trickle", Home.deviceRow(device("generic 3")).note,
            "2.4 GHz, channel 6 \u00b7 72.2 Mb/s down \u00b7 98 b/s down, 154 b/s up \u00b7 "
            + "10.0.0.99");
        expect("a device in a profile", Home.deviceRow(Object.assign({}, device("printer 1"),
            { profile: "3002" }), {}, model.profiles).note.split(" \u00b7 ").slice(-1)[0],
            "profile 2");
        const cabled = Object.assign({}, device("generic 3"), {
            ethernet: true,
            band: null,
            channel: null,
            signal_dbm: null,
            rx: null,
            tx: null,
            port: { name: "2", mbps: 1000 }
        });
        expect("a device over Ethernet", Home.deviceRow(cabled), {
            name: "generic 3",
            side: "Ethernet",
            note: "Ethernet \u00b7 98 b/s down, 154 b/s up \u00b7 10.0.0.99",
            mark: "wired"
        });
        expect("a device named only by its address", Home.nameOf({ mac: "02:00:00:00:00:99" }),
            "02:00:00:00:00:99");

        // The internet's drawer: every figure the contract places there.
        const internet = byId(Drawers.internet(model, now));
        const value = (facts, id) => facts[id] ? facts[id].value : null;
        for (const [id, wanted] of [
            ["state", "connected"],
            ["mesh", "connected"],
            ["isp", "Example ISP \u00b7 AS64496"],
            ["public", "203.0.113.1"],
            ["lease", "DHCP \u00b7 203.0.113.1 / 255.255.254.0 \u00b7 router 203.0.113.2"],
            ["gateway", "10.0.0.1"],
            ["mode", "NAT \u00b7 addresses automatic"],
            ["dns", "automatic \u00b7 203.0.113.3, 203.0.113.4 \u00b7 caching off \u00b7 "
                + "IPv6 automatic"],
            ["ipv6", "off"],
            ["speed-test", "369.9 Mb/s down, 11.9 Mb/s up \u00b7 5 Nov 10:52"],
            ["down", "369.9 Mb/s"],
            ["up", "11.9 Mb/s"],
            ["kept", "15 since 24 Sep 09:53"],
            ["sqm", "optimized"],
            ["upnp", "on"],
            ["randomization", "off"],
            ["steering", "on"],
            ["wpa3", "off"],
            ["wifi-mode", "default"],
            ["mlo", "disabled"],
            ["thread", "off \u00b7 name 56 \u00b7 channel 25"],
            ["guest", "off \u00b7 name 49"],
            ["ddns", "off"],
            ["backup", "on"],
            ["power-saving", "off"],
            ["outages", "none recorded"],
            ["nodes", "3 nodes online"],
            ["devices", "24 eero counts"],
            ["firmware", "v7.17.4-5 \u00b7 up to date"],
            ["updates", "at 02:00 \u00b7 last 07:19"],
            ["traffic", "167 GB down \u00b7 73 GB up"],
            ["read", "22:54 \u00b7 answering"]
        ])
            expect(`the internet's ${id}`, value(internet, id), wanted);
        expect("the speed tests sparked", internet.down.figure.shape, "spark");
        expect("a spark's newest last, scaled to the most", internet.up.figure.values.slice(-1)[0]
            <= 1 && internet.up.figure.values.length, 15);
        expect("the day's traffic sparked by the hour", internet.traffic.figure.values.length, 20);

        // A node's drawer: which, how it stands, each radio and port.
        const hall = byId(Drawers.node(model, node("2003"), now));
        for (const [id, wanted] of [
            ["model", "eero Pro 6 \u00b7 K010011"],
            ["serial", "SERIAL0003"],
            ["state", "online \u00b7 heard 6 min ago"],
            ["backhaul", "Ethernet \u00b7 5 of 5 bars"],
            ["devices", "2 \u00b7 0 Ethernet \u00b7 2 Wi-Fi"],
            ["2.4 GHz", "02:00:00:00:00:0c"],
            ["port 1", "1 Gb/s of 1 Gb/s \u00b7 internet \u00b7 backhaul \u00b7 0 B in \u00b7 "
                + "153 MB out"],
            ["port 2", "1 Gb/s of 1 Gb/s \u00b7 0 B in \u00b7 54 kB out"],
            ["firmware", "v7.17.4-5 \u00b7 up to date"],
            ["up", "15 h 31 min \u00b7 since 07:22"],
            ["cloud", "15 h 6 min"],
            ["added", "13 Nov 2020"],
            ["address", "10.0.0.1"],
            ["ipv6", "fe80::7/64, fe80::8/64"],
            ["mac", "02:00:00:00:00:21"],
            ["light", "on \u00b7 100%"],
            ["power", "USB"],
            ["traffic", "150 MB down \u00b7 10 MB up"]
        ])
            expect(`the gateway node's ${id}`, value(hall, id), wanted);
        expect("no support end eero didn't give", hall.support, undefined);
        const radios = Drawers.node(model, node("2003"), now).facts
            .filter(fact => fact.label !== undefined && fact.figure);
        expect("each radio by band", radios.map(fact => [fact.label, fact.value]), [
            ["2.4 GHz", "channel 6 \u00b7 20 MHz \u00b7 30 dBm \u00b7 1 device \u00b7 8% busy"],
            ["5 GHz high", "channel 149 \u00b7 80 MHz \u00b7 28 dBm \u00b7 1 device \u00b7 41% busy"],
            ["5 GHz low", "channel 52 \u00b7 80 MHz \u00b7 22 dBm \u00b7 0 devices \u00b7 5% busy"]
        ]);
        expect("how busy, as a stroke", radios[1].figure, {
            shape: "stroke", values: [0.41], warn: false
        });
        const study = byId(Drawers.node(model, node("2002"), now));
        expect("a node's backhaul over Wi-Fi", value(study, "backhaul"),
            "Wi-Fi from Hall over 5 GHz high \u00b7 5 of 5 bars");
        expect("a port not linked", value(study, "port 1"), "not linked, up to 1 Gb/s");
        const onHall = Drawers.node(model, node("2003"), now).facts;
        expect("the devices on a node, strongest first", onHall.slice(onHall.findIndex(fact =>
            fact.heading === "on-it") + 1), [
            { label: "generic 3", value: "-48 dBm \u00b7 2.4 GHz, channel 6" },
            { label: "phone 1", value: "-52 dBm \u00b7 5 GHz, channel 149" }
        ]);
        expect("the eyebrow of a node", Drawers.node(model, node("2002"), now).eyebrow,
            "node \u00b7 online");

        // A device's drawer: each way's rate and how it's made, and the rest.
        const generic = Drawers.device(model, device("generic 2"), now);
        const facts = byId(generic);
        for (const [id, wanted] of [
            ["kind", "generic"],
            ["node", "Den \u00b7 eero Pro 6"],
            ["signal", "-37 dBm \u00b7 71 dB over the noise"],
            ["score", "1 \u00b7 5 of 5 bars"],
            ["band", "5 GHz, channel 52 \u00b7 5,260 MHz"],
            ["bssid", "02:00:00:00:00:02"],
            ["rx", "1,200.9 Mb/s \u00b7 MCS 11 \u00b7 2 streams \u00b7 80 MHz \u00b7 800 ns guard "
                + "\u00b7 Wi-Fi 6"],
            ["tx", "1,080.6 Mb/s \u00b7 MCS 10 \u00b7 2 streams \u00b7 80 MHz \u00b7 800 ns guard "
                + "\u00b7 Wi-Fi 6"],
            ["speed", "idle"],
            ["retries", "0% \u00b7 quiet for 540 ms"],
            ["packets", "52,846 down \u00b7 94,913 up \u00b7 147,759 in all"],
            ["resent", "735 \u00b7 0 per million"],
            ["failed", "5,494 per million"],
            ["dropped", "0 \u00b7 0 per million"],
            ["network", "home \u00b7 WPA2 \u00b7 main"],
            ["address", "10.0.0.45"],
            ["addresses", "fe80::6"],
            ["mac", "02:00:00:00:00:0a \u00b7 private"],
            ["first-seen", "22 Oct 16:54"],
            ["on-since", "12:43"],
            ["last-active", "22:49"]
        ])
            expect(`a device's ${id}`, value(facts, id), wanted);
        expect("a device's eyebrow", [generic.eyebrow, generic.names], ["on Den \u00b7 Wi-Fi",
            true]);
        expect("no hostname that only says its name again", facts.hostname, undefined);
        expect("no profile it isn't in", facts.profile, undefined);
        expect("its signal as a stroke", facts.signal.figure.shape, "stroke");
        const older = byId(Drawers.device(model, device("printer 1"), now));
        expect("a Wi-Fi 4 device's rate", value(older, "rx"),
            "72.2 Mb/s \u00b7 MCS 7 \u00b7 1 stream \u00b7 20 MHz \u00b7 400 ns guard \u00b7 Wi-Fi 4");
        expect("no rate up it never gave", older.tx, undefined);
        const inProfile = byId(Drawers.device(Object.assign({}, model, {
            profiles: [{ id: "3001", name: "profile 1", paused: true, devices: [] }]
        }), Object.assign({}, device("phone 1"), { profile: "3001" }), now));
        expect("a device's profile", value(inProfile, "profile"), "profile 1 \u00b7 paused");

        // What happened, newest first, in the internet's drawer, and each
        // device's own in its.
        const events = Drawers.internet(model, now).facts;
        const timeline = events.slice(events.findIndex(fact => fact.heading === "events") + 1);
        expect("what happened, newest first", timeline.slice(0, 2), [
            { label: "22:42", value: "tablet 1 connected to Den eero Pro 6 on 5.2GHz channel 52." },
            { label: "22:42", value: "tablet 1 disconnected from Study eero Pro 6." }
        ]);
        expect("every event", timeline.length, 5);
        const tablet = Drawers.device(model, device("tablet 1"), now).facts;
        expect("a device's own events", tablet.slice(tablet.findIndex(fact =>
            fact.heading === "events") + 1).map(fact => fact.value), [
            "tablet 1 connected to Den eero Pro 6 on 5.2GHz channel 52.",
            "tablet 1 disconnected from Study eero Pro 6."
        ]);
        expect("none of another's", Drawers.device(model, device("generic 2"), now).facts
            .some(fact => fact.heading === "events"), false);

        // Each device's readings kept as records come, the newest last, so its
        // drawer sparks its signal and its speed.
        const once = Home.remembered({}, model, 3);
        expect("a first reading", once["02:00:00:00:00:0a"], { signal: [-37], speed: [0] });
        const later = Object.assign({}, model, {
            devices: [Object.assign({}, device("generic 2"), {
                signal_dbm: -41,
                speed: { down_mbps: 2, up_mbps: 0.5 }
            })]
        });
        let kept = once;
        for (let read = 0; read < 3; read++)
            kept = Home.remembered(kept, later, 3);
        expect("readings kept to the most", kept["02:00:00:00:00:0a"], {
            signal: [-41, -41, -41], speed: [2.5, 2.5, 2.5]
        });
        expect("a device gone is let go", Object.keys(kept).length, 1);
        const sparked = byId(Drawers.device(model, device("generic 2"), now, {
            readings: Home.remembered(once, model, 20)["02:00:00:00:00:0a"]
        }));
        expect("a signal read twice, sparked", sparked.signal.figure.shape, "spark");
        expect("a speed read twice, sparked", sparked.speed.figure.shape, "spark");

        // A profile: how it stands, and each device in it.
        expect("a profile's row", Home.profileRow(model, model.profiles[0]), {
            name: "profile 1",
            side: "active",
            note: "10 devices \u00b7 none on now"
        });
        const held = Object.assign({}, model.profiles[1], {
            paused: true,
            devices: ["02:00:00:00:00:0a", "02:00:00:00:00:35"]
        });
        expect("a profile held off", Home.profileRow(model, held).side, "paused");
        const plate = Drawers.profile(model, held);
        expect("a profile's plate", [plate.eyebrow, plate.title], ["profile \u00b7 paused",
            "profile 2"]);
        expect("a profile's facts", plate.facts, [
            { id: "state", value: "Active \u00b7 paused" },
            { id: "devices", value: "2 \u00b7 1 on now" },
            { heading: "listed" },
            { label: "generic 2", value: "on Den" },
            { label: "02:00:00:00:00:35", value: "not on now" }
        ]);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}

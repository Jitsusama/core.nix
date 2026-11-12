// What the Home Network place's internet drawer says: every figure the
// eero contract places there
// (modules/eero/contract/README.md, Where Each Figure Shows), as a
// fact keyed by its part's id, which HomeNetwork.qml names through the
// launcher's vocabulary, so a name is said once, in words.json. A fact is
// { id, value, fill, figure }, a heading { heading }, and one named by
// what eero says rather than by a word of ours, a radio's band, carries
// its `label` itself. A figure eero didn't give is no fact at all. What
// the daemon keeps of the last hour and day is drawn by
// HomeNetworkHistory.js, which holds the pieces a fact is made of too,
// and how the network has and hands out addresses by
// HomeNetworkAddresses.js. A profile's, a node's and a device's drawers
// are the eero daemon's (modules/eero's drawers.rs), served ready, as
// this will be in turn.
// Plain reading of the record, held in Qt's own engine
// (tests/home-network.qml).
.pragma library
.import "HomeNetwork.js" as Home
.import "HomeNetworkAddresses.js" as Addresses
.import "HomeNetworkHistory.js" as History
.import "Machine.js" as Machine

// A graph holds twenty readings, the newest last.
const GRAPHED = 20;
// The pieces a fact is made of, as the history's facts are made of them.
const known = History.known;
const joined = History.joined;
const fact = History.fact;
const kept = History.kept;
const onOff = History.onOff;
// A state eero words as disabled or enabled, said off or on, as every
// setting's state is.
const STATES = { disabled: "off", enabled: "on" };

// Readings as Graph's spark: the newest twenty, scaled to their most.
function spark(values) {
    const recent = values.filter((value) => Number.isFinite(value)).slice(-GRAPHED);
    return recent.length > 1 ? { shape: "spark", values: Machine.spark(recent, 0), warn: false } : null;
}

// Bytes by hour, each way, as one history: what moved in each hour.
function hourly(usage) {
    const down = usage?.download?.hours ?? [];
    const up = usage?.upload?.hours ?? [];
    return down.map((hour, at) => hour.bytes + (up[at]?.bytes ?? 0));
}

// The day's traffic, "150 MB down · 3.10 MB up", sparked by the hour.
function trafficFact(usage, settings) {
    if (!usage) {
        return null;
    }
    const down = Home.bytes(usage.download?.total_bytes, settings);
    const up = Home.bytes(usage.upload?.total_bytes, settings);
    const value = joined([down === "" ? "" : `${down} down`, up === "" ? "" : `${up} up`]);
    return fact("traffic", value, { figure: spark(hourly(usage)) });
}

// The internet's drawer: first the last speed test, the nodes and
// outages, who carries it, the firmware and the day's traffic, so a
// narrow drawer shows those before its more; then this machine's last
// look at it, each outage, every speed test, how its addresses are had,
// its switches and what happened to it, each under a heading. It holds
// only the internet: each device, its address and what happened to it
// are in its own drawer, and who came and went in its node's (Joel,
// 2026-11-11: "home network internet seems to have devices connected
// to the LAN thrown in"). How it
// stands is its row's side, so the drawer doesn't say it again.
// Its eyebrow is where it comes from, as every drawer's here is where
// the thing is, then its kind (VKCF3Z, 2026-11-07).
function internet(model, now, settings) {
    const net = model?.internet ?? {};
    const firmware = net.firmware ?? {};
    return {
        eyebrow: known(net.isp) ? `via ${net.isp}` : "",
        names: true,
        title: "Internet",
        facts: kept([
            fact("speed-test", joined([Home.speedTestOf(net.speed_test, settings),
                History.when(net.speed_test?.at, now)])),
            fact("nodes", Home.nodesSaid(net.nodes)),
            fact("outages", History.outagesSaid(net)),
            fact("isp", joined([net.isp, known(net.asn) ? `AS${net.asn}` : "",
                net.isp_up === false ? "down" : ""])),
            fact("mesh", net.mesh),
            fact("firmware", joined([firmware.running, firmware.has_update
                ? `${firmware.next ?? "an update"} ready` : "up to date"])),
            fact("updates", joined([known(firmware.update_hour)
                ? `at ${String(firmware.update_hour).padStart(2, "0")}:00` : "",
                firmware.last_started ? `last ${History.when(firmware.last_started, now)}` : ""])),
            trafficFact(net.usage, settings),
            fact("read", joined([History.when(model?.read, now), model?.cloud?.state])),
        ]).concat(History.probe(net, now, settings), History.outages(net, now),
            speedTests(net, now, settings), Addresses.addressing(model), switches(net),
            happened(model, History.timeline(model).filter((event) => !ofADevice(model, event)),
                now)),
    };
}

// Whether something that happened is about a device, on now or not, which
// its own drawer says, so the internet's holds only the internet's.
function ofADevice(model, event) {
    return (model?.devices ?? []).concat(model?.devices_off ?? [])
        .some((one) => about(event, one));
}

// Whether something eero's activity list says is about a device: its
// words start with the device's name, as "tablet 1 connected to Den".
function about(event, one) {
    return String(event.text ?? "").startsWith(`${Home.nameOf(one)} `);
}

// What happened, newest first, each by when, in the words eero's activity
// list or the daemon's notice said it.
function happened(model, events, now) {
    if (events.length === 0) {
        return [];
    }
    return [{ heading: "events" }].concat(events.map((event) => ({
        label: History.when(event.at, now),
        value: History.sentence(model, event.text),
        words: true,
    })));
}

// Every speed test kept, a spark of each way, the last's figures beside.
function speedTests(net, now, settings) {
    const tests = (net.speed_tests ?? []).slice().reverse();
    const oldest = tests.length > 0 ? tests[0].at : null;
    const counted = Home.counted(tests.length, "speed test");
    return kept([
        tests.length > 1 ? { heading: "speed-tests" } : null,
        tests.length > 1 ? fact("down", Home.rate(net.speed_test?.down_mbps, settings),
            { figure: spark(tests.map((test) => test.down_mbps)) }) : null,
        tests.length > 1 ? fact("up", Home.rate(net.speed_test?.up_mbps, settings),
            { figure: spark(tests.map((test) => test.up_mbps)) }) : null,
        tests.length > 1 ? fact("kept", `${counted} since ${History.when(oldest, now)}`) : null,
    ]);
}

// The home network's settings, each a switch eero says on or off, or
// as eero words it.
function switches(net) {
    const thread = net.thread ?? {};
    const guest = net.guest ?? {};
    return kept([
        { heading: "settings" },
        // Its name is eero's long one less its verb, so it stays whole beside
        // its value in a narrow drawer.
        fact("sqm", optimized(net.sqm)),
        fact("upnp", onOff(net.upnp)),
        fact("randomization", onOff(net.nat_port_randomization)),
        fact("steering", onOff(net.band_steering)),
        fact("wpa3", onOff(net.wpa3)),
        fact("security", Addresses.security(net.security)),
        fact("wifi-mode", net.wireless_mode),
        fact("ac-compatibility", onOff(net.ac_compatibility)),
        fact("name-clash", Addresses.nameClash(net.name_clash)),
        fact("mlo", STATES[net.mlo_mode] ?? net.mlo_mode),
        fact("thread", joined([onOff(thread.on), thread.name,
            known(thread.channel) ? `channel ${thread.channel}` : ""])),
        fact("guest", joined([onOff(guest.on), guest.name])),
        fact("ddns", onOff(net.ddns)),
        fact("backup", onOff(net.backup_internet)),
        fact("power-saving", onOff(net.power_saving)),
    ]);
}

function optimized(on) {
    if (!known(on)) {
        return "";
    }
    return on ? "optimized" : "not optimized";
}

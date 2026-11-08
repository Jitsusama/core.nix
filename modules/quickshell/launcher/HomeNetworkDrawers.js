// What the Home Network place's drawers say: every figure the eero
// contract places in the internet's, a node's and a device's drawer
// (modules/eero/contract/README.md, Where Each Figure Shows), each as a
// fact keyed by its part's id, which HomeNetwork.qml names through the
// launcher's vocabulary, so a name is said once, in words.json. A fact is
// { id, value, fill, figure }, a heading { heading }, and one named by
// what eero says rather than by a word of ours, a radio's band, carries
// its `label` itself. A figure eero didn't give is no fact at all. Plain
// reading of the record, held in Qt's own engine (tests/home-network.qml).
.pragma library
.import "HomeNetwork.js" as Home
.import "Interfaces.js" as Interfaces
.import "Machine.js" as Machine

// A graph holds twenty readings, the newest last.
const GRAPHED = 20;
const PERCENT = 100;
const NANO = "ns";
const DOT = " \u00b7 ";

function known(value) {
    return value !== null && value !== undefined && value !== "";
}

// Parts said together, those not known left out.
function joined(parts) {
    return parts.filter((part) => known(part)).join(DOT);
}

// A switch as words: "on" or "off".
function onOff(value) {
    if (!known(value)) {
        return "";
    }
    return value ? "on" : "off";
}

// A fact, or nothing where its value isn't known.
function fact(id, value, extra) {
    return known(value) ? Object.assign({ id, value: String(value) }, extra ?? {}) : null;
}

function kept(facts) {
    return facts.filter((one) => one !== null);
}

// A history as Graph's spark: the newest last, scaled to its most.
function spark(values) {
    const recent = values.filter((value) => Number.isFinite(value)).slice(-GRAPHED);
    return recent.length > 1 ? { shape: "spark", values: Machine.spark(recent, 0), warn: false } : null;
}

// A share as Meter's stroke, warning past `warn` where it's given.
function stroke(share, warn) {
    const value = Math.max(0, Math.min(1, share));
    return { shape: "stroke", values: [value], warn: warn !== undefined && value >= warn };
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

// The internet's drawer: first the last speed test, the nodes, devices
// and outages, who carries it, the firmware and the day's traffic, so a
// narrow drawer shows those before its more; then every speed test, how
// its addresses are had, and its switches, each under a heading. How it
// stands is its row's side, so the drawer doesn't say it again.
// Its eyebrow is where it comes from, as every drawer's here is where
// the thing is, then its kind (VKCF3Z, 2026-11-07).
function internet(model, now, settings) {
    const net = model?.internet ?? {};
    const lease = net.lease ?? {};
    const dns = net.dns ?? {};
    const firmware = net.firmware ?? {};
    return {
        eyebrow: known(net.isp) ? `via ${net.isp}` : "",
        names: true,
        title: "Internet",
        facts: kept([
            fact("speed-test", joined([Home.speedTestOf(net.speed_test, settings),
                Home.when(net.speed_test?.at, now)])),
            fact("nodes", Home.nodesSaid(net.nodes)),
            // eero's count is every device it knows, on now or not, so the
            // ones on now are counted beside it, by how each is on.
            fact("devices", joined([known(net.devices) ? `${net.devices} known` : "",
                devicesOn(model.devices ?? [])])),
            fact("outages", outages(net.outages)),
            fact("isp", joined([net.isp, known(net.asn) ? `AS${net.asn}` : "",
                net.isp_up === false ? "down" : ""])),
            fact("mesh", net.mesh),
            fact("firmware", joined([firmware.running, firmware.has_update
                ? `${firmware.next ?? "an update"} ready` : "up to date"])),
            fact("updates", joined([known(firmware.update_hour)
                ? `at ${String(firmware.update_hour).padStart(2, "0")}:00` : "",
                firmware.last_started ? `last ${Home.when(firmware.last_started, now)}` : ""])),
            trafficFact(net.usage, settings),
            fact("read", joined([Home.when(model?.read, now), model?.cloud?.state])),
        ]).concat(speedTests(net, now, settings), kept([
            { heading: "addressing" },
            fact("public", joined([net.public_address, net.double_nat ? "behind another NAT" : ""])),
            fact("lease", joined([net.wan_type, lease.address && lease.mask
                ? `${lease.address} / ${lease.mask}` : lease.address,
                lease.router ? `router ${lease.router}` : ""])),
            fact("gateway", net.gateway_address),
            fact("mode", joined([String(net.mode ?? "").toUpperCase(),
                net.dhcp ? `addresses ${net.dhcp}` : ""])),
            fact("dns", joined([dns.mode, (dns.servers ?? []).join(DOT),
                known(dns.caching) ? `caching ${onOff(dns.caching)}` : "",
                dns.ipv6_mode ? `IPv6 ${dns.ipv6_mode}` : ""])),
            fact("ipv6", onOff(net.ipv6)),
        ]), switches(net), happened(model?.events ?? [], now)),
    };
}

// The devices on now as counts of what they count, a part that's none
// giving way: "12 on Wi-Fi \u00b7 3 on Ethernet", "15 on Wi-Fi".
function devicesOn(listed) {
    const ethernet = listed.filter((each) => each.ethernet).length;
    const wifi = listed.length - ethernet;
    return joined([wifi > 0 ? `${wifi} on Wi-Fi` : "",
        ethernet > 0 ? `${ethernet} on Ethernet` : ""]);
}

// Whether something eero's activity list says is about a device: its
// words start with the device's name, as "tablet 1 connected to Den".
function about(event, one) {
    return String(event.text ?? "").startsWith(`${Home.nameOf(one)} `);
}

// What happened, newest first, each by when, as eero's activity list says
// it.
function happened(events, now) {
    if (events.length === 0) {
        return [];
    }
    return [{ heading: "events" }].concat(events.map((event) => ({
        label: Home.when(event.at, now),
        value: event.text,
    })));
}

// A profile's drawer: how many devices it lists, and each of them, saying
// which node it's on now or that it isn't on. How it stands is its row's
// side, so the drawer doesn't say it again.
function profile(model, one) {
    const devices = model?.devices ?? [];
    const listed = one.devices ?? [];
    const nodes = model?.nodes ?? [];
    const onNow = listed.filter((mac) => devices.some((each) => each.mac === mac)).length;
    return {
        eyebrow: "profile",
        names: true,
        title: one.name ?? one.id,
        facts: kept([
            fact("devices", joined([`${listed.length} listed`,
                onNow > 0 ? `${onNow} on now` : ""])),
        ]).concat(listed.length > 0 ? [{ heading: "listed" }] : [], listed.map((mac) => {
            const on = devices.find((each) => each.mac === mac) ?? null;
            const where = on ? nodes.find((each) => each.id === on.node) ?? null : null;
            return {
                label: on ? Home.nameOf(on) : mac,
                value: where ? `on ${Home.nodeName(where)}` : "not on now",
            };
        })),
    };
}

// Every speed test kept, a spark of each way, the last's figures beside.
function speedTests(net, now, settings) {
    const tests = (net.speed_tests ?? []).slice().reverse();
    const oldest = tests.length > 0 ? tests[0].at : null;
    return kept([
        tests.length > 1 ? { heading: "speed-tests" } : null,
        tests.length > 1 ? fact("down", Home.rate(net.speed_test?.down_mbps, settings),
            { figure: spark(tests.map((test) => test.down_mbps)) }) : null,
        tests.length > 1 ? fact("up", Home.rate(net.speed_test?.up_mbps, settings),
            { figure: spark(tests.map((test) => test.up_mbps)) }) : null,
        tests.length > 1 ? fact("kept", `${tests.length} since ${Home.when(oldest, now)}`) : null,
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
        fact("wifi-mode", net.wireless_mode),
        fact("mlo", net.mlo_mode),
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

function outages(list) {
    const recorded = (list ?? []).length;
    return recorded === 0 ? "none recorded" : `${recorded} recorded`;
}

// A node's drawer: when it was last heard from, how it reaches the one
// before it and how many devices are on it, first, as a narrow drawer has
// room for those before its more; then which node it is, each radio and
// port, its firmware, uptime and addresses, its light, and the day's
// traffic through it. Its state is said only where it isn't online, which
// its row says too, as the first fact. Its eyebrow is what it is, then
// its kind: "eero Pro 6 \u00b7 node".
function node(model, one, now, settings) {
    const devices = one.devices ?? {};
    const state = Home.stateOf(one);
    return {
        eyebrow: joined([one.model, one.gateway ? "gateway node" : "node"]),
        names: true,
        title: Home.nodeName(one),
        facts: kept([
            fact("state", state !== "online" ? state : null),
            fact("heard", heartbeat(one, now)),
            fact("backhaul", joined([Home.backhaulOf(one), known(one.backhaul?.bars)
                ? `${one.backhaul.bars} of 5 bars` : ""]), {
                figure: Home.backhaulLevel(one) === null ? null : stroke(Home.backhaulLevel(one)),
            }),
            fact("devices", devicesCount(devices)),
            fact("model", joined([one.model, one.model_number])),
            fact("serial", one.serial),
        ]).concat(radios(one), ports(one, settings), kept([
            { heading: "about" },
            fact("firmware", joined([one.firmware, updateWords(one.update_available)])),
            fact("support", one.support_ends ? Home.when(one.support_ends, now) : null),
            fact("up", joined([known(one.up_s) ? Interfaces.lasted(one.up_s) : "",
                one.last_reboot ? `since ${Home.when(one.last_reboot, now)}` : ""])),
            fact("cloud", known(one.cloud_s) ? Interfaces.lasted(one.cloud_s) : null),
            fact("added", one.joined ? Home.when(one.joined, now) : null),
            fact("address", one.address),
            fact("ipv6", (one.ipv6 ?? []).join(DOT)),
            fact("mac", one.mac),
            fact("light", one.light ? joined([onOff(one.light.on), known(one.light.brightness)
                ? `${one.light.brightness}%` : ""]) : null),
            fact("power", one.power),
            trafficFact(one.usage, settings),
        ]), onIt(model, one, settings)),
    };
}

// The devices on a node, as eero lists them, strongest first: each by
// name, with its signal, band and channel, or its port over Ethernet.
function onIt(model, one, settings) {
    const listed = (model?.devices ?? []).filter((each) => each.node === one.id);
    if (listed.length === 0) {
        return [];
    }
    return [{ heading: "on-it" }].concat(listed.map((each) => ({
        label: Home.nameOf(each),
        value: each.ethernet
            ? joined(["Ethernet", each.port ? each.port.name : "", linkSpeed(each.port?.mbps,
                settings)])
            : joined([Home.signalOf(each), Home.bandOf(each)]),
    })));
}

// Whether a node's firmware has an update, as words.
function updateWords(available) {
    if (!known(available)) {
        return "";
    }
    return available ? "update available" : "up to date";
}

// When a node was last heard from, "2 h 15 min ago", and late where eero
// says it is.
function heartbeat(one, now) {
    if (!one.last_heartbeat) {
        return one.heartbeat_ok === false ? "never" : "";
    }
    return joined([Home.ago(one.last_heartbeat, now), one.heartbeat_ok === false ? "late" : ""]);
}

// How many devices are on a node, by how each is on, a part that's none
// giving way: "9, all on Wi-Fi", or "9 \u00b7 6 on Wi-Fi \u00b7 3 on Ethernet".
function devicesCount(devices) {
    if (!known(devices.all)) {
        return "";
    }
    const wifi = devices.wifi ?? 0;
    const ethernet = devices.ethernet ?? 0;
    if (devices.all > 0 && wifi === devices.all) {
        return `${devices.all}, all on Wi-Fi`;
    }
    if (devices.all > 0 && ethernet === devices.all) {
        return `${devices.all}, all on Ethernet`;
    }
    return joined([String(devices.all), wifi > 0 ? `${wifi} on Wi-Fi` : "",
        ethernet > 0 ? `${ethernet} on Ethernet` : ""]);
}

// Each radio by band: its channel, width, power, devices and how busy, a
// stroke of the last; then each radio's hardware address.
function radios(one) {
    const list = (one.radios ?? []).slice().sort((a, b) => a.band.localeCompare(b.band));
    if (list.length === 0) {
        return [];
    }
    const lines = list.map((radio) => ({
        label: radio.band,
        value: joined([known(radio.channel) ? `channel ${radio.channel}` : "",
            known(radio.width_mhz) ? `${radio.width_mhz} MHz` : "",
            known(radio.power_dbm) ? `${Home.signed(radio.power_dbm)} dBm` : "",
            known(radio.devices) ? Home.counted(radio.devices, "device") : "",
            known(radio.busy_pct) ? `${radio.busy_pct}% busy` : ""]),
        figure: known(radio.busy_pct) ? stroke(radio.busy_pct / PERCENT) : null,
    }));
    const addresses = list.filter((radio) => radio.bssid).map((radio) => ({
        label: radio.band,
        value: radio.bssid,
    }));
    return [{ heading: "radios" }].concat(lines,
        addresses.length > 0 ? [{ heading: "bssids" }].concat(addresses) : []);
}

// A link speed in Mb/s as the machine says one: "1 Gb/s", "100 Mb/s".
function linkSpeed(value, settings) {
    return known(value) ? Home.rate(value, settings) : "";
}

// Each port: linked at what speed of its best, whether it's the internet's
// or the backhaul's, and what's gone through it each way.
function ports(one, settings) {
    const list = one.ports ?? [];
    if (list.length === 0) {
        return [];
    }
    return [{ heading: "ports" }].concat(list.map((port) => ({
        id: "port",
        fill: { n: port.name },
        value: joined([portLink(port, settings),
            port.wan ? "internet" : "", port.upstream ? "backhaul" : "",
            known(port.rx_bytes) ? `${Home.bytes(port.rx_bytes, settings)} in` : "",
            known(port.tx_bytes) ? `${Home.bytes(port.tx_bytes, settings)} out` : ""]),
    })));
}

// How a port is linked: "1.00 Gb/s of 1.00 Gb/s", or "not linked \u00b7 up
// to 1.00 Gb/s".
function portLink(port, settings) {
    const best = known(port.best_mbps) ? linkSpeed(port.best_mbps, settings) : "";
    if (!port.linked) {
        return best === "" ? "not linked" : `not linked${DOT}up to ${best}`;
    }
    const now = linkSpeed(port.mbps, settings);
    return best === "" ? now : `${now} of ${best}`;
}

// A rate and how the radio makes it: "1.20 Gb/s · MCS 11 · 2 streams ·
// 80 MHz · 800 ns guard · Wi-Fi 6".
function rateOf(rate, settings) {
    if (!rate) {
        return "";
    }
    return joined([Home.rate(rate.mbps, settings),
        known(rate.mcs) ? `MCS ${rate.mcs}` : "",
        known(rate.streams) ? Home.counted(rate.streams, "stream") : "",
        known(rate.width_mhz) ? `${rate.width_mhz} MHz` : "",
        known(rate.guard_ns) ? `${rate.guard_ns} ${NANO} guard` : "",
        rate.generation]);
}

// A device's drawer: how well it hears and is heard, its band, each way's
// rate and how it's made and its speed now, first, as a narrow drawer has
// room for those before its more; then what it is and which node it's on,
// what had to be sent again, its network and addresses, when it was seen,
// and its profile.
// `asked` gives the launcher's number settings and the device's readings
// since the shell started, { settings, readings }, each optional.
function device(model, one, now, asked) {
    const settings = asked?.settings;
    const on = (model?.nodes ?? []).find((each) => each.id === one.node) ?? null;
    const seen = asked?.readings ?? { signal: [], speed: [] };
    const packets = one.packets ?? {};
    const group = (model?.profiles ?? []).find((each) => each.id === one.profile) ?? null;
    return {
        // Where it is, short enough for a narrow drawer's one line: what it
        // is starts its facts.
        eyebrow: joined([on ? `on ${Home.nodeName(on)}` : "", one.ethernet ? "Ethernet" : "Wi-Fi"]),
        names: on !== null,
        title: Home.nameOf(one),
        facts: kept([
            fact("signal", joined([Home.signalOf(one), known(one.snr_db)
                ? `${one.snr_db} dB over the noise` : ""]), {
                figure: signalFigure(one, seen.signal),
            }),
            // eero's score is a share, from nothing to all, said as a
            // percentage as every other share here is.
            fact("score", joined([known(one.score) ? `${Home.number(one.score * PERCENT, 0,
                settings)}%` : "", known(one.bars) ? `${one.bars} of 5 bars` : ""])),
            fact("band", one.ethernet ? null : joined([Home.bandOf(one), known(one.frequency_mhz)
                ? `${Home.number(one.frequency_mhz, 0, settings)} MHz` : ""])),
            fact("port", one.port ? joined([one.port.name, linkSpeed(one.port.mbps, settings)])
                : null),
            fact("rx", rateOf(one.rx, settings)),
            fact("tx", rateOf(one.tx, settings)),
            fact("speed", Home.speedOf(one, settings), { figure: spark(seen.speed) }),
            fact("kind", joined([Home.worded(one.kind), one.make, one.model, one.manufacturer])),
            fact("node", on ? joined([Home.nodeName(on), on.model]) : null),
            fact("bssid", one.bssid),
            fact("retries", joined([known(one.retry_pct) ? `${Home.number(one.retry_pct, 1,
                settings)}%` : "", known(one.idle_ms) ? `quiet for ${Home.number(one.idle_ms, 0,
                settings)} ms` : ""])),
            fact("packets", joined([count(packets.rx_packets, "down", settings),
                count(packets.tx_packets, "up", settings),
                count(packets.total_packets, "in all", settings)])),
            fact("resent", joined([count(packets.tx_retries, "", settings),
                count(packets.tx_retransmit_ppm, "per million", settings)])),
            fact("failed", count(packets.tx_fail_ppm, "per million", settings)),
            fact("dropped", joined([count(packets.rx_drops, "", settings),
                count(packets.rx_drop_ppm, "per million", settings)])),
            { heading: "where" },
            fact("network", joined([one.ssid, String(one.auth ?? "").toUpperCase(), one.subnet,
                one.guest ? "guest" : ""])),
            fact("address", one.address),
            fact("addresses", (one.addresses ?? []).filter((each) => each !== one.address)
                .join(DOT)),
            fact("mac", joined([one.mac, one.private ? "private" : ""])),
            fact("hostname", one.hostname !== Home.nameOf(one) ? one.hostname : null),
            fact("nickname", one.nickname),
            fact("first-seen", Home.when(one.first_seen, now)),
            fact("on-since", Home.when(one.first_active, now)),
            fact("last-active", Home.when(one.last_active, now)),
            fact("profile", group ? joined([group.name, group.paused ? "paused" : ""])
                : null),
            fact("held", joined([one.paused ? "paused" : "", one.blocked ? "blocked" : ""])),
        ]).concat(happened((model?.events ?? []).filter((event) => about(event,
            one)), now)),
    };
}

// A signal's figure: a spark of how it's read since the shell started,
// each from the faintest heard to the strongest, once there are two
// readings, else a stroke of how strong it is now.
function signalFigure(one, readings) {
    if (readings.length > 1) {
        return {
            shape: "spark",
            values: readings.slice(-GRAPHED).map((dbm) => Interfaces.level(dbm)),
            warn: false,
        };
    }
    return known(one.signal_dbm) ? stroke(Interfaces.level(one.signal_dbm)) : null;
}

// A count grouped, with what it counts after.
function count(value, what, settings) {
    if (!known(value)) {
        return "";
    }
    const figure = Home.number(value, 0, settings);
    return what === "" ? figure : `${figure} ${what}`;
}

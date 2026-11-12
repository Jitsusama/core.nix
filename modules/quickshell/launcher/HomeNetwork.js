// The home network as the launcher's Home Network place lays it out, from
// the record the eero daemon writes (modules/eero/home-network.json, each
// key documented in modules/eero/src/model.rs): the order of its rows, the
// internet first, then each node with its devices under it, and what each
// row says, every figure in its own unit, said one way. Eero.qml reads the
// file and HomeNetwork.qml draws the rows; everything here is plain
// reading of the record, so it's held in Qt's own engine
// (tests/home-network.qml). A figure eero doesn't give is null in the
// record, and said by nothing here rather than guessed.
.pragma library
.import "Numbers.js" as Numbers
.import "Interfaces.js" as Interfaces
.import "Order.js" as Order

const KILO = 1000;
const BARS = 5;
const SECOND = 1000;
// Past this many seconds ago, a moment is said in minutes and hours.
const SECONDS_SAID = 100;
// Every figure takes three significant digits, so 369.9 reads 370 and
// 1,200.9 Mb/s reads 1.20 Gb/s (VKCF3Z, 2026-11-07).
const DIGITS = 3;
// A rate's units from Mb/s, as eero gives one, each a thousand of the
// one before.
const RATE_UNITS = [[1e-6, "b/s"], [1e-3, "kb/s"], [1, "Mb/s"], [KILO, "Gb/s"]];
// The real minus, never a hyphen, before a figure under nothing.
const MINUS = "\u2212";
const DOT = " \u00b7 ";
// Bytes in SI units, as the file picker says a size.
const BYTE_UNITS = ["B", "kB", "MB", "GB", "TB"];
// How eero names a backhaul's radio, by the band the node's own radios
// call it: 5.8 GHz is the upper 5 GHz radio, 5.2 GHz the lower.
const RADIOS = {
    "2.4GHz": "2.4 GHz",
    "5.2GHz": "5 GHz low",
    "5.8GHz": "5 GHz high",
    "5GHz": "5 GHz",
    "6GHz": "6 GHz",
};
const GHZ = /^([\d.]+)\s*GHz$/i;
// The theme's mark for each kind of device eero names, by eero's word;
// any other wears Wi-Fi's or Ethernet's, as how it's on the network is
// all that's known of it.
const MARKS = {
    phone: "phone",
    laptop_computer: "computer",
    unknown_computer: "computer",
    computer: "computer",
    desktop_computer: "computer",
    audio: "speaker",
    speaker: "speaker",
    game_console: "gamepad",
};
const UNDERSCORES = /_/g;

// A number grouped and pointed as the launcher's settings say, at most
// `places` decimals.
function number(value, places, settings) {
    const set = settings ?? {};
    return Numbers.format(value, places, set.group, set.point);
}

// A figure to three significant digits, by the launcher's one rule
// (Numbers.significant), grouped and pointed as the settings say: 369.9
// as "370", 11.94 as "11.9", 1.2 as "1.20".
function significant(value, settings) {
    return Numbers.significant(value, settings?.group, settings?.point);
}

// A figure that may be under nothing, with the real minus: "\u221248".
function signed(value) {
    return value < 0 ? `${MINUS}${Math.abs(value)}` : String(value);
}

// A rate given in Mb/s, in whichever unit reads it in three digits:
// "370 Mb/s", "1.20 Gb/s", "98.0 b/s".
function rate(value, settings) {
    if (value === null || value === undefined) {
        return "";
    }
    const size = Math.abs(Number(Number(value).toPrecision(DIGITS)));
    const unit = RATE_UNITS.slice().reverse().find((each) => size >= each[0])
        ?? RATE_UNITS[0];
    return `${significant(value / unit[0], settings)} ${unit[1]}`;
}

// Bytes in SI units, to three digits: "940 kB", "2.40 MB".
function bytes(value, settings) {
    if (value === null || value === undefined) {
        return "";
    }
    let scaled = Number(Number(value).toPrecision(DIGITS));
    let unit = 0;
    while (scaled >= KILO && unit < BYTE_UNITS.length - 1) {
        scaled /= KILO;
        unit += 1;
    }
    return `${unit === 0 ? number(scaled, 0, settings) : significant(scaled, settings)} `
        + BYTE_UNITS[unit];
}

// A backhaul's radio as the node's radios name their bands.
function radioBand(said) {
    const text = String(said ?? "");
    if (RADIOS[text] !== undefined) {
        return RADIOS[text];
    }
    const ghz = GHZ.exec(text);
    return ghz ? `${ghz[1]} GHz` : text;
}

// eero's word for a thing, as words: "laptop_computer" as "laptop computer".
function worded(said) {
    return String(said ?? "").replace(UNDERSCORES, " ").trim();
}

// What a device is called: the name eero shows, else its nickname, its
// hostname, or at least its hardware address.
function nameOf(device) {
    return device.name || device.nickname || device.hostname || device.mac;
}

// What a node is called: where it stands, as eero names it.
function nodeName(node) {
    return node.location || node.model || node.id;
}

// The theme's mark for a device, by name.
function markOf(device) {
    return MARKS[device.kind ?? ""] ?? (device.ethernet ? "wired" : "wireless");
}

// A node's state in lower case, as a row says it: "online".
function stateOf(node) {
    return String(node.state ?? "").toLowerCase();
}

// The nodes as the place lists them: the gateway node first, then each node after the one it hangs
// from, as the backhaul runs, siblings by location, and any whose upstream eero didn't name last.
function nodesInOrder(nodes) {
    const byName = Order.byName(nodeName);
    const listed = [];
    const placed = new Set();
    const under = (node) => {
        listed.push(node);
        placed.add(node.id);
        const below = nodes.filter((other) => !placed.has(other.id)
            && other.backhaul?.upstream === node.location);
        for (const other of Order.sorted(below, byName)) {
            under(other);
        }
    };
    for (const node of Order.sorted(nodes.filter((one) => one.gateway), byName)) {
        under(node);
    }
    for (const node of Order.sorted(nodes, byName)) {
        if (!placed.has(node.id)) {
            under(node);
        }
    }
    return listed;
}

// Each node in that order with its devices, as eero orders them (by node,
// the strongest signal first) but for this machine, first under its node,
// and the devices on no node eero listed, as one group with no node, last.
function grouped(model) {
    const nodes = nodesInOrder(model?.nodes ?? []);
    const listed = model?.devices ?? [];
    const devices = listed
        .filter((device) => device.this_machine)
        .concat(listed.filter((device) => !device.this_machine));
    const known = new Set(nodes.map((node) => node.id));
    const groups = nodes.map((node) => ({
        node,
        devices: devices.filter((device) => device.node === node.id),
    }));
    const astray = devices.filter((device) => !known.has(device.node));
    return astray.length > 0 ? groups.concat([{ node: null, devices: astray }]) : groups;
}

// How a node reaches the one before it, in a few words: "Ethernet", or
// "Wi-Fi from Hall over 5 GHz high".
function backhaulOf(node) {
    const way = node.backhaul ?? {};
    if (way.kind === "ethernet") {
        return "Ethernet";
    }
    const from = way.upstream ? ` from ${way.upstream}` : "";
    const over = way.radio ? ` over ${radioBand(way.radio)}` : "";
    return `Wi-Fi${from}${over}`;
}

// A backhaul's bars as a share, for its row's graph; Ethernet has none.
function backhaulLevel(node) {
    const way = node.backhaul ?? {};
    return way.kind !== "ethernet" && typeof way.bars === "number" ? way.bars / BARS : null;
}

// A count with its noun, singular for one: "1 device", "9 devices".
function counted(count, noun) {
    return `${count} ${count === 1 ? noun : `${noun}s`}`;
}

// A backhaul with its bars over Wi-Fi: "Wi-Fi from Hall over 5 GHz high
// \u00b7 5 of 5 bars".
function backhaulSaid(node) {
    const way = node.backhaul ?? {};
    const bars = way.kind !== "ethernet" && way.bars !== null && way.bars !== undefined
        ? `${DOT}${way.bars} of ${BARS} bars`
        : "";
    return `${backhaulOf(node)}${bars}`;
}

// What a node's row says: how many devices are on it at its side, in the
// column the devices under it say their signal in, and after its name
// how it's doing where that isn't online, whether it's the gateway node,
// its backhaul, its model and its firmware, the firmware giving way first
// as the row narrows.
function nodeRow(node) {
    const state = stateOf(node);
    const devices = node.devices?.all;
    const note = [
        state !== "" && state !== "online" ? state : "",
        node.gateway ? "gateway node" : "",
        backhaulSaid(node),
        node.model ?? "",
        node.firmware ?? "",
    ].filter((part) => part !== "");
    return {
        name: nodeName(node),
        side: devices === null || devices === undefined ? "" : counted(devices, "device"),
        note: note.join(" \u00b7 "),
    };
}

// A device's band and channel: "5 GHz \u00b7 channel 52".
function bandOf(device) {
    if (device.ethernet) {
        return "Ethernet";
    }
    const channel = typeof device.channel === "number" ? `channel ${device.channel}` : "";
    return [device.band ?? "", channel].filter((part) => part !== "").join(DOT);
}

// Rates each way, given in Mb/s, as items: "1.20 Gb/s down \u00b7 1.08 Gb/s
// up", or one way where eero says only one.
function eachWay(down, up, settings) {
    const ways = [
        rate(down, settings) === "" ? "" : `${rate(down, settings)} down`,
        rate(up, settings) === "" ? "" : `${rate(up, settings)} up`,
    ];
    return ways.filter((way) => way !== "").join(DOT);
}

// A device's rate each way.
function ratesOf(device, settings) {
    return eachWay(device.rx?.mbps, device.tx?.mbps, settings);
}

// A device's speed now each way, "1.20 Mb/s down \u00b7 24.0 kb/s up", or idle
// when it moves nothing either way.
function speedOf(device, settings) {
    const now = device.speed ?? {};
    if (now.down_mbps === 0 && now.up_mbps === 0) {
        return "idle";
    }
    const down = rate(now.down_mbps, settings);
    const up = rate(now.up_mbps, settings);
    return [down === "" ? "" : `${down} down`, up === "" ? "" : `${up} up`]
        .filter((way) => way !== "")
        .join(DOT);
}

// A signal in dBm, as Wi-Fi says one, with the real minus.
function signalOf(device) {
    return typeof device.signal_dbm === "number" ? `${signed(device.signal_dbm)} dBm` : "";
}

// What a device's row says: its signal at its side, as a bar too, and
// after its name its band and channel, its rate each way, its speed now,
// its address and the profile it's in, given the profiles. This machine
// says so at its side, which every row shows, not in its note, which only
// the cursor's row does.
function deviceRow(device, settings, profiles) {
    const profile = (profiles ?? []).find((one) => one.id === device.profile) ?? null;
    const side = [device.this_machine ? "this machine" : "", device.ethernet ? "Ethernet" : signalOf(device)];
    return {
        name: nameOf(device),
        side: side.filter((part) => part !== "").join(" \u00b7 "),
        note: [
            bandOf(device),
            ratesOf(device, settings),
            speedOf(device, settings),
            device.address ?? "",
            profile ? profile.name ?? "" : "",
        ]
            .filter((part) => part !== "")
            .join(" \u00b7 "),
        mark: markOf(device),
    };
}

// The nodes by state, as the head says them: "3 nodes online", with any
// offline or pending after.
function nodesSaid(counts) {
    const online = counts?.online;
    if (online === null || online === undefined) {
        return "";
    }
    const rest = [
        counts.offline > 0 ? `${counts.offline} offline` : "",
        counts.pending > 0 ? `${counts.pending} pending` : "",
    ].filter((part) => part !== "");
    return [`${counted(online, "node")} online`].concat(rest).join(DOT);
}

// The last speed test, "370 Mb/s down \u00b7 11.9 Mb/s up".
function speedTestOf(test, settings) {
    return test ? eachWay(test.down_mbps, test.up_mbps, settings) : "";
}

// The internet's outages: the 30 days' the history keeps, else eero's.
function outagesOf(internet) {
    const kept = internet.history?.outages;
    const count = (kept ?? internet.outages ?? []).length;
    return `${count === 0 ? "no outages" : counted(count, "outage")}${kept ? " in 30 days" : ""}`;
}

// What the internet's row says, the place's head: its state then the
// last speed test's down at its side, and after its name the test's up
// and when, how it's optimized, its outages, the nodes by state and how
// many devices are on now. Who carries it is the drawer's eyebrow's and
// its Provider's, so the row doesn't say it again (VKCF3Z, 2026-11-08),
// and while eero isn't answering, its side says that alone. Asked at
// `now`, in milliseconds, for when, and said by `dayOn`, the day as
// HomeNetworkHistory.js says one by When.js. Each is an item, so the row
// gives them way whole from the end (Choice.qml).
function internetRow(model, now, settings, dayOn) {
    const internet = model?.internet ?? {};
    const test = internet.speed_test ?? null;
    const down = eachWay(test?.down_mbps, null, settings);
    const up = eachWay(null, test?.up_mbps, settings);
    const day = test?.at ? dayOn(test.at, now) : "";
    const sqm = typeof internet.sqm === "boolean" ? (internet.sqm ? "on" : "off") : "";
    const answering = !["signed-out", "unreachable"].includes(model?.cloud?.state);
    const onNow = (model?.devices ?? []).length;
    const devices = onNow === 0 ? "" : `${counted(onNow, "device")} on now`;
    // The nodes' network as a whole is said only once it isn't connected,
    // as the internet's state is what the head is about.
    const mesh = String(internet.mesh ?? "");
    return {
        name: "Internet",
        side: [stateWords(model), answering ? down : ""].filter((part) => part !== "").join(DOT),
        note: [
            mesh !== "" && mesh !== "connected" ? `home network ${mesh}` : "",
            internet.isp_up === false ? "provider down" : "",
            up,
            up === "" && down === "" ? "" : day,
            sqm === "" ? "" : `conferencing and gaming ${sqm}`,
            outagesOf(internet),
            nodesSaid(internet.nodes),
            devices,
        ]
            .filter((part) => part !== "")
            .join(" \u00b7 "),
    };
}

// How the internet stands, and whether eero is answering at all: its
// state while eero's cloud answers, else why nothing is known now.
function stateWords(model) {
    const cloud = model?.cloud?.state ?? "";
    if (cloud === "signed-out") {
        return "signed out of eero";
    }
    if (cloud === "unreachable") {
        return "eero unreachable";
    }
    return String(model?.internet?.state ?? "").toLowerCase();
}

// How long ago a moment was, as the launcher says it elsewhere.
function ago(iso, now) {
    const at = new Date(iso).getTime();
    return Number.isNaN(at) ? "" : agoMs(at, now);
}

// How long before `now` the moment `at` was, both in milliseconds: in
// seconds while that's few, as a heartbeat comes every few, else in
// minutes, hours and days.
function agoMs(at, now) {
    const seconds = Math.max(0, Math.round((now - at) / SECOND));
    return seconds < SECONDS_SAID
        ? `${seconds} s ago`
        : `${Interfaces.lasted(seconds)} ago`;
}

// How a profile stands, in lower case: "active", or "paused" while
// it's held off.
function profileState(profile) {
    return profile.paused ? "paused" : String(profile.state ?? "").toLowerCase();
}

// What a profile's row says: how it stands at its side, and how many
// devices it holds and how many of them are on now.
function profileRow(model, profile) {
    const listed = profile.devices ?? [];
    const devices = model?.devices ?? [];
    const onNow = listed.filter((mac) => devices.some((each) => each.mac === mac)).length;
    return {
        name: profile.name ?? profile.id,
        side: profileState(profile),
        note: `${counted(listed.length, "device")} \u00b7 ${onNow === 0 ? "none" : onNow} on now`,
    };
}

// How many nodes and devices the place holds, as its row says at home.
function holding(model) {
    const nodes = (model?.nodes ?? []).length;
    const devices = (model?.devices ?? []).length;
    return `${counted(nodes, "node")} \u00b7 ${counted(devices, "device")}`;
}

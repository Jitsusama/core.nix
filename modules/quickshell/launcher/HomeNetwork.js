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
.import "Traffic.js" as Traffic
.import "Interfaces.js" as Interfaces

const BITS = 8;
const MEGA = 1e6;
const KILO = 1000;
const TENS = 10;
const BARS = 5;
const SECOND = 1000;
const DAY_MS = 86_400_000;
// Past this many seconds ago, a moment is said in minutes and hours.
const SECONDS_SAID = 100;
// One decimal for a rate or a speed test's figure, as eero gives them.
const PLACES = 1;
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

// A rate or a speed test's figure in Mb/s, to eero's tenth.
function mbps(value, settings) {
    return value === null || value === undefined ? "" : number(value, PLACES, settings);
}

// A device's speed now, given in Mb/s, as the machine's own Speed says a
// rate: in b/s, kb/s, Mb/s or Gb/s, whichever reads.
function speed(value, settings) {
    if (value === null || value === undefined) {
        return "";
    }
    return Traffic.bits((value * MEGA) / BITS, (figure, places) => number(figure, places, settings));
}

// Bytes in SI units, one decimal under ten: "940 kB", "2.4 MB".
function bytes(value, settings) {
    if (value === null || value === undefined) {
        return "";
    }
    let scaled = value;
    let unit = 0;
    while (scaled >= KILO && unit < BYTE_UNITS.length - 1) {
        scaled /= KILO;
        unit += 1;
    }
    return `${number(scaled, scaled < TENS && unit > 0 ? 1 : 0, settings)} ${BYTE_UNITS[unit]}`;
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

// The nodes as the place lists them: the gateway node first, then each
// node after the one it hangs from, as the backhaul runs, siblings by
// location, and any whose upstream eero didn't name last.
function nodesInOrder(nodes) {
    const byName = (a, b) => nodeName(a).localeCompare(nodeName(b));
    const listed = [];
    const placed = new Set();
    const under = (node) => {
        listed.push(node);
        placed.add(node.id);
        const below = nodes.filter((other) => !placed.has(other.id)
            && other.backhaul?.upstream === node.location);
        for (const other of below.sort(byName)) {
            under(other);
        }
    };
    for (const node of nodes.filter((one) => one.gateway).sort(byName)) {
        under(node);
    }
    for (const node of nodes.slice().sort(byName)) {
        if (!placed.has(node.id)) {
            under(node);
        }
    }
    return listed;
}

// Each node in that order with its devices, as eero orders them (by node,
// the strongest signal first), and the devices on no node eero listed,
// as one group with no node, last.
function grouped(model) {
    const nodes = nodesInOrder(model?.nodes ?? []);
    const devices = model?.devices ?? [];
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
    return way.kind === "ethernet" || way.bars === null || way.bars === undefined
        ? null
        : way.bars / BARS;
}

// A count with its noun, singular for one: "1 device", "9 devices".
function counted(count, noun) {
    return `${count} ${count === 1 ? noun : `${noun}s`}`;
}

// What a node's row says: its backhaul at its side, and after its name
// whether it's the gateway node and how it's doing, its model, its
// devices and its firmware.
function nodeRow(node) {
    const state = stateOf(node);
    const devices = node.devices?.all;
    const note = [
        state !== "" && state !== "online" ? state : "",
        node.gateway ? "gateway node" : "",
        node.model ?? "",
        devices === null || devices === undefined ? "" : counted(devices, "device"),
        node.firmware ?? "",
    ].filter((part) => part !== "");
    return {
        name: nodeName(node),
        side: backhaulOf(node),
        note: note.join(" \u00b7 "),
        level: backhaulLevel(node),
    };
}

// A device's band and channel: "5 GHz, channel 52".
function bandOf(device) {
    if (device.ethernet) {
        return "Ethernet";
    }
    const channel = device.channel === null || device.channel === undefined
        ? ""
        : `channel ${device.channel}`;
    return [device.band ?? "", channel].filter((part) => part !== "").join(", ");
}

// Figures each way in Mb/s, as one: "1,200.9 Mb/s down, 1,080.6 Mb/s up",
// or one way where eero says only one.
function eachWay(down, up, settings) {
    const ways = [
        mbps(down, settings) === "" ? "" : `${mbps(down, settings)} Mb/s down`,
        mbps(up, settings) === "" ? "" : `${mbps(up, settings)} Mb/s up`,
    ];
    return ways.filter((way) => way !== "").join(", ");
}

// A device's rate each way.
function ratesOf(device, settings) {
    return eachWay(device.rx?.mbps, device.tx?.mbps, settings);
}

// A device's speed now each way, "1.2 Mb/s down, 24 kb/s up", or idle
// when it moves nothing either way.
function speedOf(device, settings) {
    const now = device.speed ?? {};
    if (now.down_mbps === 0 && now.up_mbps === 0) {
        return "idle";
    }
    const down = speed(now.down_mbps, settings);
    const up = speed(now.up_mbps, settings);
    return [down === "" ? "" : `${down} down`, up === "" ? "" : `${up} up`]
        .filter((way) => way !== "")
        .join(", ");
}

// A signal in dBm, as Wi-Fi says one.
function signalOf(device) {
    return device.signal_dbm === null || device.signal_dbm === undefined
        ? ""
        : `${device.signal_dbm} dBm`;
}

// What a device's row says: its signal at its side, as a bar too, and
// after its name its band and channel, its rate each way and its speed
// now.
function deviceRow(device, settings) {
    const signal = signalOf(device);
    return {
        name: nameOf(device),
        side: device.ethernet ? "Ethernet" : signal,
        note: [bandOf(device), ratesOf(device, settings), speedOf(device, settings)]
            .filter((part) => part !== "")
            .join(" \u00b7 "),
        level: device.signal_dbm === null || device.signal_dbm === undefined || device.ethernet
            ? null
            : Interfaces.level(device.signal_dbm),
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
    return [`${counted(online, "node")} online`].concat(rest).join(", ");
}

// The last speed test, "369.9 Mb/s down, 11.9 Mb/s up".
function speedTestOf(test, settings) {
    return test ? eachWay(test.down_mbps, test.up_mbps, settings) : "";
}

// The outages eero recorded, in a few words.
function outagesOf(outages) {
    const count = (outages ?? []).length;
    return count === 0 ? "no outages" : counted(count, "outage");
}

// What the internet's row says, the place's head: its state at its side,
// and after its name who carries it, the last speed test and when, how
// it's optimized, its outages, the nodes by state and how many devices
// eero counts. Asked at `now`, in milliseconds, for when.
function internetRow(model, now, settings) {
    const internet = model?.internet ?? {};
    const test = internet.speed_test ?? null;
    const tested = speedTestOf(test, settings);
    const day = test?.at ? ` ${dayOf(test.at, now)}` : "";
    const optimized = internet.sqm === null || internet.sqm === undefined
        ? ""
        : `conferencing and gaming ${internet.sqm ? "on" : "off"}`;
    const devices = internet.devices === null || internet.devices === undefined
        ? ""
        : counted(internet.devices, "device");
    return {
        name: "Internet",
        side: stateWords(model),
        note: [
            internet.isp ?? "",
            tested === "" ? "" : `${tested}${day}`,
            optimized,
            outagesOf(internet.outages),
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

// A day as the launcher says one beside the time it is now: "today",
// "yesterday", "on 5 Nov", or "on 13 Nov 2020" in another year.
function dayOf(iso, now) {
    const at = new Date(iso);
    const today = new Date(now);
    const days = Math.round((startOf(today) - startOf(at)) / DAY_MS);
    if (days === 0) {
        return "today";
    }
    if (days === 1) {
        return "yesterday";
    }
    return at.getFullYear() === today.getFullYear()
        ? `on ${Qt.formatDate(at, "d MMM")}`
        : `on ${Qt.formatDate(at, "d MMM yyyy")}`;
}

// A day's first moment, in milliseconds.
function startOf(date) {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
}

// A moment as a drawer says it: its time today, else its day and time
// this year, else its day.
function when(iso, now) {
    if (!iso) {
        return "";
    }
    const at = new Date(iso);
    if (Number.isNaN(at.getTime())) {
        return "";
    }
    const day = dayOf(iso, now);
    const time = Qt.formatTime(at, "HH:mm");
    if (day === "today") {
        return time;
    }
    if (day === "yesterday") {
        return `yesterday ${time}`;
    }
    return new Date(now).getFullYear() === at.getFullYear()
        ? `${Qt.formatDate(at, "d MMM")} ${time}`
        : Qt.formatDate(at, "d MMM yyyy");
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

// How many nodes and devices the place holds, as its row says at home.
function holding(model) {
    const nodes = (model?.nodes ?? []).length;
    const devices = (model?.devices ?? []).length;
    return `${counted(nodes, "node")} \u00b7 ${counted(devices, "device")}`;
}

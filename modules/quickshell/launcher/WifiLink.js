// A Wi-Fi card's link, made sense of: what iw reads of it (its signal,
// rates, beacons and framing), and each way of it as wifid and iw together
// say it. Wireless.qml runs iw and asks wifid; everything here is plain
// reading of what they said, so it's tested offscreen (tests/answers.qml).
// The machine's other connections are Interfaces.js's.
.pragma library
.import "Interfaces.js" as Interfaces

const FIELD = /^\s*([^:]+):\s*(.*)$/;
const SECTION = /^@@ (\w+)$/;
const NUMBER = /-?\d+(?:\.\d+)?/;
const CHAINS = /\[([^\]]*)\]/;
const RATE = /([\d.]+) MBit\/s/;
const WIDTH = /(\d+)MHz/;
const TX_POWER = /txpower ([\d.]+) dBm/;
const CHANNEL = /channel (\d+) \((\d+) MHz\)/;
// Milliseconds in a second, seconds in a minute, and the seconds under
// which one keeps a decimal.
const SECOND = 1000;
const MINUTE = 60;
const TEN_SECONDS = 10;
// Wi-Fi 4's rates name each stream count by eight indexes.
const HT_PER_STREAM = 8;
const WIFI_4 = 4;
const WIFI_5 = 5;
const WIFI_6 = 6;
const WIFI_7 = 7;
// How iw marks a rate of each newer Wi-Fi, the newest first.
const GENERATIONS = [
    { tag: "EHT", generation: WIFI_7 },
    { tag: "HE", generation: WIFI_6 },
    { tag: "VHT", generation: WIFI_5 },
];
// The guard intervals Wi-Fi 6 and 7 name by index (HE-GI 0 to 2), and the
// two Wi-Fi 4 and 5 choose between, "short GI" or not, in microseconds.
const NEWER_GUARDS = [0.8, 1.6, 3.2];
const SHORT_GUARD = 0.4;
const LONG_GUARD = 0.8;

// A number at the start of what iw says, or null.
function figure(text) {
    const found = NUMBER.exec(String(text ?? ""));
    return found ? Number(found[0]) : null;
}

// A rate as iw says it, "1200.9 MBit/s 80MHz HE-MCS 11 HE-NSS 2 HE-GI 0
// HE-DCM 0", as { mbps, width, generation, mcs, streams, guard, dcm }: HE is
// Wi-Fi 6, EHT 7, VHT 5 and HT 4, whose index says the streams too; anything
// older has no generation, and no guard interval said. Only HE says its
// dual carrier.
function rate(text) {
    const said = String(text ?? "");
    const field = (key) => {
        const found = new RegExp(`\\b${key} (\\d+)`).exec(said);
        return found ? Number(found[1]) : null;
    };
    const mbps = RATE.exec(said);
    const width = WIDTH.exec(said);
    const known = GENERATIONS.find((each) => field(`${each.tag}-MCS`) !== null);
    const coded = { generation: null, mcs: null, streams: null, guard: null, dcm: null };
    const chosen = said.includes("short GI") ? SHORT_GUARD : LONG_GUARD;
    if (known) {
        coded.generation = known.generation;
        coded.mcs = field(`${known.tag}-MCS`);
        coded.streams = field(`${known.tag}-NSS`);
        const guard = field(`${known.tag}-GI`);
        coded.guard = known.generation >= WIFI_6
            ? (guard !== null ? (NEWER_GUARDS[guard] ?? null) : null) : chosen;
        const dcm = field(`${known.tag}-DCM`);
        coded.dcm = dcm !== null ? dcm === 1 : null;
    } else if (field("MCS") !== null) {
        coded.generation = WIFI_4;
        coded.mcs = field("MCS") % HT_PER_STREAM;
        coded.streams = Math.floor(field("MCS") / HT_PER_STREAM) + 1;
        coded.guard = chosen;
    }
    return Object.assign(
        {
            mbps: mbps ? Number(mbps[1]) : null,
            width: width ? Number(width[1]) : null,
        },
        coded,
    );
}

// What iw said of each part it was asked, by the section the script heads
// it with ("@@ link", "@@ station", "@@ info"), each a map of its fields.
function sections(text) {
    const said = { link: {}, station: {}, info: {}, infoLines: [] };
    let at = "";
    for (const line of String(text).split("\n")) {
        const heading = SECTION.exec(line.trim());
        if (heading) {
            at = String(heading[1]);
        } else if (at !== "" && said[at] !== undefined) {
            const found = FIELD.exec(line);
            if (found && said[at][found[1].trim()] === undefined) {
                said[at][found[1].trim()] = found[2].trim();
            }
            if (at === "info") {
                said.infoLines.push(line.trim());
            }
        }
    }
    return said;
}

// A Wi-Fi card's link as iw reads it: its signal now, at each antenna, on
// average and of the access point's beacons, in dBm; what it transmits at;
// how many frames it sent again, couldn't send and dropped, and beacons it
// missed; how long it's been on, in seconds; whether its management frames
// are protected; how often the access point's beacons come, in its time
// units, and one in how many says what it holds for a card asleep (DTIM);
// the preamble its frames open with, whether it waits the short slot time
// and sorts traffic by kind (WMM); and its rates each way. Null when it's
// on no network.
function radio(text) {
    const said = sections(text);
    const { station } = said;
    if (Object.keys(station).length === 0) {
        return null;
    }
    const chains = CHAINS.exec(station.signal ?? "");
    const info = said.infoLines.join("\n");
    const power = TX_POWER.exec(info);
    const channel = CHANNEL.exec(info);
    return {
        signal: figure(station.signal),
        antennas: chains ? chains[1].split(",").map((each) => Number(each.trim())) : [],
        average: figure(station["signal avg"]),
        beacons: figure(station["beacon signal avg"]),
        power: power ? Number(power[1]) : null,
        retries: figure(station["tx retries"]),
        failed: figure(station["tx failed"]),
        missed: figure(station["beacon loss"]),
        dropped: figure(station["rx drop misc"]),
        connected: figure(station["connected time"]),
        protected: yes(station.MFP),
        interval: figure(station["beacon interval"]),
        dtim: figure(station["DTIM period"]),
        preamble: station.preamble ?? null,
        shortSlot: yes(station["short slot time"]),
        wmm: yes(station["WMM/WME"]),
        channel: channel ? Number(channel[1]) : null,
        frequency: figure(said.link.freq),
        receive: rate(station["rx bitrate"]),
        send: rate(station["tx bitrate"]),
    };
}

// Whether iw said yes of something, or null when it said nothing of it.
function yes(text) {
    return text === undefined ? null : text === "yes";
}

// One way of a card's link, as wifid read it, with what iw read of it
// (a rate, as rate() makes it) filling whatever wifid doesn't say, such as
// each way's width, or the coding of Wi-Fi 7, which wpa_supplicant doesn't
// read: { mbps, width, generation, mcs, streams, guard, dcm, bytes,
// packets }, each null where neither says.
function way(wifid, iw) {
    const keys = [
        "mbps",
        "width",
        "generation",
        "mcs",
        "streams",
        "guard",
        "dcm",
        "bytes",
        "packets",
    ];
    const said = {};
    for (const key of keys) {
        said[key] = wifid?.[key] ?? iw?.[key] ?? null;
    }
    return said;
}

// A figure each way, as the network's speed says one: once when both ways
// agree, "MCS 10 in \u00b7 MCS 9 out" when they don't, "MCS 9 out" when only
// one way is known, and "" when neither is. `say` writes one way's.
function eachWay(inward, outward, say) {
    const known = (value) => value !== null && value !== undefined;
    if (!known(inward)) {
        return known(outward) ? `${say(outward)} out` : "";
    }
    if (!known(outward)) {
        return `${say(inward)} in`;
    }
    return inward === outward ? say(inward) : `${say(inward)} in \u00b7 ${say(outward)} out`;
}

// How long since a frame came or went, from milliseconds: "1 ms", "2.4 s",
// or as long as Interfaces.lasted() says, for a card idle a minute or more.
function idle(milliseconds, number) {
    if (milliseconds < SECOND) {
        return `${Math.round(milliseconds)} ms`;
    }
    if (milliseconds < MINUTE * SECOND) {
        const seconds = milliseconds / SECOND;
        return `${number(seconds, seconds < TEN_SECONDS ? 1 : 0)} s`;
    }
    return Interfaces.lasted(Math.floor(milliseconds / SECOND));
}

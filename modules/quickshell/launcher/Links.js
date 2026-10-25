// What the launcher's Bluetooth makes of how each link is doing, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// the file the machine's link reader writes (modules/bluetooth-links), read
// into each link's signal and how well it carries, with their last reads,
// which channels it skips as noisy and how it's held, and each adapter's
// traffic, version and maker; and the codec PipeWire names, in its usual
// spelling, with what it carries. Accessories.qml reads the file and draws
// the rows.
.pragma library

const PERCENT = 100;
// Link quality is said from 0 to 255.
const QUALITY_MOST = 255;
// A Low Energy link's signal in dBm, from one too faint to hold to one
// beside the machine, for its bar.
const FAINTEST = -100;
const LOUDEST = -30;
// A classic link's signal is said in dB from the range its radio hears
// best; this far below it, its bar is empty.
const BELOW_MOST = 40;
// A strip draws twenty places.
const PLACES = 20;

// Each codec PipeWire's Bluetooth names, as it's usually written, and
// those only a call uses, which sound worse than music's.
const CODECS = [
    ["sbc", "SBC"], ["sbc_xq", "SBC-XQ"], ["aac", "AAC"], ["aac_eld", "AAC-ELD"],
    ["ldac", "LDAC"], ["aptx", "aptX"], ["aptx_hd", "aptX HD"],
    ["aptx_ll", "aptX Low Latency"], ["aptx_ll_duplex", "aptX Low Latency"],
    ["faststream", "FastStream"], ["faststream_duplex", "FastStream"],
    ["lc3plus_hr", "LC3plus"], ["lc3", "LC3"], ["cvsd", "CVSD"], ["msbc", "mSBC"],
    ["lc3_swb", "LC3-SWB"]
];
const CALL_CODECS = ["cvsd", "msbc", "lc3_swb"];
// Each kind of Opus the headphones and PipeWire agree on is Opus.
const OPUS = /^opus/;
const IN_USE = "1";

// What the file says, or null when it isn't the reader's file.
function read(text) {
    let said = null;
    try {
        said = JSON.parse(text);
    } catch (_error) {
        return null;
    }
    return said && Array.isArray(said.links) ? said : null;
}

// Every link the file says, or null when it isn't the reader's file: no
// reader, so nothing is said of any link.
function links(text) {
    const said = read(text);
    return said ? said.links : null;
}

// Every adapter the file says, or none.
function adapters(text) {
    const said = read(text);
    return said && Array.isArray(said.adapters) ? said.adapters : [];
}

// An adapter by the kernel's name for it (hci0), or null.
function adapter(all, name) {
    return (all ?? []).find(each => each.name === name) ?? null;
}

// The link to a device, by its address, or null.
function of(all, address) {
    return (all ?? []).find(link => link.address === address) ?? null;
}

function kept(value) {
    return Math.max(0, Math.min(1, value));
}

function figure(value) {
    return typeof value === "number" && Number.isFinite(value);
}

// How strong the signal is: what its row says, its bar from 0 to 1 and
// its note, or null when the controller didn't say. A classic link says
// how far it is from the range its radio hears best, a Low Energy link
// the signal itself. How loud this end sends goes in the note.
function signal(link) {
    if (!(link && figure(link.rssi))) {
        return null;
    }
    const { rssi } = link;
    const sending = figure(link.power) ? `sending at ${link.power} dBm` : "";
    const join = parts => parts.filter(part => part !== "").join(", ");
    if (link.le) {
        return {
            side: `${rssi} dBm`,
            value: kept((rssi - FAINTEST) / (LOUDEST - FAINTEST)),
            note: sending
        };
    }
    if (rssi === 0) {
        return { side: "in range", value: 1, note: join(["where its radio hears best", sending]) };
    }
    if (rssi > 0) {
        return {
            side: `+${rssi} dB`,
            value: 1,
            note: join(["above the range its radio hears best", sending])
        };
    }
    return {
        side: `${rssi} dB`,
        value: kept(1 + rssi / BELOW_MOST),
        note: join(["below the range its radio hears best", sending])
    };
}

// A history's reads, each the controller said, the newest last: those it
// didn't are left out, since a gap isn't a reading of nothing.
function reads(link, field) {
    const history = link?.history?.[field];
    const said = Array.isArray(history) ? history.filter(figure) : [];
    return said.length > 0 || !(link && figure(link[field])) ? said : [link[field]];
}

// The signal's last reads as a spark draws them, each as its bar would be.
function signals(link) {
    return reads(link, "rssi").map(rssi => signal({ rssi, le: link.le }).value);
}

// The link quality's last reads as a spark draws them.
function qualities(link) {
    return reads(link, "quality").map(each => kept(each / QUALITY_MOST));
}

// How well the link carries, as a share of the 255 its controller counts
// to, or null.
function quality(link) {
    if (!(link && figure(link.quality))) {
        return null;
    }
    const share = kept(link.quality / QUALITY_MOST);
    return {
        side: `${Math.round(share * PERCENT)}%`,
        value: share,
        note: "as its radio judges it"
    };
}

// Which channels the link hops over, as a strip: each place the share of
// its channels in use, so a band skipped shows as a gap where it sits.
function strip(map) {
    const per = Math.ceil(map.length / PLACES);
    const values = [];
    for (let at = 0; at < map.length; at += per) {
        const part = map.slice(at, at + per);
        values.push(part.split("").filter(channel => channel === IN_USE).length / part.length);
    }
    return values;
}

// How many channels are clear and how many skipped, or null.
function channels(link) {
    const map = link && typeof link.map === "string" ? link.map : "";
    if (map === "") {
        return null;
    }
    const clear = map.split("").filter(channel => channel === IN_USE).length;
    const skipped = map.length - clear;
    return {
        side: `${clear} of ${map.length} clear`,
        values: strip(map),
        clear,
        note: skipped === 0 ? "the link skips none as noisy"
            : `the link skips the other ${skipped} as noisy`
    };
}

// How a link is held, as its Connection says it: over which radio, which
// end leads it and how well it's kept from others, then who started it, or
// null when the reader doesn't say.
const KEPT = {
    secure: ["encrypted", ", with Secure Connections\u2019 keys"],
    encrypted: ["encrypted", ", with older keys"],
    authenticated: ["not encrypted", ", though the device proved who it is"],
    none: ["not encrypted", ""]
};

function held(link) {
    const how = link?.held;
    if (!how) {
        return null;
    }
    const [side, more] = KEPT[how.security] ?? ["", ""];
    const role = how.central ? "central" : "peripheral";
    return {
        side: [link.le ? "Low Energy" : "Classic", role, side].filter(part => part !== "")
            .join(", "),
        note: `${how.outgoing ? "this machine" : "the device"} started it${more}`
    };
}

// The version of Bluetooth a controller follows, by the number it says,
// as the specification is named, or "".
const VERSIONS = ["1.0b", "1.1", "1.2", "2.0", "2.1", "3.0", "4.0", "4.1", "4.2", "5.0",
    "5.1", "5.2", "5.3", "5.4", "6.0", "6.1", "6.2"];

function version(number) {
    return figure(number) && VERSIONS[number] ? `Bluetooth ${VERSIONS[number]}` : "";
}

// Who made a controller, by the number the Bluetooth SIG gives each
// company, for those that make laptops' and dongles' radios, or "".
const MAKERS = {
    2: "Intel", 10: "Qualcomm", 13: "Texas Instruments", 15: "Broadcom", 18: "Zeevo",
    29: "Qualcomm", 48: "STMicroelectronics", 70: "MediaTek", 93: "Realtek", 305: "Cypress",
    1521: "Infineon"
};

function maker(number) {
    return MAKERS[number] ?? "";
}

// Bytes a second, said in bits a second, as the network's speed is
// (Traffic.js's bits), since Bluetooth is a link too.
const KILO = 1e3;
const MEGA = 1e6;
const GIGA = 1e9;
const BITS = 8;
const UNITS = [[GIGA, "Gb/s"], [MEGA, "Mb/s"], [KILO, "kb/s"]];
const TENS = 10;

function rate(count) {
    const bits = count * BITS;
    const unit = UNITS.find(each => bits >= each[0]);
    if (!unit) {
        return `${Math.round(bits)} b/s`;
    }
    const scaled = bits / unit[0];
    return `${scaled < TENS ? scaled.toFixed(1) : Math.round(scaled)} ${unit[1]}`;
}

// The least a traffic spark's top stands for, so the radio's few bytes a
// second of housekeeping draw as almost nothing.
const LEAST_TRAFFIC = 1e3;

// What an adapter carries: its row's words, a spark of its last reads,
// each to the most of them, and its note, or null before it's been read
// twice.
function traffic(told) {
    if (!(told && figure(told.received) && figure(told.sent))) {
        return null;
    }
    const said = Array.isArray(told.traffic) ? told.traffic.filter(figure) : [];
    const most = Math.max(LEAST_TRAFFIC, ...said);
    return {
        side: rate(told.received + told.sent),
        values: said.map(each => each / most),
        note: `${rate(told.received)} in, ${rate(told.sent)} out`
    };
}

// What a codec carries, as its parts say it, from the properties of the
// PipeWire node it's sent from and how many channels the node has: what
// it's for, music or calls, its sample rate and its channels, each ""
// when the node doesn't say.
const PROFILES = [
    [/^a2dp/, "music"], [/^bap/, "LE Audio"], [/^(headset|hfp|hsp)/, "calls"]
];
const LAYOUTS = ["", "mono", "stereo"];
// A node says its rate as audio.rate or as its clock, 1/48000.
const CLOCK = /^1\/(\d+)$/;

function carried(properties, count) {
    const said = properties ?? {};
    const profile = PROFILES.find(each => each[0].test(said["api.bluez5.profile"] ?? ""));
    const clock = CLOCK.exec(String(said["node.rate"] ?? ""));
    const hertz = Number(said["audio.rate"] ?? (clock ? clock[1] : 0));
    const many = Number(said["audio.channels"] ?? count ?? 0);
    const layout = LAYOUTS[many] ?? "";
    return {
        profile: profile ? profile[1] : "",
        rate: hertz > 0 ? `${hertz / KILO} kHz` : "",
        channels: many > 0 ? [String(many), layout].filter(part => part !== "").join(", ") : ""
    };
}

// A codec as it's usually written, and whether it's a call's, or null.
function codec(id) {
    if (typeof id !== "string" || id === "") {
        return null;
    }
    const known = CODECS.find(each => each[0] === id);
    const note = CALL_CODECS.includes(id) ? "call quality, while its microphone is in use" : "";
    if (known) {
        const [, name] = known;
        return { name, note };
    }
    return { name: OPUS.test(id) ? "Opus" : id.toUpperCase(), note };
}

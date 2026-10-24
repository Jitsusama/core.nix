// What the launcher's Bluetooth makes of how each link is doing, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// the file the machine's link reader writes (modules/bluetooth-links), read
// into each link's signal, how well it carries and which channels it skips
// as noisy, and the codec PipeWire names, in its usual spelling.
// Accessories.qml reads the file and draws the rows.
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

// Every link the file says, or null when it isn't the reader's file: no
// reader, so nothing is said of any link.
function links(text) {
    let said = null;
    try {
        said = JSON.parse(text);
    } catch (_error) {
        return null;
    }
    return said && Array.isArray(said.links) ? said.links : null;
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

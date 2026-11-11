// A Wi-Fi card's link, made sense of: what the kernel says of it beyond
// its rates (its antennas, beacons and framing) and each way of it, as
// wifid reads them (wifid's detail.rs and nl80211.rs). Everything here is
// plain reading of what wifid said, so it's tested offscreen
// (tests/answers.qml). The machine's other connections are Interfaces.js's.
.pragma library
.import "Interfaces.js" as Interfaces

const WPA3 = /WPA3/;
const WPA2 = /WPA2/;
const WEP = /WEP/;
// A width as wpa_supplicant says it, "20 MHz (no HT)" or "80+80 MHz": the
// width, and what it says after it, of which "no HT" is a radio older than
// Wi-Fi 4.
const WIDTH_SAID = /^\s*(\d+(?:\+\d+)?)\s*MHz(.*)$/;
const NO_HT = /no HT/;
// Milliseconds in a second, seconds in a minute, and the seconds under
// which one keeps a decimal.
const SECOND = 1000;
const MINUTE = 60;
const TEN_SECONDS = 10;

// A card's link as the kernel says it, from the card in wifid's detail
// (`joined`): its signal now, at each antenna, on average and of the
// access point's beacons, in dBm; what it transmits at; how many frames it
// sent again, couldn't send and dropped, and beacons it missed; how long
// it's been on, in seconds; whether its management frames are protected;
// how often the access point's beacons come, in its time units, and one in
// how many says what it holds for a card asleep (DTIM); the preamble its
// frames open with, whether it waits the short slot time and sorts traffic
// by kind (WMM); and its channel. Null until the kernel's answered for it.
function station(joined) {
    const kernel = joined?.station;
    if (!kernel) {
        return null;
    }
    const known = (value) => value ?? null;
    const preamble = known(kernel.shortPreamble);
    return {
        signal: known(joined.signal),
        antennas: kernel.chains ?? [],
        average: known(joined.average),
        beacons: known(kernel.beacons),
        power: known(kernel.power),
        retries: known(kernel.retries),
        failed: known(kernel.failed),
        missed: known(kernel.missed),
        dropped: known(kernel.dropped),
        connected: known(kernel.connected),
        protected: known(kernel.protected),
        interval: known(kernel.interval),
        dtim: known(kernel.dtim),
        preamble: preamble === null ? null : preamble ? "short" : "long",
        shortSlot: known(kernel.shortSlot),
        wmm: known(kernel.wmm),
        channel: known(kernel.channel),
        frequency: known(joined.frequency),
    };
}

// One way of a card's link, as wifid read it: { mbps, width, generation,
// mcs, streams, guard, dcm, bytes, packets }, each null where it doesn't
// say.
function way(wifid) {
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
        said[key] = wifid?.[key] ?? null;
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

// How hard a network's protection is to break, by its standard as wifid
// names it: WPA3 strong, WPA2 good, WEP broken, and anything else weak.
function strength(standard) {
    const said = String(standard ?? "");
    if (WPA3.test(said)) {
        return "strong";
    }
    if (WPA2.test(said)) {
        return "good";
    }
    return WEP.test(said) ? "broken" : "weak";
}

// What had to be sent again or never was since the card joined, as the
// kernel counts it: "none", or "12 sent again \u00b7 1 never sent", each
// counted as `number` writes it.
function retried(retries, failed, number) {
    const never = failed ?? 0;
    return retries + never === 0
        ? "none"
        : `${number(retries)} sent again \u00b7 ${number(never)} never sent`;
}

// The network connected's drawer, from what Wireless.qml read of the card
// on it (`known`, each figure already written, "" or null where it isn't
// read): the band it's on and what it is above its name; what moves first,
// its signal, speed, traffic and retries; then how the link is made, its
// generation, coding, streams, width and channel; then the access points
// heard and how it's protected; how long it's been connected; and the
// access point's hardware address last, an identifier (VKCF3Z, 2026-11-07:
// what moves most first, identifiers last). Each fact is named by its id
// (`named`), and one it doesn't say is left out, never said as unknown.
// `said` is the vocabulary's wifi:drawer words.
function drawer(known, named, said) {
    const dotted = (parts) =>
        parts.filter((part) => part !== null && part !== undefined && part !== "").join(" \u00b7 ");
    const fact = (id, value) => ({ label: named(id), value: String(value ?? "") });
    const margin =
        known.margin === null || known.margin === undefined
            ? ""
            : said.over.replace("{margin}", known.margin);
    const since = known.since ? said.since.replace("{time}", known.since) : "";
    const facts = [
        fact("signal", dotted([known.signal, margin])),
        fact("speed", known.speed),
        fact("expected", known.expected),
        fact("carried", known.carried),
        fact("retries", known.retries),
        fact("generation", known.generation),
        fact("coding", known.coding),
        fact("streams", known.streams),
        fact("width", known.width),
        fact("channel", known.channel),
        fact("heard", known.heard),
        fact("protection", known.protection),
        fact("standard", known.standard),
        fact("connected", dotted([known.connected, since])),
        // Its identifier in the Name face (DrawerPage.faceOf).
        Object.assign(fact("bssid", known.bssid), { identifies: true }),
    ].filter((one) => one.value.trim() !== "");
    return {
        eyebrow: known.band ? said.eyebrow.replace("{band}", known.band) : said.kind,
        // Set as written, since a unit is never re-cased: "on 2.4 GHz"
        // (VKCF3Z, 2026-11-09).
        eyebrowNames: true,
        title: known.name,
        facts,
    };
}

// A width as wpa_supplicant says it, said as ours: "20 MHz (no HT)" is
// { width: "20 MHz", legacy: true }, the radio older than Wi-Fi 4 its
// generation's to say; what isn't a width stays as it was said.
function widthOf(said) {
    const found = WIDTH_SAID.exec(String(said ?? ""));
    if (!found) {
        return { width: String(said ?? "").trim(), legacy: false };
    }
    return { width: `${found[1]} MHz`, legacy: NO_HT.test(found[2]) };
}

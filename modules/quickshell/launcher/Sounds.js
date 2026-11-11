// What the launcher's Sound makes of what a device and a stream say of
// themselves, in plain JavaScript so a check can run it in Qt's own engine,
// as the shell does: which device a filter puts itself in front of, the
// chips a sound card names, how much a stream asks for at once and at what
// rate, a channel's name as it's said, a level in decibels, whether a
// device's jack has anything plugged in and whether that's still current,
// the ports its card offers it and how they're read, one at a time, and
// what a Bluetooth device's sound is for. Sound.qml reads the nodes and
// draws the rows.
.pragma library

// One "key = value" of what a smart filter aims at, as the sound server
// writes it, or as JSON with a colon.
const AIMED = /"?([A-Za-z0-9_.-]+)"?\s*[=:]\s*"([^"]*)"/g;
// A stream's "frames/rate", as it asks for its latency or says its rate.
const FRACTION = /^\s*(\d+)\s*\/\s*(\d+)\s*$/;
// pw-top's marks for a node running, as against idle (I), suspended (S)
// or created (C): R, or T and t where its transport runs or starts, as a
// JACK client's does (pw-top.c's state_as_string).
const RUNNING = ["R", "T", "t"];
// Where pw-top's batch line holds each figure: its state, id, quantum,
// rate, wait and busy times, those as shares of the cycle (W/Q, B/Q),
// then its error count; its format and name follow, a follower's name
// after a lone "+", or "=" where it follows asynchronously (pw-top.c's
// do_refresh).
const COLUMN = { state: 0, id: 1, quantum: 2, rate: 3, busy: 5, errors: 8 };
const WHOLE = /^\d+$/;
const FOLLOWS = ["+", "="];
// A time as pw-top prints it, to a tenth of its unit; "---" and "???"
// say it has none, and "+++" that the node didn't finish in its cycle.
const TIME = /^(\d+(?:\.\d+)?)(us|ms|s)$/;
const SECONDS = { us: 1e-6, ms: 1e-3, s: 1 };
const UNFINISHED = "+++";
const HEADER = /^\s*S\s+ID\s+QUANT\b/;
const PERCENT = 100;
// A USB card names itself by its ids.
const USB_IDS = /^USB([0-9a-f]{4}):([0-9a-f]{4})$/i;
// A channel's name runs its words together: FrontLeft.
const WORD_START = /([a-z0-9])([A-Z])/g;
const SPACES = /\s+/;
const FRONT = /^front /;
// A microphone chip's digital microphones are named after it.
const DIGITAL = /-dmic$/;
const HERTZ_PER_KILOHERTZ = 1000;
const MILLISECONDS = 1000;
// A rate names a format, as "44.1 kHz" or "176 kHz", in three significant
// digits without the zeros a measure keeps, since a name keeps its own digits
// (VKCF3Z, 2026-11-08) and the card's own figures say it so.
const SIGNIFICANT = 3;
// A sample format as the sound server names it short: signed, unsigned or
// floating, its bits, then how it's laid out, S24_32LE or F32LE.
const SAMPLE = /^([SUF])(\d+)/;
// The units a figure is said in, as desk-facts says a card's.
const KILOHERTZ = "kHz";
const BITS = "bit";
const FLOATING = "float";
const APART = " \u00b7 ";
// A level is a share of the whole cubed, as the sound server and wpctl
// give it, so its gain in decibels is twenty times the log of the cube.
const DECIBELS_CUBED = 60;
const TENTHS = 10;
// What a Bluetooth device's profile carries, as the vocabulary says it.
const CARRIES = [[/^a2dp/, "music"], [/^(headset|hfp|hsp)/, "calls"], [/^bap/, "le-audio"]];
// WirePlumber's stand-in for a headset's microphone says whose it is only
// in its name.
const STAND_IN = /^bluez_input\.([0-9A-F]{2}(?::[0-9A-F]{2}){5})$/;
// The machine's own speakers, microphones and jack, by the port the sound
// server names them for, and the vocabulary's word for each: its own
// names ("Speaker", "Microphones") read as one speaker, many microphones,
// and nothing of where they are.
const BUILT_IN = [
    [/HiFi__Speaker__sink$/, "speakers"],
    [/HiFi__D?Mic\d*__source$/, "microphones"],
    [/HiFi__Headphones__sink$/, "jack"],
    [/HiFi__Headset__source$/, "jack-microphone"],
];
// A Bluetooth headset's microphone, which the sound server names as it
// does the headset itself.
const HEADSET_INPUT = /^bluez_input\./;

// What a smart filter's target says a device must have, by property:
// { "node.name": "alsa_output…" }, or nothing when it says nothing.
function target(text) {
    const wanted = {};
    const aimed = new RegExp(AIMED.source, "g");
    const said = String(text ?? "");
    for (let found = aimed.exec(said); found !== null; found = aimed.exec(said)) {
        wanted[found[1]] = found[2];
    }
    return wanted;
}

// Whether a device's properties are all a filter's target asks for; a
// target asking nothing aims at nothing.
function aims(wanted, properties) {
    const keys = Object.keys(wanted);
    return keys.length > 0 && keys.every((key) => String(properties[key] ?? "") === wanted[key]);
}

// The chips a sound card's components name, in capitals as their makers
// write them: { amplifiers, speaker, headset, microphone, digital, usb }.
// "cfg-amp:4 hs:cs42l45 mic:cs42l45-dmic spk:cs35l56" is four CS35L56
// amplifiers, a CS42L45 for the headset and its digital microphones.
function components(text) {
    const chips = { amplifiers: 0, speaker: "", headset: "", microphone: "", digital: false, usb: "" };
    for (const word of String(text ?? "").split(SPACES)) {
        const usb = USB_IDS.exec(word);
        const at = word.indexOf(":");
        const key = at < 0 ? "" : word.slice(0, at);
        const value = word.slice(at + 1);
        if (usb) {
            chips.usb = `${usb[1]}:${usb[2]}`.toLowerCase();
        } else if (key === "cfg-amp") {
            chips.amplifiers = Number(value) || 0;
        } else if (key === "spk") {
            chips.speaker = value.toUpperCase();
        } else if (key === "hs") {
            chips.headset = value.toUpperCase();
        } else if (key === "mic") {
            chips.digital = DIGITAL.test(value);
            chips.microphone = value.replace(DIGITAL, "").toUpperCase();
        }
    }
    return chips;
}

// A rate in Hz as it's said: 44100 is "44.1 kHz", 48000 "48 kHz".
function kilohertz(hertz) {
    const whole = hertz % HERTZ_PER_KILOHERTZ === 0;
    return `${(hertz / HERTZ_PER_KILOHERTZ).toFixed(whole ? 0 : 1)} kHz`;
}

// What a stream's "frames/rate" says: { frames, hertz, ms }, or null.
function fraction(text) {
    const found = FRACTION.exec(String(text ?? ""));
    if (!found || Number(found[2]) === 0) {
        return null;
    }
    const frames = Number(found[1]);
    const hertz = Number(found[2]);
    return { frames, hertz, ms: (frames / hertz) * MILLISECONDS };
}

// A sample format's bits, as desk-facts says a card's, a floating one said
// so: S32LE is "32 bit", F32LE "32 bit float", one it can't read its
// own short name. S24_32LE is "24 bit": the bits it holds are its
// resolution, while its 32-bit container and its byte order are how
// the graph lays them in memory, which no figure Joel reads turns on,
// so they're left out, as desk-facts leaves a card's out.
function sample(name) {
    const found = SAMPLE.exec(String(name ?? ""));
    if (!found) {
        return String(name ?? "");
    }
    return [`${found[2]} ${BITS}`].concat(found[1] === "F" ? [FLOATING] : []).join(" ");
}

// The format the sound server moves a node's audio in, as pipewire's
// Format says it: "48 kHz · 32 bit float", its rate to three significant
// digits; "" while it has none. Its channels are its Channels' to say.
function graphFormat(format) {
    if (!format || !(format.rate > 0)) {
        return "";
    }
    const measured = Number((format.rate / HERTZ_PER_KILOHERTZ).toPrecision(SIGNIFICANT));
    return [`${measured} ${KILOHERTZ}`].concat(format.sample ? [sample(format.sample)] : [])
        .join(APART);
}

// Whether the card already says the format the graph moves: its running
// stream's figures, "48 kHz · 32 bit · period 21.3 ms (1024)", open with
// the same rate and bits. They're compared whole, since a graph in float
// over a card in integer is the difference Graph Format is there to show,
// and desk-facts names a float card's format by ALSA's name, FLOAT_LE.
function sameFormat(graph, card) {
    return card === graph || String(card ?? "").startsWith(`${graph}${APART}`);
}

// A time pw-top prints, in seconds, or null where it has none.
function seconds(text) {
    const found = TIME.exec(String(text ?? ""));
    return found ? Number(found[1]) * SECONDS[found[2]] : null;
}

// How a node is keeping time, from one line of `pw-top --batch-mode`:
// "R  79  1024  48000  20.1us  10.2us  0.00  0.00  0  S32LE 2 48000 name",
// its state, id, quantum in frames, clock rate, wait and busy times and
// their shares of the cycle, and its xruns since it started. Only a
// running node has a cycle: { id, quantum, rate, ms, busy, unfinished,
// errors, drives }, busy its busy time over the cycle (B/Q has too few
// places for three significant digits), null where pw-top can't say;
// null for the header, a node not running and a line it can't read. A
// follower's quantum and rate are the latency it asks for, so it takes
// its driver's, the line before it in the frame, and is null without.
function timing(line, driver) {
    const fields = String(line ?? "").trim().split(SPACES);
    const whole = [COLUMN.id, COLUMN.quantum, COLUMN.rate, COLUMN.errors]
        .every(at => WHOLE.test(fields[at] ?? ""));
    if (!RUNNING.includes(fields[COLUMN.state]) || !whole) {
        return null;
    }
    const drives = !follower(line);
    const cycle = drives ? {
        quantum: Number(fields[COLUMN.quantum]),
        rate: Number(fields[COLUMN.rate])
    } : driver ?? { quantum: 0, rate: 0 };
    if (cycle.quantum === 0 || cycle.rate === 0) {
        return null;
    }
    const length = cycle.quantum / cycle.rate;
    const busy = seconds(fields[COLUMN.busy]);
    return {
        id: Number(fields[COLUMN.id]),
        quantum: cycle.quantum,
        rate: cycle.rate,
        ms: length * MILLISECONDS,
        busy: busy === null ? null : busy / length,
        unfinished: fields[COLUMN.busy] === UNFINISHED,
        errors: Number(fields[COLUMN.errors]),
        drives
    };
}

// Whether a line of `pw-top --batch-mode` is a follower's, so a line
// that isn't, running or not, ends the driver its followers run in.
function follower(line) {
    return String(line ?? "").trim().split(SPACES).slice(COLUMN.errors + 1)
        .some(field => FOLLOWS.includes(field));
}

// Whether a line of `pw-top --batch-mode` opens its next frame, the
// header it prints over every node each second, so what the frame
// before held stands whole and a node gone from running drops out.
function heading(line) {
    return HEADER.test(String(line ?? ""));
}

// How much of each cycle a node was busy, in percent: 0.0312 is 3.12.
// It's said to three significant digits by Numbers.significant, the
// one rule every measure keeps.
function load(busy) {
    return busy * PERCENT;
}

// A cycle's length in milliseconds, said by the same rule: a quantum of
// 1024 at 48 kHz is 21.333, said 21.3.
function cycleMs(timed) {
    return timed.ms;
}

// A stream's rate, from its "1/44100".
function rate(text) {
    const said = fraction(text);
    return said && said.frames === 1 ? kilohertz(said.hertz) : "";
}

// A channel's name as the sound server spells it, said as words:
// FrontLeft is "Front left".
function channel(name) {
    const words = String(name).replace(WORD_START, "$1 $2").toLowerCase();
    return words.charAt(0).toUpperCase() + words.slice(1);
}

// A device's channels each at its level, as its drawer says them after
// its layout (VKCF3Z, 2026-11-07): `balanced` alone where every one is
// at the same level, since a part equal to the rest gives way; and
// otherwise each by its name and level, a stereo pair's without "front",
// as it has no back: ["left 100%", "right 80%"]. `names` are as channel()
// says them, `levels` as shown.
function channelLevels(names, levels, balanced) {
    if (levels.length === 0) {
        return [];
    }
    if (levels.every((level) => level === levels[0])) {
        return [balanced];
    }
    const pair = names.length === 2;
    return names.map((name, at) => {
        const said = name.toLowerCase();
        return `${pair ? said.replace(FRONT, "") : said} ${levels[at] ?? ""}`.trim();
    });
}

// Each stream by the program it's of, in the order first heard, so a
// program with two streams is one part.
function grouped(streams, nameOf) {
    const groups = [];
    for (const stream of streams) {
        const name = nameOf(stream);
        const group = groups.find((each) => each.name === name);
        if (group) {
            group.streams.push(stream);
        } else {
            groups.push({ name, streams: [stream] });
        }
    }
    return groups;
}

// A level a step louder or quieter, kept to whole steps and to the range,
// and rounded, so a twentieth more than 40% is 45% and not a hair over.
const PLACES = 6;

function stepped(level, by, step) {
    const kept = Math.min(1, Math.max(0, Math.round((level + by) / step) * step));
    return Number(kept.toFixed(PLACES));
}

// A level in decibels, to a tenth, as a mixer says it: 100% is 0 dB,
// 50% is -18.1 dB, past the whole is above it, and nothing is -\u221e dB.
function decibels(level) {
    const share = Number(level);
    if (!(share > 0)) {
        return "-\u221e dB";
    }
    const tenths = Math.round(DECIBELS_CUBED * Math.log10(share) * TENTHS) / TENTHS;
    if (tenths === 0) {
        return "0 dB";
    }
    return `${tenths > 0 ? "+" : ""}${tenths.toFixed(1)} dB`;
}

// The vocabulary's word for what a device is called, by its node's name:
// the machine's own by their port, a headset's microphone as its, or ""
// for one whose own name says what it is. A microphone that shares its
// name with an output, as a USB microphone with a headphone socket does,
// is its device's microphone too, so the two rows never read the same.
function called(name, shared) {
    const text = String(name ?? "");
    const found = BUILT_IN.find((each) => each[0].test(text));
    if (found) {
        return found[1];
    }
    return HEADSET_INPUT.test(text) || shared === true ? "its-microphone" : "";
}

// The Bluetooth address a device's sound is of, from the sound server's
// properties or, for a headset microphone's stand-in, its name, or "".
function address(name, properties) {
    const said = properties?.["api.bluez5.address"];
    if (said) {
        return String(said);
    }
    const found = STAND_IN.exec(String(name ?? ""));
    return found ? found[1] : "";
}

// Whether a device is WirePlumber's stand-in for a headset's microphone,
// which switches the headset to calls once anything listens through it.
function standIn(properties) {
    return String(properties?.["bluez5.loopback"] ?? "") === "true";
}

// What a Bluetooth profile carries, as the vocabulary words it: A2DP's
// "music", a hands-free profile's "calls", LE Audio's "le-audio", or "".
function carries(profile) {
    const found = CARRIES.find((each) => each[0].test(String(profile ?? "")));
    return found ? found[1] : "";
}

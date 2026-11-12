// What the launcher's Sound makes of what a device and a stream say of
// themselves, in plain JavaScript so a check can run it in Qt's own engine,
// as the shell does: a level in decibels and a step louder or quieter, a
// program's streams together, what a device is called, and what a
// Bluetooth device's sound is for. Sound.qml reads the nodes and draws the
// rows; what a device's drawer says, its formats, channels and timing, is
// the sound daemon's (modules/sound's device_drawer.rs).
.pragma library

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

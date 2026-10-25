// What the launcher's Sound makes of what a device and a stream say of
// themselves, in plain JavaScript so a check can run it in Qt's own engine,
// as the shell does: which device a filter puts itself in front of, the
// chips a sound card names, how much a stream asks for at once and at what
// rate, and a channel's name as it's said. Sound.qml reads the nodes and
// draws the rows.
.pragma library

// One "key = value" of what a smart filter aims at, as the sound server
// writes it, or as JSON with a colon.
const AIMED = /"?([A-Za-z0-9_.-]+)"?\s*[=:]\s*"([^"]*)"/g;
// A stream's "frames/rate", as it asks for its latency or says its rate.
const FRACTION = /^\s*(\d+)\s*\/\s*(\d+)\s*$/;
// A USB card names itself by its ids.
const USB_IDS = /^USB([0-9a-f]{4}):([0-9a-f]{4})$/i;
// A channel's name runs its words together: FrontLeft.
const WORD_START = /([a-z0-9])([A-Z])/g;
const SPACES = /\s+/;
// A microphone chip's digital microphones are named after it.
const DIGITAL = /-dmic$/;
const HERTZ_PER_KILOHERTZ = 1000;
const MILLISECONDS = 1000;

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

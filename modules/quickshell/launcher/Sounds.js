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
// A USB card names itself by its ids.
const USB_IDS = /^USB([0-9a-f]{4}):([0-9a-f]{4})$/i;
// A channel's name runs its words together: FrontLeft.
const WORD_START = /([a-z0-9])([A-Z])/g;
const SPACES = /\s+/;
// A microphone chip's digital microphones are named after it.
const DIGITAL = /-dmic$/;
const HERTZ_PER_KILOHERTZ = 1000;
const MILLISECONDS = 1000;
// A level is a share of the whole cubed, as the sound server and wpctl
// give it, so its gain in decibels is twenty times the log of the cube.
const DECIBELS_CUBED = 60;
const TENTHS = 10;
// What desk-facts says a jack has plugged into it, and its word for none.
const PLUGGED = "plugged in";
const NOTHING = "nothing";
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
// A sound card's object id, as the sound server numbers it, and all pw-dump
// is ever given: anything else could be taken for one of its options.
const OBJECT_ID = /^[0-9]+$/;
// A control group's events say whether anything still runs in it.
const POPULATED = /^populated 1$/m;

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

// What desk-facts says a device's jack has plugged into it ("nothing",
// "headphones"), or "" for a device it says no jack of, such as speakers.
function plugged(facts) {
    const found = (facts ?? []).find((fact) => String(fact.name).toLowerCase() === PLUGGED);
    return found ? String(found.value) : "";
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

// Whether a device is a jack with nothing plugged into it, as the
// headphones are while none are: its port is there, but nothing hears it.
function unplugged(facts) {
    return plugged(facts) === NOTHING;
}

// Whether a device is hidden for its jack: only while what's said of it is
// current, since an old "nothing" would hide headphones plugged in since.
function hidden(facts, current) {
    return current && unplugged(facts);
}

// Whether a control group's events (cgroup.events) say a service still runs.
function populated(text) {
    return POPULATED.test(String(text ?? ""));
}

// What pw-dump is given to read a device's card: its card's object id,
// or "" when that isn't one, and nothing is read.
function dumpable(id) {
    const said = String(id ?? "");
    return OBJECT_ID.test(said) ? said : "";
}

// Port reads, one at a time, as a read is asked, prints and ends. `reads`
// is { run, next }: the run under way, { node, text, ended, ok }, and the
// device asked for while it was, read once it's done. Each run keeps the
// device it's for, and is done only once it has both printed and ended,
// whichever comes first, so one device's ports are never taken for
// another's. An event is { asked: node }, { printed: text }, { ended: ok }
// or { gaveUp: true }, for a run that never ended of itself. It gives the
// reads after, a device to start reading now or null, and what was read,
// { node, text }, from a run that ended well, or null.
function portReads(reads, event) {
    const now = reads ?? { run: null, next: null };
    const begun = (node) => ({ node, text: null, ended: false, ok: false });
    if (event.asked !== undefined) {
        if (now.run !== null) {
            return { reads: { run: now.run, next: event.asked }, start: null, take: null };
        }
        return { reads: { run: begun(event.asked), next: null }, start: event.asked, take: null };
    }
    if (now.run === null) {
        return { reads: now, start: null, take: null };
    }
    const run = Object.assign({}, now.run);
    if (event.printed !== undefined) {
        run.text = String(event.printed);
    }
    if (event.ended !== undefined) {
        run.ended = true;
        run.ok = event.ended === true;
    }
    if (event.gaveUp) {
        run.text = run.text ?? "";
        run.ended = true;
        run.ok = false;
    }
    if (run.text === null || !run.ended) {
        return { reads: { run, next: now.next }, start: null, take: null };
    }
    const take = run.ok ? { node: run.node, text: run.text } : null;
    if (now.next === null) {
        return { reads: { run: null, next: null }, start: null, take };
    }
    return { reads: { run: begun(now.next), next: null }, start: now.next, take };
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

// What pw-dump says, as a list, or none when it isn't pw-dump's.
function dumped(text) {
    let said = null;
    try {
        said = JSON.parse(text);
    } catch (_error) {
        return [];
    }
    return Array.isArray(said) ? said : [];
}

// The ports a device's card offers it, from what pw-dump says of the card
// (its EnumRoute, Route and Profile), each { index, name, available,
// active }: those of the device's own part of the card (its
// card.profile.device), in the profile the card is in, going its way, by
// the index wpctl set-route takes. `available` is the server's "yes",
// "no" or "unknown".
function ports(text, properties, playing) {
    const said = properties ?? {};
    const card = Number(said["device.id"]);
    const device = Number(said["card.profile.device"]);
    const found = dumped(text).find((each) => each?.id === card && Boolean(each.info));
    const params = found ? (found.info.params ?? {}) : {};
    const [profile] = (params.Profile ?? []).map((each) => each.index);
    const route = (params.Route ?? []).find((each) => each.device === device);
    const way = playing ? "Output" : "Input";
    return (params.EnumRoute ?? [])
        .filter(
            (each) =>
                each.direction === way &&
                (each.devices ?? []).includes(device) &&
                (profile === undefined || (each.profiles ?? []).includes(profile)),
        )
        .map((each) => ({
            index: each.index,
            name: String(each.description || each.name || ""),
            available: String(each.available ?? "unknown"),
            active: route !== undefined && route.index === each.index,
        }));
}

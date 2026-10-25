// What the launcher's battery and light answers make of the kernel's power
// supply, USB-C and light sensor files, in plain JavaScript so a check can
// run it in Qt's own engine, as the shell does: which of a list the kernel
// marks as in use, what a USB-C port's protocol is called, what each port
// gives, the date a battery was made, the order a charge limit's two files
// are written in, and how bright a room is. Power.qml and Light.qml read the
// files and say the figures.
.pragma library

// The one of a kernel's list in brackets: "Trickle Fast [Custom]".
const MARKED = /\[([^\]]+)\]/;
const BRACKETS = /^\[|\]$/g;
const SPACES = /\s+/;

// What a list the kernel marks one of says: the one in use and all of
// them, in its order. A file saying one thing alone, as an older kernel's
// charge_type does, has that in use.
function chosen(text) {
    const said = String(text ?? "").trim();
    if (said === "") {
        return { now: "", all: [] };
    }
    const marked = MARKED.exec(said);
    const all = said.split(SPACES).map((word) => word.replace(BRACKETS, ""));
    if (marked) {
        return { now: marked[1], all };
    }
    return { now: all.length === 1 ? all[0] : "", all };
}

// The protocols a USB-C port's power supply names, as a person calls them:
// power delivery negotiated, programmable, or only Type-C's own current.
const PROTOCOLS = {
    PD: "USB PD",
    PD_PPS: "USB PD PPS",
    PD_DRP: "USB PD",
    C: "USB-C",
};

// A USB-C port's protocol, from its power supply's usb_type: "C [PD]
// PD_PPS" is USB PD; anything not named is plain USB, and nothing said is
// nothing.
function protocol(usbType) {
    const now = chosen(usbType).now;
    if (now === "") {
        return "";
    }
    return PROTOCOLS[now] ?? "USB";
}

// What a USB-C port gives, in a few words: its watts, volts, amps and
// protocol while it charges the machine, that it's the machine powering
// what's plugged in, that something's plugged in giving nothing, or that
// it's empty. `number` writes a figure to so many places.
function port(said, number) {
    if (said.online && said.volts > 0 && said.amps > 0) {
        const watts = number(said.volts * said.amps, 0);
        const figures = `${watts} W, ${number(said.volts, 0)} V at ${number(said.amps, 1)} A`;
        return said.protocol === "" ? figures : `${figures}, ${said.protocol}`;
    }
    if (said.online) {
        return said.protocol === "" ? "charging" : `charging, ${said.protocol}`;
    }
    if (said.giving) {
        return "powering what's plugged in";
    }
    if (said.partner) {
        return "plugged in, giving nothing";
    }
    return "empty";
}

// The years a battery's date can be, and the months and days of one.
const FIRST_YEAR = 1990;
const MONTHS = 12;
const DAYS = 31;

function twoDigits(value) {
    return String(value).padStart(2, "0");
}

// When a battery was made, as a date in full, or its month alone when it
// says no day, or nothing when what it says isn't a date, as some don't.
function made(year, month, day) {
    if (!(year >= FIRST_YEAR && month >= 1 && month <= MONTHS)) {
        return "";
    }
    const yearMonth = `${year}-${twoDigits(month)}`;
    if (day >= 1 && day <= DAYS) {
        return `${yearMonth}-${twoDigits(day)}`;
    }
    return yearMonth;
}

// The writes that move a charge limit to stop at `wanted`, starting again
// `gap` below it, as [file, value] in the order the kernel takes them: it
// refuses a start at or above the stop, so a limit raised moves its stop
// first and one lowered its start first. With no start file (start null),
// only the stop is written.
function limitWrites(start, stop, wanted, gap) {
    const end = ["charge_control_end_threshold", String(wanted)];
    if (start === null) {
        return [end];
    }
    const begin = ["charge_control_start_threshold", String(Math.max(0, wanted - gap))];
    return wanted >= stop ? [end, begin] : [begin, end];
}

// A figure a sysfs file gives, or `otherwise` when it gives none.
function figure(text, otherwise) {
    const said = String(text ?? "").trim();
    const value = Number(said);
    return said === "" || Number.isNaN(value) ? otherwise : value;
}

// A light sensor's lux, from its raw reading, scale and offset as IIO
// gives them; a missing scale is one and a missing offset none.
function lux(raw, scale, offset) {
    const counted = figure(raw, null);
    if (counted === null) {
        return null;
    }
    return (counted + figure(offset, 0)) * figure(scale, 1);
}

// The brightest a room is drawn as: daylight at a window.
const DAYLIGHT = 10_000;

// How bright a room is, as a bar from dark to daylight, on the scale an
// eye sees it: each tenfold step of lux the same way along.
function roomLevel(seen) {
    if (seen === null || seen <= 0) {
        return 0;
    }
    return Math.min(1, Math.log10(seen + 1) / Math.log10(DAYLIGHT + 1));
}

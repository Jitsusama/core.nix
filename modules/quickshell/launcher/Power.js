// What the launcher's battery and light answers make of the kernel's power
// supply, USB-C and light sensor files, in plain JavaScript so a check can
// run it in Qt's own engine, as the shell does: which of a list the kernel
// marks as in use, what a USB-C port's protocol is called, what each port
// gives, the date a battery was made, the order a charge limit's two files
// are written in, how bright a room is, and what the power profile's drawer
// says. Power.qml, Light.qml and Session.qml read the files and say the
// figures.
.pragma library

// The one of a kernel's list in brackets: "Trickle Fast [Custom]".
const MARKED = /\[([^\]]+)\]/;
const BRACKETS = /^\[|\]$/g;
const SPACES = /\s+/;

// A template's {fields}, filled.
const FIELD = /\{(\w+)\}/g;

function filled(template, fields) {
    return template.replace(FIELD, (_whole, name) => String(fields[name] ?? ""));
}

// The power profile's drawer: who holds it, why it's slowed, and what the
// firmware and the processor were told, each as written, then the drivers
// that tell them. `known` holds what power-profiles-daemon says (`holds`
// as { app, reason }, `slowed` as a said key or "") and what sysfs does
// (`firmware`, `offers`, `processor`, `processorDriver`,
// `firmwareDrivers`). The holds are one fact, parted by dots, so a dot
// only ever parts holds (VKCF3Z, 2026-11-08).
function profileDrawer(known, named, said) {
    const fact = (id, value) => ({ label: named(`session:profile:${id}`), value });
    const held = (known.holds ?? []).map(hold => hold.reason === "" ? hold.app
        : filled(said.held, hold)).join(" \u00b7 ");
    return {
        eyebrow: said.eyebrow,
        facts: [
            fact("held", held),
            fact("slowed", known.slowed === "" ? "" : said[known.slowed] ?? ""),
            fact("firmware", known.firmware),
            fact("offers", String(known.offers ?? "").trim().split(SPACES).join(" \u00b7 ")),
            fact("processor", known.processor),
            fact("processor-driver", known.processorDriver),
            fact("firmware-driver", (known.firmwareDrivers ?? []).join(" \u00b7 "))
        ].filter(each => (each.value ?? "") !== "")
    };
}

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

// Every protocol a USB-C port's power supply names, in use or not, once
// each as a person calls them: "C [PD] PD_PPS" is "USB-C \u00b7 USB PD
// \u00b7 USB PD PPS".
function protocols(usbType) {
    const each = chosen(usbType).all.map(one => PROTOCOLS[one] ?? "USB");
    return each.filter((one, at) => each.indexOf(one) === at).join(" \u00b7 ");
}

// The figures a value is said to: three, "45.0", "3.00", "100".
const SIGNIFICANT = 3;

// A figure to three significant figures, written by `number` to so many
// places.
function significant(value, number) {
    const whole = Math.floor(Math.log10(Math.max(Math.abs(value), 1))) + 1;
    return number(value, Math.max(0, SIGNIFICANT - whole));
}

// What a USB-C port gives, in a few words: its watts, volts, amps and
// protocol while it charges the machine, each figure its own part
// (VKCF3Z, 2026-11-08: "45.0 W \u00b7 15.0 V \u00b7 3.00 A", never a comma),
// that it's the machine powering what's plugged in, that something's
// plugged in giving nothing, or that it's empty, in the vocabulary's
// `words` (words.json's power:port). `number` writes a figure to so many
// places.
function port(said, number, words) {
    if (said.online && said.volts > 0 && said.amps > 0) {
        return [`${significant(said.volts * said.amps, number)} W`,
            `${significant(said.volts, number)} V`, `${significant(said.amps, number)} A`,
            said.protocol].filter(part => part !== "").join(" \u00b7 ");
    }
    if (said.online) {
        return [words.charging, said.protocol].filter(part => part !== "").join(" \u00b7 ");
    }
    if (said.giving) {
        return words.giving;
    }
    if (said.partner) {
        return words.plugged;
    }
    return words.empty;
}

// A figure a power source offers by name, never measured: as many digits
// as it has, to the hundredth power delivery counts in, and no zeros it
// hasn't (VKCF3Z, 2026-11-08: "5 V \u00b7 20 V at 3 A"), "3.3", "2.25".
function offered(value) {
    return String(Math.round(value * 100) / 100);
}

// What a charger offers, from its source capabilities, each { volts, amps }
// or a programmable one's { volts, to, amps }: "5 V \u00b7 9 V \u00b7 20 V at
// 3 A" when every one gives the same current, or each with its own, in
// the vocabulary's `said` (words.json's power:port:drawer).
function offers(capabilities, said) {
    const each = offer => offer.to === undefined ? `${offered(offer.volts)} V`
        : filled(said.range, { from: offered(offer.volts), to: offered(offer.to) });
    const at = (what, offer) => filled(said.at, { offers: what, amps: offered(offer.amps) });
    if (capabilities.length === 0) {
        return "";
    }
    if (capabilities.every(offer => offer.amps === capabilities[0].amps)) {
        return at(capabilities.map(each).join(" \u00b7 "), capabilities[0]);
    }
    return capabilities.map(offer => at(each(offer), offer)).join(" \u00b7 ");
}

// A USB-C port's drawer, from what the kernel's USB-C class says of it:
// which way its power and data go, how it negotiates, what's plugged in
// and what that offers, then the power delivery revisions that identify
// them, and its power supply's name, in the Name face, last. `known`
// holds the port's number (`n`), its `power` and `data` roles as the
// kernel lists them, its `mode`, the partner's `accessory` mode,
// `capabilities` (offers' input), its supply's `usbType` and `supply`
// name, and the `revision` and `partnerRevision`.
function portDrawer(known, named, said) {
    const fact = (id, value) => ({ label: named(`power:port:${id}`), value });
    const accessory = known.accessory ?? "";
    const partnerRevision = known.partnerRevision ?? "";
    return {
        eyebrow: filled(said.eyebrow, { n: known.n }),
        facts: [
            fact("power-role", chosen(known.power).now),
            fact("data-role", chosen(known.data).now),
            fact("mode", known.mode ? said[`mode:${known.mode}`] ?? known.mode : ""),
            fact("offers", offers(known.capabilities ?? [], said)),
            fact("protocols", protocols(known.usbType ?? "")),
            fact("partner", accessory === "" || accessory === "none" ? ""
                : filled(said.accessory, { mode: accessory })),
            fact("revision", known.revision ?? ""),
            fact("partner-revision", partnerRevision === "0.0" ? "" : partnerRevision),
            Object.assign(fact("supply", known.supply ?? ""), { identifies: true })
        ].filter(each => (each.value ?? "") !== "")
    };
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

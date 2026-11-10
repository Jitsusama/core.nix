// What the launcher's battery and light answers make of the machine
// daemon's supply records and the light sensor's files, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// which supply is the battery and which the USB-C ports, what the battery
// is doing and how long it has, the energy it holds, which of a list the
// kernel marks as in use, what a USB-C port's protocol is called, what
// each port gives and its drawer, the date a battery was made, the order
// a charge limit's two files are written in, how bright a room is, and
// what the power profile's drawer says, and why a profile picked didn't
// take. Power.qml, Light.qml and Session.qml say the figures.
.pragma library
.import "Order.js" as Order

// The one of a kernel's list in brackets: "Trickle Fast [Custom]".
const MARKED = /\[([^\]]+)\]/;
// A string D-Bus property as busctl prints it: s "balanced".
const BUS_STRING = /^s "([^"]*)"$/;
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

// The profile power-profiles-daemon says it's on, from busctl's answer to
// its ActiveProfile, by the daemon's own name ("power-saver", "balanced",
// "performance"); "" when it didn't answer.
function activeProfile(text) {
    return BUS_STRING.exec(String(text ?? "").trim())?.[1] ?? "";
}

// Why a profile picked by the daemon's name, `wanted`, didn't take, from
// the one it said back, `heard`: "" when it did. `told` holds the daemon's
// `holds`, each { profile, app }, `named`, each profile's name as the
// launcher says it, by the daemon's, and `said`, words.json's
// session:profile words: nothing heard, a program holding the one heard,
// or the daemon keeping it for its own reasons.
function profileRefusal(wanted, heard, told) {
    if (heard === wanted) {
        return "";
    }
    if (heard === "") {
        return told.said.silent;
    }
    const profile = told.named[heard] ?? heard;
    const holding = (told.holds ?? []).find(hold => hold.profile === heard);
    return holding ? filled(told.said.held, { profile, app: holding.app })
        : filled(told.said.kept, { profile });
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

// A kernel's list again as it wrote it, its choice in brackets, from
// the daemon's choices and the one chosen: ["C", "PD", "PD_PPS"] and
// "PD" are "C [PD] PD_PPS", as protocols() reads.
function listed(all, now) {
    return all.map(one => (one === now ? `[${one}]` : one)).join(" ");
}

// What a USB-C port gives, in a few words: its watts, volts, amps and
// protocol while it charges the machine, each figure its own part
// (VKCF3Z, 2026-11-08: "45.0 W \u00b7 15.0 V \u00b7 3.00 A", never a comma),
// that it's the machine powering what's plugged in, that something's
// plugged in giving nothing, or that it's empty, in the vocabulary's
// `words` (words.json's power:port). `measured` writes a figure to three
// significant digits, its zeros kept (Numbers.significant).
function port(said, measured, words) {
    if (said.online && said.volts > 0 && said.amps > 0) {
        return [`${measured(said.volts * said.amps)} W`, `${measured(said.volts)} V`,
            `${measured(said.amps)} A`, said.protocol].filter(part => part !== "").join(" \u00b7 ");
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
// hasn't (VKCF3Z, 2026-11-08: "5 V \u00b7 20 V at 3 A"), "3.3", "2.25",
// written by `number` in the machine's own mark (Numbers.format).
function offered(value, number) {
    return number(value, 2);
}

// What a charger offers, from its source capabilities, each { volts, amps }
// or a programmable one's { volts, to, amps }: "5 V \u00b7 9 V \u00b7 20 V at
// 3 A" when every one gives the same current, or each with its own, in
// the vocabulary's `said` (words.json's power:port:drawer).
function offers(capabilities, said, number) {
    const each = offer => offer.to === undefined ? `${offered(offer.volts, number)} V`
        : filled(said.range, { from: offered(offer.volts, number),
            to: offered(offer.to, number) });
    const at = (what, offer) => filled(said.at,
        { offers: what, amps: offered(offer.amps, number) });
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
// name, and the `revision` and `partnerRevision`; `number` writes what it
// offers.
function portDrawer(known, named, said, number) {
    const fact = (id, value) => ({ label: named(`power:port:${id}`), value });
    const accessory = known.accessory ?? "";
    const partnerRevision = known.partnerRevision ?? "";
    return {
        eyebrow: filled(said.eyebrow, { n: known.n }),
        facts: [
            fact("power-role", chosen(known.power).now),
            fact("data-role", chosen(known.data).now),
            fact("mode", known.mode ? said[`mode:${known.mode}`] ?? known.mode : ""),
            fact("offers", offers(known.capabilities ?? [], said, number)),
            fact("protocols", protocols(known.usbType ?? "")),
            fact("partner", accessory === "" || accessory === "none" ? ""
                : filled(said.accessory, { mode: accessory })),
            fact("revision", known.revision ?? ""),
            fact("partner-revision", partnerRevision === "0.0" ? "" : partnerRevision),
            Object.assign(fact("supply", known.supply ?? ""), { identifies: true })
        ].filter(each => (each.value ?? "") !== "")
    };
}

// A USB-C port's supply, as the kernel names one: ucsi-source-psy-
// USBC000:001 is port 1.
const USB_C = /USBC\d+:0*(\d+)$/;

// The machine's own battery, of the daemon's supplies, by key: a
// battery whose scope is a device's, as a mouse's is, is that device's.
function batteryOf(records) {
    return (
        Order.sorted(records, Order.byName((each) => each.key)).find(
            (each) => each.type === "Battery" && each.scope !== "Device",
        ) ?? null
    );
}

// Each USB-C port the machine charges through, { n, record }, by its
// number, online or not.
function portsOf(records) {
    return records
        .map((record) => ({ record, port: USB_C.exec(String(record.name ?? "")) }))
        .filter((each) => each.port !== null)
        .map((each) => ({ n: Number(each.port[1]), record: each.record }))
        .sort((a, b) => a.n - b.n);
}

// Whether a charger is plugged in other than at a USB-C port, as a
// barrel or a dock is.
function pluggedElsewhere(records) {
    return records.some(
        (each) =>
            each.type === "Mains" && !USB_C.test(String(each.name ?? "")) && each.online === true,
    );
}

// What a USB-C port's record says of it, as port() reads it: whether it's
// charging the machine, what it gives (now where it says so, else what
// it's offered), its protocol, whether anything's plugged in and whether
// the machine is giving the power.
function portOf(record) {
    const typec = record.typec ?? null;
    return {
        online: record.online === true,
        volts: record.volts ?? record.max_volts ?? null,
        amps: record.max_amps ?? record.amps ?? null,
        protocol: protocol(record.usb_type),
        partner: typec !== null && typec.partner !== null && typec.partner !== undefined,
        giving: typec?.power_role === "source",
    };
}

// What the battery is doing, as UPower works it out, else as the kernel
// says it: "charging", "discharging", "full", "pending charge" and so on.
function stateOf(record) {
    return record.upower?.state ?? String(record.status ?? "unknown").toLowerCase();
}

// How the battery stands, as its headline says it: its charge, then how
// long it has as UPower works it out ("36% \u00b7 3 h 10 min to empty",
// VKCF3Z, 2026-11-08), or what it's doing where UPower says no time.
// `lasted` says a span of seconds, and `said` is Power's words. A charge
// the battery doesn't say is left out, never said as none.
function doing(record, lasted, said) {
    const upower = record.upower ?? {};
    const how = doingSaid(stateOf(record), upower, lasted, said);
    const charge = record.capacity_pct ?? null;
    return charge === null ? how : `${charge}% \u00b7 ${how}`;
}

// What the battery is doing, as its headline says it after its charge.
function doingSaid(state, upower, lasted, said) {
    switch (state) {
        case "charging":
            return upower.time_to_full_s > 0
                ? String(said.full ?? "").replace("{time}", lasted(upower.time_to_full_s))
                : said.charging ?? "";
        case "discharging":
            return upower.time_to_empty_s > 0
                ? String(said.empty ?? "").replace("{time}", lasted(upower.time_to_empty_s))
                : said.battery ?? "";
        case "full":
            return said.whole ?? "";
        default:
            return said.holding ?? "";
    }
}

// The energy the battery holds now, full and as designed, in watt hours:
// as it counts them, or from its charge at its design voltage where it
// counts charge, as UPower works it out. Each null where it can't be.
function energyOf(record) {
    const designed = record.min_design_volts ?? null;
    const of = (wh, ah) => record[wh] ?? (record[ah] !== null && record[ah] !== undefined
        && designed !== null ? record[ah] * designed : null);
    return {
        now: of("energy_wh", "charge_ah"),
        full: of("energy_full_wh", "charge_full_ah"),
        design: of("energy_design_wh", "charge_design_ah"),
    };
}

// When a battery was made, from the daemon's "2026-07-20", as made() says
// it.
function madeOf(text) {
    const [year, month, day] = String(text ?? "").split("-").map(Number);
    return made(year, month, day);
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

// The steps a sleep goes through, in the order Linux takes them, each as
// its suspend_stats names it.
const SLEEP_STEPS = ["freeze", "prepare", "suspend", "suspend_late", "suspend_noirq",
    "resume_noirq", "resume_early", "resume"];
const MICROSECONDS = 1_000_000;

// How sleep stands, from the machine daemon's sleep:main record, or null
// before it has answered: the mode it sleeps in and the others it could,
// what it can do, how often it slept and failed since it started, each
// step's failures in the order they're taken, the step, device and
// error it last failed with, and the time in hardware sleep, in seconds,
// with the most its counter can tell.
function sleepOf(record) {
    if (!record) {
        return null;
    }
    const failedAt = SLEEP_STEPS.map((step) => ({ step, count: record.failed_at?.[step] ?? 0 }))
        .filter((each) => each.count > 0);
    const lastStep = record.last_failed_step ?? "";
    const mode = record.mode ?? "";
    return {
        mode,
        others: (record.modes ?? []).filter((each) => each !== mode),
        states: record.states ?? [],
        slept: record.successes ?? null,
        failures: record.failures ?? 0,
        failedAt,
        lastStep,
        // How often the step it last failed at has failed.
        lastFailures: failedAt.find((each) => each.step === lastStep)?.count ?? 0,
        lastDevice: record.last_failed_device ?? "",
        errno: record.last_failed_errno ?? null,
        lastHardware: seconds(record.last_hardware_sleep_us),
        allHardware: seconds(record.total_hardware_sleep_us),
        countsTo: seconds(record.hardware_sleep_counts_to_us)
    };
}

function seconds(microseconds) {
    return microseconds === null || microseconds === undefined ? null
        : microseconds / MICROSECONDS;
}

// The errors a sleep most often fails with, by their names in Linux's
// own headers, since a number alone says nothing.
const ERRORS = { 1: "EPERM", 5: "EIO", 11: "EAGAIN", 12: "ENOMEM", 16: "EBUSY", 19: "ENODEV",
    22: "EINVAL", 110: "ETIMEDOUT" };

// An error number as Linux gives it, -16, by its name alone, "EBUSY",
// or where it has none the number with the real minus, "\u221271".
function errorOf(errno) {
    return ERRORS[Math.abs(errno)] ?? (errno < 0 ? `\u2212${-errno}` : String(errno));
}

// What Sleep's row says of how sleep stands, in the words `said` gives
// (words.json's Sleep), with `lasted` saying a time: its mode as the
// side, the step it last failed at and how often as the note, "failed
// while freezing \u00b7 2 times", and its drawer's facts by their words'
// ids, the failures at every step but the one the note names, in the
// order taken (VKCF3Z, 2026-11-08: no fact repeats the side), why the last
// failure came in words, `errors` by the error's name ("a device was
// busy"), and last, as identifiers, the device that blocked it, as Linux
// names it, and the error's own name, "EBUSY". A figure that's none or
// zero gives way.
function sleepSaid(sleep, said, lasted, errors) {
    const times = (count) => count === 1 ? said.time : String(said.times).replace("{count}", count);
    const step = (name) => said[name] ?? name;
    const at = (each) => String(said.at).replace("{count}", each.count).replace("{step}",
        step(each.step));
    const took = (value) => value === null || value <= 0 ? "" : lasted(value);
    const code = sleep.errno === null ? "" : errorOf(sleep.errno);
    const failed = sleep.lastStep !== "" && sleep.lastFailures > 0;
    const elsewhere = sleep.failedAt.filter((each) => !failed || each.step !== sleep.lastStep);
    const facts = [
        { id: "sleep:others", value: sleep.others.join(" \u00b7 ") },
        { id: "sleep:ways", value: sleep.states.join(" \u00b7 ") },
        { id: "sleep:slept", value: sleep.slept > 0 ? times(sleep.slept) : "" },
        { id: "sleep:failed", value: elsewhere.map(at).join(" \u00b7 ") },
        { id: "sleep:error", value: code === "" ? "" : errors?.[code] ?? "" },
        { id: "sleep:hardware", value: took(sleep.lastHardware) },
        { id: "sleep:hardware-all", value: took(sleep.allHardware) },
        { id: "sleep:counts-to", value: took(sleep.countsTo) },
        { id: "sleep:device", value: sleep.lastDevice, identifies: true },
        { id: "sleep:code", value: code, identifies: true }
    ].filter((fact) => fact.value !== "");
    return {
        side: sleep.mode,
        note: failed ? `${String(said.failed).replace("{step}", step(sleep.lastStep))} \u00b7 ${
            times(sleep.lastFailures)}` : "",
        facts
    };
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

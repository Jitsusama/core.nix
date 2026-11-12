// What the launcher's session and light answers make of the power
// profile's, sleep's and the light sensor's readings, in plain JavaScript
// so a check can run it in Qt's own engine, as the shell does: what the
// power profile's drawer says, and why a profile picked didn't take; how
// sleep stands and what Sleep's row says of it; and how bright a room is.
// Session.qml and Displays.qml say the figures. The battery's and the
// chargers' rows are the machine daemon's, served ready
// (modules/machine's power_rows.rs), and none of it is worked out here.
.pragma library

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

// A power profile's own drawer, under its row among Power Profile's
// choices, as facts by their words' ids: which profile is on instead,
// while this one isn't; which programs hold this one, as the power
// profile's drawer says them; why Full Performance is held back, on its
// own drawer alone; and while it's the one on, what it set, the firmware's
// profile and the processor's preference as power-profiles-daemon wrote
// them. `known` holds `on`, whether it's on, `current`, the name of the
// one on, `holds`, each { profile, app, reason } by the daemon's names,
// `slowed`, a said key or "", and `firmware` and `processor` as read, or
// none before they are.
function choiceFacts(daemon, known, said) {
    const held = (known.holds ?? []).filter(hold => hold.profile === daemon)
        .map(hold => hold.reason === "" ? hold.app : filled(said.held, hold)).join(" \u00b7 ");
    return [
        { id: "session:profile:now", value: known.on ? "" : known.current ?? "" },
        { id: "session:profile:held", value: held },
        { id: "session:profile:slowed", value: daemon !== "performance" || !known.slowed ? ""
            : said[known.slowed] ?? "" },
        { id: "session:profile:firmware", value: known.on ? known.firmware ?? "" : "" },
        { id: "session:profile:processor", value: known.on ? known.processor ?? "" : "" }
    ].filter(fact => fact.value !== "");
}

// The programs whose windows are open, the most windows first, then by
// name, each { name, count }, from niri's windows (Niri.qml's, by id),
// each named by `named` from its app_id, and how many windows in all, a
// window with no app_id counted there alone.
function byProgram(windows, named) {
    const counted = Object.create(null);
    let open = 0;
    for (const window of Object.values(windows ?? {})) {
        open += 1;
        const id = String(window?.app_id ?? "");
        if (id !== "") {
            const name = named(id);
            counted[name] = (counted[name] ?? 0) + 1;
        }
    }
    const programs = Object.keys(counted).map(name => ({ name, count: counted[name] }))
        .sort((one, other) => other.count - one.count || one.name.localeCompare(other.name));
    return { open, programs };
}

// How many programs a drawer names before saying how many more.
const PROGRAMS_NAMED = 4;

// What's open, as Lock's drawer says it stays running behind the lock:
// how many windows, "14 windows", and whose, the most first, "kitty 5 \u00b7
// Chrome 4 \u00b7 Slack 2 \u00b7 3 more", in the words `said` gives; each ""
// when none is open.
function openSaid(open, said) {
    if (open.open === 0) {
        return { windows: "", programs: "" };
    }
    const named = open.programs.slice(0, PROGRAMS_NAMED).map(each => `${each.name} ${each.count}`);
    const more = open.programs.length - PROGRAMS_NAMED;
    if (more > 0) {
        named.push(filled(said.more, { count: more }));
    }
    return {
        windows: open.open === 1 ? said.window : filled(said.windows, { count: open.open }),
        programs: named.join(" \u00b7 ")
    };
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

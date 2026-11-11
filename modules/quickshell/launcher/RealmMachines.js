// Every machine of the realms this one links, as Machines shows them:
// each row's name, note and side, its drawer, and the order they come
// in, from the records the realms' mycelium nodes serve (kind `machine`,
// mycelium's PROTOCOL.md). Said in one place, out of Machines.qml, so
// tests/machines.qml holds it in Qt's own engine. Nothing here guesses:
// a figure a node didn't say is left out rather than made up (Joel,
// 2026-11-06: "treated way more like our hardware integrations"). The
// design is VKCF3Z's ruling of 2026-11-07 (KP0L6A's design-machines.md).
.pragma library
.import "Order.js" as Order
.import "Machine.js" as Machine

const PERCENT = 100;
const SHORT_COMMIT = 7;
// A pressure is said to a tenth.
const TENTHS = 10;
const BETWEEN = " \u00b7 ";

// The words Machines says, filled in: `said` is the vocabulary's own,
// words.json's machines.said.
function filled(said, key, fills) {
    let text = String(said[key] ?? key);
    for (const name of Object.keys(fills ?? {})) {
        text = text.split(`{${name}}`).join(String(fills[name]));
    }
    return text;
}

// Bytes as Machine says them, in decimal units to three digits, worked
// out from bytes: "1.07 GB", "915 MB" (docs/design.md: sizes are
// decimal, never a GiB).
function bytes(count) {
    return Machine.bytes(count);
}

// A pressure, in percent, to a tenth: "88.8%".
function percent(share) {
    return `${(Math.round(Number(share) * TENTHS) / TENTHS).toFixed(1)}%`;
}

// Whether a node says it reaches the machine now.
function reached(record) {
    return record.state !== "unreachable";
}

// What a machine is, in words: its maker's name for its kind where the
// node says one ("MacBook Air"), else a Linux machine's maker and product,
// which are words already, else the system's own kind ("Mac"), never an
// identifier such as Mac14,2, which goes last in the drawer.
function kindOf(record, named) {
    if ((record.model_name ?? "") !== "") {
        return record.model_name;
    }
    if (record.system === "linux" && (record.model ?? "") !== "") {
        return record.model;
    }
    return systemOf(record, named);
}

// The systems a machine is named by, each a name of its own.
const SYSTEMS = ["macos", "linux"];

// The system's own kind, "Mac" or "Linux", or "Machine" where the node
// names none it knows.
function systemOf(record, named) {
    return named(`kind:${SYSTEMS.includes(record.system) ? record.system : "machine"}`);
}


// The row's note: what it is and what it runs, "MacBook Air \u00b7 macOS
// 26.6.2", and its realm only while two or more are linked.
function note(record, place) {
    const parts = [kindOf(record, place.named), record.os ?? ""];
    if (place.realms > 1) {
        parts.push(record.realm ?? "");
    }
    return parts.filter(part => part !== "").join(BETWEEN);
}

// What it's short of, as its queue says it is past a limit: "short of
// memory", "short of memory and processors".
function short(over, said) {
    const each = (over ?? []).map(limit => filled(said, limit));
    if (each.length === 0) {
        return "";
    }
    // Qt's engine has no Array.prototype.at, so the last is sliced off.
    const [last] = each.slice(-1);
    const what = each.length === 1 ? last
        : filled(said, "and", { first: each.slice(0, -1).join(", "), last });
    return filled(said, "short", { what });
}

// The row's side: how it stands first, then what qualifies it. This
// machine is "this machine". One out of reach says since when; one whose
// queue would start no more work is busy, else ready; then what it's
// short of and, while it runs on its battery, how full that is.
function side(record, place) {
    const { said } = place;
    if (record.here === true) {
        return filled(said, "here");
    }
    if (!reached(record)) {
        return [filled(said, "unreachable"),
            filled(said, "since", { time: when(record.measured_at, place) })].join(BETWEEN);
    }
    const counts = record.room?.takes;
    const busy = counts && counts.builds === 0 && counts.vm_tests === 0;
    const battery = record.power?.on_battery === true && typeof record.power.charge === "number"
        ? filled(said, "battery", { charge: record.power.charge }) : "";
    return [filled(said, busy ? "busy" : "ready"), short(record.room?.over, said), battery]
        .filter(part => part !== "").join(BETWEEN);
}

// This machine's side, as items that give way whole (Choice.qml's
// sideItems): what tells its row apart first, "this machine", then what
// it is and runs, which its note would say of another, so its system gives
// way first and its model next, "this machine \u00b7 XPS 14 \u00b7 NixOS
// 26.11" (VKCF3Z, 2026-11-09).
function hereItems(record, place) {
    return [{ said: filled(place.said, "here"), keep: 3 },
        { said: kindOf(record, place.named), keep: 2 }, { said: record.os ?? "", keep: 1 }]
        .filter(item => item.said !== "");
}

// A machine as Start a Session lists it, from the same records as
// Machines (the design's Elsewhere): why it can't take a session, or what
// it would rather not start one under, as the node's placement says; else
// how it stands, as its row in Machines says it. With no record, as from a
// node too old to list this machine, it says nothing it wasn't told.
function startNote(placed, record, place) {
    if (placed.cannot !== "") {
        return placed.cannot;
    }
    if (placed.rather !== "") {
        return placed.rather;
    }
    return record ? side(record, place) : "";
}

// The machines in the order they're listed: this one first, then those
// reached, then those out of reach, each by name, as Order.js orders live
// states. A machine two realms both list is one machine, kept once with
// the realm that said it last.
function ordered(records) {
    const latest = {};
    for (const record of records) {
        const kept = latest[record.name];
        if (!kept || Date.parse(record.measured_at) > Date.parse(kept.measured_at)) {
            latest[record.name] = Object.assign({}, record, { here: record.here === true
                || kept?.here === true });
        }
    }
    const rank = record => {
        if (record.here === true) {
            return 0;
        }
        return reached(record) ? 1 : 2;
    };
    return Order.sorted(Object.values(latest),
        Order.live({ stands: rank, name: record => record.name }));
}

// A fact of the drawer, or none where its value isn't said.
function fact(label, value, figure) {
    if (value === "" || value === null || value === undefined) {
        return null;
    }
    return figure ? { label, value, figure } : { label, value };
}

// Its pressures, each with Meter's stroke of the share of the last ten
// seconds some work waited, warning where its queue says it's past that
// limit.
function pressures(room, named) {
    const pressure = room?.pressure;
    if (!pressure) {
        return [];
    }
    const over = room.over ?? [];
    const each = (id, limit) => fact(named(id), percent(pressure[limit]), {
        shape: "stroke", values: [Number(pressure[limit]) / PERCENT], warn: over.includes(limit)
    });
    return [each("pressure:cpu", "cpu"),
        each("pressure:memory", "memory"), each("pressure:io", "io")];
}

// How full and what it charges from, "62% · on battery", a run of parts read by dots as
// Power's is.
function charge(power, said) {
    if (!power || typeof power.charge !== "number") {
        return "";
    }
    return filled(said, power.on_battery ? "charging:battery" : "charging:charger",
        { charge: power.charge });
}

// The work its queue would start now: "1 build, 1 VM test".
function takes(room, said) {
    const counts = room?.takes;
    if (!counts) {
        return "";
    }
    const one = (count, key) => filled(said, count === 1 ? key : `${key}s`, { count });
    return [one(counts.builds, "build"), one(counts.vm_tests, "vmTest")].join(", ");
}

// A moment, as the launcher says one through When: "23:40" today, else
// its day before it, "yesterday 23:40", "Friday 23:40" or "7 Nov 23:40";
// nothing where its node says none (VKCF3Z, 2026-11-09: "failed \u00b7
// yesterday 23:40").
function when(at, place) {
    const time = Date.parse(String(at ?? ""));
    if (Number.isNaN(time)) {
        return "";
    }
    return [place.day(time, place.now), place.clock(time)].filter(part => part !== "")
        .join(" ");
}

// Its update: the commit its machine runs, shortened, and core's beside
// this machine's where both say one, then how its last update ended and
// when, then why it said it needs a reboot.
function update(updated, place) {
    if (!updated) {
        return [];
    }
    const { said, named } = place;
    const core = updated.followed?.core ?? "";
    const shortened = core.slice(0, SHORT_COMMIT);
    const coreSaid = core !== "" && core === place.core
        ? filled(said, "sameCore", { commit: shortened }) : shortened;
    // Last Update wears the update's mark where its page lists it, as
    // Update It does (`mark`, the words' id it's drawn from).
    const step = fact(named("step"), [updated.step ?? "", when(updated.at, place)]
        .filter(part => part !== "").join(BETWEEN));
    return [
        fact(named("commit"), String(updated.commit ?? "").slice(0, SHORT_COMMIT)),
        fact(named("core"), coreSaid),
        step && Object.assign(step, { mark: "machines:step" }),
        fact(named("reboot"), updated.restart ?? "")
    ];
}

// Its disk: how much is free of its size where the node says both, "1.07
// GB free of 4.08 GB", else its size, saying it is that, then how full
// its guard bands it.
function disk(record, place) {
    const { said } = place;
    if (!record.disk) {
        return "";
    }
    const size = typeof record.disk_free === "number"
        ? filled(said, "freeOf", { free: bytes(record.disk_free), total: bytes(record.disk) })
        : filled(said, "sized", { total: bytes(record.disk) });
    return [size, place.band(record.disk_band ?? "")].filter(part => part !== "").join(BETWEEN);
}

// What a processor's maker calls it, without the marks typed for lawyers
// or the word for what it is, its clock's unit spaced as every figure's:
// "Intel(R) Xeon(R) CPU @ 2.80GHz" is "Intel Xeon @ 2.80 GHz". Its
// model's own digits stay as its maker wrote them.
function processor(model) {
    return String(model ?? "").replace(/\((R|TM|C)\)/giu, "").replace(/\bCPU\b/gu, "")
        .replace(/(\d)([GM]Hz)\b/gu, "$1 $2").replace(/\s+/gu, " ").trim();
}

// Each probe its configuration asks, as reached or not.
function reach(record, place) {
    const probes = record.reached ?? {};
    return Object.keys(probes).sort().map(probe => fact(probe,
        filled(place.said, probes[probe] ? "reaches" : "unreaches")));
}

// How an update a machine was asked for ended, as the report says it,
// from its node's answer to the daemon interface's `update` ask,
// `{"updated": ending}` with the ending its updater said, or
// `{"error": {"message": why}}`, or null when the node went before it
// answered. "skiff updated to c0ffee1", "skiff was on main
// already", "skiff didn't update" or "skiff refused the update";
// anything else, an answer that never came included, is why it couldn't
// be asked, as the node said it.
function updateEnded(name, answer, said) {
    const step = String(answer?.updated?.step ?? "");
    const commit = String(answer?.updated?.commit ?? "").slice(0, SHORT_COMMIT);
    const key = step === "done" && commit === "" ? "updated:done:bare" : `updated:${step}`;
    if (step !== "" && said[key] !== undefined) {
        return filled(said, key, { name, commit });
    }
    const refused = answer?.error?.message;
    const why = typeof refused === "string" && refused !== "" ? refused
        : filled(said, "unanswered");
    return filled(said, "notUpdated", { name, why });
}

// Its drawer, from what moves most to what never does: its pressures,
// memory, charge, the work it would take, its disk, what it reaches, its
// update, when it was measured, then its processor, accelerators and
// system, and its model's identifier last. Never the row's side again.
// Out of reach, every figure keeps its last value and Measured says as
// of when. The eyebrow says where and what kind of thing it is, "on
// personal \u00b7 machine"; its system is System's (VKCF3Z, 1ad76438).
function drawer(record, place) {
    const { said, named } = place;
    const { room } = record;
    const cpu = record.cpu ?? {};
    const measuredAt = when(record.measured_at, place);
    const measured = reached(record) ? measuredAt
        : filled(said, "asOf", { time: measuredAt });
    const accelerators = (record.accelerators ?? [])
        .map(one => `${one.driver} ${filled(said, one.kind)}`).join(", ");
    const facts = pressures(room, named).concat([
        fact(named("memory"), typeof room?.memory_available === "number" && record.memory
            ? filled(said, "freeOf", { free: bytes(room.memory_available),
                total: bytes(record.memory) }) : ""),
        fact(named("charge"), charge(record.power, said)),
        fact(named("takes"), takes(room, said)),
        fact(named("disk"), disk(record, place)),
        ...reach(record, place),
        ...update(record.updated, place),
        fact(named("measured"), measured),
        fact(named("processor"), [processor(cpu.model), cpu.cores
            ? filled(said, "cores", { count: cpu.cores }) : ""]
            .filter(part => part !== "").join(BETWEEN)),
        fact(named("accelerators"), accelerators),
        fact(named("system"), [record.os ?? "", record.arch ?? ""]
            .filter(part => part !== "").join(BETWEEN)),
        fact(named("model"), record.model ?? "")
    ]);
    return {
        eyebrow: filled(said, "on", { realm: record.realm ?? "" }),
        title: record.name,
        facts: facts.filter(one => one !== null)
    };
}

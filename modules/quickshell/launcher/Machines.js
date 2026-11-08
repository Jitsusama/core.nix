// Every machine of the realms this one links, as Machines shows them:
// each row's name, note and side, its drawer, and the order they come
// in, from the records the realms' mycelium nodes serve (kind `machine`,
// mycelium's PROTOCOL.md). Said in one place, out of Machines.qml, so
// tests/machines.qml holds it in Qt's own engine. Nothing here guesses:
// a figure a node didn't say is left out rather than made up (Joel,
// 2026-11-06: "treated way more like our hardware integrations"). The
// design is VKCF3Z's ruling of 2026-11-07 (KP0L6A's design-machines.md).
.pragma library

const PERCENT = 100;
const SHORT_COMMIT = 7;
const KIB = 1024;
const GIB = KIB * KIB * KIB;
const TIB = GIB * KIB;
// A figure under ten is said to a tenth, as Machine's are.
const WHOLE_FROM = 10;
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

// Bytes as Machine says them: "24 GiB", "8.6 GiB", "1.8 TiB".
function bytes(count) {
    const [size, unit] = count >= TIB ? [count / TIB, "TiB"] : [count / GIB, "GiB"];
    return `${size >= WHOLE_FROM ? Math.round(size) : Math.round(size * WHOLE_FROM) / WHOLE_FROM}`
        + ` ${unit}`;
}

// A pressure, in percent, to a tenth: "88.8%".
function percent(share) {
    return `${(Math.round(Number(share) * WHOLE_FROM) / WHOLE_FROM).toFixed(1)}%`;
}

// Whether a node says it reaches the machine now.
function reached(record) {
    return record.state !== "unreachable";
}

// What a machine is, in words: its maker's name for its kind where the
// node says one ("MacBook Air"), else a Linux machine's maker and product,
// which are words already, else the system's own kind ("Mac"), never an
// identifier such as Mac14,2, which goes last in the drawer.
function kindOf(record, said) {
    if ((record.model_name ?? "") !== "") {
        return record.model_name;
    }
    if (record.system === "linux" && (record.model ?? "") !== "") {
        return record.model;
    }
    return filled(said, record.system ?? "machine");
}

// A moment as the clock says it today, "14:20", or the day before then,
// "7 Nov".
function moment(at, now) {
    const when = new Date(at);
    const today = new Date(now);
    const sameDay = when.toDateString() === today.toDateString();
    return sameDay ? Qt.formatTime(when, "HH:mm") : Qt.formatDate(when, "d MMM");
}

// The row's note: what it is and what it runs, "MacBook Air \u00b7 macOS
// 26.6.2", and its realm only while two or more are linked.
function note(record, place) {
    const parts = [kindOf(record, place.said), record.os ?? ""];
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
            filled(said, "since", { time: moment(record.measured_at, place.now) })].join(BETWEEN);
    }
    const counts = record.room?.takes;
    const busy = counts && counts.builds === 0 && counts.vm_tests === 0;
    const battery = record.power?.on_battery === true && typeof record.power.charge === "number"
        ? filled(said, "battery", { charge: record.power.charge }) : "";
    return [filled(said, busy ? "busy" : "ready"), short(record.room?.over, said), battery]
        .filter(part => part !== "").join(BETWEEN);
}

// The machines in the order they're listed: this one first, then those
// reached, then those out of reach, each by name. A machine two realms
// both list is one machine, kept once with the realm that said it last.
function ordered(records) {
    const byName = {};
    for (const record of records) {
        const kept = byName[record.name];
        if (!kept || Date.parse(record.measured_at) > Date.parse(kept.measured_at)) {
            byName[record.name] = Object.assign({}, record, { here: record.here === true
                || kept?.here === true });
        }
    }
    const rank = record => {
        if (record.here === true) {
            return 0;
        }
        return reached(record) ? 1 : 2;
    };
    return Object.values(byName)
        .sort((a, b) => rank(a) - rank(b) || String(a.name).localeCompare(String(b.name)));
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

// What it charges from and how full, "62%, on battery".
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

// Its update: the commit its machine runs, shortened, and core's beside
// this machine's where both say one, then the step, then what it said
// needs a restart.
function update(updated, place) {
    if (!updated) {
        return [];
    }
    const { said, named } = place;
    const core = updated.followed?.core ?? "";
    const shortened = core.slice(0, SHORT_COMMIT);
    const coreSaid = core !== "" && core === place.core
        ? filled(said, "sameCore", { commit: shortened }) : shortened;
    return [
        fact(named("commit"), String(updated.commit ?? "").slice(0, SHORT_COMMIT)),
        fact(named("core"), coreSaid),
        fact(named("step"), updated.step ?? ""),
        fact(named("restart"), updated.restart ?? "")
    ];
}

// Each probe its configuration asks, as reached or not.
function reach(record, place) {
    const probes = record.reached ?? {};
    return Object.keys(probes).sort().map(probe => fact(probe,
        filled(place.said, probes[probe] ? "reaches" : "unreaches")));
}

// How an update a machine was asked for ended, as the report says it,
// from what `mycelium update` printed: its node's answer,
// `{"Ok": {"Updated": ending}}` with the ending its updater said, or
// `{"Err": why}`. "penelope updated to c0ffee1", "penelope was on main
// already", "penelope didn't update" or "penelope refused the update";
// anything else, an answer that never came included, is why it couldn't
// be asked, as the node said it.
function updateEnded(name, text, said) {
    let answer = null;
    try {
        answer = JSON.parse(text);
    } catch (_error) {
        answer = null;
    }
    const step = String(answer?.Ok?.Updated?.step ?? "");
    const commit = String(answer?.Ok?.Updated?.commit ?? "").slice(0, SHORT_COMMIT);
    const key = step === "done" && commit === "" ? "updated:done:bare" : `updated:${step}`;
    if (step !== "" && said[key] !== undefined) {
        return filled(said, key, { name, commit });
    }
    const why = typeof answer?.Err === "string" ? answer.Err : filled(said, "unanswered");
    return filled(said, "notUpdated", { name, why });
}

// Its drawer, from what moves most to what never does: its pressures,
// memory, charge, the work it would take, its disk, what it reaches, its
// update, when it was measured, then its processor, accelerators and
// system, and its model's identifier last. Never the row's side again.
// Out of reach, every figure keeps its last value and Measured says as
// of when. The eyebrow says where and what kind: "on personal \u00b7 Mac".
function drawer(record, place) {
    const { said, named } = place;
    const { room } = record;
    const cpu = record.cpu ?? {};
    const measured = reached(record)
        ? place.ago(Date.parse(record.measured_at), place.now)
        : filled(said, "asOf", { time: moment(record.measured_at, place.now) });
    const accelerators = (record.accelerators ?? [])
        .map(one => `${one.driver} ${filled(said, one.kind)}`).join(", ");
    const facts = pressures(room, named).concat([
        fact(named("memory"), typeof room?.memory_available === "number" && record.memory
            ? filled(said, "freeOf", { free: bytes(room.memory_available),
                total: bytes(record.memory) }) : ""),
        fact(named("charge"), charge(record.power, said)),
        fact(named("takes"), takes(room, said)),
        fact(named("disk"), record.disk ? [bytes(record.disk),
            place.band(record.disk_band ?? "")].filter(part => part !== "").join(", ") : ""),
        ...reach(record, place),
        ...update(record.updated, place),
        fact(named("measured"), measured),
        fact(named("processor"), [cpu.model ?? "", cpu.cores
            ? filled(said, "cores", { count: cpu.cores }) : ""]
            .filter(part => part !== "").join(", ")),
        fact(named("accelerators"), accelerators),
        fact(named("system"), [record.os ?? "", record.arch ?? ""]
            .filter(part => part !== "").join(BETWEEN)),
        fact(named("model"), record.model ?? "")
    ]);
    return {
        eyebrow: filled(said, "on", { realm: record.realm ?? "",
            kind: filled(said, record.system ?? "machine") }),
        title: record.name,
        facts: facts.filter(one => one !== null)
    };
}

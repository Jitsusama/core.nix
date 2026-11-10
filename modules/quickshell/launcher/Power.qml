// The battery and what feeds it, answered in the field: "battery", "power",
// "charger", "plugged in" or "energy". The headline is its charge and how
// long it has; the rows are the detail a status line leaves out, each a
// part named first: the Charger, holding each USB-C port and what it
// gives, its drawer what the port and what's plugged into it say of
// themselves; the Charge, as a bar; what the machine is Drawing or
// Charging at, and the Voltage, each a spark of the last reads; the Energy
// it holds, its drawer the charge behind it; its Health against its design
// and the cycles it's been through; the Cycles alone; the Charge type and
// Charge limit; its Temperature, as a bar; and its Model, its drawer its
// serial, and when it was Made. A charge limit whose files Joel's account
// may write is a setting, its choices a step in; one only root may set is
// a figure saying so. A part has its own words too, "battery health" or
// "charge limit", which answer it alone. The power profile is the
// session's to show and change (Session.qml), and its row comes beside
// these. Every name and word is the vocabulary's (launcher/words.json,
// Words.qml).
//
// The figures are the machine daemon's (modules/machine), its supply
// records, each power supply's every figure with what UPower works out of
// the battery; it reads them once a second while Power's rows can be seen
// (Seen.js), as Machine's Power row is, and once as the shell starts and
// as UPower says a device came or went (Daemon.qml's refresh). The charge
// limit is moved by the daemon too, asked over the socket held for the
// figures, which tells the new limit before it answers.
import qs
import Quickshell
import Quickshell.Services.UPower
import QtQuick
import "Numbers.js" as Numbers
import "Machine.js" as Machine
import "Power.js" as Power
import "Interfaces.js" as Interfaces
import "Seen.js" as Seen

Provider {
    id: provider

    examples: ["battery", "charger"]

    // The daemon's supplies, as held now.
    function supplies() {
        const held = daemon.revision >= 0 ? daemon.held : {};
        return Object.keys(held).filter(key => key.startsWith("supply:")).map(key => held[key]);
    }

    readonly property var battery: daemon.revision >= 0 ? Power.batteryOf(supplies()) : null
    // A battery coming or going changes the answer, so it's said: the
    // launcher keeps what's inside each place until its source says it
    // changed. Unsaid, Battery's Health was never found from the root
    // (the net's goes-places run, 2026-11-07). Its key is followed rather
    // than its record, which is new with every read.
    // Machine's Power row says the charge too, so the places held are
    // said again as the battery first comes from the daemon.
    readonly property string batteryKey: battery?.key ?? ""
    onBatteryKeyChanged: {
        provider.answered();
        provider.launcher?.placesRead?.();
    }

    function number(value, decimals) {
        return Numbers.format(value, decimals, settings.group, settings.point);
    }

    // A figure read off the machine, to three significant digits, its
    // zeros kept to say how closely (VKCF3Z's one rule for every measure).
    function measured(value) {
        return Numbers.significant(value, settings.group, settings.point);
    }

    // The same, with the real minus, for a figure that can be negative.
    function figure(value) {
        return measured(value).replace("-", "\u2212");
    }

    // Below this share, draining, the battery wants Joel to plug in, as
    // the margin's battery says by default (pieces/Battery.qml), so its bar
    // takes the warning colour; a full battery wants nothing.
    readonly property int low: 20
    // A lithium battery's temperature is drawn against the most it's made
    // to run at, and warns from where it's too warm to charge well.
    readonly property int hottest: 60
    readonly property int warm: 45
    // The limits a step into the charge limit offers; the daemon starts
    // charging again a little below each, as the firmware's own is set.
    readonly property var stops: [60, 70, 80, 90, 100]

    // A part's name, as the vocabulary says it.
    function named(id, state, fill) {
        return Words.name(`power:${id}`, state ?? "", fill);
    }

    // What a part's words say, by the phrase's name, its braces filled:
    // Power's own where `id` is "".
    function says(id, phrase, fill) {
        let said = String((Words.of(id === "" ? "power" : `power:${id}`).said ?? {})[phrase]
            ?? "");
        for (const key of Object.keys(fill ?? {}))
            said = said.replace(`{${key}}`, fill[key]);
        return said;
    }

    // The parts asked for alone, by their own words, as the part's name.
    readonly property var partsAsked: {
        const out = {};
        for (const id of Words.of("power").parts)
            for (const question of Words.answers(id))
                out[question] = Words.name(id);
        return out;
    }

    // The words it answers whole, and by their start (Provider.qml).
    // "charger" asks it whole, so the charger's old names do too.
    questions: Words.started("power").concat(Object.keys(partsAsked),
        Words.olds(Words.of("power:charger")))

    // Whether Power's rows can be seen: its answer, or words typed that
    // ask it, a row of it, its own place, or Machine's holding the Power
    // row that says how it stands, so the daemon reads the supplies only
    // then.
    readonly property bool seen: Seen.seen(launcher, provider, ["energy", "power"],
        typed => Seen.asks(questions, plainly(typed)))

    // The supplies are read once though nothing shows them, as the shell
    // starts and as UPower says a device came or went. What's inside the
    // battery is made while the launcher is closed, from what's held, so a
    // battery read only while seen was missing from it, or kept after it
    // had gone: its Health was never found from the root (the net's
    // goes-places runs, 2026-11-09). UPower says so without being polled.
    readonly property int upowerDevices: UPower.devices.values.length
    onUpowerDevicesChanged: daemon.refresh()
    Component.onCompleted: daemon.refresh()

    // Whether the daemon hasn't yet answered this watch whole, so a
    // terminal's show asks again for the figures rather than printing
    // what it held before (Provider.stillReading).
    stillReading: daemon.live && !daemon.answeredThisWatch && daemon.state !== "not answering"

    function answer(typed) {
        const question = asks(typed);
        if (question === "")
            return null;
        if (battery === null)
            return null;
        const whole = told();
        const part = partsAsked[question];
        if (part === undefined)
            return whole;
        const row = whole.rows.find(each => each.name === part);
        if (!row)
            return null;
        return {
            headline: row.side,
            copy: row.copy ?? row.side,
            graph: row.graph,
            glyph: Theme.glyph.charge,
            rows: row.verbs ? row.verbs() : [],
            note: row.note !== "" ? row.note : whole.note,
            live: provider
        };
    }

    // How Power stands, said on its row where Machine holds it (Launcher's
    // standsOf): the charge and how long it has, then the power profile in
    // use, as the session says it (Session.qml), so the row reads "36%"
    // and "6 h 41 min to full" and then "Full Performance", a dot between. A
    // machine with no battery says its profile alone.
    function stands() {
        const session = (launcher?.sources ?? []).find(one => one.settings.name === "session");
        return [battery ? doing() : "", session?.profile?.side ?? ""]
            .filter(part => part !== "").join(" \u00b7 ");
    }

    // The charge and what it's doing, a run of parts with a middle dot
    // between each, never a comma (VKCF3Z, 2026-11-09), as it reads at
    // Power's right at home and in Machine.
    function doing() {
        return Power.doing(battery, Interfaces.lasted, Words.of("power").said ?? {});
    }

    // A USB-C port's drawer, the one Power.js's portDrawer lays out, from
    // what the machine daemon holds of its supply and its USB-C port:
    // each role as the kernel lists it, what's plugged in offers, its
    // protocols as its supply's usb_type lists them, and the supply.
    function portDrawer(port) {
        const record = port.record;
        const typec = record.typec ?? {};
        const partner = typec.partner ?? {};
        const known = {
            n: port.n,
            usbType: Power.listed(record.usb_types ?? [], record.usb_type ?? ""),
            supply: record.name ?? "",
            power: typec.power_role ?? "",
            data: typec.data_role ?? "",
            mode: typec.operation_mode ?? "",
            accessory: partner.accessory ?? "",
            capabilities: (partner.source_capabilities ?? []).map(offer => offer.to_volts === null
                ? { volts: offer.volts, amps: offer.amps }
                : { volts: offer.volts, to: offer.to_volts, amps: offer.amps }),
            revision: typec.power_delivery_revision ?? "",
            partnerRevision: partner.power_delivery_revision ?? ""
        };
        return Object.assign({ title: named("port", "", { n: port.n }), eyebrowNames: true },
            Power.portDrawer(known, id => Words.name(id), Words.of("power:port:drawer").said,
                provider.number));
    }

    // The Charger: what's plugged in, briefly, holding each USB-C port and
    // what it gives, or only saying so where the machine has none.
    function chargerPart() {
        const all = supplies();
        const usb = Power.portsOf(all)
            .map(port => Object.assign({ said: Power.portOf(port.record) }, port));
        const charging = usb.find(port => port.said.online && port.said.volts > 0
            && port.said.amps > 0);
        // A charger on a barrel or the dock rather than a USB-C port, said
        // as plugged in, the glossary's word being charger, never mains.
        const plugged = Power.pluggedElsewhere(all);
        const online = usb.find(port => port.said.online);
        const side = chargerSaid(charging, online, plugged);
        const fields = { key: "power:charger", name: named("charger"), side: side, copy: side };
        if (usb.length === 0)
            return copyRow(Object.assign(fields, { glyph: "" }));
        return holderRow(Object.assign(fields, {
            glyph: Theme.glyph.charging,
            note: usb.length === 1 ? says("charger", "port")
                : says("charger", "ports", { count: usb.length })
        }), () => usb.map(port => {
            const what = Power.port(port.said, provider.measured,
                Words.of("power:port").said);
            return copyRow({
                thing: true,
                key: `power:port:${port.n}`,
                name: provider.named("port", "", { n: port.n }),
                side: what,
                copy: what,
                // The mark the Charger's own row wears, so a port beside
                // Copy wears one too (VKCF3Z, 2026-11-08).
                glyph: Theme.glyph.charging,
                // How its power and data go and what's plugged in offers,
                // under the row (Power.js's portDrawer).
                drawer: provider.portDrawer(port)
            });
        }));
    }

    // What the Charger's row says: what charges and on which port, or
    // where something is plugged in, or that nothing is.
    function chargerSaid(charging, online, plugged) {
        if (charging)
            return says("charger", "charging", {
                watts: figure(charging.said.volts * charging.said.amps), n: charging.n });
        if (online)
            return says("charger", "on", { n: online.n });
        return says("charger", plugged ? "plugged" : "none");
    }

    // The last reads of what moves, each kept with its time: the watts
    // drawn or charging at, and the volts. Kept at most once a second, and
    // a gap longer than `stale` starts the sparks afresh.
    property var series: ({ at: 0, watts: [], volts: [] })
    readonly property int second: 1000
    readonly property int stale: 5000
    readonly property int graphed: 20

    function sample(record) {
        const now = Date.now();
        const kept = series;
        if (now - kept.at < second)
            return kept;
        const fresh = now - kept.at <= stale;
        const watts = record.watts ?? record.upower?.rate_w ?? null;
        const add = (list, value) => value === null ? list
            : (fresh ? list : []).concat([value]).slice(-graphed);
        series = { at: now, watts: add(kept.watts, watts), volts: add(kept.volts, record.volts) };
        return series;
    }

    function spark(values, least) {
        return { shape: "spark", values: Machine.spark(values, least), warn: false };
    }

    function bar(value, warn) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, value))], warn: warn === true };
    }

    // A part saying what it is, Return copying it, wearing no glyph.
    function leaf(id, side, fields) {
        return copyRow(Object.assign({
            key: `power:${id}`,
            name: named(id),
            side: side,
            copy: side,
            glyph: ""
        }, fields ?? {}));
    }

    // Has the machine daemon move the charge limit to stop at `wanted`,
    // starting again a little below it where the battery can (its
    // supply.rs, which writes the kernel's files in the order it takes
    // them). The row moves as the daemon tells the supply again.
    function setLimit(name, wanted) {
        daemon.ask({ ask: "act", act: "charge-limit", supply: name, stop_pct: wanted }, said => {
            if (said?.error)
                console.warn(`the charge limit stayed: ${said.error.message}`);
        });
    }

    // The Charge limit: where charging stops and starts again, a setting
    // when Joel's account may write it, and otherwise a figure saying only
    // root sets it.
    function limitPart(record) {
        const stop = record.limit_stop_pct;
        if (stop === null || stop === undefined)
            return null;
        const start = record.limit_start_pct ?? null;
        const side = limitSaid(start, stop);
        if (record.limit_settable !== true)
            return leaf("limit", side, {
                note: record.limit_settable === false ? says("limit", "firmware") : ""
            });
        return holderRow({
            key: "power:limit",
            name: named("limit"),
            side: side,
            glyph: Theme.glyph.charge,
            offers: true
        }, () => provider.stops.map(each => provider.thing({
            key: `verb:power-limit-${each}`,
            name: Words.name("power:limit:stop", "", { n: each }),
            verb: Words.verb("power:limit:stop"),
            glyph: Theme.glyph.charge,
            hue: "action",
            side: stop === each ? says("limit:stop", "chosen") : "",
            stays: true,
            run: () => provider.setLimit(record.name, each)
        })));
    }

    // The Model's drawer: who made it in its eyebrow, where the battery
    // says, then its serial in the Name face.
    function modelDrawer(record) {
        const maker = said(record.manufacturer)
            ? says("model", "eyebrow", { maker: record.manufacturer }) : "";
        return {
            eyebrow: maker,
            eyebrowNames: true,
            title: named("model"),
            facts: [{ label: named("serial"), value: record.serial, identifies: true }]
        };
    }

    // Where charging stops and starts again, as the limit's row says it.
    function limitSaid(start, stop) {
        if (stop === 100 && (start === null || start === 0))
            return says("limit", "full");
        return start === null ? says("limit", "stop", { stop: stop })
            : says("limit", "both", { stop: stop, start: start });
    }

    // Its health against its design and the cycles it's been through, each
    // a part of its side, "92% of design" and "312 cycles" (VKCF3Z,
    // 2026-11-08), or none where the battery says neither.
    function healthItems(ratio, cycles) {
        const items = [];
        if (ratio !== null)
            items.push(says("health", "design", { pct: Math.round(ratio * 100) }));
        if (cycles !== null)
            items.push(cycles === 1 ? says("health", "cycle")
                : says("health", "cycles", { count: number(cycles, 0) }));
        return items;
    }

    // Whether the kernel said something of it: neither nothing nor
    // "Unknown".
    function said(text) {
        return text !== null && text !== undefined && text !== "" && text !== "Unknown";
    }

    // The battery's rows as they stand, the last reads sampled first, so
    // the sparks move while it's shown.
    function told() {
        return toldOf(battery, sample(battery));
    }

    // Every row the battery's answer holds, read from the reads kept and
    // sampling nothing, so Now's Battery can borrow a row's drawer
    // (Piece.qml's drawnBy) without moving the sparks.
    readonly property var all: battery === null ? [] : toldOf(battery, series).rows

    // The battery's rows from `record` and the reads `kept` of it, making
    // nothing and keeping nothing.
    function toldOf(record, kept) {
        const rows = [chargerPart()];
        const state = Power.stateOf(record);

        // The charge, as a bar: the headline says it, but not once it's
        // stepped into. The kernel's word for its level goes with it. A
        // charge the battery doesn't say is left out, never said as none.
        const charge = record.capacity_pct ?? null;
        const barred = charge === null ? undefined
            : bar(charge / 100, state === "discharging" && charge <= low);
        if (charge !== null)
            rows.push(leaf("charge", `${charge}%`, {
                graph: barred,
                note: said(record.capacity_level)
                    ? says("charge", "level", { level: record.capacity_level.toLowerCase() }) : ""
            }));

        const watts = kept.watts.length > 0 ? kept.watts[kept.watts.length - 1] : 0;
        if (watts >= 0.05) {
            const charging = state === "charging";
            rows.push(leaf("rate", `${figure(watts)} W`, {
                name: named("rate", charging ? "charging" : ""),
                graph: spark(kept.watts, 1),
                note: says("rate", charging ? "into" : "from")
            }));
        }
        // A stand-in's energy and voltage can be too small to say, and say
        // nothing true, so each is said from a tenth of its unit.
        const energy = Power.energyOf(record);
        if (energy.full !== null && energy.full >= 0.1) {
            const charged = [["energy:charge", record.charge_ah], ["energy:full",
                record.charge_full_ah], ["energy:design", record.charge_design_ah]]
                .filter(each => each[1] !== null && each[1] !== undefined)
                .map(each => ({ label: named(each[0]), value: `${figure(each[1])} Ah` }));
            if (energy.design !== null)
                charged.push({ label: named("energy:designed"),
                    value: `${figure(energy.design)} Wh` });
            // The battery as the kernel names it, last, in the Name face,
            // rather than in the eyebrow (VKCF3Z, 2026-11-08).
            if (charged.length > 0)
                charged.push({ label: named("device"), value: record.name, identifies: true });
            const side = energy.now === null ? `${figure(energy.full)} Wh`
                : says("energy", "of", { now: figure(energy.now), full: figure(energy.full) });
            rows.push(leaf("energy", side, {
                thing: charged.length > 0,
                note: says("energy", energy.now === null ? "full" : "note"),
                drawer: charged.length > 0 ? {
                    eyebrow: says("energy", "eyebrow"),
                    eyebrowNames: true,
                    title: named("energy"),
                    facts: charged
                } : undefined
            }));
        }
        const volts = kept.volts.length > 0 ? kept.volts[kept.volts.length - 1] : null;
        const designed = record.min_design_volts ?? null;
        // A battery's voltage moves by tenths of a volt around its own, so
        // its spark is drawn above the lowest read, by a fifth of a volt at
        // least, or it would be a flat line at the top.
        if (volts !== null && volts >= 0.1) {
            const lowest = Math.min(...kept.volts);
            rows.push(leaf("voltage", `${figure(volts)} V`, {
                graph: spark(kept.volts.map(each => each - lowest), 0.2),
                note: designed ? says("voltage", "designed", { volts: figure(designed) }) : ""
            }));
        }

        // Its health against its design and the cycles it's been through,
        // a dot between ("92% of design" and "312 cycles", VKCF3Z,
        // 2026-11-08); the kernel's own word for it is the note.
        const ratio = record.health_ratio ?? null;
        const cycles = record.cycles ?? null;
        const items = healthItems(ratio, cycles);
        if (items.length > 0 || said(record.health)) {
            // A battery holding under four fifths of its design is worn,
            // as makers count it, so its bar warns.
            const side = items.length > 0 ? items.join(" \u00b7 ") : record.health;
            rows.push(leaf("health", side, {
                sideItems: items.map((item, at) => ({ said: item, keep: at === 0 ? 2 : 1 })),
                graph: ratio !== null ? bar(ratio, ratio < 0.8) : undefined,
                note: said(record.health) && items.length > 0
                    ? says("health", "says", { health: record.health.toLowerCase() }) : ""
            }));
        }
        if (cycles !== null)
            rows.push(leaf("cycles", number(cycles, 0)));
        const types = record.charge_types ?? [];
        if (said(record.charge_type))
            rows.push(leaf("type", record.charge_type, {
                note: types.length > 1 ? says("type", "of", { some: types.slice(0, -1).join(", "),
                    last: types[types.length - 1] }) : ""
            }));
        const limit = limitPart(record);
        if (limit !== null)
            rows.push(limit);
        const degrees = record.temperature_c ?? null;
        if (degrees !== null) {
            rows.push(leaf("temperature", `${figure(degrees)} \u00b0C`, {
                graph: bar(degrees / hottest, degrees >= warm),
                note: says("temperature", "of", { most: hottest })
            }));
        }
        const what = [record.manufacturer ?? "", record.model ?? ""]
            .filter(part => said(part)).join(" ");
        if (what !== "")
            rows.push(leaf("model", said(record.technology) ? `${what}, ${record.technology}`
                : what, {
                thing: said(record.serial),
                drawer: said(record.serial) ? modelDrawer(record) : undefined
            }));
        const made = Power.madeOf(record.made);
        if (made !== "")
            rows.push(leaf("made", made));

        return {
            headline: doing(),
            graph: barred,
            glyph: Theme.glyph.charge,
            rows: rows,
            note: "",
            live: provider
        };
    }

    // While its answer shows, it's asked again as the daemon's records
    // change, so its figures move and its sparks fill.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider

    Connections {
        target: daemon

        function onHeard(keys) {
            if (provider.shown)
                provider.answered();
        }
    }

    // The machine daemon's supplies, held while Power's rows can be seen.
    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "supply"
        live: provider.seen
        paced: provider.launcher
        keptThroughGap: false
        answersWhole: true
    }
}

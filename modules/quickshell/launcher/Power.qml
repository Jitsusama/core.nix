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
// the battery; it reads them once a second once Power is asked, as
// Machine's Power row asks it, until the launcher closes. The charge limit
// is still written here, through the kernel's own files.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Numbers.js" as Numbers
import "Machine.js" as Machine
import "Power.js" as Power
import "Interfaces.js" as Interfaces

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
    readonly property string batteryKey: battery?.key ?? ""
    onBatteryKeyChanged: provider.answered()

    function number(value, decimals) {
        return Numbers.format(value, decimals, settings.group, settings.point);
    }

    // A figure read off the machine, to three significant digits, its
    // zeros kept to say how closely (VKCF3Z's one rule for every measure).
    function measured(value) {
        return Numbers.significant(value, settings.group, settings.point);
    }

    // A figure to three significant digits, with the real minus.
    function figure(value) {
        return Machine.digits(value).replace("-", "\u2212");
    }

    // Below this share, draining, the battery wants Joel to plug in, as
    // the margin's battery says by default (pieces/Battery.qml), so its bar
    // takes the warning colour; a full battery wants nothing.
    readonly property int low: 20
    // A lithium battery's temperature is drawn against the most it's made
    // to run at, and warns from where it's too warm to charge well.
    readonly property int hottest: 60
    readonly property int warm: 45
    // The limits a step into the charge limit offers, and how far below
    // each charging starts again, as the firmware's own is set.
    readonly property var stops: [60, 70, 80, 90, 100]
    readonly property int restart: 5

    // A part's name, as the vocabulary says it.
    function named(id, state, fill) {
        return Words.name(`power:${id}`, state ?? "", fill);
    }

    // What a part's words say, by the phrase's name.
    function says(id, phrase) {
        return (Words.of(`power:${id}`).said ?? {})[phrase] ?? "";
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

    // Whether Power has been asked since the launcher opened, so the
    // daemon reads the supplies only then.
    property bool asked: false

    function ask() {
        if (!asked)
            Qt.callLater(() => provider.asked = true);
    }

    function answer(typed) {
        const question = asks(typed);
        if (question === "")
            return null;
        ask();
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
        ask();
        const session = (launcher?.sources ?? []).find(one => one.settings.name === "session");
        return [battery ? doing() : "", session?.profile?.side ?? ""]
            .filter(part => part !== "").join(" \u00b7 ");
    }

    function doing() {
        return Power.doing(battery, Interfaces.lasted);
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
        const usb = Power.portsOf(all).map(port => Object.assign({ said: Power.portOf(port.record) },
            port));
        const charging = usb.find(port => port.said.online && port.said.volts > 0
            && port.said.amps > 0);
        // A charger on a barrel or the dock rather than a USB-C port, said
        // as plugged in, the glossary's word being charger, never mains.
        const plugged = Power.pluggedElsewhere(all);
        const online = usb.find(port => port.said.online);
        const side = charging ? `${figure(charging.said.volts * charging.said.amps)} W `
            + `on port ${charging.n}`
            : online ? `on port ${online.n}` : plugged ? "plugged in" : "none plugged in";
        const fields = { key: "power:charger", name: named("charger"), side: side, copy: side };
        if (usb.length === 0)
            return copyRow(Object.assign(fields, { glyph: "" }));
        return holderRow(Object.assign(fields, {
            glyph: Theme.glyph.charging,
            note: usb.length === 1 ? "one USB-C port" : `${usb.length} USB-C ports`
        }), () => usb.map(port => {
            const what = Power.port(port.said, provider.measured,
                Words.of("power:port").said);
            return copyRow({
                thing: true,
                key: `power:port:${port.n}`,
                name: provider.named("port", "", { n: port.n }),
                side: what,
                copy: what,
                glyph: "",
                // How its power and data go and what's plugged in offers,
                // under the row (Power.js's portDrawer).
                drawer: provider.portDrawer(port)
            });
        }));
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

    // Moves the charge limit, each file written in the order the kernel
    // takes them.
    function setLimit(name, start, stop, wanted) {
        for (const write of Power.limitWrites(start, stop, wanted, restart)) {
            writer.path = `/sys/class/power_supply/${name}/${write[0]}`;
            writer.setText(`${write[1]}\n`);
        }
    }

    // The Charge limit: where charging stops and starts again, a setting
    // when Joel's account may write it, and otherwise a figure saying only
    // root sets it.
    function limitPart(record) {
        const stop = record.limit_stop_pct;
        if (stop === null || stop === undefined)
            return null;
        const start = record.limit_start_pct ?? null;
        const side = stop === 100 && (start === null || start === 0) ? "charges fully"
            : start !== null ? `stops at ${stop}%, starts below ${start}%` : `stops at ${stop}%`;
        if (record.limit_settable !== true)
            return leaf("limit", side, {
                note: record.limit_settable === false ? "set in the firmware's settings" : ""
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
            side: stop === each ? "chosen" : "",
            stays: true,
            run: () => provider.setLimit(record.name, start, stop, each)
        })));
    }

    // Whether the kernel said something of it: neither nothing nor
    // "Unknown".
    function said(text) {
        return text !== null && text !== undefined && text !== "" && text !== "Unknown";
    }

    function told() {
        const record = battery;
        const rows = [chargerPart()];
        const kept = sample(record);
        const state = Power.stateOf(record);

        // The charge, as a bar: the headline says it, but not once it's
        // stepped into. The kernel's word for its level goes with it.
        const charge = record.capacity_pct ?? 0;
        const barred = bar(charge / 100, state === "discharging" && charge <= low);
        rows.push(leaf("charge", `${charge}%`, {
            graph: barred,
            note: said(record.capacity_level)
                ? `its level ${record.capacity_level.toLowerCase()}` : ""
        }));

        const watts = kept.watts.length > 0 ? kept.watts[kept.watts.length - 1] : 0;
        if (watts >= 0.05) {
            const charging = state === "charging";
            rows.push(leaf("rate", `${figure(watts)} W`, {
                name: named("rate", charging ? "charging" : ""),
                graph: spark(kept.watts, 1),
                note: charging ? "into the battery" : "from the battery"
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
            rows.push(leaf("energy", `${figure(energy.now ?? 0)} of ${figure(energy.full)} Wh`, {
                note: "now, of what it holds full",
                drawer: charged.length > 0 ? { title: named("energy"), facts: charged } : undefined
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
                note: designed ? `${figure(designed)} V as designed` : ""
            }));
        }

        // Its health against its design and the cycles it's been through,
        // a dot between ("92% of design" and "312 cycles", VKCF3Z,
        // 2026-11-08); the kernel's own word for it is the note.
        const ratio = record.health_ratio ?? null;
        const cycles = record.cycles ?? null;
        const healthItems = [ratio !== null ? `${Math.round(ratio * 100)}% of design` : "",
            cycles !== null ? `${cycles} cycles` : ""].filter(part => part !== "");
        if (healthItems.length > 0 || said(record.health)) {
            // A battery holding under four fifths of its design is worn,
            // as makers count it, so its bar warns.
            const side = healthItems.length > 0 ? healthItems.join(" \u00b7 ") : record.health;
            rows.push(leaf("health", side, {
                sideItems: healthItems.map((item, at) => ({ said: item, keep: at === 0 ? 2 : 1 })),
                graph: ratio !== null ? bar(ratio, ratio < 0.8) : undefined,
                note: said(record.health) && healthItems.length > 0
                    ? `the battery says it's ${record.health.toLowerCase()}` : ""
            }));
        }
        if (cycles !== null)
            rows.push(leaf("cycles", String(cycles)));
        const types = record.charge_types ?? [];
        if (said(record.charge_type))
            rows.push(leaf("type", record.charge_type, {
                note: types.length > 1 ? `of ${types.slice(0, -1).join(", ")} `
                    + `and ${types[types.length - 1]}` : ""
            }));
        const limit = limitPart(record);
        if (limit !== null)
            rows.push(limit);
        const degrees = record.temperature_c ?? null;
        if (degrees !== null) {
            rows.push(leaf("temperature", `${figure(degrees)} \u00b0C`, {
                graph: bar(degrees / hottest, degrees >= warm),
                note: `of the ${hottest} \u00b0C it's made to run at`
            }));
        }
        const what = [record.manufacturer ?? "", record.model ?? ""]
            .filter(part => said(part)).join(" ");
        if (what !== "")
            rows.push(leaf("model", said(record.technology) ? `${what}, ${record.technology}`
                : what, {
                drawer: said(record.serial) ? { title: named("model"), facts: [{
                    label: named("serial"), value: record.serial, named: true }] } : undefined
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

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.asked = false;
        }
    }

    // The machine daemon's supplies, held once Power has been asked since
    // the launcher opened, until it closes.
    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "supply"
        live: provider.launcher !== null && provider.launcher.open && provider.asked
        keptThroughGap: false
        answersWhole: true
    }

    // The kernel's files can't be replaced, only written in place.
    FileView {
        id: writer
        preload: false
        atomicWrites: false
        blockWrites: true
    }
}

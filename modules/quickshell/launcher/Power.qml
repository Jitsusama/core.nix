// The battery and what feeds it, answered in the field: "battery", "power",
// "charger", "plugged in" or "energy". The headline is its charge and what
// it's doing; the rows are the detail a status line leaves out, each a part
// named first: the Charger, holding each USB-C port and what it gives; the
// Charge, as a bar; what the machine is Drawing or Charging at, and the
// Voltage, each a spark of the last reads; the Energy it holds; its Health
// against its design; the Cycles it's been through; the Charge type and
// Charge limit; its Temperature, as a bar; and its Model and when it was
// Made. A charge limit whose files Joel's account may write is a setting,
// its choices a step in; one only root may set is a figure saying so. A
// part has its own words too, "battery health" or "charge limit", which
// answer it alone. The power profile is the session's to show and change
// (Session.qml), and its row comes beside these. Every name and word is
// the vocabulary's (launcher/words.json, Words.qml).
//
// UPower says how it's doing and the kernel's power supply and USB-C files
// the rest, read when asked, so nothing is watched. While the answer shows,
// it's asked again once a second, never more, so its figures move and its
// sparks fill; whether the limit's files are writable is asked once, by
// one spawned test, the first time a limit is shown.
import qs
import Quickshell.Io
import Quickshell.Services.UPower
import QtQuick
import "Numbers.js" as Numbers
import "Machine.js" as Machine
import "Power.js" as Power

Provider {
    id: provider

    examples: ["battery", "charger"]

    readonly property var devices: UPower.devices.values
    readonly property UPowerDevice battery: devices.find(device => device.isLaptopBattery) ?? null

    // What a file says, or "" when it says nothing.
    function said(file) {
        probe.path = file;
        return read(probe).trim();
    }

    // What the power supply's file says, or "" when it says nothing.
    function supplied(device, file) {
        return said(`/sys/class/power_supply/${device}/${file}`);
    }

    // A value in millionths, as the kernel gives volts, amps and amp hours.
    function millionths(device, file) {
        const text = supplied(device, file);
        const value = Number(text);
        return text === "" || isNaN(value) ? null : value / 1e6;
    }

    function number(value, decimals) {
        return Numbers.format(value, decimals, settings.group, settings.point);
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

    // How long, as the margin's battery says it.
    function duration(seconds) {
        const minutes = Math.round(seconds / 60);
        return `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}`;
    }

    // A part's name, as the vocabulary says it.
    function named(id, state, fill) {
        return Words.name(`power:${id}`, state ?? "", fill);
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

    function answer(typed) {
        const question = asks(typed);
        if (battery === null || question === "")
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

    function doing() {
        const charge = Math.round(battery.percentage * 100);
        switch (battery.state) {
        case UPowerDeviceState.Charging:
            return battery.timeToFull > 0
                ? `${charge}%, full in ${duration(battery.timeToFull)}` : `${charge}%, charging`;
        case UPowerDeviceState.Discharging:
            return battery.timeToEmpty > 0
                ? `${charge}%, ${duration(battery.timeToEmpty)} left` : `${charge}%, on battery`;
        case UPowerDeviceState.FullyCharged:
            return `${charge}%, full`;
        default:
            return `${charge}%, on power, holding`;
        }
    }

    // Each USB-C port the machine charges through, by its number, oldest
    // first, as UPower lists its power supplies, online or not.
    function ports() {
        return devices
            .filter(device => device.type === UPowerDeviceType.LinePower)
            .map(device => ({ device: device, port: /USBC\d+:0*(\d+)$/.exec(device.nativePath) }))
            .filter(each => each.port !== null)
            .map(each => ({ path: each.device.nativePath, n: Number(each.port[1]) }))
            .sort((a, b) => a.n - b.n);
    }

    // What a USB-C port says of itself: its power supply, what it's
    // offered, and from the port the kernel's USB-C class numbers one below
    // it, whether anything's plugged in and which way the power goes.
    function portSaid(port) {
        const typec = `/sys/class/typec/port${port.n - 1}`;
        return {
            online: supplied(port.path, "online") === "1",
            volts: millionths(port.path, "voltage_now") ?? millionths(port.path, "voltage_max"),
            amps: millionths(port.path, "current_max") ?? millionths(port.path, "current_now"),
            protocol: Power.protocol(supplied(port.path, "usb_type")),
            partner: said(`${typec}-partner/accessory_mode`) !== "",
            giving: Power.chosen(said(`${typec}/power_role`)).now === "source"
        };
    }

    // The Charger: what's plugged in, briefly, holding each USB-C port and
    // what it gives, or only saying so where the machine has none.
    function chargerPart() {
        const usb = ports().map(port => ({ n: port.n, said: portSaid(port) }));
        const charging = usb.find(port => port.said.online && port.said.volts > 0
            && port.said.amps > 0);
        // A charger on a barrel or the dock rather than a USB-C port, said
        // as plugged in, the glossary's word being charger, never mains.
        const plugged = devices.some(device => device.type === UPowerDeviceType.LinePower
            && device.nativePath !== "" && !/USBC/.test(device.nativePath)
            && supplied(device.nativePath, "online") === "1");
        const online = usb.find(port => port.said.online);
        const side = charging ? `${number(charging.said.volts * charging.said.amps, 0)} W `
            + `on port ${charging.n}`
            : online ? `on port ${online.n}` : plugged ? "plugged in" : "none plugged in";
        const fields = { key: "power:charger", name: named("charger"), side: side, copy: side };
        if (usb.length === 0)
            return copyRow(Object.assign(fields, { glyph: "" }));
        return holderRow(Object.assign(fields, {
            glyph: Theme.glyph.charging,
            note: usb.length === 1 ? "one USB-C port" : `${usb.length} USB-C ports`
        }), () => usb.map(port => {
            const what = Power.port(port.said, provider.number);
            return copyRow({
                key: `power:port:${port.n}`,
                name: provider.named("port", "", { n: port.n }),
                side: what,
                copy: what,
                glyph: ""
            });
        }));
    }

    // The last reads of what moves, each kept with its time: the watts
    // drawn or charging at, and the volts. Read at most once a second, and
    // a gap longer than `stale` starts the sparks afresh.
    property var series: ({ at: 0, watts: [], volts: [] })
    readonly property int second: 1000
    readonly property int stale: 5000
    readonly property int graphed: 20

    function sample(path) {
        const now = Date.now();
        const kept = series;
        if (now - kept.at < second)
            return kept;
        const fresh = now - kept.at <= stale;
        const volts = millionths(path, "voltage_now");
        const amps = millionths(path, "current_now");
        const power = millionths(path, "power_now");
        const watts = power ?? (volts !== null && amps !== null ? Math.abs(volts * amps)
            : battery.changeRate);
        const add = (list, value) => value === null ? list
            : (fresh ? list : []).concat([value]).slice(-graphed);
        series = { at: now, watts: add(kept.watts, watts), volts: add(kept.volts, volts) };
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

    // Whether Joel's account may write the charge limit's files, asked
    // once: null until it's known.
    property var settable: null

    Process {
        id: access
        onExited: code => provider.settable = code === 0
    }

    function askSettable(path) {
        if (provider.settable !== null || access.running)
            return;
        const where = `/sys/class/power_supply/${path}`;
        access.command = ["sh", "-c", "test -w \"$1\" && { test ! -e \"$2\" || test -w \"$2\"; }",
            "sh", `${where}/charge_control_end_threshold`,
            `${where}/charge_control_start_threshold`];
        access.running = true;
    }

    // Moves the charge limit, each file written in the order the kernel
    // takes them.
    function setLimit(path, start, stop, wanted) {
        for (const write of Power.limitWrites(start, stop, wanted, restart)) {
            writer.path = `/sys/class/power_supply/${path}/${write[0]}`;
            writer.setText(`${write[1]}\n`);
        }
    }

    // The Charge limit: where charging stops and starts again, a setting
    // when it can be set, and otherwise a figure saying only root sets it.
    function limitPart(path) {
        const stop = supplied(path, "charge_control_end_threshold");
        if (stop === "")
            return null;
        const start = supplied(path, "charge_control_start_threshold");
        askSettable(path);
        const side = stop === "100" && (start === "" || start === "0") ? "charges fully"
            : start !== "" ? `stops at ${stop}%, starts below ${start}%` : `stops at ${stop}%`;
        if (settable !== true)
            return leaf("limit", side, {
                note: settable === false ? "set in the firmware's settings" : ""
            });
        const begun = start === "" ? null : Number(start);
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
            side: Number(stop) === each ? "in use" : "",
            stays: true,
            run: () => provider.setLimit(path, begun, Number(stop), each)
        })));
    }

    function told() {
        const path = battery.nativePath;
        const rows = [chargerPart()];
        const kept = sample(path);

        // The charge, as a bar: the headline says it, but not once it's
        // stepped into. The kernel's word for its level goes with it.
        const charge = Math.round(battery.percentage * 100);
        const barred = bar(battery.percentage,
            battery.state === UPowerDeviceState.Discharging && charge <= low);
        const level = supplied(path, "capacity_level");
        rows.push(leaf("charge", `${charge}%`, {
            graph: barred,
            note: level !== "" && level !== "Unknown" ? `its level ${level.toLowerCase()}` : ""
        }));

        const watts = kept.watts.length > 0 ? kept.watts[kept.watts.length - 1] : 0;
        if (watts >= 0.05) {
            const charging = battery.state === UPowerDeviceState.Charging;
            rows.push(leaf("rate", `${number(watts, 1)} W`, {
                name: named("rate", charging ? "charging" : ""),
                graph: spark(kept.watts, 1),
                note: charging ? "into the battery" : "from the battery"
            }));
        }
        // A stand-in's energy and voltage can be too small to say, and say
        // nothing true, so each is said from a tenth of its unit.
        if (battery.energyCapacity >= 0.1)
            rows.push(leaf("energy", `${number(battery.energy, 1)} of `
                + `${number(battery.energyCapacity, 1)} Wh`, {
                note: "now, of what it holds full"
            }));
        const volts = kept.volts.length > 0 ? kept.volts[kept.volts.length - 1] : null;
        const designed = millionths(path, "voltage_min_design");
        // A battery's voltage moves by tenths of a volt around its own, so
        // its spark is drawn above the lowest read, by a fifth of a volt at
        // least, or it would be a flat line at the top.
        if (volts !== null && volts >= 0.1) {
            const lowest = Math.min(...kept.volts);
            rows.push(leaf("voltage", `${number(volts, 2)} V`, {
                graph: spark(kept.volts.map(each => each - lowest), 0.2),
                note: designed ? `${number(designed, 2)} V as designed` : ""
            }));
        }

        const full = millionths(path, "charge_full") ?? millionths(path, "energy_full");
        const design = millionths(path, "charge_full_design")
            ?? millionths(path, "energy_full_design");
        const health = supplied(path, "health");
        if (full && design) {
            const share = `${Math.round(full / design * 100)}% of its design`;
            const hours = millionths(path, "charge_full") !== null && designed;
            // A battery holding under four fifths of its design is worn,
            // as makers count it, so its bar warns.
            rows.push(leaf("health", health !== "" && health !== "Unknown"
                ? `${health}, ${share}` : share, {
                graph: bar(full / design, full / design < 0.8),
                note: hours ? `${number(full * designed, 1)} of the `
                    + `${number(design * designed, 1)} Wh it was made to hold` : ""
            }));
        } else if (health !== "" && health !== "Unknown") {
            rows.push(leaf("health", health));
        }
        const cycles = supplied(path, "cycle_count");
        if (cycles !== "")
            rows.push(leaf("cycles", cycles));
        const types = Power.chosen(supplied(path, "charge_types") || supplied(path, "charge_type"));
        if (types.now !== "")
            rows.push(leaf("type", types.now, {
                note: types.all.length > 1 ? `of ${types.all.slice(0, -1).join(", ")} `
                    + `and ${types.all[types.all.length - 1]}` : ""
            }));
        const limit = limitPart(path);
        if (limit !== null)
            rows.push(limit);
        const tenths = supplied(path, "temp");
        if (tenths !== "") {
            const degrees = Number(tenths) / 10;
            rows.push(leaf("temperature", `${number(degrees, 1)} \u00b0C`, {
                graph: bar(degrees / hottest, degrees >= warm),
                note: `of the ${hottest} \u00b0C it's made to run at`
            }));
        }
        const what = [supplied(path, "manufacturer"), supplied(path, "model_name")]
            .filter(part => part !== "").join(" ");
        const technology = supplied(path, "technology");
        if (what !== "")
            rows.push(leaf("model", technology !== "" && technology !== "Unknown"
                ? `${what}, ${technology}` : what));
        const made = Power.made(Number(supplied(path, "manufacture_year")),
            Number(supplied(path, "manufacture_month")), Number(supplied(path, "manufacture_day")));
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

    // While its answer shows, it's asked again each second, so its figures
    // move and its sparks fill.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider

    Timer {
        interval: provider.second
        repeat: true
        running: provider.shown
        onTriggered: provider.answered()
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }

    // The kernel's files can't be replaced, only written in place.
    FileView {
        id: writer
        preload: false
        atomicWrites: false
        blockWrites: true
    }
}

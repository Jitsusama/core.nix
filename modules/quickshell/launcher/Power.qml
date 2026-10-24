// The battery and what feeds it, answered in the field: "battery", "power",
// "charger", "plugged in" or "energy". The headline is its charge and what it's doing; the rows are
// the detail a status line leaves out: the charger and what it offers, what
// the machine draws, the battery's health against its design and how many
// cycles it's been through, where charging stops and starts again, its
// temperature, and what it is and when it was made. The power profile is
// the session's to show and change (Session.qml), and its rows come
// beside these. Each row is a part, named as the vocabulary names it
// (launcher/words.json, Words.qml), with what it says beside it, and the
// words that ask the answer are the vocabulary's too.
// UPower says how it's doing and the kernel's power supply files the rest,
// each read when asked, so nothing is spawned and nothing is watched.
import qs
import Quickshell.Io
import Quickshell.Services.UPower
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["battery", "charger"]

    readonly property var devices: UPower.devices.values
    readonly property UPowerDevice battery: devices.find(device => device.isLaptopBattery) ?? null

    // What the power supply's file says, or "" when it says nothing.
    function said(device, file) {
        probe.path = `/sys/class/power_supply/${device}/${file}`;
        return read(probe).trim();
    }

    // A value in millionths, as the kernel gives volts, amps and amp hours.
    function millionths(device, file) {
        const text = said(device, file);
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

    // How long, as the margin's battery says it.
    function duration(seconds) {
        const minutes = Math.round(seconds / 60);
        return `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}`;
    }

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.started("power")

    function answer(typed) {
        if (battery === null || asks(typed) === "")
            return null;
        return told();
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

    // Each charger plugged in, by what it offers: a USB-C port's most, or a
    // barrel's being there at all.
    function chargers() {
        return devices
            .filter(device => device.type === UPowerDeviceType.LinePower && device.nativePath !== "")
            .filter(device => said(device.nativePath, "online") === "1")
            .map(device => {
                const volts = millionths(device.nativePath, "voltage_max");
                const amps = millionths(device.nativePath, "current_max");
                const port = /USBC\d+:0*(\d+)$/.exec(device.nativePath);
                const kind = /\[(PD[^\]]*)\]/.exec(said(device.nativePath, "usb_type"));
                const where = port ? `USB-C port ${port[1]}${kind ? `, ${kind[1]}` : ""}` : "mains";
                return volts && amps
                    ? `${number(volts * amps, 0)} W, ${number(volts, 0)} V at ${number(amps, 1)} A, ${where}`
                    : where;
            });
    }

    function told() {
        const path = battery.nativePath;
        const glyph = String.fromCodePoint(0xf0079);
        const rows = [];
        // A part by its name, what it says beside it, Return copying that.
        const part = (id, side, state, graph) => rows.push(copyRow({
            name: Words.name(`power:${id}`, state ?? ""),
            side: side,
            copy: side,
            glyph: glyph,
            graph: graph
        }));

        const plugged = chargers();
        // UPower's own device for the USB-C ports and the mains overlap, so
        // the most precise one plugged in is said.
        const charger = plugged.find(text => text.includes(" W, ")) ?? plugged[0] ?? null;
        part("charger", charger ?? "none plugged in");
        // The charge, as a bar: the headline says it, but not once it's
        // stepped into.
        const charge = Math.round(battery.percentage * 100);
        const bar = {
            shape: "bar",
            values: [battery.percentage],
            warn: battery.state === UPowerDeviceState.Discharging && charge <= low
        };
        part("charge", `${charge}%`, "", bar);
        if (battery.changeRate >= 0.05)
            part("rate", `${number(battery.changeRate, 1)} W`,
                battery.state === UPowerDeviceState.Charging ? "charging" : "");

        const full = millionths(path, "charge_full");
        const design = millionths(path, "charge_full_design");
        const volts = millionths(path, "voltage_min_design");
        if (full && design)
            part("health", `${Math.round(full / design * 100)}% of its design${
                volts ? `, ${number(full * volts, 1)} of ${number(design * volts, 1)} Wh` : ""}`);
        const cycles = said(path, "cycle_count");
        if (cycles !== "")
            part("cycles", cycles);
        const stop = said(path, "charge_control_end_threshold");
        const start = said(path, "charge_control_start_threshold");
        if (stop !== "" && stop !== "100")
            part("limit",
                start !== "" ? `stops at ${stop}%, starts below ${start}%` : `stops at ${stop}%`);
        const tenths = said(path, "temp");
        if (tenths !== "")
            part("temperature", `${number(Number(tenths) / 10, 1)} °C`);
        // A date only when it's one, since some batteries say nonsense.
        const year = Number(said(path, "manufacture_year"));
        const month = Number(said(path, "manufacture_month"));
        const dated = year >= 1990 && month >= 1 && month <= 12;
        const what = [said(path, "manufacturer"), said(path, "model_name"), said(path, "technology")]
            .filter(part => part !== "").join(" ");
        if (what !== "")
            part("model", dated ? `${what}, made ${year}-${String(month).padStart(2, "0")}` : what);

        return { headline: doing(), graph: bar, rows: rows, note: "from UPower and the kernel" };
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}

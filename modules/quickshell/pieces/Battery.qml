// The battery: its charge, the time left or until full, or that it's on
// power and holding, and the power it's drawing when there's any to tell.
// It's true while it runs low on battery, below `low` percent, when it wants
// Joel to plug in and takes the warning colour, and below `critical` it
// wants him now, in the alert colour. It offers to save power.
import qs
import Quickshell.Services.UPower
import QtQuick

Piece {
    id: piece

    readonly property UPowerDevice battery: UPower.displayDevice
    readonly property int charge: Math.round(battery.percentage * 100)
    readonly property bool draining: battery.state === UPowerDeviceState.Discharging
    readonly property bool critical: active && charge <= (settings.critical ?? 10)

    present: battery.isLaptopBattery
    active: draining && charge <= (settings.low ?? 20)
    role: !active ? "text" : critical ? "alert" : "warning"
    // Low, it wants Joel to plug in; critical, it wants him now.
    wants: active
    now: critical
    sign: battery.state === UPowerDeviceState.Charging ? "charging" : settings.sign ?? ""
    // The charge, and the time left while it drains.
    least: charge + "%"
    glance: charge + "%" + (draining && battery.timeToEmpty > 0
        ? " " + duration(battery.timeToEmpty) + " left" : "")
    value: {
        const parts = [charge + "%"];
        if (battery.state === UPowerDeviceState.Charging)
            parts.push(battery.timeToFull > 0 ? "full in " + duration(battery.timeToFull) : "charging");
        else if (draining && battery.timeToEmpty > 0)
            parts.push(duration(battery.timeToEmpty) + " left");
        else if (!draining)
            parts.push("on power");
        // A tenth of a watt is the least it says, so it never shows 0.0.
        if (battery.changeRate >= 0.05)
            parts.push(battery.changeRate.toFixed(1) + " W");
        return parts.join(", ");
    }

    title: "Battery"
    // Running low, and then critical, is said as it happens.
    arrives: true
    app: "Battery"
    hint: battery.timeToEmpty > 0
        ? `${charge}% left, about ${Math.round(battery.timeToEmpty / 60)} minutes`
        : `${charge}% left`
    context: battery.changeRate >= 0.05
        ? `drawing ${battery.changeRate.toFixed(1)} W` : "plug in to keep going"
    actions: PowerProfiles.profile === PowerProfile.PowerSaver ? [] : [{
        glyph: Words.glyph("verb:on:low:save"),
        name: Words.name("verb:on:low:save"),
        verb: Words.verb("verb:on:low:save"),
        stays: true,
        run: () => PowerProfiles.profile = PowerProfile.PowerSaver
    }]

    function duration(seconds: real): string {
        const minutes = Math.round(seconds / 60);
        return Math.floor(minutes / 60) + ":" + String(minutes % 60).padStart(2, "0");
    }
}

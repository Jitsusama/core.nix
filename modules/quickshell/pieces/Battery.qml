// The battery: its charge, the time left or until full, or that it's on
// power and holding, and the power it's drawing when there's any to tell.
// It's true while it runs low on battery, below `low` percent, and takes the
// warning colour then and the alert colour below `critical`.
import Quickshell.Services.UPower
import QtQuick

Piece {
    id: piece

    readonly property UPowerDevice battery: UPower.displayDevice
    readonly property int charge: Math.round(battery.percentage * 100)
    readonly property bool draining: battery.state === UPowerDeviceState.Discharging

    present: battery.isLaptopBattery
    active: draining && charge <= (settings.low ?? 20)
    role: !active ? "text" : charge <= (settings.critical ?? 10) ? "alert" : "warning"
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

    function duration(seconds: real): string {
        const minutes = Math.round(seconds / 60);
        return Math.floor(minutes / 60) + ":" + String(minutes % 60).padStart(2, "0");
    }
}

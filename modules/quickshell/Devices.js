// The Bluetooth devices whose batteries run low, worked out apart from
// the piece that says them (pieces/Devices.qml) so tests/devices.qml can
// hold it in Qt's own engine with plain objects standing in for BlueZ's.
// A device is read as Quickshell.Bluetooth gives it: its `address`, `name`,
// `icon`, whether it's `connected`, and `battery`, 0 to 1, where it
// reports one (`batteryAvailable`).
.pragma library
.import "Crossing.js" as Crossing

// The theme's glyph for a device, by name, from the icon BlueZ gives it, so
// the Bluetooth place and a device's mark draw it the same.
const KINDS = {
    "audio-headphones": "headphones",
    "audio-headset": "headset",
    "audio-card": "speaker",
    "input-mouse": "mouse",
    "input-tablet": "mouse",
    "input-keyboard": "keyboard",
    "input-gaming": "gamepad",
    phone: "phone",
    computer: "computer",
    "video-display": "display"
};

function kind(icon) {
    return KINDS[icon] ?? "bluetooth";
}

// Every glyph `kind` can give, so tests/drawings.qml can hold each to
// having a drawing, since a device running low is drawn by it in the margin.
function kinds() {
    return [...new Set([...Object.values(KINDS), "bluetooth"])];
}

function charge(device) {
    return Math.round(device.battery * 100);
}

// How each connected device with a battery stands now, by address, from
// how it stood before: "low", "critical" or "". One gone is forgotten, so
// it's news again should it come back still low. BlueZ doesn't say when a
// device charges, so only its charge rising re-arms it (Crossing.js).
function standings(devices, before, marks) {
    const now = {};
    for (const device of devices) {
        if (device.connected && device.batteryAvailable) {
            now[device.address] = Crossing.next(before[device.address] ?? "", charge(device),
                false, marks);
        }
    }
    return now;
}

// The devices running low, as things of a piece (pieces/Piece.qml's
// `each`), each signed by its own name and drawn by its own kind's glyph, so
// it reads as that device and never as the laptop's battery. It wants Joel
// to charge it, and at the critical mark wants him now.
function low(devices, standing) {
    return devices.filter(device => (standing[device.address] ?? "") !== "")
        .map(device => {
            const critical = standing[device.address] === "critical";
            return {
                key: device.address,
                address: device.address,
                active: true,
                wants: true,
                now: critical,
                role: critical ? "alert" : "warning",
                title: device.name,
                app: device.name,
                markGlyph: kind(device.icon),
                value: `${charge(device)}%`,
                hint: `${charge(device)}% left`,
                context: critical ? "charge it now" : "charge it soon",
                arrives: true
            };
        });
}

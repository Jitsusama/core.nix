// The Bluetooth devices whose batteries run low, and the adapters whose
// controllers stopped answering, worked out apart from the piece that says
// them (pieces/Devices.qml) so tests/devices.qml can hold it in Qt's own
// engine with plain objects standing in for BlueZ's.
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
// to charge it, and at the critical mark wants him now. In Now it's the
// device's own row in Bluetooth, its key the thing's mark, so one device is
// drawn one way wherever it's listed (VKCF3Z, 2026-10-29).
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
                place: "bluetooth",
                value: `${charge(device)}%`,
                hint: `${charge(device)}% left`,
                context: critical ? "charge it now" : "charge it soon",
                // Said as it runs low, and taken back once it's charged.
                arrives: true,
                withdraws: true
            };
        });
}

// The adapters whose controllers are up but have stopped answering, from
// bluetooth-links's stopped.json: each one's `name`, and the `device` and
// `driver` it sits on as sysfs names them, null where it doesn't. No file,
// or one not its own, says none have.
function halted(text) {
    try {
        const { adapters } = JSON.parse(text);
        return Array.isArray(adapters) ? adapters
            .filter(adapter => typeof adapter?.name === "string")
            .map(adapter => ({
                name: adapter.name,
                device: adapter.device ?? null,
                driver: adapter.driver ?? null
            })) : [];
    } catch (_malformed) {
        return [];
    }
}

// A device as the kernel names it on its bus, as 0000:00:14.7 or 1-10:1.0,
// and a driver's folder in sysfs, nothing else, since the line they make is
// typed into a root shell.
const DEVICE = /^[0-9A-Za-z][0-9A-Za-z:._-]*$/u;
const DRIVER = /^\/sys\/bus\/[a-z0-9_-]+\/drivers\/[A-Za-z0-9_][A-Za-z0-9_.-]*$/u;

// The line that starts a stopped controller again: its driver let go of
// its device and given it back, which starts the controller afresh as it
// does at boot. "" where what it sits on isn't known, or isn't a name the
// line could carry safely.
function restart(adapter) {
    const device = adapter.device ?? "";
    const driver = adapter.driver ?? "";
    if (!(DEVICE.test(device) && DRIVER.test(driver))) {
        return "";
    }
    return `echo ${device} | sudo tee ${driver}/unbind; echo ${device} | sudo tee ${driver}/bind`;
}

const AFTERWARDS = "Afterwards, your mouse may need pairing again.";

// The adapters stopped, as things of a piece (pieces/Piece.qml's `each`):
// BlueZ still says such a one is on while nothing it carries works, so it
// wants Joel, said once as a notice with the line that starts it again and
// taken back once it answers. The one Quickshell names as `own`, the
// default, is the Adapter's row in Bluetooth, its key the thing's mark, so
// Now draws it as that row; another is a fact of its own.
function stopped(adapters, own) {
    return adapters.map(adapter => {
        const line = restart(adapter);
        const mine = adapter.name === own;
        return {
            key: mine ? "adapter" : `adapter:${adapter.name}`,
            adapter: adapter.name,
            active: true,
            wants: true,
            now: false,
            role: "warning",
            title: "Bluetooth",
            app: "Bluetooth",
            markGlyph: "bluetooth",
            place: mine ? "bluetooth" : "",
            value: "not answering",
            hint: "Bluetooth stopped answering",
            context: line === "" ? "restart the machine to start it again"
                : "type the line in a terminal to start it again",
            message: line === "" ? `Restarting the machine starts it again. ${AFTERWARDS}`
                : `Type this in a terminal to start it again:\n${line}\n${AFTERWARDS}`,
            line,
            // Said as it stops, and taken back once it answers again.
            arrives: true,
            withdraws: true
        };
    });
}

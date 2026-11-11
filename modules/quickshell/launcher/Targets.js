.pragma library
// Where a program's sound goes (VKCF3Z, 2026-11-09): which device each of
// its streams is sent to, as the sound daemon serves the default
// metadata's target.object, by stream id, and which of its streams a send
// asks the daemon to move. WirePlumber moves a stream live when its
// target.object changes, and keeps it for the program's next stream too.
// Plain JavaScript, so tests/sound.qml holds it in Qt's own engine, as the
// shell runs it.

// Where a program's streams are sent: the one target they share, or ""
// where they share none, as when none is sent anywhere and all follow the
// default.
function sentTo(streams, targets) {
    const sent = streams.map(stream => targets[String(stream.id)] ?? "");
    return sent.length > 0 && sent.every(each => each === sent[0]) ? sent[0] : "";
}

// Whether a device is the one a target names, by either name WirePlumber
// takes.
function names(device, target) {
    return target !== "" && (device.name === target
        || String(device.properties?.["object.serial"] ?? "") === target);
}

// Where a program's sound can go: the default first, then each device in
// the order given, the one it's sent to chosen, or the default where it's
// sent to none listed, since it follows the default then.
function choices(devices, target) {
    const sent = devices.find(device => names(device, target)) ?? null;
    return [{ device: null, chosen: sent === null }]
        .concat(devices.map(device => ({ device, chosen: device === sent })));
}

// The ids of the streams a send moves, each a whole number, as pipewire's
// are, the rest left alone.
function sendable(streams) {
    return streams.filter(stream => Number.isInteger(stream.id) && stream.id >= 0)
        .map(stream => String(stream.id));
}

// The value :setlocal takes for the default, beside each device's name.
const DEFAULT = "default";

function byDefault() {
    return DEFAULT;
}

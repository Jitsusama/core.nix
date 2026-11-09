.pragma library
// Where a program's sound goes (VKCF3Z, 2026-11-09): which device each of
// its streams is sent to, as `pw-metadata -m` says the default metadata's
// target.object, and the commands that send one to a device or back to
// the default. WirePlumber moves a stream live when its target.object
// changes, and keeps it for the program's next stream too. Plain
// JavaScript, so tests/sound.qml holds it in Qt's own engine, as the
// shell runs it.

// The key WirePlumber sends a stream by, to a device's node.name or its
// object.serial.
const TARGET = "target.object";
// An update as pw-metadata prints one, "update: id:93 key:'target.object'
// value:'stand-in-headphones' type:''", a key or value deleted printed as
// (null).
const UPDATE = /^update: id:(\d+) key:(?:'([^']*)'|\(null\)) value:(?:'([^']*)'|\(null\))/;
// What a stream sent nowhere is said to be sent to.
const NOWHERE = ["", "-1"];

// The targets once a line of pw-metadata's is heard, by stream id, each
// the name it's sent to. Only target.object counts; a stream's id
// deleted whole, or its target, sends it nowhere.
function heard(targets, line) {
    const update = UPDATE.exec(String(line).trim());
    if (update === null) {
        return targets;
    }
    const [, id, key, value] = update;
    if (key !== undefined && key !== TARGET) {
        return targets;
    }
    const next = Object.assign({}, targets);
    if (key === undefined || value === undefined || NOWHERE.includes(value)) {
        delete next[id];
    } else {
        next[id] = value;
    }
    return next;
}

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

// The targets once each stream is sent to a device, by its name, or back
// to the default, given "", as `sending` sends them.
function sentAs(targets, streams, target) {
    const next = Object.assign({}, targets);
    for (const stream of streams) {
        if (target === "") {
            delete next[String(stream.id)];
        } else {
            next[String(stream.id)] = target;
        }
    }
    return next;
}

// The commands that send each stream to a device, by its name, or back to
// the default, given "".
function sending(program, streams, target) {
    return streams.map(stream => target === ""
        ? [program, "-n", "default", "-d", String(stream.id), TARGET]
        : [program, "-n", "default", String(stream.id), TARGET, target]);
}

// The Bluetooth devices running low: each connected one whose battery
// falls to `low` percent is a thing of its own (Piece.qml's `each`), said
// once as a notice signed by its name, and once more, urgent, at `critical`,
// the laptop battery's own marks, so one rule holds for every battery
// (docs/design.md). It's in the margin by its kind's glyph and in Now, as
// its own row in Bluetooth, until it rises past the mark or goes. Its
// notice offers to show it there. Its charge is read from the
// devices Quickshell.Bluetooth already holds, as the Bluetooth place
// reads it, so nothing more is opened for it, and nothing for each device.
import qs
import Quickshell.Bluetooth
import QtQuick
import "../Devices.js" as Devices

Piece {
    id: piece

    readonly property var adapter: Bluetooth.defaultAdapter
    readonly property var marks: ({ low: settings.low ?? 20, critical: settings.critical ?? 10 })

    // Each device as plain values, read here so a change to any of them,
    // and only these, reads them again.
    readonly property var readings: adapter === null ? [] : adapter.devices.values
        .filter(device => device.deviceName !== "")
        .map(device => ({
            address: device.address,
            name: device.name,
            icon: device.icon,
            connected: device.connected,
            batteryAvailable: device.batteryAvailable,
            battery: device.battery
        }))

    // How each stood at its last reading, by address (Devices.js).
    property var standing: ({})

    present: adapter !== null
    each: []

    onReadingsChanged: read()
    Component.onCompleted: read()

    // How they stand, and the things running low, worked out together at
    // each reading, so no thing is ever made from a reading and the
    // standing before it.
    function read() {
        standing = Devices.standings(readings, standing, marks);
        each = Devices.low(readings, standing).map(own => thing(Object.assign(own, {
            actions: [{
                glyph: Words.glyph("verb:on:device:show"),
                name: Words.name("verb:on:device:show"),
                verb: Words.verb("verb:on:device:show"),
                run: shell => piece.show(shell, own.address)
            }]
        })));
    }

    // Into the device's own place in Bluetooth, where its battery is, or
    // Bluetooth itself should it have gone by then.
    function show(shell, address) {
        shell.launcher.into("bluetooth");
        shell.launcher.enter(shell.launcher.matches.find(row => row.key === `bluetooth:${address}`));
    }
}

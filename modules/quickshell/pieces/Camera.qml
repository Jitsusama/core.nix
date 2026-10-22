// A program with a camera open: true while any of the machine's cameras is
// read, and which programs, from the file each camera's module keeps naming
// them, one to a line and empty when nobody reads or the camera is down
// (modules/v4l2loopback, modules/ir-camera): /run/camera-holders/camera and
// /run/camera-holders/ir-camera. A machine without them keeps no such file,
// and the piece is never there. The files are replaced whole as programs
// come and go, which the folder's watch hears, and the folder stays from
// boot to shutdown, so the watch is never lost and nothing is asked.
import Quickshell.Io
import QtQuick

Piece {
    id: piece

    // Each camera's file, by what it's called.
    readonly property var cameras: [
        { name: "the camera", file: front },
        { name: "the infrared camera", file: infrared }
    ]
    readonly property var reading: cameras.map(camera => ({
        name: camera.name,
        programs: camera.file.loaded
            ? camera.file.text().split("\n").map(line => line.trim()).filter(line => line !== "")
            : []
    }))
    // Quickshell's JavaScript has no flatMap.
    readonly property var programs: [...new Set(reading.reduce((all, camera) => all.concat(camera.programs), []))]

    present: front.loaded || infrared.loaded
    active: programs.length > 0
    value: programs.length === 0 ? "off" : programs.join(", ")
    title: "Camera"
    hint: `${programs.join(" and ")} ${programs.length === 1 ? "has" : "have"} the camera open`
    context: reading.filter(camera => camera.programs.length > 0)
        .map(camera => `${camera.name}: ${camera.programs.join(", ")}`).join("; ")

    FileView {
        id: front
        path: "/run/camera-holders/camera"
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
    }

    FileView {
        id: infrared
        path: "/run/camera-holders/ir-camera"
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
    }
}

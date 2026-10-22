// A program with a camera open: true while any of the machine's cameras is
// read, and which programs, from the file v4l2loopback keeps for each
// camera it names (`jitsusama.v4l2loopback.holders`), one program to a
// line and empty when nobody reads or the camera is down, at
// /run/camera-holders/<name>. A machine naming none has no piece. The
// files are replaced whole as programs come and go, which the folder's
// watch hears, and the folder stays from boot to shutdown, so the watch is
// never lost and nothing is asked.
//
// A holder is named by the program itself, so this says who has a camera,
// not who may: it's for seeing at a glance, not for trusting.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import QtQuick

Piece {
    id: piece

    // What each camera is called, by the name the machine gives it.
    function called(name) {
        return name === "camera" ? "the camera"
            : name === "ir-camera" ? "the infrared camera" : `the ${name}`;
    }

    readonly property var names: settings.holders ?? []
    property var held: ({})
    readonly property var reading: names.map(name => ({
        name: called(name),
        programs: held[name] ?? []
    }))
    // Quickshell's JavaScript has no flatMap.
    readonly property var programs: [...new Set(reading.reduce((all, camera) => all.concat(camera.programs), []))]

    present: names.length > 0
    active: programs.length > 0
    value: programs.length === 0 ? "off" : programs.join(", ")
    title: "Camera"
    hint: `${programs.join(" and ")} ${programs.length === 1 ? "has" : "have"} the camera open`
    context: reading.filter(camera => camera.programs.length > 0)
        .map(camera => `${camera.name}: ${camera.programs.join(", ")}`).join("; ")

    Instantiator {
        model: piece.names

        FileView {
            required property string modelData
            path: `/run/camera-holders/${modelData}`
            watchChanges: true
            printErrors: false
            onFileChanged: reload()
            onLoaded: piece.heard(modelData, text())
            onLoadFailed: piece.heard(modelData, "")
        }
    }

    function heard(name, text) {
        const programs = text.split("\n").map(line => line.trim()).filter(line => line !== "");
        held = Object.assign({}, held, { [name]: programs });
    }
}

// A program with a camera open: true while any of the machine's cameras is
// read, and which programs, from the file kept for each camera at
// /run/camera-holders/<name>, one program to a line and empty when nobody
// reads or the camera is down. v4l2loopback keeps one for each loopback
// camera the machine names (`jitsusama.v4l2loopback.holders`), and
// usb-camera-holders one for each USB camera while it's plugged in, named
// for the camera and listing programs only while they stream. So the
// cameras are the ones the machine names and whatever else the folder
// holds, and a machine with neither has no piece. The files are replaced
// whole as programs come and go, which each file's watch hears, and the
// folder's watch hears a camera come and go; the folder stays from boot
// to shutdown, so neither watch is lost and nothing is asked.
//
// A holder is named by the program itself, so this says who has a camera,
// not who may: it's for seeing at a glance, not for trusting.
pragma ComponentBehavior: Bound
import Quickshell.Io
import Qt.labs.folderlistmodel
import QtQuick

Piece {
    id: piece

    // What each camera is called: by the name the machine gives it, or a
    // USB camera by its own, which its file is named for.
    function called(name) {
        return name === "camera" ? "the camera"
            : name === "ir-camera" ? "the infrared camera"
            : configured.includes(name) ? `the ${name}` : name;
    }

    readonly property var configured: settings.holders ?? []
    // The cameras with a file in the folder, listed again whenever the
    // folder's model says it changed. A file being replaced is written
    // beside it first, under its name and a number, and is no camera of its
    // own; that file renamed onto a new camera's leaves the count as it
    // was, and the model says so only as changed rows, so the count alone
    // won't do. The list is kept when the names are the same, so each
    // file's watch stays rather than being made again.
    property var listed: []

    function list() {
        const names = [];
        for (let i = 0; i < folder.count; i++) {
            const name = folder.get(i, "fileName");
            if (!/\.\d+$/.test(name))
                names.push(name);
        }
        if (names.join("\n") !== listed.join("\n"))
            listed = names;
    }
    readonly property var names: [...new Set(configured.concat(listed))]
    property var held: ({})
    readonly property var reading: names.map(name => ({
        name: called(name),
        programs: held[name] ?? []
    }))
    // Quickshell's JavaScript has no flatMap.
    readonly property var programs: [...new Set(reading.reduce((all, camera) => all.concat(camera.programs), []))]

    present: names.length > 0
    active: programs.length > 0
    kind: "onAir"
    value: programs.length === 0 ? "off" : programs.join(", ")
    title: "Camera"
    hint: `${programs.join(" and ")} ${programs.length === 1 ? "has" : "have"} the camera open`
    context: reading.filter(camera => camera.programs.length > 0)
        .map(camera => `${camera.name}: ${camera.programs.join(", ")}`).join("; ")

    FolderListModel {
        id: folder
        folder: "file:///run/camera-holders"
        showDirs: false
        showDotAndDotDot: false
        onRowsInserted: piece.list()
        onRowsRemoved: piece.list()
        onDataChanged: piece.list()
        onModelReset: piece.list()
    }

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

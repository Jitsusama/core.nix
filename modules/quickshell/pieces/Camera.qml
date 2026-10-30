// A program with a camera open: true while any of the machine's cameras is
// read, and which programs, from the file kept for each camera at
// /run/camera-holders/<name>, one program to a line and empty when nobody
// reads or the camera is down. v4l2loopback keeps one for each loopback
// camera the machine names (`jitsusama.v4l2loopback.holders`), and
// usb-camera-holders one for each USB camera while it's plugged in, named
// for the camera and listing programs only while they stream. So the
// cameras are the ones the machine names and whatever else the folder
// holds, and a machine with neither has no piece. The files are replaced
// whole as programs come and go, written beside and renamed over, so the
// folder's one watch hears every change, and one reader reads every file
// as it does; the folder stays from boot to shutdown, so the watch is
// never lost and nothing is asked.
//
// A holder is named by the program itself, so this says who has a camera,
// not who may: it's for seeing at a glance, not for trusting.
pragma ComponentBehavior: Bound
import qs
import Quickshell.Io
import Qt.labs.folderlistmodel
import QtQuick

Piece {
    id: piece

    // What each camera is called in a sentence, as the vocabulary says it
    // (launcher/words.json): the machine's own by their own words or by the
    // name the machine gives them, or a USB camera by its own, which its
    // file is named for.
    function called(name) {
        const own = Words.of(`cameras:${name}`).said;
        if (own !== undefined)
            return own;
        const machines = Words.of("cameras").said ?? "{name}";
        return configured.includes(name) ? machines.replace("{name}", name) : name;
    }

    readonly property var configured: settings.holders ?? []
    // The cameras with a file in the folder, listed again whenever the
    // folder's model says it changed. A file being replaced is written
    // beside it first, under its name and a number, and is no camera of its
    // own; that file renamed onto a new camera's leaves the count as it
    // was, and the model says so only as changed rows, so the count alone
    // won't do. The list is kept when the names are the same, so nothing
    // drawn from it is made again.
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

    function changed() {
        list();
        read();
    }
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
    // A program opens a camera by itself, so it's said when it does.
    arrives: true
    app: "Camera"
    // The cameras being read. With one, the hint says which and who, and
    // nothing more needs saying; with more, the context says who has each.
    readonly property var open: reading.filter(camera => camera.programs.length > 0)
    hint: open.length === 1
        ? `${open[0].programs.join(" and ")} ${open[0].programs.length === 1 ? "has" : "have"}`
            + ` ${open[0].name} open`
        : `${programs.join(" and ")} have cameras open`
    context: open.length > 1
        ? open.map(camera => `${camera.name}: ${camera.programs.join(", ")}`).join("; ") : ""

    FolderListModel {
        id: folder
        folder: "file:///run/camera-holders"
        showDirs: false
        showDotAndDotDot: false
        onRowsInserted: piece.changed()
        onRowsRemoved: piece.changed()
        onDataChanged: piece.changed()
        onModelReset: piece.changed()
    }

    // Every file in the folder read in one run, each after a record
    // separator and its name on a line of its own, since a program's name
    // may hold anything but that.
    Process {
        id: reader
        command: ["sh", "-c", "for file in \"$1\"/*; do [ -f \"$file\" ] || continue;"
            + " printf '\\036%s\\n' \"${file##*/}\"; cat \"$file\"; done", "sh",
            "/run/camera-holders"]
        onExited: {
            if (piece.again) {
                piece.again = false;
                reader.running = true;
            }
        }
        stdout: StdioCollector {
            onStreamFinished: piece.heard(text)
        }
    }

    // Asked to read while reading, so the last change always counts.
    property bool again: false

    function read() {
        if (reader.running)
            again = true;
        else
            reader.running = true;
    }

    Component.onCompleted: read()

    // Who holds each camera, kept as it was when nobody came or went, so
    // nothing drawn from it is made again.
    function heard(text) {
        const next = {};
        for (const part of text.split("\x1e")) {
            const end = part.indexOf("\n");
            if (end < 0)
                continue;
            const name = part.slice(0, end);
            if (/\.\d+$/.test(name))
                continue;
            next[name] = part.slice(end + 1).split("\n").map(line => line.trim())
                .filter(line => line !== "");
        }
        if (JSON.stringify(next) !== JSON.stringify(held))
            held = next;
    }
}

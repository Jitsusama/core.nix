// A program with a camera open: true while any of the machine's cameras is
// read, and which programs, from the file kept for each camera at
// /run/camera-holders/<name>, one program to a line and empty when nobody
// reads or the camera is down. v4l2loopback keeps one for each loopback
// camera the machine names (`jitsusama.v4l2loopback.holders`), and
// usb-camera-holders one for each USB camera while it's plugged in, named
// for the camera and listing programs only while they stream. So the
// cameras are the ones the machine names and whatever else the folder
// holds, and a machine with neither has no piece. The machine daemon
// reads the folder as each file is renamed into place and tells each
// camera's holders as a record (`camera:<name>`, modules/machine), over
// the socket the shell holds, so nothing here is run or read to know
// who's on camera, and nothing at all while nobody comes or goes.
//
// A holder is named by the program itself, so this says who has a camera,
// not who may: it's for seeing at a glance, not for trusting.
pragma ComponentBehavior: Bound
import qs
import Quickshell
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
    // The cameras with a file in the folder, as the daemon says them, kept
    // when the names are the same so nothing drawn from them is made again.
    property var listed: []
    readonly property var names: [...new Set(configured.concat(listed))]
    // Who holds each camera, by its name, kept as it was when nobody came
    // or went, for the same reason.
    property var held: ({})
    readonly property var reading: names.map(name => ({
        name: called(name),
        programs: held[name] ?? []
    }))
    // Quickshell's JavaScript has no flatMap.
    readonly property var programs: [...new Set(reading.reduce((all, camera) => all.concat(camera.programs), []))]

    // The camera in use's drawer, or the first camera's while none is, as
    // Cameras draws it (launcher/Now.qml's borrowed).
    drawnBy: names.length === 0 ? null : {
        place: "cameras",
        key: `camera:${names.find(name => (held[name] ?? []).length > 0) ?? names[0]}`
    }

    present: names.length > 0
    active: programs.length > 0
    kind: "onAir"
    value: programs.length === 0 ? "off" : programs.join(", ")
    title: "Camera"
    // A program opens a camera by itself, so it's said when it does, and
    // taken back once it's let go: "chrome has MX Brio 705 for Business
    // open" stayed on Joel's desktop with no camera held (2026-11-11).
    arrives: true
    withdraws: true
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

    // Watched always, as the piece is: the daemon reads the folder only as
    // a file is renamed into it, so being watched costs nothing while
    // nobody comes or goes. A daemon not answering says nobody, rather
    // than someone who may have gone.
    Daemon {
        id: holders

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "camera"
        keptThroughGap: false
        answersWhole: true
        onHeard: piece.heard()
    }

    function heard() {
        const next = {};
        for (const key in holders.held) {
            const camera = holders.held[key];
            next[camera.name] = camera.programs ?? [];
        }
        const names = Object.keys(next);
        if (names.join("\n") !== listed.join("\n"))
            listed = names;
        if (JSON.stringify(next) !== JSON.stringify(held))
            held = next;
    }
}

// The machine's cameras, each a thing found by its name: the built-in
// camera, the IR camera, and each USB camera while it's plugged in. Return
// opens the viewfinder (modules/camera) on the camera it takes pictures
// with, and steps in on the others; the right arrow lists its parts, most
// asked first: who has it In use, each program a step in; its Privacy
// light and IR light; what it's Streaming; its Sensor and Where it sits;
// whether it's Powered; the Modes it offers, each format a step in; then
// its Connection, Firmware and whatever else desk-facts says of it.
//
// The cameras are the ones the margin's camera piece reads holders for
// (pieces/Camera.qml), so who has one has one home. A built-in camera is a
// loopback the machine makes (modules/v4l2loopback), whose format file in
// sysfs says what it gives, found by the card name the machine gives it
// (`cards`); its sensor and lights are the ones the vocabulary's table of
// sensors names for it (cameras:sensor in words.json), found on the bus,
// and its power and lights are read from sysfs. A USB camera's facts come
// from desk-facts (modules/desk-facts), as the built-in ones' will once it
// says them. A camera's node is never opened, not even to ask what it
// streams: whatever opens one is a holder, and would light the light.
//
// "camera", "webcam" or "who has the camera" answers whether one is in
// use, by which programs, and lists the cameras themselves as its rows, so
// the answer leads to each camera rather than saying its parts again.
// "privacy light" or "ir light" answers that part alone. Everything is
// read when it's asked or stepped into, and once a second while a camera's
// parts are shown, never for a key.
import qs
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import QtQuick
import "Cameras.js" as Cameras

Provider {
    id: provider

    what: "camera"

    // The margin's camera piece, which hears every camera's holders.
    readonly property var piece: {
        const fact = Facts.all.find(each => each.entry.name === "camera");
        return fact && fact.piece && fact.piece.present ? fact.piece : null;
    }
    readonly property var cameraNames: piece ? piece.names : []
    readonly property var heldBy: piece ? piece.held : ({})

    // The camera pictures are taken with is the one the machine calls
    // "camera" (hardware's loopback, or a machine's own), where it has one.
    readonly property string viewfound: (settings.viewfinder ?? null) === null ? "" : "camera"

    // A camera's name, as the vocabulary has the machine's own, or a USB
    // camera's own.
    function called(name) {
        const entry = Words.of(`cameras:${name}`);
        return entry.name !== `cameras:${name}` ? entry.name : name;
    }

    function cameraRow(name) {
        const programs = heldBy[name] ?? [];
        const known = Words.of(`cameras:${name}`).name !== `cameras:${name}`;
        const fields = {
            key: `camera:${name}`,
            name: called(name),
            keywords: Words.keywords(known ? `cameras:${name}` : "cameras:plugged"),
            glyph: Theme.glyph.camera,
            hue: "machine",
            plain: false,
            side: programs.length > 0 ? `in use by ${programs.join(", ")}` : ""
        };
        if (name !== viewfound)
            return holderRow(fields, () => provider.partRows(name));
        return thing(Object.assign(fields, {
            verb: Words.verb(`cameras:${name}`),
            later: true,
            run: () => Niri.spawn(["camera", "picture"]),
            verbs: () => provider.partRows(name)
        }));
    }

    things: cameraNames.map(name => cameraRow(name))
    onThingsChanged: changed()

    // How rows are drawn: a figure says something and Return closes; a
    // leaf worth having copies what it says.
    function shownAs(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    function leafOf(fields) {
        return copyRow(Object.assign({ copy: fields.side, hue: "machine", glyph: "", note: "" },
            fields));
    }

    function partName(id) {
        return partNamed("cameras", id);
    }

    // A camera's parts, most asked first.
    function partRows(name) {
        const sensed = sensorOf(name);
        const split = Cameras.formats(factsOf(name));
        let rows = [usedBy(name)].concat(lightRows(name, sensed));
        const streaming = streamingOf(name, split.rest);
        if (streaming !== null)
            rows.push(streaming);
        if (sensed !== null)
            rows = rows.concat(sensorRows(name, sensed));
        if (split.modes.length > 0)
            rows.push(holderRow({
                key: `camera:${name}:modes`,
                name: partName("modes"),
                side: split.modes.map(fact => fact.name).join(", "),
                glyph: Theme.glyph.camera,
                note: "each format's largest mode, and its fastest"
            }, () => split.modes.map(fact => leafOf({
                key: `camera:${name}:mode:${fact.name}`,
                name: fact.name,
                side: fact.value
            }))));
        // What the parts above say already isn't said again: desk-facts'
        // facing and power are the Where and Powered read from the bus.
        const shown = ["streaming", "sensor", "where", "powered", "facing", "power", "privacy",
            "flood"];
        const rest = split.rest.filter(fact => !shown.includes(fact.name.toLowerCase()));
        return rows.concat(factParts("cameras", name, rest, ["usb", "firmware"]));
    }

    // Who has it open, each program a step in.
    function usedBy(name) {
        const programs = heldBy[name] ?? [];
        const fields = { key: `camera:${name}:holders`, name: partName("holders") };
        if (programs.length === 0)
            return shownAs(Object.assign(fields, { side: "no" }));
        return holderRow(Object.assign(fields, {
            side: programs.join(", "),
            glyph: Theme.glyph.camera,
            note: "as each program names itself"
        }), () => programs.map(program => shownAs({
            key: `camera:${name}:holder:${program}`,
            name: program
        })));
    }

    // The lights its sensor's power controller names, as each is lit now.
    readonly property var lightNotes: ({
        privacy: "as the kernel sets it; the camera's own bridge may light it too",
        flood: "lit only while the camera streams, for its pictures"
    })

    function lightRows(name, sensed) {
        if (sensed === null)
            return [];
        const rows = [];
        for (let i = 0; i < leds.count; i++) {
            const led = leds.get(i, "fileName");
            const id = Cameras.light(led, sensed.lights);
            if (id === "")
                continue;
            const lit = Number(said(`/sys/class/leds/${led}/brightness`)) > 0;
            rows.push(shownAs({
                key: `camera:${name}:${id}`,
                name: partName(id),
                side: lit ? "on" : "off",
                note: lightNotes[id]
            }));
        }
        return rows;
    }

    // What it streams: as desk-facts says while it streams, or a
    // loopback's format, which is what it gives whoever reads it.
    function streamingOf(name, facts) {
        const fact = facts.find(each => each.name.toLowerCase() === "streaming");
        const loop = fact ? "" : Cameras.loopback(formatOf(name));
        const side = fact ? fact.value : loop;
        if (side === "")
            return null;
        const held = (heldBy[name] ?? []).length > 0;
        return leafOf({
            key: `camera:${name}:streaming`,
            name: partName("streaming"),
            side: side,
            note: held ? "now" : "what it gives a program that opens it"
        });
    }

    // Its sensor, where it sits and whether it's powered, read from the bus.
    function sensorRows(name, sensed) {
        const folder = `/sys/bus/i2c/devices/${sensed.folder}`;
        const rows = [leafOf({
            key: `camera:${name}:sensor`,
            name: partName("sensor"),
            side: sensed.name,
            note: `the firmware's ${sensed.id}`
        })];
        const where = Cameras.place({
            panel: said(`${folder}/physical_location/panel`),
            vertical: said(`${folder}/physical_location/vertical_position`),
            horizontal: said(`${folder}/physical_location/horizontal_position`),
            lid: said(`${folder}/physical_location/lid`)
        });
        if (where !== "")
            rows.push(shownAs({
                key: `camera:${name}:where`,
                name: partName("where"),
                side: where
            }));
        const status = said(`${folder}/power/runtime_status`);
        if (status !== "") {
            const active = Number(said(`${folder}/power/runtime_active_time`));
            rows.push(shownAs({
                key: `camera:${name}:powered`,
                name: partName("powered"),
                side: status === "active" ? "on" : status === "suspended" ? "off" : status,
                note: `on for ${Cameras.lasted(active)} since the machine started`
            }));
        }
        return rows;
    }

    // The sensor the vocabulary's table names for a camera, found on the
    // bus: { folder, id, lights, name }, or null.
    function sensorOf(name) {
        for (let i = 0; i < sensors.count; i++) {
            const folder = sensors.get(i, "fileName");
            const sensed = Cameras.sensor(folder);
            if (sensed === null)
                continue;
            const entry = Words.of(`cameras:sensor:${sensed.id.toLowerCase()}`);
            if (entry.camera === name)
                return Object.assign(sensed, { folder: folder, name: entry.name });
        }
        return null;
    }

    // A loopback's sysfs format file, found by the card name the machine
    // gives it, or for the viewfinder's camera by its node's own name; ""
    // for one with neither. Only sysfs is read, never the node.
    function formatOf(name) {
        const card = (settings.cards ?? {})[name] ?? "";
        if (card !== "")
            for (let i = 0; i < nodes.count; i++) {
                const node = nodes.get(i, "fileName");
                if (said(`/sys/class/video4linux/${node}/name`) === card)
                    return said(`/sys/class/video4linux/${node}/format`);
            }
        const device = /^\/dev\/(video\d+)$/.exec(name === viewfound ? settings.viewfinder : "");
        return device ? said(`/sys/class/video4linux/${device[1]}/format`) : "";
    }

    // What desk-facts knows of each camera, by the name its holders file
    // has, or by its card's; a camera it says nothing of, or no file at
    // all, has no facts.
    property var facts: ({})

    function factsOf(name) {
        return facts[name] ?? facts[(settings.cards ?? {})[name] ?? ""] ?? [];
    }

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/desk/cameras.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.facts = provider.parsed(text())
        onLoadFailed: provider.facts = ({})
    }

    // The parts asked for alone by their own words, as the part's id.
    readonly property var partsAsked: {
        const out = {};
        for (const id of ["privacy", "flood"])
            for (const question of Words.answers(`cameras:${id}`))
                out[question] = id;
        return out;
    }
    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.started("cameras").concat(Object.keys(partsAsked))

    function answer(typed) {
        const question = asks(typed);
        if (question === "" || piece === null)
            return null;
        const id = partsAsked[question];
        if (id !== undefined)
            return lightAsked(id);
        return {
            headline: piece.active ? `in use by ${piece.programs.join(", ")}` : "not in use",
            glyph: Theme.glyph.camera,
            rows: things,
            note: "from the cameras' own records"
        };
    }

    // A light asked for alone: how the one camera that has it has it lit,
    // or each camera's beside its name.
    function lightAsked(id) {
        const found = [];
        for (const name of cameraNames) {
            const part = lightRows(name, sensorOf(name)).find(row => row.name === partName(id));
            if (part)
                found.push({ name: name, part: part });
        }
        if (found.length === 0)
            return null;
        return {
            headline: found.length === 1 ? found[0].part.side
                : found.map(each => each.part.side).join(", "),
            copy: "",
            glyph: Theme.glyph.camera,
            note: found.length === 1 ? called(found[0].name) : found[0].part.note,
            rows: found.length === 1 ? [] : found.map(each => shownAs({
                key: `camera:${each.name}:${id}:said`,
                name: called(each.name),
                side: each.part.side
            }))
        };
    }

    // A camera's parts are read again each second while they're shown, and
    // the launcher told when what they say has changed, so a light coming
    // on or the sensor waking shows without a key.
    readonly property string heldCamera: {
        const steps = launcher && launcher.open ? launcher.held ?? [] : [];
        const key = steps.length > 0 ? steps[0].row.key ?? "" : "";
        return key.startsWith("camera:") ? key.slice("camera:".length) : "";
    }
    property string lastRead: ""

    function readAgain() {
        if (heldCamera === "")
            return;
        const now = partRows(heldCamera).map(row => `${row.name}\u241f${row.side}`).join("\u241e");
        if (now !== lastRead) {
            lastRead = now;
            changed();
        }
    }

    Timer {
        interval: 1000
        repeat: true
        running: provider.heldCamera !== ""
        onRunningChanged: provider.lastRead = ""
        onTriggered: provider.readAgain()
    }

    // What a file in /sys says, or "" when it says nothing.
    function said(file) {
        probe.path = file;
        return read(probe).trim();
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }

    // The bus the sensors are on, the lights and the video nodes, each
    // listed by Qt as it changes, so nothing is listed for a key.
    FolderListModel {
        id: sensors
        folder: "file:///sys/bus/i2c/devices"
        showFiles: false
        showDotAndDotDot: false
    }

    FolderListModel {
        id: leds
        folder: "file:///sys/class/leds"
        showFiles: false
        showDotAndDotDot: false
    }

    FolderListModel {
        id: nodes
        folder: "file:///sys/class/video4linux"
        showFiles: false
        showDotAndDotDot: false
    }
}

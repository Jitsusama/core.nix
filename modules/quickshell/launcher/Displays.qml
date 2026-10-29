// Displays, a collection: the displays niri drives, the built-in one first,
// each with its size, refresh rate and scale in the footer. Return moves
// focus to one, or turns on one that's off; the right arrow lists its
// Scale, Refresh, Resolution and Rotation as parts saying how each is set,
// each stepping into its choices (which what's typed there finds too, so
// "scale 150" needs no step in), and turning it off while another is on.
// "colour depth", "refresh rate" and the like ask the focused display's
// part, or each display's while several are on. niri forgets these choices when its configuration is next read,
// so they last the session, and what lasts belongs in the configuration.
// Everything goes over niri's socket, asked again whenever the launcher
// opens or the workspaces change, as they do when a display comes or goes,
// since niri sends no event for a display itself. What a display's EDID
// says of itself, who made it and when, its size, native mode, refresh
// range, colour depth, colours and HDR, desk-facts writes to a file
// (modules/desk-facts), and the right arrow lists after the settings,
// named first as parts are (Colour depth "10 bit"), Return copying one.
//
// The panel's backlight is the one brightness, lid shut or not: the
// backlight daemon (modules/backlight) sets every monitor it reaches to
// the same share, so one Brightness says it, in one screen's place: the
// built-in one while it's on, else the first lit monitor that follows it,
// as desk-facts says by a brightness among its facts (lid shut and
// docked, the monitor). It leads that screen's settings: its share as a
// bar, its levels a step in, which set it as the brightness keys do, so
// the daemon, which follows the room while the lid is open, learns it as
// Joel's pick; then, on the built-in screen, the Room light its sensor
// hears. "brightness" asks it, and "brightness 40" sets it on Return. The
// backlight is read from the kernel's files when asked, its folder listed
// once by Qt, so nothing is spawned or watched; the room's light, which
// the sensor takes a tenth of a second to say, is read in the background
// and shown once it's come. While its answer shows, it's asked again once
// a second, never more, so the screen following the room moves as it does.
//
// Where the backlight daemon warms the built-in screen toward the room's
// light, it says how that stands in a file (modules/backlight), and the
// Warmth follows the Brightness there: on or off, its note the warmth and
// whether it follows the room, and Return switching it, which the daemon
// is told over the session's bus and keeps. "screen warmth" asks it.
import qs
import Quickshell
import Quickshell.Io
import Qt.labs.folderlistmodel
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Numbers.js" as Numbers
import "Power.js" as Power

Provider {
    id: provider

    what: "display"

    collection: Words.collection("displays")

    // What niri says of each display, by its connector's name.
    property var outputs: ({})

    function refresh() {
        Niri.request("Outputs", answer => provider.outputs = answer.Outputs);
    }

    Component.onCompleted: refresh()

    Connections {
        target: Niri

        function onWorkspacesChanged() {
            provider.refresh();
        }
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.refresh();
        }
    }

    // Tells niri what to change, then asks again how things stand.
    function change(output, action) {
        Niri.request({ Output: { output: output.name, action: action } }, () => provider.refresh());
    }

    function builtIn(output) {
        return output.name.startsWith("eDP");
    }

    // Whether a monitor's brightness follows the panel's, as desk-facts
    // says by the brightness the daemon read back from it.
    function follows(output) {
        return (facts[output.name] ?? []).some(fact => fact.name.toLowerCase() === "brightness");
    }

    // Whether the room's light is heard here: the built-in screen's lid
    // holds the sensor, and so does the only screen there is, as in a
    // virtual machine's.
    function inTheRoom(output) {
        return builtIn(output) || shown.length === 1;
    }

    // Whether this screen's place holds the Brightness: the built-in one
    // while it's on, the only screen there is, or else the first lit
    // monitor that follows the panel. Only one holds it, since every
    // screen shows the same level.
    function backlit(output) {
        const home = lit.find(each => inTheRoom(each))
            ?? lit.find(each => follows(each)) ?? null;
        return home !== null && home.name === output.name;
    }

    function readOut(file) {
        probe.path = file;
        return read(probe).trim();
    }

    // The first backlight the kernel lists, as its steps and its share, or
    // null with none.
    function backlight() {
        for (let i = 0; i < backlights.count; i++) {
            const folder = `/sys/class/backlight/${backlights.get(i, "fileName")}`;
            const now = Number(readOut(`${folder}/brightness`));
            const most = Number(readOut(`${folder}/max_brightness`));
            if (most > 0)
                return { now: now, most: most, share: Math.round(now / most * 100) };
        }
        return null;
    }

    // The levels a step into Brightness offers, brightest first.
    readonly property var levels: [100, 90, 80, 70, 60, 50, 40, 30, 20, 10]

    // Sets the screen to a share, as the brightness keys do; the first step
    // stays lit, since none at all leaves the screen black.
    function brighten(share) {
        Niri.spawn(["brightnessctl", "--class=backlight", "set", `${Math.max(1, share)}%`]);
    }

    // A share as a bar. However bright, it wants nothing, so it never warns.
    function level(now, most) {
        return { shape: "bar", values: [most > 0 ? Math.max(0, Math.min(1, now / most)) : 0],
            warn: false };
    }

    function lux(value) {
        return `${Numbers.format(value, value >= 10 ? 0 : 1, settings.group, settings.point)} lx`;
    }

    // The Brightness and, where the room is heard, the Room light, or
    // nothing for a screen whose place doesn't hold them.
    function lightRows(output) {
        const light = backlit(output) ? backlight() : null;
        if (light === null)
            return [];
        const rows = [Object.assign(setting(output, "brightness", `${light.share}%`, "brightness",
            levels.map(each => choice(`brightness-${each}`,
                Words.name("displays:brightness:level", "", { n: each }), "brightness",
                light.share === each, () => provider.brighten(each)))), {
            graph: level(light.now, light.most),
            // With the lid shut nothing hears the room, and a monitor has
            // no steps of the panel's, so it only says what sets it.
            note: inTheRoom(output)
                ? `${light.now} of ${light.most} steps; it follows the room, and the keys set it`
                : "every screen follows it, and the keys set it"
        })];
        if (inTheRoom(output) && askRoom() && room !== null)
            rows.push(copyRow({
                key: `displays:${output.name}:room`,
                name: partName("room"),
                side: lux(room),
                copy: lux(room),
                glyph: "",
                hue: "machine",
                graph: level(Power.roomLevel(room), 1),
                note: "as the screen's sensor hears it, from dark to daylight"
            }));
        if (inTheRoom(output) && warmth !== null)
            rows.splice(1, 0, warmthRow(output));
        return rows;
    }

    // How the screen's warmth stands, as the backlight daemon says it, or
    // null where nothing warms the screen: no file, or `{}`, which the
    // daemon leaves when it stops, since a file watched here is never
    // seen again once it's removed.
    property var warmth: null
    onWarmthChanged: changed()

    function warmed(text) {
        const said = parsed(text);
        return typeof said.kelvin === "number" && typeof said.followsTheRoom === "boolean"
            ? said : null;
    }

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/backlight-warmth`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.warmth = provider.warmed(text())
        onLoadFailed: provider.warmth = null
    }

    // The Warmth, a switch: Return turns its following the room off or
    // back on, the launcher staying to show it change.
    function warmthRow(output) {
        const follows = warmth.followsTheRoom;
        const stands = follows ? `${warmth.kelvin} K, following the room`
            : `${warmth.kelvin} K, the screen's own white`;
        return thing({
            key: `displays:${output.name}:warmth`,
            name: partName("warmth"),
            verb: Words.verb("displays:warmth", follows ? "" : "off"),
            glyph: Theme.glyph.warmth,
            hue: "machine",
            side: follows ? "on" : "off",
            said: stands,
            note: stands,
            stays: true,
            run: () => provider.followTheRoom(!follows)
        });
    }

    // Tells the backlight daemon whether the screen follows the room.
    function followTheRoom(follows) {
        Niri.spawn(["busctl", "--user", "set-property", "io.jitsusama.Backlight",
            "/io/jitsusama/Backlight", "io.jitsusama.Backlight", "FollowsTheRoom", "b",
            follows ? "true" : "false"]);
    }

    // The room's light, as the first ambient light sensor the kernel lists
    // hears it, in lux, or null until it's been read. The sensor's raw
    // reading is slow to come, so it's read in the background at most once
    // a second, and its scale and offset, which are quick, as it's asked.
    property var room: null
    property real roomAt: 0
    property string roomSensor: ""
    property bool roomPending: false
    // Whether it's been read at all, or failed to be, since until then
    // the answer waits for it.
    property bool roomTried: false

    function sensor() {
        for (let i = 0; i < sensors.count; i++) {
            const folder = `/sys/bus/iio/devices/${sensors.get(i, "fileName")}`;
            if (readOut(`${folder}/name`) === "als"
                && readOut(`${folder}/in_illuminance_scale`) !== "")
                return folder;
        }
        return "";
    }

    function askRoom() {
        const folder = sensor();
        if (folder === "")
            return false;
        const now = Date.now();
        if (now - roomAt >= 1000 && !roomPending) {
            roomAt = now;
            roomSensor = folder;
            roomPending = true;
            if (roomRead.path === `${folder}/in_illuminance_raw`)
                roomRead.reload();
            else
                roomRead.path = `${folder}/in_illuminance_raw`;
        }
        return true;
    }

    function heardRoom(raw) {
        roomPending = false;
        roomTried = true;
        room = Power.lux(raw, readOut(`${roomSensor}/in_illuminance_scale`),
            readOut(`${roomSensor}/in_illuminance_offset`));
        // The launcher asks again only while the answer is out at the top.
        answered();
    }

    // Setting the screen to a share, Return doing it.
    function setTo(share) {
        const light = backlight();
        if (light === null)
            return null;
        const kept = Math.max(1, share);
        return {
            headline: `set the screen to ${kept}%`,
            verb: Words.command("command:brightness").verb,
            stays: true,
            run: () => provider.brighten(kept),
            rows: [],
            note: `now ${light.share}%`
        };
    }

    // While a light's answer shows, it's asked again each second, so what
    // follows the room moves as it does.
    readonly property bool lightShown: launcher !== null && launcher.open
        && (launcher.held ?? []).length === 0 && launcher.answer?.live === provider

    Timer {
        interval: 1000
        repeat: true
        running: provider.lightShown
        onTriggered: provider.answered()
    }

    FolderListModel {
        id: backlights
        folder: "file:///sys/class/backlight"
        showFiles: false
        showDotAndDotDot: false
    }

    FolderListModel {
        id: sensors
        folder: "file:///sys/bus/iio/devices"
        showFiles: false
        showDotAndDotDot: false
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }

    FileView {
        id: roomRead
        printErrors: false
        onLoaded: provider.heardRoom(text())
        onLoadFailed: provider.heardRoom("")
    }

    // Its maker and model, leaving out a model that's only a number.
    function named(output) {
        if (builtIn(output))
            return "Built-in screen";
        const model = /^0x[0-9a-f]+$/i.test(output.model) ? "" : output.model;
        return `${output.make} ${model}`.trim() || output.name;
    }

    function hertz(mode) {
        return Math.round(mode.refresh_rate / 1000);
    }

    function mode(output) {
        return output.current_mode === null ? null : output.modes[output.current_mode];
    }

    function described(output) {
        const now = mode(output);
        if (now === null || output.logical === null)
            return "off";
        // The scale as its part says it, "175%".
        return `${now.width} \u00d7 ${now.height} \u00b7 ${hertz(now)} Hz `
            + `\u00b7 scale ${percent(output.logical.scale)}`;
    }

    readonly property var shown: Object.values(outputs).sort((a, b) =>
        Number(builtIn(b)) - Number(builtIn(a)) || a.name.localeCompare(b.name))
    readonly property var lit: shown.filter(output => output.logical !== null)

    readonly property string focusedOutput: {
        Niri.revision;
        const workspace = Niri.workspace();
        return workspace ? workspace.output : "";
    }

    function made(output) {
        const on = output.logical !== null;
        const focused = lit.length > 1 && output.name === focusedOutput;
        return member({
            key: `display:${output.name}`,
            verb: on ? "focus" : "turn on",
            name: named(output),
            keywords: output.name,
            glyph: builtIn(output) ? Theme.glyph.computer : Theme.glyph.display,
            hue: "machine",
            side: !on ? "off" : focused ? "focused" : "",
            note: described(output),
            run: on ? () => Niri.act({ FocusMonitor: { output: output.name } })
                : () => provider.change(output, "On"),
            verbs: () => provider.verbs(output)
        });
    }

    readonly property var all: shown.map(output => made(output))
    onAllChanged: changed()

    // A choice wears the theme's mark of the setting it's for, by name.
    // Setting it keeps the launcher, the setting saying what it's at now.
    function choice(key, name, mark, chosen, run) {
        return thing({
            key: `verb:display-${key}`,
            name: name,
            glyph: Theme.glyph[mark],
            hue: "action",
            side: chosen ? "chosen" : "",
            stays: true,
            run: run
        });
    }

    // What desk-facts knows of each display, by its connector's name; one
    // it says nothing of, or no file at all, has no facts.
    property var facts: ({})

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/desk/displays.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.facts = provider.parsed(text())
        onLoadFailed: provider.facts = ({})
    }

    // What desk-facts says of a display, each named first as a part is:
    // Colour depth "10 bit", Colours, then who made it and when, the depth
    // first since it's what's asked.
    function factRows(output) {
        // A monitor's brightness is the panel's, which the Brightness
        // setting says, so it isn't said again as a fact.
        const said = (facts[output.name] ?? [])
            .filter(fact => fact.name.toLowerCase() !== "brightness");
        return factParts("displays", output.name, said, askedFirst);
    }

    // The facts asked of a display most, which lead the rest.
    readonly property var askedFirst: ["depth", "colours", "hdr"]

    // A part's name, as the vocabulary says it (Provider's partNamed).
    function partName(id) {
        return partNamed("displays", id);
    }

    // A display's right arrow: turning it on while it's off; while it's
    // on, each of its settings as a part saying how it's set, whose step
    // in offers the others (and whose choices what's typed finds from
    // here, so "scale 150" is Scale to 150%), then turning it off while
    // another is on, then what its EDID says.
    function verbs(output) {
        if (output.logical === null)
            return [choice("on", "Turn It On", "display", false, () => change(output, "On"))]
                .concat(factRows(output));
        const rows = setUp(output);
        // The last display on stays on, or nothing would show the launcher.
        if (lit.length > 1)
            rows.push(choice("off", "Turn It Off", "displayOff", false,
                () => change(output, "Off")));
        return rows.concat(factRows(output));
    }

    // A setting as a part: its name, how it's set, and the choices a step
    // in, or only how it's set when there's nothing else to choose.
    function setting(output, id, side, mark, choices) {
        const fields = {
            key: `displays:${output.name}:${id}`,
            name: partName(id),
            side: side
        };
        if (choices.length === 0)
            return copyRow(Object.assign(fields, { copy: "", hue: "machine", glyph: "" }));
        return holderRow(Object.assign(fields, { glyph: Theme.glyph[mark], offers: true }),
            () => choices);
    }

    // How niri turns a display, as a part says it and as its choice does,
    // in the vocabulary's words; niri turns "90" counter-clockwise.
    readonly property var turns: ["Normal", "90", "180", "270"].map(transform => {
        const side = (Words.of("displays:rotation").said ?? {})[transform] ?? transform;
        return {
            transform: transform,
            side: side,
            name: Words.name("displays:rotate", "", { way: side })
        };
    })

    readonly property var scales: [1, 1.25, 1.5, 1.75, 2, 2.5, 3]

    function percent(scale) {
        return `${Math.round(scale * 100)}%`;
    }

    // How it's set: Brightness and the Room light where the backlight
    // lights it, then Scale, Refresh, Resolution, Rotation.
    function setUp(output) {
        const now = mode(output);
        const scale = output.logical.scale;
        const rows = lightRows(output).concat([setting(output, "scale", percent(scale), "scale",
            scales.map(each => choice(`scale-${each}`, `Scale to ${percent(each)}`, "scale",
                Math.abs(each - scale) < 0.01,
                () => change(output, { Scale: { scale: { Specific: each } } }))))]);
        if (now !== null) {
            const rates = output.modes.filter(m => m.width === now.width && m.height === now.height);
            const refreshes = rates.length < 2 ? [] : rates.map(each =>
                choice(`rate-${hertz(each)}`, `Refresh at ${hertz(each)} Hz`, "refresh",
                    each === now, () => change(output, { Mode: { mode: { Specific: {
                        width: each.width,
                        height: each.height,
                        refresh: each.refresh_rate / 1000
                    } } } })));
            if (output.vrr_supported) {
                const vrr = { vrr: !output.vrr_enabled, on_demand: false };
                refreshes.push(choice("vrr", output.vrr_enabled ? "Stop variable refresh"
                    : "Use variable refresh", "refresh", false,
                    () => change(output, { Vrr: { vrr } })));
            }
            const variable = output.vrr_enabled ? ", variable" : "";
            rows.push(setting(output, "refresh", `${hertz(now)} Hz${variable}`, "refresh",
                refreshes));
            const sizes = [];
            for (const each of output.modes)
                if (!sizes.some(size => size.width === each.width && size.height === each.height))
                    sizes.push(each);
            rows.push(setting(output, "resolution", `${now.width} \u00d7 ${now.height}`,
                "resolution",
                sizes.length < 2 ? [] : sizes.map(each =>
                    choice(`size-${each.width}x${each.height}`,
                        `Use ${each.width} \u00d7 ${each.height}`, "resolution",
                        each.width === now.width && each.height === now.height,
                        () => change(output, { Mode: { mode: { Specific: {
                            width: each.width,
                            height: each.height,
                            refresh: null
                        } } } })))));
        }
        const turned = turns.find(each => each.transform === output.logical.transform);
        rows.push(setting(output, "rotation", turned ? turned.side : "flipped", "rotation",
            turns.map(each => choice(`turn-${each.transform}`, each.name, "rotation",
                output.logical.transform === each.transform,
                () => change(output, { Transform: { transform: each.transform } })))));
        return rows;
    }

    // Asked of a display by a part's words, "colour depth" or "refresh
    // rate": the focused one's, or each one's when several are on.
    readonly property var partsAsked: {
        const out = {};
        for (const id of ["brightness", "warmth", "room", "scale", "refresh", "resolution",
            "rotation", "depth", "colours", "hdr"])
            for (const question of Words.answers(`displays:${id}`))
                out[question] = id;
        return out;
    }
    questions: Object.keys(partsAsked)

    // What sets the brightness: "brightness 40", or "brightness to 40%".
    readonly property var brightnessTyped: /^brightness\s+(?:to\s+)?(\d{1,3})\s*%?$/

    function answer(typed) {
        const set = brightnessTyped.exec(typed.trim().toLowerCase());
        if (set)
            return setTo(Math.min(100, Number(set[1])));
        const id = partsAsked[asks(typed)];
        if (id === undefined || lit.length === 0)
            return null;
        const partOf = output => verbs(output).find(row => row.name === partName(id)) ?? null;
        // The focused screen's, or the first that has it: only one has a
        // Brightness.
        const focused = lit.find(output => output.name === focusedOutput && partOf(output))
            ?? lit.find(output => partOf(output) !== null);
        if (focused === undefined)
            return null;
        const part = partOf(focused);
        const others = lit.filter(output => output !== focused && partOf(output) !== null);
        const light = id === "brightness" || id === "room";
        return {
            // Until the room's first reading comes, a light's answer is still
            // being asked, so Return waits for it; while it shows, it's
            // asked again each second.
            asking: light && !roomTried && sensor() !== "",
            live: light ? provider : null,
            // A switch says how it stands in full, as its note does.
            headline: part.said ?? part.side,
            glyph: Theme.glyph.display,
            note: named(focused),
            rows: others.length === 0 ? (part.verbs ? part.verbs() : [])
                : others.map(output => copyRow({
                    key: `displays:${output.name}:${id}:said`,
                    name: named(output),
                    side: partOf(output).side,
                    copy: "",
                    glyph: ""
                }))
        };
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    function everything() {
        return all.slice();
    }

    function counted() {
        const each = shown.length === 1 ? "screen" : "screens";
        return lit.length === shown.length ? `${shown.length} ${each}`
            : `${lit.length} of ${shown.length} on`;
    }
}

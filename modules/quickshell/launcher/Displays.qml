// Displays, a collection: the displays niri drives, the built-in one first,
// each with its size, refresh rate and scale in the footer. Return moves
// focus to one, or turns on one that's off; the right arrow lists its
// Scale, Refresh, Resolution and Rotation as parts saying how each is set,
// each stepping into its choices (which what's typed there finds too, so
// "scale 1.5" needs no step in), and turning it off while another is on.
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
import "Backlight.js" as Backlight
import "Numbers.js" as Numbers
import "Order.js" as Order
import "Plate.js" as Plate
import "Power.js" as Power
import "Screens.js" as Screens

Provider {
    id: provider

    what: "display"

    collection: Words.collection("displays")

    // What niri says of each display, by its connector's name.
    property var outputs: ({})

    // :set brightness=60 sets the screen as its Brightness row does, on a
    // machine with a backlight, and :set nowarmth gives it its own white as
    // its Warmth row does, where the backlight daemon warms it. :set
    // noscreens turns every screen off, as Turn Every Screen Off does; they
    // are on whenever the launcher can be seen, so screens alone does
    // nothing.
    setOptions: Object.assign({
        screens: {
            kind: "switch",
            now: () => true,
            set: to => {
                if (!to)
                    provider.sleepAll();
                return "";
            }
        }
    }, backlights.count === 0 ? {} : {
        brightness: {
            kind: "number",
            least: 0,
            most: 100,
            now: () => provider.backlight()?.share ?? 0,
            set: to => {
                provider.brighten(to);
                return "";
            }
        }
    }, warmth === null ? {} : {
        warmth: {
            kind: "switch",
            now: () => provider.warmth?.followsTheRoom ?? false,
            set: to => {
                provider.followTheRoom(to);
                return "";
            }
        }
    })

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

    // The room's light as it's measured, Room Light's row and fact alike.
    function lux(value) {
        return Backlight.reading(value, settings.group, settings.point);
    }

    // The Brightness and, where the room is heard, the Room light, or
    // nothing for a screen whose place doesn't hold them.
    function lightRows(output) {
        const light = backlit(output) ? provider.light : null;
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
        if (inTheRoom(output) && room !== null)
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
        // Warmth sits right after Brightness, found by its key, so a row
        // put above Brightness later can't move it.
        if (inTheRoom(output) && warmth !== null) {
            const brightness = `displays:${output.name}:brightness`;
            rows.splice(rows.findIndex(row => row.key === brightness) + 1, 0, warmthRow(output));
        }
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

    // The level the backlight daemon last settled, what its curve asks for
    // the room it heard and whether it follows the room, as it writes them
    // each time a step ends or a pick is heard (Backlight.js), or null
    // where no daemon says: no file, or an empty one, as it leaves when it
    // stops. sysfs's brightness can't be watched, so this is what moves the
    // drawer with each step the daemon takes.
    property var settled: null
    onSettledChanged: {
        readLight();
        changed();
    }

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/backlight-now`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.settled = Backlight.now(text())
        onLoadFailed: provider.settled = null
    }

    // The daemon's curve, dimmest first, where it keeps it: its standard
    // points with each of Joel's picks learned into them.
    property var curve: []
    onCurveChanged: changed()

    FileView {
        path: `${Quickshell.env("XDG_STATE_HOME") || `${Quickshell.env("HOME")}/.local/state`}`
            + "/backlight/learned"
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.curve = Backlight.curve(text())
        onLoadFailed: provider.curve = []
    }

    // The Warmth, a switch: Return turns its following the room off or
    // back on, the launcher staying to show it change.
    function warmthRow(output) {
        const follows = warmth.followsTheRoom;
        const stands = follows ? `${warmth.kelvin} K, following the room`
            : `${warmth.kelvin} K, the screen's own white`;
        return thing({
            key: `displays:${output.name}:warmth`,
            // Its words, which its key never walks back to, in its state,
            // so the : line says :set nowarmth or warmth.
            wordsId: "displays:warmth",
            wordState: follows ? "" : "off",
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

    // The backlight as last read, for a screen's drawer and the rows a step
    // into it holds. The launcher builds both inside bindings, where reading
    // sysfs or asking the sensor writes state the binding reads, a loop; so
    // they read only this and `room`, which are asked for here, each second
    // while the launcher is open, from the moment it opens, since the
    // Brightness and Room Light answers are found through those rows too,
    // and as the daemon says it stepped.
    property var light: null
    onLightChanged: changed()

    function readLight() {
        const now = backlight();
        if (JSON.stringify(now) !== JSON.stringify(light))
            light = now;
    }

    Timer {
        interval: 1000
        repeat: true
        triggeredOnStart: true
        running: provider.launcher !== null && provider.launcher.open
        onTriggered: {
            provider.askRoom();
            provider.readLight();
        }
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
        // The scale as its part and its drawer say it, "1.75\u00d7".
        return `${now.width} \u00d7 ${now.height} \u00b7 ${hertz(now)} Hz `
            + `\u00b7 scale ${Screens.scale(output.logical.scale)}`;
    }

    // Screens are choices: the built-in first, as it's always there, then by
    // name, and the one in use never moves for being chosen.
    readonly property var shown: Order.sorted(Object.values(outputs), Order.choices({
        natural: (a, b) => Number(builtIn(b)) - Number(builtIn(a)), name: output => output.name }))
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
            thing: true,
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
            verbs: () => provider.verbs(output),
            drawer: () => provider.drawerOf(output)
        });
    }

    // Every screen off, as niri's power-off-monitors turns them off: each
    // stays connected and enabled, so the machine still counts as docked
    // and a monitor's hub and network card stay powered, which taking it
    // out of niri's outputs doesn't keep (Joel, 2026-10-24: "is there a way
    // to turn my external display off without shutting down its hub or
    // network card or anything? i want the laptop to be able to keep going
    // without blinding me in my sleep"). Any key or the pointer wakes them.
    readonly property var sleepRow: thing({
        key: "displays:sleep",
        verb: Words.verb("displays:sleep"),
        name: Words.name("displays:sleep"),
        glyph: Words.glyph("displays:sleep"),
        hue: "action",
        note: Words.of("displays:sleep").said?.note ?? "",
        run: () => provider.sleepAll()
    })

    // What the vocabulary says of a screen, by the part it's of and its
    // own word: displays:modes' "preferred".
    function partSaid(id, key) {
        return String((Words.of(id === "" ? "displays" : `displays:${id}`).said ?? {})[key]
            ?? key);
    }

    // How niri turns a screen, mirrored ones too, in the vocabulary's words.
    function turnedSaid(transform) {
        return partSaid("rotation", transform);
    }

    // A screen's drawer: its connector and what it is over its name, both
    // as written, since a connector is a name from the world (VKCF3Z,
    // 2026-11-07); then what moves most, the light where it lights this
    // screen (its Brightness with its steps, the Room light, the Warmth);
    // then what niri says it runs, where it sits and what it offers
    // (Screens.js); then what its EDID says, as its step lists it, its
    // size from niri where desk-facts says none. Its focus is its row's
    // to say, so it isn't said again.
    function drawerOf(output) {
        const facts = [];
        const light = output.logical !== null && backlit(output) ? provider.light : null;
        if (light !== null) {
            facts.push({
                label: partName("brightness"),
                // A monitor following the panel has none of its steps,
                // and a hundred steps are its share again.
                value: inTheRoom(output) && light.most !== 100 ? Plate.runs([`${light.share}%`,
                    partSaid("", "steps").replace("{now}", light.now)
                        .replace("{most}", light.most)])
                    : `${light.share}%`,
                figure: { shape: "stroke", values: [light.now / light.most], warn: false }
            });
            // The room as the sensor hears it now, which moves before the
            // level does, and is asked only for the screen it lights; or
            // that no light is heard, where the daemon follows nothing.
            const said = (key, values) => Backlight.fill(partSaid("", key), values);
            const heard = inTheRoom(output) ? Backlight.room(settled, room, said,
                each => lux(each)) : "";
            if (heard !== "")
                facts.push({ label: partName("room"), value: heard });
            // What sets the level, the Warmth, then the curve, which moves
            // only with a pick, so goes last of the light (VKCF3Z, 2026-11-08).
            const follows = inTheRoom(output) ? Backlight.follows(settled, said) : "";
            if (follows !== "")
                facts.push({ label: partName("follows"), value: follows });
            if (inTheRoom(output) && warmth !== null)
                facts.push({
                    label: partName("warmth"),
                    value: Plate.runs([`${Numbers.format(warmth.kelvin, 0, settings.group,
                        settings.point)} K`,
                        partSaid("", warmth.followsTheRoom ? "following" : "own-white")])
                });
            if (inTheRoom(output) && curve.length > 0)
                facts.push({ label: partName("curve"),
                    value: Backlight.curveSaid(curve, said,
                        each => Backlight.chosen(each, settings.group, settings.point)) });
        }
        const edid = Plate.facts(factRows(output));
        const physical = edid.some(fact => fact.label === partName("size")) ? ""
            : Screens.physical(output);
        return {
            eyebrow: Plate.runs([output.name, partSaid("", builtIn(output) ? "built-in"
                : "screen")]),
            eyebrowNames: true,
            title: named(output),
            // Its serial, from niri or the EDID, identifies it, so ends it.
            facts: Plate.identified(facts.concat(Screens.facts(output, {
                named: id => provider.partName(id),
                said: (id, key) => provider.partSaid(id, key),
                turned: transform => provider.turnedSaid(transform)
            }), edid, physical === "" ? [] : [{ label: partName("size"), value: physical }]),
                [partName("serial")])
        };
    }

    readonly property var all: shown.map(output => made(output))
        .concat(lit.length > 0 ? [sleepRow] : [])
    onAllChanged: changed()

    // niri wakes the screens at any key, so the one that asked, still held
    // or let go just after, would wake them at once: they're turned off
    // once it has come up and the launcher has closed.
    function sleepAll() {
        sleeping.restart();
    }

    Timer {
        id: sleeping
        interval: 750
        onTriggered: Niri.act({ PowerOffMonitors: {} })
    }

    // A choice wears the theme's mark of the setting it's for, by name.
    // Setting it keeps the launcher, the setting saying what it's at now.
    function choice(key, name, mark, chosen, run, keywords) {
        return thing({
            key: `verb:display-${key}`,
            name: name,
            keywords: keywords ?? "",
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
    // here, so "scale 1.5" is Scale to 1.5\u00d7), then turning it off while
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

    // How it's set: Brightness and the Room light where the backlight
    // lights it, then Scale, Refresh, Resolution, Rotation.
    function setUp(output) {
        const now = mode(output);
        const scale = output.logical.scale;
        // A scale is a multiple, as the theme says one, never a share, so
        // the row, the part and the drawer call it one way (VKCF3Z,
        // 2026-11-07). Its share is a hidden word, so "scale 150", as niri
        // and other systems say it, still finds 1.5\u00d7.
        const rows = lightRows(output).concat([setting(output, "scale", Screens.scale(scale),
            "scale", scales.map(each => choice(`scale-${each}`,
                `Scale to ${Screens.scale(each)}`, "scale",
                Math.abs(each - scale) < 0.01,
                () => change(output, { Scale: { scale: { Specific: each } } }),
                `${Math.round(each * 100)}%`)))]);
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
        rows.push(setting(output, "rotation", turnedSaid(output.logical.transform), "rotation",
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

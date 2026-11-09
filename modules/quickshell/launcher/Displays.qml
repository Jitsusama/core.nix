// Displays, a collection: the displays niri drives, the built-in one first,
// each with its size, refresh rate and scale in the footer. Return moves
// focus to one, or turns on one that's off; the right arrow lists its
// Scale, Refresh, Resolution and Rotation as parts saying how each is set,
// each stepping into its choices (which what's typed there finds too, so
// "scale 1.5" needs no step in), and turning it off while another is on.
// "colour depth", "refresh rate" and the like ask the focused display's
// part, or each display's while several are on. niri forgets these
// choices when its configuration is next read, so they last the session,
// and what lasts belongs in the configuration. Everything goes over
// niri's socket, asked again whenever the launcher opens or the workspaces
// change, as they do when a display comes or goes, since niri sends no
// event for a display itself. What a display's EDID says of itself, who
// made it and when, its size, native mode, refresh range, colour depth,
// colours, white point, primaries and HDR, desk-facts writes to a file
// (modules/desk-facts), and the right arrow lists after the settings,
// named first as parts are (Colour depth "10 bit"), Return copying one.
// The white point and primaries come as figures, said here by Joel's
// grouping and decimal point (Screens.js).
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
// The backlight daemon warms the built-in screen at night from sunset to
// sunrise, and toward the room's light where its sensor sees colour, and
// says how that stands in a file (modules/backlight), so the Warmth
// follows the Brightness on every machine with a panel to warm: its side
// the warmth now, its note what holds it, and Return stepping into the
// Night Light and Follow the Room switches, which the daemon is told over
// the session's bus and keeps, then the room's white, the night's sunset
// and sunrise and its white (Warmth.js; VKCF3Z, 2026-11-08). "screen
// warmth" asks it.
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
import "Warmth.js" as Warmth

Provider {
    id: provider

    what: "display"

    collection: Words.collection("displays")

    // What niri says of each display, by its connector's name.
    property var outputs: ({})

    // :set brightness=60 sets the screen as its Brightness row does, on a
    // machine with a backlight, and :set nowarmth gives it its own white by
    // day as Warmth's Follow the Room does, where a sensor sees colour. :set
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
        // :set nightlight and nonightlight turn the night light on and off,
        // as its switch in Warmth does.
        nightlight: {
            kind: "switch",
            now: () => provider.warmth?.nightLight ?? false,
            set: to => {
                provider.nightLight(to);
                return "";
            }
        }
    }, warmth === null || !warmth.seesColour ? {} : {
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
            levels.map(each => choice({
                key: `brightness-${each}`,
                name: Words.name("displays:brightness:level", "", { n: each }),
                mark: "brightness",
                chosen: light.share === each,
                run: () => provider.brighten(each)
            }))), {
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
        return Warmth.read(parsed(text));
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

    // How Warmth says a time and a kelvin: by the clock, and with its
    // thousands grouped (VKCF3Z, 2026-11-08: "4,000 K").
    readonly property var warmthTold: ({
        clock: at => Qt.formatDateTime(new Date(at), "HH:mm"),
        kelvin: value => `${Numbers.format(value, 0, settings.group ?? ",",
            settings.point ?? ".")} K`
    })
    readonly property var warmthSaid: Words.of("displays:warmth").said ?? {}

    // A switch of Warmth's, in its words' state, so the : line says its
    // command: Return turns it off or back on, the launcher staying to
    // show it change.
    function warmthSwitch(output, id, on, run) {
        return thing({
            key: `displays:${output.name}:${id}`,
            wordsId: `displays:${id}`,
            wordState: on ? "" : "off",
            name: Words.name(`displays:${id}`),
            verb: Words.verb(`displays:${id}`, on ? "" : "off"),
            glyph: Words.glyph(`displays:${id}`),
            hue: "machine",
            side: on ? "on" : "off",
            stays: true,
            run: () => run(!on)
        });
    }

    // The Warmth, a thing: its side the warmth now, its note what holds
    // it, and Return stepping into the Night Light, Follow the Room where
    // a sensor sees colour, then its figures, which its plate shows too,
    // under where it's set and what it warms (VKCF3Z, 2026-11-08).
    function warmthRow(output) {
        const now = Date.now();
        const figures = Warmth.figures(warmth, now, warmthSaid, warmthTold);
        const name = partName("warmth");
        return thingRow({
            key: `displays:${output.name}:warmth`,
            name: name,
            glyph: Theme.glyph.warmth,
            side: warmthTold.kelvin(warmth.kelvin),
            note: Warmth.note(warmth, now, warmthSaid, warmthTold),
            thing: true,
            // Where it's set, then what it warms, which gives way first
            // where the plate is narrow (VKCF3Z: where, then kind).
            drawer: () => ({
                eyebrow: [warmthSaid.via, warmthSaid.screen].join(" \u00b7 "),
                eyebrowItems: [{ said: warmthSaid.via ?? "", keep: 2 },
                    { said: warmthSaid.screen ?? "", keep: 1 }],
                eyebrowNames: true,
                title: name,
                facts: figures.map(figure => ({
                    label: Words.name(`displays:warmth:${figure.id}`),
                    value: figure.side
                }))
            })
        }, () => [warmthSwitch(output, "night-light", warmth.nightLight,
            on => provider.nightLight(on))]
            .concat(warmth.seesColour ? [warmthSwitch(output, "follow-room",
                warmth.followsTheRoom, follows => provider.followTheRoom(follows))] : [])
            .concat(figures.map(figure => copyRow({
                name: Words.name(`displays:warmth:${figure.id}`),
                side: figure.side,
                copy: figure.side,
                glyph: ""
            }))));
    }

    // Tells the backlight daemon whether the night light is on.
    function nightLight(on) {
        Niri.spawn(["busctl", "--user", "set-property", "io.jitsusama.Backlight",
            "/io/jitsusama/Backlight", "io.jitsusama.Backlight", "NightLight", "b",
            on ? "true" : "false"]);
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
        roomHeard = true;
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
        running: provider.launcher !== null
            && (provider.launcher.open || provider.terminalLooks)
        onTriggered: {
            provider.roomless = !provider.askRoom();
            provider.readLight();
            provider.lightAsked = true;
        }
    }

    // Whether a terminal's `launcher show` walked into Screens or to a
    // screen (Launcher.qml's terminalSees), so its drawer's light is read
    // with the launcher closed too.
    readonly property bool terminalLooks: (launcher?.terminalSees.walked ?? [])
        .some(row => row?.from === provider || String(row?.key ?? "").startsWith("display:"))
    // Whether the light, and the room's, have been read since the terminal
    // looked, or there's no sensor to hear the room by.
    property bool lightAsked: false
    property bool roomHeard: false
    property bool roomless: false
    onTerminalLooksChanged: {
        lightAsked = false;
        roomHeard = false;
    }
    // A terminal's look waits for a fresh read of each, as the launcher
    // shows them once it opens (Provider.stillReading).
    stillReading: terminalLooks && !(launcher?.open ?? false)
        && (!lightAsked || (!roomHeard && !roomless))

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

    function mode(output) {
        return output.current_mode === null ? null : output.modes[output.current_mode];
    }

    function described(output) {
        const now = mode(output);
        if (now === null || output.logical === null)
            return "off";
        // The scale as its part and its drawer say it, "1.75\u00d7".
        return `${now.width} \u00d7 ${now.height} \u00b7 ${Screens.hertz(now.refresh_rate)} `
            + `\u00b7 scale ${Screens.scale(output.logical.scale)}`;
    }

    // Screens are choices: the built-in first, as it's always there, then by
    // name, and the one in use never moves for being chosen.
    readonly property var shown: Order.sorted(Object.values(outputs), Order.choices({
        natural: (a, b) => Number(builtIn(b)) - Number(builtIn(a)), name: output => output.name }))
    readonly property var lit: shown.filter(output => output.logical !== null)

    readonly property string focusedOutput: {
        void Niri.revision;
        const workspace = Niri.workspace();
        return workspace ? workspace.output : "";
    }

    // A screen dark by name is still in niri's outputs, so it's turned on
    // by name, and says only that its hub is on, "dark" beside it saying
    // the rest.
    function made(output) {
        const stands = Screens.standing(output);
        const focused = lit.length > 1 && output.name === focusedOutput;
        const turnsOn = stands === "dark" ? () => provider.ask({
            Action: { PowerOnMonitor: { output: output.name } }
        }) : () => provider.change(output, "On");
        return member({
            thing: true,
            key: `display:${output.name}`,
            verb: stands === "lit" ? "focus" : "turn on",
            name: named(output),
            keywords: output.name,
            // A screen, built in or not, wears the screen's mark, its
            // eyebrow saying it's built in; `computer` is Machine's (VKCF3Z,
            // 2026-11-08: one mark, one meaning).
            glyph: Theme.glyph.display,
            hue: "machine",
            side: stands === "dark" ? screenSaid.dark
                : stands === "off" ? "off" : focused ? "focused" : "",
            note: stands === "dark" ? screenSaid.hubOn : described(output),
            run: stands === "lit" ? () => Niri.act({ FocusMonitor: { output: output.name } })
                : turnsOn,
            verbs: () => provider.verbs(output),
            drawer: () => provider.drawerOf(output),
            options: {
                power: provider.screenOption(output, "power"),
                unplugged: provider.screenOption(output, "unplugged")
            }
        });
    }

    // A screen's option for the : line's :setl, which turns it as its Turn
    // It Off, Turn It On and Unplug It rows do (Screens.wayTo); one it
    // can't take, as unplugging the last lit screen, says so and does
    // nothing else.
    function screenOption(output, option) {
        return {
            kind: "switch",
            now: () => Screens.optionNow(output, option),
            set: on => {
                const asked = { option: option, on: on };
                const told = Screens.toldOf(output, shown, Niri.powersOffEach, asked);
                if (told !== "")
                    return String(screenSaid[told] ?? "").replace("{name}", named(output));
                const way = Screens.wayTo(output, shown, Niri.powersOffEach, asked);
                if (way.waits)
                    provider.darkenLast(way.asks);
                else
                    provider.ask(way.asks);
                return "";
            }
        };
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

    // The vocabulary as Screens.js says a screen in it.
    readonly property var screenWords: ({
        named: id => provider.partName(id),
        said: (id, key) => provider.partSaid(id, key),
        turned: transform => provider.turnedSaid(transform)
    })

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
                        // What holds it, as its row's note says it: the
                        // night light by night, not the room.
                        Warmth.note(warmth, Date.now(), warmthSaid, warmthTold)])
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
            facts: Plate.identified(facts.concat(Screens.facts(output, screenWords, settings),
                edid,
                physical === "" ? [] : [{ label: partName("size"), value: physical }]),
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
        darkening = null;
        sleeping.restart();
    }

    Timer {
        id: sleeping
        interval: 750
        onTriggered: provider.darkening === null ? Niri.act({ PowerOffMonitors: {} })
            : provider.ask(provider.darkening)
    }

    // A setting's choice, said by its value alone, its setting named in the
    // path above (design.md), with what it is beside it, as a mode the
    // screen prefers, muted in its note, and Return's verb "set", as
    // every choice's is (VKCF3Z, 2026-11-09). It wears the theme's mark of
    // the setting it's for, by name, and its setting's name is a hidden
    // word, so "scale 1.5" still finds 1.5\u00d7 from outside. Setting it
    // keeps the launcher, the setting saying what it's at now. `made`
    // holds its key, name, mark, whether it's chosen, its run, and its
    // hidden words and note where it has them.
    function choice(made) {
        return thing({
            key: `verb:display-${made.key}`,
            name: made.name,
            verb: partSaid("", "set"),
            keywords: [partName(made.mark), made.keywords ?? ""].join(" ").trim(),
            note: made.note ?? "",
            glyph: Theme.glyph[made.mark],
            hue: "action",
            side: made.chosen ? partSaid("", "chosen") : "",
            stays: true,
            run: made.run
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
            .filter(fact => fact.name.toLowerCase() !== "brightness")
            .map(fact => {
                const told = toldFigures[fact.name.toLowerCase()];
                return told ? { name: fact.name, value: told(fact.value) } : fact;
            })
            .filter(fact => fact.value !== "");
        return factParts("displays", output.name, said, askedFirst);
    }

    // The facts desk-facts gives as figures, said here by the launcher's
    // grouping and decimal point, as Warmth beside them is (Y2YBTS,
    // 2026-11-09).
    readonly property var toldFigures: {
        const words = { said: (id, key) => provider.partSaid(id, key) };
        return {
            white: figures => Screens.whitePoint(figures, words, settings),
            primaries: figures => Screens.primaries(figures, words, settings)
        };
    }

    // The facts asked of a display most, which lead the rest, its white
    // point and primaries beside its colours (VKCF3Z, 2026-11-09).
    readonly property var askedFirst: ["depth", "colours", "white", "primaries", "hdr"]

    // A part's name, as the vocabulary says it (Provider's partNamed).
    function partName(id) {
        return partNamed("displays", id);
    }

    // A display's right arrow: turning it on while it's off or dark; while
    // it's lit, each of its settings as a part saying how it's set, whose
    // step in offers the others (and whose choices what's typed finds from
    // here, so "scale 1.5" is Scale's 1.5\u00d7), then the ways it turns
    // off that Screens.js gives, then what its EDID says.
    function verbs(output) {
        const ways = Screens.ways(output, shown, Niri.powersOffEach).map(way => powerRow(way));
        if (Screens.standing(output) !== "lit")
            return ways.concat(factRows(output));
        return setUp(output).concat(ways, factRows(output));
    }

    // Words a screen's standing is said in, from the vocabulary.
    readonly property var screenSaid: Words.of("displays").said ?? {}

    // One way a screen turns off or on, as a choice: its words the
    // vocabulary's, its note why it matters, and the last lit screen's
    // darkening closing the launcher, since nothing would show it, and
    // waiting for the key to come up, which would light every screen.
    function powerRow(way) {
        const id = `verb:display-${way.word}`;
        return thing({
            key: id,
            name: Words.name(id),
            glyph: Words.glyph(id),
            hue: "action",
            note: way.note === "" ? "" : Words.of(id).said?.[way.note] ?? "",
            stays: !way.waits,
            run: way.waits ? () => provider.darkenLast(way.asks) : () => provider.ask(way.asks)
        });
    }

    // Asks niri to change a screen, then asks again how they stand, so a
    // screen dark by name says so at once.
    function ask(asked) {
        Niri.request(asked, () => provider.refresh());
    }

    function darkenLast(asked) {
        darkening = asked;
        sleeping.restart();
    }

    // What the last lit screen's darkening asks, once the key is up; none
    // when every screen goes off.
    property var darkening: null

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

    // How niri turns a display, its four turns then its four mirrored, as a
    // part says it and as its choice does, in the vocabulary's words; niri
    // turns "90" counter-clockwise.
    readonly property var turns: Screens.transforms().map(transform => ({
        transform: transform,
        name: turnedSaid(transform)
    }))

    // Variable Refresh, a switch keeping its name as it flips, saying on or
    // off, Return flipping it and the launcher staying (VKCF3Z,
    // 2026-11-09). niri's on-demand variable refresh, on only while a
    // window rule asks, would make it a setting of three; nothing here
    // sets that, so it's on or off.
    function variableSwitch(output) {
        const on = output.vrr_enabled === true;
        return thing({
            key: `displays:${output.name}:variable`,
            wordsId: "displays:variable",
            wordState: on ? "" : "off",
            name: partName("variable"),
            verb: Words.verb("displays:variable", on ? "" : "off"),
            // Its own mark, as a switch wears its thing's (VKCF3Z,
            // 2026-11-09): one mark, one meaning, so never Refresh's.
            glyph: Theme.glyph.variableRefresh,
            hue: "machine",
            side: partSaid("variable", on ? "on" : "off"),
            stays: true,
            run: () => change(output, { Vrr: { vrr: { vrr: !on, on_demand: false } } })
        });
    }

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
            "scale", scales.map(each => choice({
                key: `scale-${each}`,
                name: Screens.scale(each),
                mark: "scale",
                chosen: Math.abs(each - scale) < 0.01,
                run: () => change(output, { Scale: { scale: { Specific: each } } }),
                keywords: `${Math.round(each * 100)}%`
            })))]);
        if (now !== null) {
            // Each by its value, every digit niri gives, the mode the
            // screen prefers noted so, as a custom one is (VKCF3Z,
            // 2026-11-09).
            const rates = Screens.refreshes(output, screenWords);
            // Its side says the rate as its choices do, every digit, so
            // 59.94 Hz running isn't said as the 60 Hz beside it.
            rows.push(setting(output, "refresh", Screens.hertz(now.refresh_rate), "refresh",
                rates.length < 2 ? [] : rates.map(each => choice({
                    // By the mode's place in niri's list, the one key a
                    // mode has even where two share a size and a rate.
                    key: `mode-${output.modes.indexOf(each.mode)}`,
                    name: each.value,
                    mark: "refresh",
                    chosen: each.chosen,
                    run: () => change(output, { Mode: { mode: { Specific: {
                        width: each.mode.width,
                        height: each.mode.height,
                        refresh: each.mode.refresh_rate / 1000
                    } } } }),
                    note: each.note
                }))));
            if (output.vrr_supported)
                rows.push(variableSwitch(output));
            const sizes = Screens.sizes(output, screenWords);
            rows.push(setting(output, "resolution", `${now.width} \u00d7 ${now.height}`,
                "resolution",
                sizes.length < 2 ? [] : sizes.map(each => choice({
                    key: `size-${each.mode.width}x${each.mode.height}`,
                    name: each.value,
                    mark: "resolution",
                    chosen: each.chosen,
                    run: () => change(output, { Mode: { mode: { Specific: {
                        width: each.mode.width,
                        height: each.mode.height,
                        refresh: null
                    } } } }),
                    note: each.note
                }))));
        }
        rows.push(setting(output, "rotation", turnedSaid(output.logical.transform), "rotation",
            turns.map(each => choice({
                key: `turn-${each.transform}`,
                name: each.name,
                mark: "rotation",
                chosen: output.logical.transform === each.transform,
                run: () => change(output, { Transform: { transform: each.transform } })
            }))));
        return rows;
    }

    // Asked of a display by a part's words, "colour depth" or "refresh
    // rate": the focused one's, or each one's when several are on.
    readonly property var partsAsked: {
        const out = {};
        for (const id of ["brightness", "warmth", "room", "scale", "refresh", "resolution",
            "rotation", "depth", "colours", "white", "primaries", "hdr"])
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
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score);
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

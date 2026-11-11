// Displays, a collection: the displays niri drives, the built-in one first,
// each with its size, refresh rate and scale in the footer. Return moves
// focus to one, or turns on one that's off; the right arrow lists its
// Scale, Refresh, Resolution and Rotation as parts saying how each is set,
// each stepping into its choices (which what's typed there finds too, so
// "scale 1.5" needs no step in), and turning it off while another is on.
// "colour depth", "refresh rate" and the like ask the focused display's
// part, or each display's while several are on. niri forgets these
// choices when its configuration is next read, so they last the session,
// and what lasts belongs in the configuration. Each screen's row comes
// ready from the windows daemon (modules/windows, screens.rs), its name,
// how it stands, what it runs and its drawer, light and all, said there,
// with niri's record of it, which its settings and acts read here; the
// daemon asks niri for its screens as a display comes or goes, so the
// shell asks niri nothing of them itself. A change made here goes over niri's
// socket, which says nothing of it in niri's stream, so the daemon is
// asked to look again (refresh). What a display's EDID says of itself, who
// made it and when, its size, native mode, refresh range, colour depth,
// colours, white point, primaries and HDR, desk-facts writes to a file
// (modules/desk-facts), which the daemon's drawer says, and the right
// arrow lists after the settings, named first as parts are (Colour depth
// "10 bit"), Return copying one. The white point and primaries come as figures, said here by Joel's
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
// backlight and the room's light are the machine daemon's (modules/machine,
// light.rs), read once a second while they can be seen and told here as
// they move, so the shell reads no file for them, and the sensor's slow
// count is the daemon's to wait on, on a thread of its own. The screen the backlight's device
// names as its connector holds Brightness, and is never said, since its
// drawer's eyebrow already names it.
//
// The backlight daemon warms the built-in screen at night from sunset to
// sunrise, and toward the room's light where its sensor sees colour, and
// says how that stands, which the machine daemon serves as it changes
// (modules/backlight, modules/machine's steered.rs), so the Warmth
// follows the Brightness on every machine with a panel to warm: its side
// the warmth now, its note what holds it, and Return stepping into the
// Night Light and Follow the Room switches, which the daemon is told over
// the session's bus and keeps, then the room's white, the night's sunset
// and sunrise and its white (Warmth.js; VKCF3Z, 2026-11-08). "screen
// warmth" asks it.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Backlight.js" as Backlight
import "Numbers.js" as Numbers
import "Order.js" as Order
import "Power.js" as Power
import "ScreenAnswers.js" as ScreenAnswers
import "Screens.js" as Screens
import "Seen.js" as Seen
import "Warmth.js" as Warmth

Provider {
    id: provider

    what: "display"

    collection: Words.collection("displays")

    // The screens' rows as the windows daemon serves them, each holding
    // niri's record of its screen and its drawer; Turn Every Screen Off is
    // the daemon's row too, while one is lit. The screen whose drawer
    // shows is watched ahead, and the one holding the Brightness while a
    // terminal looks, so the daemon reads its light from the machine
    // daemon only then (modules/windows, light.rs).
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        place: "displays"
        ahead: [provider.drawerWanted, provider.terminalLooks ? provider.backlitKey : ""]
            .filter((key, at, keys) => key !== "" && keys.indexOf(key) === at)
        make: (row, key) => provider.madeFrom(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }

    // What niri says of each display, by its connector's name, as its
    // served row holds it.
    readonly property var outputs: {
        void ready.placings;
        const out = {};
        for (const key of ready.keys) {
            const held = ready.served(key)?.held;
            if (held?.niri)
                out[held.output] = Object.assign({ name: held.output }, held.niri);
        }
        return out;
    }

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
    }, light === null ? {} : {
        brightness: {
            kind: "number",
            least: 0,
            most: 100,
            now: () => provider.light?.share ?? 0,
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

    // A change made to a screen here says nothing in niri's stream, nor
    // does a key waking the screens, so the windows daemon is asked to look
    // at niri's screens again, here and as the launcher opens, and tells
    // the rows that moved; the asking is the daemon's, off this thread.
    function refresh() {
        ready.link?.ask({ ask: "look" }, null);
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.refresh();
            // Shut, no drawer shows, so none is watched ahead.
            else
                provider.drawerWanted = "";
        }
    }

    // The lights are read once as the shell starts too, though the
    // launcher is closed, since what's inside a screen is made from what's
    // held while it's closed (Daemon.qml's refresh).
    Component.onCompleted: lights.refresh()

    // Tells niri what to change, then asks again how things stand.
    // `refusal` is what the row did, as { thing, done }, for the words
    // should niri refuse it (ask).
    function change(output, action, refusal) {
        ask({ Output: { output: output.name, action: action } }, refusal, output);
    }

    // Says a word on the launcher's warning line, where there's one.
    function warn(word) {
        if (word !== "")
            provider.launcher?.operatorSays(word);
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
        const named = light?.connector ?? null;
        const home = lit.find(each => each.name === named)
            ?? lit.find(each => inTheRoom(each))
            ?? lit.find(each => follows(each)) ?? null;
        return home !== null && home.name === output.name;
    }


    // The levels a step into Brightness offers, brightest first.
    readonly property var levels: [100, 90, 80, 70, 60, 50, 40, 30, 20, 10]

    // Sets the screen to a share, as the brightness keys do, through the
    // machine daemon (modules/machine/src/screen_acts.rs) rather than a
    // program started for each step; the first step stays lit, since none
    // at all leaves the screen black.
    function brighten(share) {
        lights.ask({ ask: "act", act: "brightness", share: Math.max(1, share) }, null);
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
        const rows = [Object.assign(setting(output, "brightness", lightRead ? `${light.share}%`
            : unreadSaid(), "brightness",
            // Each by its value, as every choice is (VKCF3Z, 2026-11-09),
            // :set brightness= its command.
            levels.map(each => choice({
                key: `brightness-${each}`,
                name: Words.name("displays:brightness:level", "", { n: each }),
                mark: "brightness",
                chosen: lightRead && light.share === each,
                run: () => provider.brighten(each),
                words: "displays:brightness:level",
                fill: { n: each }
            }))), {
            graph: level(lightRead ? light.now : 0, light.most),
            // With the lid shut nothing hears the room, and a monitor has
            // no steps of the panel's, so it only says what sets it.
            note: inTheRoom(output)
                ? partSaid("brightness", lightRead ? "followsRoom" : "followsRoomUnread")
                    .replace("{now}", grouped(light.now)).replace("{most}", grouped(light.most))
                : partSaid("brightness", "everyFollows")
        })];
        // The row stands from the sensor while what it hears is still being
        // read, so it keeps its place, and from what it heard once read.
        // Until then it says it's being read, as every figure still read
        // does (Catalogue.acts), and copies nothing: "not read yet" pasted
        // is no use anywhere, so Return only closes. Said as nothing, it
        // was a row whose Return did nothing (goes-places, 2026-11-10).
        const heard = lightRead ? room : null;
        if (inTheRoom(output) && roomSensor !== null)
            rows.push(copyRow({
                key: `displays:${output.name}:room`,
                name: partName("room"),
                side: heard === null ? unreadSaid() || "\u2026" : lux(heard),
                copy: heard === null ? "" : lux(heard),
                copiesNothing: heard === null,
                glyph: "",
                hue: "machine",
                graph: level(heard === null ? 0 : Power.roomLevel(heard), 1),
                note: partSaid("room", "hears")
            }));
        // Warmth sits right after Brightness, found by its key, so a row
        // put above Brightness later can't move it.
        if (inTheRoom(output) && warmth !== null) {
            const brightness = `displays:${output.name}:brightness`;
            rows.splice(rows.findIndex(row => row.key === brightness) + 1, 0, warmthRow(output));
        }
        return rows;
    }

    // How the screen's warmth stands, as the backlight program says it
    // (Warmth.js's of), or null where nothing warms the screen.
    property var warmth: null
    onWarmthChanged: changed()

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

    // Tells the backlight daemon whether the night light is on, through
    // the machine daemon, which sets its property on the session's bus.
    function nightLight(on) {
        lights.ask({ ask: "act", act: "night-light", on: on }, null);
    }

    // Tells the backlight daemon whether the screen follows the room, the
    // same way.
    function followTheRoom(follows) {
        lights.ask({ ask: "act", act: "follow-the-room", follows: follows }, null);
    }

    // Setting the screen to a share, Return doing it.
    function setTo(share) {
        const light = provider.light;
        if (light === null)
            return null;
        const kept = Math.max(1, share);
        return {
            headline: partSaid("", "setTo").replace("{share}", kept),
            verb: Words.command("command:brightness").verb,
            stays: true,
            run: () => provider.brighten(kept),
            rows: [],
            note: lightRead ? partSaid("", "now").replace("{share}", light.share)
                : unreadSaid()
        };
    }

    // Whether a light's answer shows, out at the top, which the launcher
    // asks again as the light moves, so what follows the room moves as it
    // does.
    readonly property bool lightShown: launcher !== null && launcher.open
        && (launcher.held ?? []).length === 0 && launcher.answer?.live === provider

    // The machine daemon's backlights and light sensors, read once a second
    // while they can be seen (lightsSeen); none is held while the daemon
    // isn't answering, and what an earlier watch left is said as not read
    // yet (lightRead), so no old level is said as now.
    Daemon {
        id: lights

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: ["backlight", "light", "brightness", "warmth"]
        live: provider.lightsSeen
        keptThroughGap: false
        answersWhole: true
    }

    // The backlight and the room's sensor as last told (Backlight.js's lit
    // and sensor), for a screen's drawer and the rows a step into it holds,
    // each replaced only when what it says changed, so the room's light
    // moving leaves the backlight be; and `room`, the lux the sensor hears,
    // or null until it's said one.
    property var light: null
    onLightChanged: changed()
    // Whether what's held of the lights was told for this watch, so says
    // the level as it is now; until then the level held from an earlier
    // watch is said as not read yet, in the same rows.
    readonly property bool lightRead: lights.answeredThisWatch
    // What a level not yet read says: nothing for its first second, which
    // the daemon's answer nearly always beats, so no word flashes past,
    // then that it's not read yet (VKCF3Z, 2026-11-10). The sensor's count
    // is read on a thread of its own, so it can come after the backlight's.
    function unreadSaid() {
        return lightWaited ? partSaid("", "unread") : "";
    }
    property bool lightWaited: false
    onLightWaitedChanged: changed()
    readonly property bool lightsAllRead: lightRead && (roomSensor === null || room !== null)
    // Fires a second into a watch that hasn't read everything, and each
    // second after, which changes nothing; it repeats so that its running
    // stays bound, since a timer that stops itself lets go of its binding.
    Timer {
        interval: 1000
        repeat: true
        running: lights.live && !provider.lightsAllRead
        onTriggered: provider.lightWaited = true
    }
    property var roomSensor: null
    onRoomSensorChanged: changed()
    readonly property var room: roomSensor?.lux ?? null

    Connections {
        target: lights

        function onRevisionChanged() {
            provider.heard();
        }

        // A light's answer waits for the watch's first answer, so it's
        // asked again once that's come, moved or not. Asked as the turn
        // ends, never at once: a watch ending as lightsSeen changes says
        // so here, inside lightsSeen's own binding, and an answer asked
        // there makes the rows lightsSeen reads again, a binding loop
        // whenever the watch was answered before the next key (the
        // answers VM test, 2026-11-10).
        function onAnsweredThisWatchChanged() {
            Qt.callLater(provider.lightAnswered);
        }

        function onLiveChanged() {
            provider.lightWaited = false;
        }
    }

    function lightAnswered() {
        if (lightShown)
            answered();
    }

    // Each of what the lights say is replaced only when it changed, so the
    // room's light moving leaves the rest be.
    function heard() {
        const records = Object.values(lights.held);
        const now = Backlight.lit(records);
        const sensed = Backlight.sensor(records);
        const warmed = Warmth.of(records);
        const moved = (was, is) => JSON.stringify(was) !== JSON.stringify(is);
        const lit = moved(light, now);
        const sensing = moved(roomSensor, sensed);
        const warming = moved(warmth, warmed);
        if (lit)
            light = now;
        if (sensing)
            roomSensor = sensed;
        if (warming)
            warmth = warmed;
        if ((lit || sensing || warming) && lightShown)
            answered();
    }

    // Whether a terminal's `launcher show` walked into Screens or to a
    // screen (Launcher.qml's terminalSees), so its drawer's light is read
    // with the launcher closed too.
    readonly property bool terminalLooks: (launcher?.terminalSees.walked ?? [])
        .some(row => row?.from === provider || String(row?.key ?? "").startsWith("display:"))
    // A terminal's look waits for the daemon's whole answer to its watch,
    // as the launcher shows the light once it opens (Provider.stillReading).
    // Its drawer waits too, while a backlight is told, for the daemon to say
    // its Brightness, read or not yet read, which it does within a second
    // of its look (modules/windows, light.rs's WAITED).
    stillReading: terminalLooks && !(launcher?.open ?? false) && lights.state !== "not answering"
        && (!lights.answeredThisWatch || light !== null && backlitKey !== ""
            && brightnessServed(backlitKey) === "")

    // The row's key of the screen whose place holds the Brightness, or "".
    readonly property string backlitKey: {
        const home = lit.find(output => backlit(output));
        return home ? `row:displays:display:${home.name}` : "";
    }

    // What the drawer the daemon serves on the row of `key` says of the
    // Brightness, "" while it says none.
    function brightnessServed(key) {
        void ready.placings;
        const named = partName("brightness");
        const fact = (ready.served(key)?.drawer?.facts ?? []).find(each => each.name === named);
        return fact?.said ?? "";
    }

    // Whether the lights can be seen, so are watched: Screens' answer,
    // Screens or a screen stepped into or found, a place holding Screens,
    // words typed asking the Brightness or the Room light, or a terminal's
    // look (Seen.js), as Heat is read; not every page the launcher opens on.
    readonly property bool lightsSeen: Seen.seen(launcher, provider, ["displays"],
        typed => brightnessTyped.test(typed.trim().toLowerCase())
            || ["brightness", "room"].includes(partsAsked[asks(typed)]))
        || terminalLooks

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

    // Screens in the order the daemon serves them: the built-in first, as
    // it's always there, then by connector.
    readonly property var shown: Object.values(outputs)
    readonly property var lit: shown.filter(output => output.logical !== null)

    readonly property string focusedOutput: {
        void Niri.revision;
        const workspace = Niri.workspace();
        return workspace ? workspace.output : "";
    }

    // What a served row is made into: a screen's, by the niri record it
    // holds, or Turn Every Screen Off.
    function madeFrom(row, key) {
        if (key === "row:displays:sleep")
            return sleepRow(row);
        const output = outputs[row.held?.output];
        return output ? made(output, row, key) : null;
    }

    // A screen's row as the daemon says it, its name, how it stands and
    // what it runs, with its acts here: a screen dark by name is still in
    // niri's outputs, so it's turned on by name.
    function made(output, row, key) {
        const stands = Screens.standing(output);
        const turnsOn = stands === "dark" ? () => provider.ask({
            Action: { PowerOnMonitor: { output: output.name } }
        }, { thing: named(output), done: "on" }, output)
            : () => provider.change(output, "On", { thing: named(output), done: "on" });
        return member({
            thing: true,
            key: `display:${output.name}`,
            verb: partSaid("", stands === "lit" ? "focus" : "turnOn"),
            name: row.name,
            keywords: output.name,
            glyph: Theme.glyph[row.glyph] ?? Theme.glyph.display,
            hue: row.hue ?? "machine",
            side: ready.sideOf(row),
            note: ready.noteOf(row),
            run: stands === "lit" ? () => Niri.act({ FocusMonitor: { output: output.name } })
                : turnsOn,
            verbs: () => provider.verbs(output),
            drawer: () => provider.drawerOf(key),
            options: Object.assign({
                power: provider.screenOption(output, "power"),
                unplugged: provider.screenOption(output, "unplugged")
            }, stands === "lit" && output.logical !== null ? provider.lineOptions(output) : {})
        });
    }

    // A setting's option on the : line, its name lowercased and joined, as
    // Vim's are (shiftwidth): Scale's scale, Variable Refresh's
    // variablerefresh (VKCF3Z, 2026-11-09).
    function lineName(id) {
        return partName(id).toLowerCase().split(" ").join("");
    }

    // A lit screen's settings as :setl takes them, as their choices set
    // them: scale=1.5, refresh=59.951 (or 60, where one mode rounds to
    // it), resolution=2560x1440 and rotation=upright, each completing
    // from its screen's own choices with their words beside, and one it
    // doesn't offer saying what it does (Screens.lineSet). Variable
    // Refresh is a switch, :setl variablerefresh and novariablerefresh,
    // as power is, where the screen has it.
    function lineOptions(output) {
        const options = {};
        for (const id of Screens.lineSettings(output)) {
            options[lineName(id)] = {
                kind: "choice",
                choices: () => Screens.lineValues(output, id, screenWords)
                    .map(each => ({ value: each.value, note: each.note })),
                now: () => Screens.lineNow(output, id, screenWords),
                set: to => {
                    const asked = Screens.lineSet(output, { setting: id, typed: String(to) },
                        screenWords, named(output));
                    if (asked.refused !== undefined)
                        return asked.refused;
                    change(output, asked.asks,
                        { thing: lineSaid(id, to, asked.asks), done: "set" });
                    return "";
                }
            };
        }
        if (output.vrr_supported)
            options[lineName("variable")] = {
                kind: "switch",
                now: () => output.vrr_enabled === true,
                set: on => {
                    change(output, { Vrr: { vrr: { vrr: on, on_demand: false } } },
                        { thing: partName("variable"), done: on ? "on" : "off" });
                    return "";
                }
            };
        return options;
    }

    // A setting typed on the : line, said as its choice's row says it: a
    // rotation by the vocabulary's words for its turn, the rest by their
    // value.
    function lineSaid(id, to, asks) {
        return id === "rotation" ? turnedSaid(asks.Transform.transform)
            : Screens.typedSaid(id, String(to));
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
                    return ScreenAnswers.toldSaid(told, named(output), screenWords);
                const way = Screens.wayTo(output, shown, Niri.powersOffEach, asked);
                if (way.waits)
                    provider.darkenLast(way.asks);
                else
                    provider.ask(way.asks, { thing: named(output), done: way.word }, output);
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
    // Its row is the daemon's, served while a screen is lit; its act is
    // made here.
    function sleepRow(row) {
        return thing({
            key: "displays:sleep",
            verb: Words.verb("displays:sleep"),
            name: row.name,
            glyph: Theme.glyph[row.glyph] ?? "",
            hue: row.hue ?? "action",
            note: ready.noteOf(row),
            run: () => provider.sleepAll()
        });
    }

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

    // The screen whose drawer shows, by its row's key, which is watched
    // ahead while it does (ready.ahead), so its light is read; set as the
    // turn ends, never inside the drawer's own binding, as Agents' is.
    property string drawerWanted: ""

    // A screen's drawer as the windows daemon serves it on its row: its
    // connector over its name, its light where it holds the Brightness,
    // what niri says it runs, then what its EDID says, its serial last
    // (modules/windows, screens.rs and light.rs).
    function drawerOf(key) {
        if (key !== drawerWanted)
            Qt.callLater(() => {
                provider.drawerWanted = key;
            });
        const row = ready.served(key);
        return row ? ready.drawerOf(row) : null;
    }

    readonly property var all: ready.rows()
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
    // hidden words and note where it has them. Its command is its words'
    // (`words`, verb:display-set's :setl scale=1.5 unless it names its
    // own), `fill` filling it in.
    function choice(made) {
        return thing({
            key: `verb:display-${made.key}`,
            wordsId: made.words ?? "verb:display-set",
            fill: made.fill,
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

    // What its spots are made of (Launcher.qml's spotsFrom): the screens as niri
    // says them, which one holds the Brightness, whether the room's sensor
    // and the warmth are there, and which facts desk-facts says; never a
    // level, a lux or a warmth's figure, which move while nothing else
    // does. modules/windows' display kind takes this over, serving the
    // screens' rows ready, by 12 November.
    property string spotsStamp: JSON.stringify([ready.placings,
        light === null ? false : light.connector ?? "", roomSensor !== null,
        warmth === null ? null : warmth.seesColour === true,
        Niri.powersOffEach, Object.keys(facts).sort().map(name => [name,
            (facts[name] ?? []).map(fact => `${fact.name}:${fact.value === ""}`)])])

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

    // A count, as of steps or of the sensor's, grouped in threes.
    function grouped(value) {
        return Numbers.format(value, undefined, settings.group, settings.point);
    }

    // A display's right arrow: turning it on while it's off or dark; while
    // it's lit, each of its settings as a part saying how it's set, whose
    // step in offers the others (and whose choices what's typed finds from
    // here, so "scale 1.5" is Scale's 1.5\u00d7), then the ways it turns
    // off that Screens.js gives, then what its EDID says.
    function verbs(output) {
        const ways = Screens.ways(output, shown, Niri.powersOffEach)
            .map(way => powerRow(way, output));
        if (Screens.standing(output) !== "lit")
            return ways.concat(factRows(output));
        return setUp(output).concat(ways, factRows(output));
    }

    // One way a screen turns off or on, as a choice: its words the
    // vocabulary's, its note why it matters, and the last lit screen's
    // darkening closing the launcher, since nothing would show it, and
    // waiting for the key to come up, which would light every screen.
    function powerRow(way, output) {
        const id = `verb:display-${way.word}`;
        return thing({
            key: id,
            name: Words.name(id),
            glyph: Words.glyph(id),
            hue: "action",
            note: way.note === "" ? "" : Words.of(id).said?.[way.note] ?? "",
            stays: !way.waits,
            run: way.waits ? () => provider.darkenLast(way.asks)
                : () => provider.ask(way.asks, { thing: named(output), done: way.word }, output)
        });
    }

    // Asks niri to change a screen, then asks again how they stand, so a
    // screen dark by name says so at once. A change niri refuses says so
    // on the warning line, in niri's words for why, and one to a screen
    // gone before it came says it's unplugged (VKCF3Z, 2026-11-09), where
    // Return would otherwise look done (ScreenAnswers.js). Where nothing
    // is there to say it, as the last lit screen darkening once the
    // launcher has closed, `refusal` is null and the shell's log has it.
    function ask(asked, refusal, output) {
        Niri.request(asked, answer => {
            if (output)
                warn(ScreenAnswers.answerSaid(answer, named(output), screenWords));
            provider.refresh();
        }, refusal ? why => {
            warn(ScreenAnswers.refusalSaid(refusal, why, screenWords));
            provider.refresh();
        } : null);
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
            run: () => change(output, { Vrr: { vrr: { vrr: !on, on_demand: false } } },
                { thing: partName("variable"), done: on ? "off" : "on" })
        });
    }

    // How a setting's choices are typed on the : line, in their order
    // (Screens.lineValues), so each choice's command is :setl's.
    function typedOf(output, id) {
        return Screens.lineValues(output, id, screenWords).map(each => each.value);
    }

    // What a setting's choice fills its command with, :setl's option and
    // its value as typed.
    function lineFill(id, typed) {
        return { setting: lineName(id), value: typed };
    }

    // How it's set: Brightness and the Room light where the backlight
    // lights it, then Scale, Refresh, Resolution, Rotation.
    function setUp(output) {
        const now = mode(output);
        const scale = output.logical.scale;
        // A scale is a multiple, as the theme says one, never a share, so
        // the row, the part and the drawer call it one way (VKCF3Z,
        // 2026-11-07). Its share is a hidden word, so "scale 150", as niri
        // and other systems say it, still finds 1.5\u00d7.
        const typedScales = typedOf(output, "scale");
        const rows = lightRows(output).concat([setting(output, "scale", Screens.scale(scale),
            "scale", Screens.scales().map((each, at) => choice({
                key: `scale-${each}`,
                name: Screens.scale(each),
                mark: "scale",
                chosen: Math.abs(each - scale) < 0.01,
                run: () => change(output, { Scale: { scale: { Specific: each } } },
                    { thing: Screens.scale(each), done: "set" }),
                keywords: `${Math.round(each * 100)}%`,
                fill: lineFill("scale", typedScales[at])
            })))]);
        if (now !== null) {
            // Each by its value, every digit niri gives, the mode the
            // screen prefers noted so, as a custom one is (VKCF3Z,
            // 2026-11-09).
            const rates = Screens.refreshes(output, screenWords);
            const typedRates = typedOf(output, "refresh");
            // Its side says the rate as its choices do, every digit, so
            // 59.94 Hz running isn't said as the 60 Hz beside it.
            rows.push(setting(output, "refresh", Screens.hertz(now.refresh_rate), "refresh",
                rates.length < 2 ? [] : rates.map((each, at) => choice({
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
                    } } } }, { thing: each.value, done: "set" }),
                    note: each.note,
                    fill: lineFill("refresh", typedRates[at])
                }))));
            if (output.vrr_supported)
                rows.push(variableSwitch(output));
            const sizes = Screens.sizes(output, screenWords);
            const typedSizes = typedOf(output, "resolution");
            rows.push(setting(output, "resolution", `${now.width} \u00d7 ${now.height}`,
                "resolution",
                sizes.length < 2 ? [] : sizes.map((each, at) => choice({
                    key: `size-${each.mode.width}x${each.mode.height}`,
                    name: each.value,
                    mark: "resolution",
                    chosen: each.chosen,
                    run: () => change(output, { Mode: { mode: { Specific: {
                        width: each.mode.width,
                        height: each.mode.height,
                        refresh: null
                    } } } }, { thing: each.value, done: "set" }),
                    note: each.note,
                    fill: lineFill("resolution", typedSizes[at])
                }))));
        }
        rows.push(setting(output, "rotation", turnedSaid(output.logical.transform), "rotation",
            turns.map(each => choice({
                key: `turn-${each.transform}`,
                name: each.name,
                mark: "rotation",
                chosen: output.logical.transform === each.transform,
                run: () => change(output, { Transform: { transform: each.transform } },
                    { thing: each.name, done: "set" }),
                fill: lineFill("rotation", partSaid("rotation", `typed-${each.transform}`))
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
            // Until the daemon's first answer comes, a light's answer is
            // still being asked, so Return waits for it; while it shows,
            // it's asked again as the light moves.
            asking: light && !lights.answeredThisWatch && lights.state !== "not answering",
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

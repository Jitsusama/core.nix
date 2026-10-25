// How bright the screen and the room are, answered in the field:
// "brightness", "screen light" or "room light". The headline
// is the screen's brightness; the rows are parts named as the vocabulary
// names them (Words.qml): each Screen as a setting, its share as a bar with
// its levels a step in, and the Room light the screen's sensor hears, as a
// figure. A keyboard's backlight is the keyboard's own (Keyboards.qml), and
// "backlight" asks it there: Joel, 2026-10-25, "backlight should refer to any
// keyboard". With a number, "brightness 40",
// Return sets the screen to that share, as the brightness keys do, and so
// does a level chosen a step into Screen, so the backlight daemon
// (modules/backlight), which follows the room, learns it as Joel's pick.
// Read from the kernel's files when asked, their folders listed once by
// Qt, so nothing is spawned and nothing is watched; the room's light,
// which the sensor takes a tenth of a second to say, is read in the
// background and shown once it's come.
// While the answer shows, it's asked again once a second, never more, so
// the screen following the room and the room itself move as they do.
//
// And the desk light, as DeskLight.qml hears it: "desk light", "lamp" or
// "litra" answers how it's lit, "desk light 60" sets its brightness,
// "desk light 4000 k" its warmth, "desk light off" puts it out, and
// "warmer" or "cooler" steps its warmth. Its row holds its Brightness and
// Warmth, each a setting with a bar, stepping brighter, dimmer, warmer or
// cooler or to a level a step in, and putting it out or lighting it; what's
// typed beside them finds their choices too, so "brighter" needs no step
// in. The bar the keys show says where it got to.
import qs
import Qt.labs.folderlistmodel
import Quickshell.Io
import QtQuick
import "Numbers.js" as Numbers
import "Power.js" as Power

Provider {
    id: provider

    examples: ["brightness"]

    function said(file) {
        probe.path = file;
        return read(probe).trim();
    }

    // The words it answers whole, and by their start (Provider.qml), as
    // the vocabulary lists them. "light" asks for every light, before its
    // start can ask the desk's.
    questions: Words.started("light").concat(desks)
    // What asks for the desk light alone.
    readonly property var desks: Words.started("light:desk")

    // How far a step goes: a tenth of its brightness, or about the warmth
    // between two of its own button's steps.
    readonly property int brightnessStep: 10
    readonly property int warmthStep: 500
    // The levels a step into a brightness offers, brightest first, and the
    // warmths a step into the desk light's Warmth does, warmest first.
    readonly property var levels: [100, 90, 80, 70, 60, 50, 40, 30, 20, 10]
    readonly property var warmths: [2700, 3000, 3500, 4000, 4500, 5000, 5500, 6000, 6500]

    function answer(typed) {
        const text = typed.trim().toLowerCase();
        const desk = /^(?:desk (?:light|lamp)|lamp|litra)\s+(?:to\s+)?(?:(\d{1,3})\s*%?|(\d{4})\s*k|(on|off))$/
            .exec(text);
        if (desk)
            return setDesk(desk[1], desk[2], desk[3]);
        if (text === "warmer" || text === "cooler")
            return stepDesk(text);
        if (desks.includes(asks(text)))
            return deskTold();
        const asked = /^(?:brightness|screen (?:light|brightness))\s+(?:to\s+)?(\d{1,3})\s*%?$/
            .exec(text);
        if (asked)
            return set(Math.min(100, Number(asked[1])));
        if (asks(text) === "")
            return null;
        return told();
    }

    // What sets the screen, or one of them by its name, to a share: the
    // first step stays lit, since none at all leaves a screen black.
    function brighten(share, screen) {
        const kept = Math.max(1, share);
        Niri.spawn(["brightnessctl"].concat(screen ? [`--device=${screen}`] : ["--class=backlight"],
            ["set", `${kept}%`]));
    }

    // Setting the screen to a share, Return doing it.
    function set(share) {
        const shown = told();
        if (shown === null || backlights.count === 0)
            return null;
        const kept = Math.max(1, share);
        return {
            headline: `set the screen to ${kept}%`,
            verb: Words.command("command:brightness").verb,
            run: () => provider.brighten(kept, ""),
            rows: shown.rows,
            note: `now ${shown.headline}`
        };
    }

    // A choice a step into a setting, as the vocabulary names it.
    function choice(key, fill, inUse, glyph, run) {
        return thing({
            key: key,
            verb: Words.verb(key),
            name: Words.name(key, "", fill),
            keywords: Words.keywords(key),
            glyph: glyph,
            hue: "action",
            side: inUse ? "in use" : "",
            now: true,
            run: run
        });
    }

    // A setting as a part: its name, how it's set and a bar, its choices a
    // step in and found from beside it by what's typed (`offers`).
    function setting(fields, choices) {
        return holderRow(Object.assign({ offers: true }, fields), choices);
    }

    // The desk light's row, or none with no light.
    function deskRows() {
        const light = DeskLight.light;
        if (light === null)
            return [];
        const lit = deskLit();
        return [holderRow({
            key: "light:desk",
            name: Words.name("light:desk"),
            side: lit,
            copy: lit,
            glyph: Theme.glyph.lamp,
            graph: level(light.on ? light.percent : 0, 100),
            note: "a camera lights it"
        }, () => provider.deskParts())];
    }

    // An act on the desk light, as the vocabulary says it, in the state
    // it's in, done as it is when it's run.
    function deskStep(key, state, run) {
        return thing({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state),
            keywords: Words.keywords(key, state),
            glyph: Theme.glyph.lamp,
            hue: "action",
            now: true,
            run: run
        });
    }

    // Inside the desk light: its Brightness and its Warmth, each a setting
    // stepping it or setting a level, then putting it out or lighting it.
    function deskParts() {
        const light = DeskLight.light;
        if (light === null)
            return [];
        const span = DeskLight.coolest - DeskLight.warmest;
        const brightness = setting({
            key: "light:desk:brightness",
            name: Words.name("light:desk:brightness"),
            side: light.on ? `${light.percent}%` : `out, ${light.percent}% once lit`,
            glyph: Theme.glyph.brightness,
            graph: level(light.on ? light.percent : 0, 100)
        }, () => provider.brightnesses());
        // Warmth fills as the light warms, from its coolest to its warmest.
        const warmth = setting({
            key: "light:desk:warmth",
            name: Words.name("light:desk:warmth"),
            side: `${light.kelvin} K`,
            glyph: Theme.glyph.lamp,
            graph: level(DeskLight.coolest - light.kelvin, span)
        }, () => provider.warmthChoices());
        return [brightness, warmth, deskStep("verb:turn", light.on ? "" : "off",
            () => DeskLight.turn(!DeskLight.light.on))];
    }

    // A step into the desk light's Brightness: a step brighter or dimmer,
    // then each level, the one it's at in use.
    function brightnesses() {
        const at = () => DeskLight.light.percent;
        return [
            deskStep("verb:brighter", "", () => DeskLight.brighten(at() + brightnessStep)),
            deskStep("verb:dimmer", "", () => DeskLight.brighten(at() - brightnessStep))
        ].concat(levels.map(each => choice("light:desk:level", { n: each }, at() === each,
            Theme.glyph.brightness, () => DeskLight.brighten(each))));
    }

    // A step into its Warmth: a step warmer or cooler, then each warmth.
    function warmthChoices() {
        const at = () => DeskLight.light.kelvin;
        return [
            deskStep("verb:warmer", "", () => DeskLight.warm(at() - warmthStep)),
            deskStep("verb:cooler", "", () => DeskLight.warm(at() + warmthStep))
        ].concat(warmths.map(each => choice("light:desk:kelvin", { k: each }, at() === each,
            Theme.glyph.lamp, () => DeskLight.warm(each))));
    }

    // How the desk light is lit, in a few words.
    function deskLit() {
        const light = DeskLight.light;
        return light.on ? `${light.percent}%, ${light.kelvin} K` : "off";
    }

    function deskTold() {
        const rows = deskRows();
        if (rows.length === 0)
            return null;
        return {
            headline: `the desk light is ${deskLit()}`,
            rows: rows,
            note: "a camera lights it"
        };
    }

    // Setting the desk light to a share, a warmth, or on or off, Return
    // doing it.
    function setDesk(share, kelvin, power) {
        const shown = deskTold();
        if (shown === null)
            return null;
        const done = power !== undefined ? {
            headline: power === "on" ? "turn the desk light on" : "turn the desk light off",
            run: () => DeskLight.turn(power === "on")
        } : share !== undefined ? {
            headline: `set the desk light to ${Math.min(100, Number(share))}%`,
            run: () => DeskLight.brighten(Number(share))
        } : {
            headline: `set the desk light to ${kelvin} K`,
            run: () => DeskLight.warm(Number(kelvin))
        };
        return Object.assign(done, {
            verb: Words.command("command:desk").verb,
            rows: shown.rows,
            note: `now ${deskLit()}`
        });
    }

    function stepDesk(way) {
        const shown = deskTold();
        if (shown === null || !DeskLight.light.on)
            return shown;
        const kelvin = DeskLight.light.kelvin + (way === "warmer" ? -warmthStep : warmthStep);
        const kept = Math.max(DeskLight.warmest, Math.min(DeskLight.coolest, kelvin));
        return {
            headline: `make the desk light ${way}, ${kept} K`,
            verb: Words.command("command:desk").verb,
            run: () => DeskLight.warm(kept),
            rows: shown.rows,
            note: `now ${deskLit()}`
        };
    }

    // Each light in a folder whose name passes, as its share and its steps.
    function lights(folder, model, wanted) {
        const found = [];
        for (let i = 0; i < model.count; i++) {
            const name = model.get(i, "fileName");
            if (!wanted.test(name))
                continue;
            const now = Number(said(`${folder}/${name}/brightness`));
            const most = Number(said(`${folder}/${name}/max_brightness`));
            if (most > 0)
                found.push({ name: name, now: now, most: most });
        }
        return found;
    }

    // A light's level as a bar. However bright, it wants nothing, so the
    // bar never warns.
    function level(now, most) {
        return { shape: "bar", values: [most > 0 ? Math.max(0, Math.min(1, now / most)) : 0],
            warn: false };
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
            if (said(`${folder}/name`) === "als" && said(`${folder}/in_illuminance_scale`) !== "")
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
        room = Power.lux(raw, said(`${roomSensor}/in_illuminance_scale`),
            said(`${roomSensor}/in_illuminance_offset`));
        // The launcher asks again only while the answer is out at the top.
        answered();
    }

    function lux(value) {
        return `${Numbers.format(value, value >= 10 ? 0 : 1, settings.group, settings.point)} lx`;
    }

    function told() {
        const screens = lights("/sys/class/backlight", backlights, /./);
        const desk = deskRows();
        const sensed = askRoom();
        if (screens.length === 0 && desk.length === 0)
            return null;
        const share = light => `${Math.round(light.now / light.most * 100)}%`;
        // Named by what it lights, numbered only when there's more than
        // one, so no two rows read alike.
        const part = (id, lights, i) => Words.name(id, lights.length > 1 ? "numbered" : "",
            { n: i + 1 });
        const rows = screens.map((light, i) => {
            const now = Math.round(light.now / light.most * 100);
            // One screen is set as the keys set it; of several, each by
            // its own name.
            const which = screens.length > 1 ? light.name : "";
            return provider.setting({
                key: `light:screen:${light.name}`,
                name: part("light:screen", screens, i),
                side: share(light),
                copy: share(light),
                glyph: Theme.glyph.brightness,
                graph: level(light.now, light.most),
                note: `${light.now} of ${light.most} steps; it follows the room`
            }, () => provider.levels.map(each => provider.choice("light:screen:level", { n: each },
                now === each, Theme.glyph.brightness, () => provider.brighten(each, which))));
        });
        if (sensed && screens.length > 0 && room !== null)
            rows.push(copyRow({
                key: "light:room",
                name: Words.name("light:room"),
                side: lux(room),
                copy: lux(room),
                glyph: "",
                graph: level(Power.roomLevel(room), 1),
                note: "as the screen's sensor hears it, from dark to daylight"
            }));
        return {
            headline: screens.length > 0 ? `${share(screens[0])} bright` : deskLit(),
            rows: rows.concat(desk),
            note: "the screen follows the room; the keys set it",
            // Until the room's first reading comes, the answer is still
            // being asked, so Return waits for it.
            asking: sensed && screens.length > 0 && !roomTried,
            live: provider
        };
    }

    // While its answer shows, it's asked again each second, so what
    // follows the room moves as it does.
    readonly property bool shown: launcher !== null && launcher.open
        && (launcher.held ?? []).length === 0 && launcher.answer?.live === provider

    Timer {
        interval: 1000
        repeat: true
        running: provider.shown
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
}

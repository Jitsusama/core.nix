// How bright the screen and the keyboard are, answered in the field:
// "brightness", "backlight", "screen light" or "keyboard light". The
// headline is the screen's brightness; the rows are each screen's light and
// each keyboard's, as a share and as the steps its driver counts. With a
// number, "brightness 40", Return sets the screen to that share, as the
// brightness keys do, so the backlight daemon (modules/backlight), which
// follows the room, learns it as Joel's pick. Read from the kernel's files when asked, their
// folders listed once by Qt, so nothing is spawned and nothing is watched.
//
// And the desk light, as DeskLight.qml hears it: "desk light", "lamp" or
// "litra" answers how it's lit, "desk light 60" sets its brightness,
// "desk light 4000 k" its warmth, "desk light off" puts it out, and
// "warmer" or "cooler" steps its warmth. Its row's right arrow steps it
// brighter, dimmer, warmer or cooler, the bar the keys show saying where
// it got to.
import qs
import Qt.labs.folderlistmodel
import Quickshell.Io
import QtQuick

Provider {
    id: provider

    examples: ["brightness"]

    function said(file) {
        probe.path = file;
        return read(probe).trim();
    }

    // The words it answers whole, and by their start (Provider.qml).
    questions: ["brightness", "backlight", "screen brightness", "screen light", "display brightness", "bright",
        "brighter", "dim", "dimmer", "keyboard light", "keyboard backlight", "keyboard brightness", "key light"]
        .concat(desks)
    // What asks for the desk light alone.
    readonly property var desks: ["desk light", "desk lamp", "lamp", "litra", "warmer", "cooler", "warmth",
        "light warmth", "colour temperature"]

    // How far a step goes: a tenth of its brightness, or about the warmth
    // between two of its own button's steps.
    readonly property int brightnessStep: 10
    readonly property int warmthStep: 500

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
        const asked = /^(?:brightness|backlight|screen (?:light|brightness))\s+(?:to\s+)?(\d{1,3})\s*%?$/
            .exec(text);
        if (asked)
            return set(Math.min(100, Number(asked[1])));
        if (asks(text) === "")
            return null;
        return told();
    }

    // Setting the screen to a share, Return doing it; the first step stays
    // lit, since none at all leaves a screen black.
    function set(share) {
        const shown = told();
        if (shown === null || backlights.count === 0)
            return null;
        const kept = Math.max(1, share);
        return {
            headline: `set the screen to ${kept}%`,
            verb: "set",
            run: () => Niri.spawn(["brightnessctl", "--class=backlight", "set", `${kept}%`]),
            rows: shown.rows,
            note: `now ${shown.headline}`
        };
    }

    // The desk light's rows, or none with no light.
    function deskRows() {
        const light = DeskLight.light;
        if (light === null)
            return [];
        const step = (key, name, run) => thing({
            key: `verb:${key}`,
            name: name,
            glyph: String.fromCodePoint(0xf095f),
            hue: "action",
            now: true,
            run: run
        });
        return [copyRow({
            name: light.on ? `${light.percent}%, ${light.kelvin} K` : "off",
            side: "the desk light",
            glyph: String.fromCodePoint(0xf095f),
            hue: "machine",
            verbs: () => [
                step("brighter", "Brighter", () => DeskLight.brighten(DeskLight.light.percent + brightnessStep)),
                step("dimmer", "Dimmer", () => DeskLight.brighten(DeskLight.light.percent - brightnessStep)),
                step("warmer", "Warmer", () => DeskLight.warm(DeskLight.light.kelvin - warmthStep)),
                step("cooler", "Cooler", () => DeskLight.warm(DeskLight.light.kelvin + warmthStep)),
                step("turn", DeskLight.light.on ? "Put it out" : "Light it", () => DeskLight.turn(!DeskLight.light.on))
            ]
        })];
    }

    function deskTold() {
        const rows = deskRows();
        if (rows.length === 0)
            return null;
        return {
            headline: DeskLight.light.on ? `the desk light is ${rows[0].name}` : "the desk light is off",
            rows: rows,
            note: "a camera lights it; → steps it"
        };
    }

    // Setting the desk light to a share, a warmth, or on or off, Return
    // doing it.
    function setDesk(share, kelvin, power) {
        const shown = deskTold();
        if (shown === null)
            return null;
        const done = power !== undefined ? {
            headline: power === "on" ? "light the desk light" : "put the desk light out",
            run: () => DeskLight.turn(power === "on")
        } : share !== undefined ? {
            headline: `set the desk light to ${Math.min(100, Number(share))}%`,
            run: () => DeskLight.brighten(Number(share))
        } : {
            headline: `set the desk light to ${kelvin} K`,
            run: () => DeskLight.warm(Number(kelvin))
        };
        return Object.assign(done, { verb: "set", rows: shown.rows, note: `now ${shown.rows[0].name}` });
    }

    function stepDesk(way) {
        const shown = deskTold();
        if (shown === null || !DeskLight.light.on)
            return shown;
        const kelvin = DeskLight.light.kelvin + (way === "warmer" ? -warmthStep : warmthStep);
        const kept = Math.max(DeskLight.warmest, Math.min(DeskLight.coolest, kelvin));
        return {
            headline: `make the desk light ${way}, ${kept} K`,
            verb: "set",
            run: () => DeskLight.warm(kept),
            rows: shown.rows,
            note: `now ${shown.rows[0].name}`
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

    function told() {
        const screens = lights("/sys/class/backlight", backlights, /./);
        const keyboards = lights("/sys/class/leds", leds, /kbd_backlight$/);
        const desk = deskRows();
        if (screens.length === 0 && keyboards.length === 0 && desk.length === 0)
            return null;
        const share = light => `${Math.round(light.now / light.most * 100)}%`;
        const rows = screens.map(light => copyRow({
            name: share(light),
            side: `the screen, ${light.now} of ${light.most}`,
            glyph: String.fromCodePoint(0xf00df)
        })).concat(keyboards.map(light => copyRow({
            name: light.now === 0 ? "off" : `${light.now} of ${light.most}`,
            side: "the keyboard's light",
            glyph: String.fromCodePoint(0xf030c)
        }))).concat(desk);
        return {
            headline: screens.length > 0 ? `${share(screens[0])} bright` : rows[0].name,
            rows: rows,
            note: "the screen follows the room; the keys set it"
        };
    }

    FolderListModel {
        id: backlights
        folder: "file:///sys/class/backlight"
        showFiles: false
        showDotAndDotDot: false
    }

    FolderListModel {
        id: leds
        folder: "file:///sys/class/leds"
        showFiles: false
        showDotAndDotDot: false
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}

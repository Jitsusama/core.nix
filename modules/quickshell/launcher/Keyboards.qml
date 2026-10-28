// Which keyboards are plugged in, answered in the field: "keyboards",
// "keyboard state", "connected keyboards" or "caps lock". The headline
// says how many and whether caps lock is on; the rows are each keyboard by
// name and how it's joined (built in, by USB or by Bluetooth), and each one
// holds its parts, a step in by Return or the right arrow, named first as
// every part is: its Caps lock, Num lock and Scroll lock, as the lights it
// has say; the Layer a board that has them is on (KeyboardLayer.qml), a
// setting whose choices lock another or let it go; how a held key Repeats,
// which modules/keyboard/repeat.nix sets for every keyboard; its Lights, a
// setting: the laptop's saying how lit they are with each level a step in,
// and how long they stay on once nothing's touched, and the ErgoDox's, which
// it can be told to change but never asked, its steps a step in; then its
// Connection, a USB board's Maker and Model, and what desk-facts says of it
// (its Polling rate, Keys and Firmware); and last, the ZSA board's Flash.
// Every name is the vocabulary's (Words.qml).
//
// The keyboards' lights are asked alone too, by "keyboard light", as
// the glossary names a keyboard's own light: one keyboard's choices are the
// answer's rows, and of several, each keyboard holds its own.
//
// Read from the kernel's list of input devices when asked, caps lock from
// the shell's own Lights, and a keyboard's parts from sysfs as it's stepped
// into, so nothing is spawned, nothing is read for a key and nothing more is
// watched.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Keyboards.js" as Keyboards

Provider {
    id: provider

    examples: ["keyboards"]

    // The words it answers whole, and by their start (Provider.qml): the
    // keyboards' first, since the Keyboard collection's part is asked by
    // the first, then their lights'.
    questions: Words.started("keyboards").concat(lightsAsked)
    readonly property var lightsAsked: Words.started("keyboards:lights")

    function answer(typed) {
        const question = asks(typed);
        if (question === "")
            return null;
        return lightsAsked.includes(question) ? lightsTold() : told();
    }

    // The laptop's own reads as what it is rather than as its chip.
    function named(device) {
        return device.name === "AT Translated Set 2 keyboard" ? part("laptop") : device.name;
    }

    // A part's name, as the vocabulary says it: "Caps lock" for
    // keyboards:caps.
    function part(id, state, fill) {
        return Words.name(`keyboards:${id}`, state ?? "", fill);
    }

    // What a file in /sys says, or "" when it says nothing.
    function said(file) {
        probe.path = file;
        return read(probe).trim();
    }

    // A light's level as a bar. However bright, it wants nothing, so the
    // bar never warns.
    function bar(now, most) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, now / most))], warn: false };
    }

    // A part that says a value, its Return copying the value.
    function figure(key, id, side, note) {
        return copyRow({
            key: key,
            name: part(id),
            side: side,
            copy: side,
            hue: "machine",
            glyph: "",
            note: note ?? ""
        });
    }

    function told() {
        const found = Keyboards.keyboards(read(devices));
        const caps = Lights.caps;
        const rows = found.map(device => thingRow({
            key: `keyboards:${device.phys || device.name}`,
            name: named(device),
            side: Keyboards.joined(device),
            glyph: Theme.glyph.keyboard
        }, () => provider.parts(device, Keyboards.lightHolder(found))));
        const count = found.length === 1 ? "one keyboard" : `${found.length} keyboards`;
        return {
            headline: `${count}, caps lock ${caps ? "on" : "off"}`,
            glyph: Theme.glyph.keyboard,
            rows: rows,
            note: "",
            live: provider
        };
    }

    // A keyboard's parts, read as it's stepped into: its locks, its layer,
    // how a held key repeats, its lights, and how it's connected and who
    // made it.
    function parts(device, holder) {
        const key = id => `keyboards:${device.phys || device.name}:${id}`;
        const rows = Keyboards.locks(device).map(lock => figure(key(lock.id), lock.id,
            Number(said(`${Lights.leds}/${lock.light}/brightness`)) > 0 ? "on" : "off"));
        const layer = layerPart(device, key);
        if (layer !== null)
            rows.push(layer);
        const repeat = settings.repeat ?? null;
        if (repeat)
            rows.push(figure(key("repeat"), "repeat",
                Keyboards.repeats(repeat, Words.of("keyboards:repeat").said)));
        for (const row of lights(device, key, holder))
            rows.push(row);
        const folder = Keyboards.usbFolder(device);
        const joined = Keyboards.connection(device, folder === "" ? null : {
            version: said(`${folder}/version`),
            speed: said(`${folder}/speed`)
        });
        if (joined !== "")
            rows.push(figure(key("connection"), "connection", joined));
        if (folder !== "") {
            const maker = said(`${folder}/manufacturer`);
            const model = said(`${folder}/product`);
            if (maker !== "")
                rows.push(figure(key("maker"), "maker", maker));
            if (model !== "")
                rows.push(figure(key("model"), "model", model,
                    `USB id ${device.vendor}:${device.product}`));
        }
        return rows.concat(factParts("keyboards", device.name, facts[device.name],
            ["polling rate", "keys"]), flashing(device));
    }

    // Flashing a board with this machine's build, the ZSA board's alone,
    // since no other can be: last, so stepping in never lands on it, and
    // still found by name from anywhere (Keyboard.qml).
    function flashing(device) {
        if (!Keyboards.zsa(device) || !KeyboardLayout.present)
            return [];
        return [thing({
            key: "keyboard:flash",
            what: "",
            verb: Words.verb("keyboard:flash"),
            name: Words.name("keyboard:flash"),
            glyph: Theme.glyph.flash,
            hue: "action",
            later: true,
            run: () => KeyboardLayout.flash()
        })];
    }

    // What desk-facts says of each keyboard, keyed by the kernel's name for
    // it: { "AT Translated Set 2 keyboard": [{ name, value }] }. No file, or
    // a keyboard it says nothing of, has no facts.
    property var facts: ({})

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/desk/keyboards.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.facts = provider.parsed(text())
        onLoadFailed: provider.facts = ({})
    }

    // A board's Layer, where its watcher says it (KeyboardLayer.qml): a
    // setting whose choices lock it on another layer or let it go, where
    // the board's layout names them, or else a figure; nothing for a board
    // without one. The Keyboard collection finds the same choices by what's
    // typed (Keyboard.qml).
    function layerPart(device, key) {
        if (!Keyboards.zsa(device) || !KeyboardLayer.present)
            return null;
        const side = KeyboardLayer.locked ? `${KeyboardLayer.name}, locked` : KeyboardLayer.name;
        if (!KeyboardLayout.present)
            return figure(key("layer"), "layer", side);
        return holderRow({
            key: key("layer"),
            name: part("layer"),
            side: side,
            glyph: Theme.glyph.layer,
            offers: true
        }, () => Keyboards.layerChoices(KeyboardLayout.layers.map(layer => layer.name),
            KeyboardLayer.name, KeyboardLayer.locked).map(choice => provider.thing({
                key: choice.key,
                verb: Words.verb(choice.id),
                name: Words.name(choice.id, "", { layer: choice.layer }),
                glyph: Theme.glyph.layer,
                hue: "action",
                stays: true,
                run: () => KeyboardLayout.lock(choice.index)
            })));
    }

    // A keyboard's lights, as a setting with its choices a step in, or
    // nothing for one without them: the laptop's own, saying how lit they
    // are, then how long they stay on once nothing's touched, and a ZSA
    // board's, which its layout's command steps (KeyboardLayout.qml).
    function lights(device, key, holder) {
        if (holder && device.phys === holder.phys && device.name === holder.name)
            return laptopLights(key);
        if (Keyboards.zsa(device) && KeyboardLayout.present)
            return [boardLights(key)];
        return [];
    }

    // The laptop keyboard's lights, each level a step in, or nothing
    // without them. Its levels are set through logind, by brightnessctl, as
    // the screen's are, since the light is root's.
    function laptopLights(key) {
        const light = Lights.backlight;
        if (light === "")
            return [];
        const now = Number(said(`${light}/brightness`));
        const most = Number(said(`${light}/max_brightness`));
        if (!(most > 0))
            return [];
        const device = light.split("/").pop();
        const levels = [];
        for (let level = 0; level <= most; level++) {
            const state = level === 0 ? "off" : "";
            const set = level;
            levels.push(thing({
                key: `verb:keyboards-lights-${level}`,
                verb: Words.verb("keyboards:lights:to", state),
                name: part("lights:to", state, { n: level, most: most }),
                glyph: Theme.glyph.brightness,
                hue: "action",
                side: level === now ? "in use" : "",
                stays: true,
                run: () => Niri.spawn(["brightnessctl", `--device=${device}`, "set", String(set)])
            }));
        }
        const rows = [holderRow({
            key: key("lights"),
            name: part("lights"),
            // Keyed by its keyboard, it carries the names its kind had.
            once: Words.of("keyboards:lights").once ?? [],
            side: Keyboards.lit(now, most),
            graph: bar(now, most),
            glyph: Theme.glyph.brightness,
            offers: true
        }, () => levels)];
        const timeout = said(`${light}/stop_timeout`);
        if (timeout !== "")
            rows.push(figure(key("timeout"), "timeout", Keyboards.timeout(timeout),
                Keyboards.wakers(said(`${light}/start_triggers`))));
        return rows;
    }

    // A ZSA board's lights: it can be told to light them, put them out or
    // step them, but never asked how they are, so the setting says nothing
    // of how they're set. The steps last until it restarts
    // (modules/ergodox/keymap/host.c).
    function boardLights(key) {
        const step = (id, state, learned, run) => provider.thing({
            key: learned,
            verb: Words.verb(id, state),
            name: Words.name(id, state),
            glyph: Theme.glyph.brightness,
            hue: "action",
            stays: true,
            run: run
        });
        return holderRow({
            key: key("lights"),
            name: part("lights"),
            once: Words.of("keyboards:lights").once ?? [],
            glyph: Theme.glyph.brightness,
            offers: true,
            note: "the board can't say how lit they are"
        }, () => [
            step("keyboards:lights:to", "off", "keyboard:lights-off",
                () => KeyboardLayout.lights(false)),
            step("keyboards:lights:on", "", "keyboard:lights-on",
                () => KeyboardLayout.lights(true)),
            step("keyboards:lights:brighter", "", "keyboard:brighter",
                () => KeyboardLayout.brighter(true)),
            step("keyboards:lights:dimmer", "", "keyboard:dimmer",
                () => KeyboardLayout.brighter(false))
        ]);
    }

    // The keyboards' lights, asked alone: one keyboard's choices, and how
    // long the laptop's stay on, as the answer's rows; of several, each
    // keyboard by name, saying how lit its lights are, holding the same.
    // Nothing where no keyboard has lights.
    function lightsTold() {
        const found = Keyboards.keyboards(read(devices));
        const holder = Keyboards.lightHolder(found);
        const lit = found.map(device => {
            const key = id => `keyboards:${device.phys || device.name}:${id}`;
            return { device: device, rows: lights(device, key, holder) };
        }).filter(each => each.rows.length > 0);
        if (lit.length === 0)
            return null;
        const inside = each => each.rows[0].verbs().concat(each.rows.slice(1));
        const first = lit[0].rows[0];
        if (lit.length === 1)
            return {
                headline: first.side !== "" ? `the keyboard's light is ${first.side}`
                    : "the keyboard's light",
                graph: first.graph,
                glyph: Theme.glyph.brightness,
                rows: inside(lit[0]),
                note: first.note,
                live: provider
            };
        return {
            headline: `${lit.length} keyboards' lights`,
            glyph: Theme.glyph.brightness,
            rows: lit.map(each => thingRow({
                key: `keyboards:${each.device.phys || each.device.name}:lit`,
                name: named(each.device),
                side: each.rows[0].side,
                graph: each.rows[0].graph,
                glyph: Theme.glyph.keyboard,
                note: each.rows[0].note
            }, () => inside(each))),
            note: "",
            live: provider
        };
    }

    // While its answer is shown, caps lock, the layer and the laptop's
    // lights changing ask it again, so the headline says how they are now;
    // they're heard rather than read on a clock.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider

    Connections {
        target: Lights

        function onCapsChanged() {
            if (provider.shown)
                provider.answered();
        }

        // The laptop's light key, heard as it's pressed, moves its lights.
        function onTurned() {
            if (provider.shown)
                provider.answered();
        }
    }

    Connections {
        target: KeyboardLayer

        function onLineChanged() {
            if (provider.shown)
                provider.answered();
        }
    }

    FileView {
        id: devices
        path: "/proc/bus/input/devices"
        blockLoading: true
        printErrors: false
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}

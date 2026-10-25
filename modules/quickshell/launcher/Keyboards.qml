// Which keyboards are plugged in, answered in the field: "keyboards",
// "keyboard state", "connected keyboards" or "caps lock". The headline
// says how many and whether caps lock is on; the rows are each keyboard by
// name and how it's joined (built in, by USB or by Bluetooth), and each one
// holds its parts, a step in by Return or the right arrow, named first as
// every part is: its Caps lock, Num lock and Scroll lock, as the lights it
// has say; the Layer a board that has them is on (KeyboardLayer.qml); how a
// held key Repeats, which modules/keyboard/repeat.nix sets for every
// keyboard; for the laptop's own, its Lights, a setting whose levels are a
// step in, and how long they stay on once nothing's touched; then its
// Connection, a USB board's Maker and Model, and what desk-facts says of it
// (its Polling rate, Keys and Firmware). The ErgoDox's own lights,
// which it can be told to change but never asked, are the Keyboard
// collection's (Keyboard.qml). Every name is the vocabulary's (Words.qml).
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

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.started("keyboards")

    function answer(typed) {
        if (asks(typed) === "")
            return null;
        return told();
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
        const rows = found.map(device => holderRow({
            key: `keyboards:${device.phys || device.name}`,
            name: named(device),
            side: Keyboards.joined(device),
            glyph: Theme.glyph.keyboard
        }, () => provider.parts(device)));
        const count = found.length === 1 ? "one keyboard" : `${found.length} keyboards`;
        return {
            headline: `${count}, caps lock ${caps ? "on" : "off"}`,
            glyph: Theme.glyph.keyboard,
            rows: rows,
            note: "from the kernel's input devices",
            live: provider
        };
    }

    // A keyboard's parts, read as it's stepped into: its locks, its layer,
    // how a held key repeats, its lights, and how it's connected and who
    // made it.
    function parts(device) {
        const key = id => `keyboards:${device.phys || device.name}:${id}`;
        const rows = Keyboards.locks(device).map(lock => figure(key(lock.id), lock.id,
            Number(said(`${Lights.leds}/${lock.light}/brightness`)) > 0 ? "on" : "off"));
        if (Keyboards.zsa(device) && KeyboardLayer.present)
            rows.push(figure(key("layer"), "layer",
                KeyboardLayer.locked ? `${KeyboardLayer.name}, locked` : KeyboardLayer.name));
        const repeat = settings.repeat ?? null;
        if (repeat)
            rows.push(figure(key("repeat"), "repeat", Keyboards.repeats(repeat)));
        if (Keyboards.builtIn(device))
            for (const row of lights(key))
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
            ["polling rate", "keys"]));
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

    // The laptop keyboard's lights, as a setting saying how lit they are
    // with each level a step in, and how long they stay on once nothing's
    // touched, or nothing without them. Its levels are set through logind,
    // by brightnessctl, as the screen's are, since the light is root's.
    function lights(key) {
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
                run: () => Niri.spawn(["brightnessctl", `--device=${device}`, "set", String(set)])
            }));
        }
        const rows = [holderRow({
            key: key("lights"),
            name: part("lights"),
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

    // While its answer is shown, caps lock and the layer changing ask it
    // again, so the headline says how they are now; they're heard rather
    // than read on a clock.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider

    Connections {
        target: Lights

        function onCapsChanged() {
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

// Which keyboards are plugged in, answered in the field: "keyboards",
// "keyboard state", "connected keyboards" or "caps lock". The headline
// says how many and whether caps lock is on; the rows are each keyboard by
// name and how it's joined (built in, by USB or by Bluetooth), then the
// layer a keyboard that has them is on (KeyboardLayer.qml), and how a held
// key repeats, which modules/keyboard/repeat.nix sets for every keyboard. A keyboard
// is a device the kernel gives both keys and lights, which leaves out the
// power button, the lid and the hotkeys, and keyd's own keyboard, which
// only passes on the others' keys, is left out too. Read from the kernel's
// list of input devices when asked, and caps lock from the shell's own
// Lights, so nothing is spawned and nothing more is watched.
import qs
import Quickshell.Io
import QtQuick

Provider {
    id: provider

    examples: ["keyboards"]

    function answer(typed) {
        const text = typed.trim().toLowerCase();
        if (!/^(keyboards|keyboard state|connected keyboards|which keyboards?|caps ?lock)$/.test(text))
            return null;
        return told();
    }

    // Each keyboard in the kernel's list: its name and where it sits.
    function keyboards() {
        return read(devices).split(/\n\s*\n/).map(block => {
            const field = key => (new RegExp(`^${key}: (.*)$`, "m").exec(block) ?? [])[1] ?? "";
            return {
                name: (/Name="(.*)"/.exec(field("N")) ?? [])[1] ?? "",
                phys: (/Phys=(.*)/.exec(field("P")) ?? [])[1] ?? "",
                handlers: (/Handlers=(.*)/.exec(field("H")) ?? [])[1] ?? ""
            };
        }).filter(device => /\bkbd\b/.test(device.handlers) && /\bleds\b/.test(device.handlers)
            && !/^keyd\b/.test(device.name));
    }

    // How a keyboard is joined, from where the kernel says it sits.
    function joined(device) {
        if (/^usb-/.test(device.phys))
            return "by USB";
        if (/^([0-9a-f]{2}:){5}[0-9a-f]{2}$/i.test(device.phys))
            return "by Bluetooth";
        if (/^isa|serio|i8042/.test(device.phys))
            return "built in";
        return "";
    }

    // The laptop's own reads as what it is rather than as its chip.
    function named(device) {
        return device.name === "AT Translated Set 2 keyboard" ? "The laptop's keyboard" : device.name;
    }

    function told() {
        const glyph = String.fromCodePoint(0xf030c);
        const found = keyboards();
        const caps = Lights.caps;
        const rows = found.map(device => copyRow({ name: named(device), side: joined(device), glyph: glyph }));
        rows.push(copyRow({
            name: caps ? "on" : "off",
            side: "caps lock",
            glyph: Theme.glyph.capitals
        }));
        if (KeyboardLayer.present)
            rows.push(copyRow({
                name: KeyboardLayer.locked ? `${KeyboardLayer.name}, locked` : KeyboardLayer.name,
                side: "layer",
                glyph: glyph
            }));
        const repeat = settings.repeat ?? null;
        if (repeat)
            rows.push(copyRow({
                name: `after ${repeat.delay} ms, ${repeat.rate} a second`,
                side: "a held key repeats",
                glyph: glyph
            }));
        const count = found.length === 1 ? "one keyboard" : `${found.length} keyboards`;
        return {
            headline: `${count}, caps lock ${caps ? "on" : "off"}`,
            rows: rows,
            note: "from the kernel's input devices"
        };
    }

    FileView {
        id: devices
        path: "/proc/bus/input/devices"
        blockLoading: true
        printErrors: false
    }

}

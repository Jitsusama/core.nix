// The cases of shortcuts for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Keyboards.js" as Keyboards
.import "../KeyCounts.js" as KeyCounts

function run(failures) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    // The keyboards connected, the laptop's and an ErgoDox's, as the
    // machine daemon lists them from the kernel's list of input devices
    // (modules/machine/src/keyboards.rs, whose tests say which it
    // leaves out).
    const keyboardsFound = [{
        name: "AT Translated Set 2 keyboard",
        phys: "isa0060/serio0/input0",
        sysfs: "/devices/platform/i8042/serio0/input/input0",
        uniq: "",
        handlers: "sysrq kbd event0 leds ",
        bus: "0011",
        vendor: "0001",
        product: "0001",
        leds: 7
    }, {
        name: "ZSA Technology Labs ErgoDox EZ",
        phys: "usb-0000:00:14.0-3.4/input0",
        sysfs: "/devices/pci0000:00/0000:00:14.0/usb3/3-3/3-3.4/3-3.4:1.0/"
            + "0003:3297:4974.0001/input/input5",
        uniq: "",
        handlers: "sysrq kbd leds event4 ",
        bus: "0003",
        vendor: "3297",
        product: "4974",
        leds: 31
    }];
    const [keyboardsLaptop, keyboardsErgodox] = keyboardsFound;
    machine("how each is joined", keyboardsFound.map(device => Keyboards.joined(device)),
        ["built in", "on USB"]);
    machine("on Bluetooth", Keyboards.joined({ phys: "f4:4e:fd:00:11:22" }), "on Bluetooth");
    machine("the built-in one", keyboardsFound.map(device => Keyboards.builtIn(device)),
        [true, false]);
    machine("ZSA's board", keyboardsFound.map(device => Keyboards.zsa(device)), [false, true]);
    machine("the laptop's light on its keyboard", Keyboards.lightHolder(keyboardsFound).name,
        "AT Translated Set 2 keyboard");
    // A laptop whose keyboard isn't on the keyboard port still has its
    // light, on the first keyboard neither paired nor ZSA's.
    const keyboardsInside = { name: "Inside keyboard", phys: "i2c-PNP0C50:00", bus: "0018" };
    machine("the laptop's light with no keyboard port",
        Keyboards.lightHolder([keyboardsErgodox, keyboardsInside]).name, "Inside keyboard");
    machine("no light with only a board", Keyboards.lightHolder([keyboardsErgodox]), null);
    // Super+K shows the layout of a keyboard connected now: the board's
    // while it's connected, the laptop's own with it alone, and the
    // laptop's too for a board whose layout isn't known.
    const boardLayers = [{ name: "Coal" }, { name: "Punk" }];
    const laptopLayers = [{ name: "Colemak" }];
    machine("the laptop's layout with it alone",
        Keyboards.layout([keyboardsLaptop], boardLayers, laptopLayers), laptopLayers);
    machine("the board's layout with both connected",
        Keyboards.layout(keyboardsFound, boardLayers, laptopLayers), boardLayers);
    machine("the laptop's layout for a board without one",
        Keyboards.layout(keyboardsFound, [], laptopLayers), laptopLayers);
    machine("the board's acts only while it's connected", [[keyboardsLaptop], keyboardsFound,
        [keyboardsErgodox], []].map(listed => Keyboards.boarded(listed, boardLayers)),
        [false, true, true, false]);
    machine("the laptop's locks", Keyboards.locks(keyboardsLaptop), [
        { id: "caps", light: "input0::capslock" },
        { id: "num", light: "input0::numlock" },
        { id: "scroll", light: "input0::scrolllock" }
    ]);
    machine("caps lock alone", Keyboards.locks(Object.assign({}, keyboardsLaptop, { leds: 2 }))
        .map(lock => lock.id), ["caps"]);
    machine("no USB folder for the laptop's", Keyboards.usbFolder(keyboardsLaptop), "");
    machine("a USB board's folder", Keyboards.usbFolder(keyboardsErgodox),
        "/sys/devices/pci0000:00/0000:00:14.0/usb3/3-3/3-3.4");
    machine("the laptop's connection", Keyboards.connection(keyboardsLaptop, null), "PS/2");
    machine("a USB board's connection", Keyboards.connection(keyboardsErgodox,
        { version: " 2.00", speed: "12" }), "USB 2.0 at 12 Mb/s");
    machine("a fast USB board's", Keyboards.connection(keyboardsErgodox,
        { version: " 3.20", speed: "5000" }), "USB 3.2 at 5 Gb/s");
    machine("a USB board unread", Keyboards.connection(keyboardsErgodox, null), "USB");
    machine("a Bluetooth board's", Keyboards.connection({ bus: "0005",
        uniq: "f4:4e:fd:00:11:22" }, null), "Bluetooth, F4:4E:FD:00:11:22");
    machine("lights at a level", Keyboards.lit(2, 2), "2 of 2");
    machine("lights off", Keyboards.lit(0, 2), "off");
    machine("a minute's timeout", Keyboards.timeout("1m\n"), "1 minute");
    machine("seconds' timeout", Keyboards.timeout("30s"), "30 seconds");
    machine("no timeout", Keyboards.timeout("0"), "never");
    machine("what lights them again", Keyboards.wakers("+keyboard +touchpad"),
        "lit again by a key or the touchpad");
    machine("nothing lights them again", Keyboards.wakers("-keyboard -touchpad"), "");
    machine("how a held key repeats", Keyboards.repeats({ delay: 200, rate: 40 },
        "after {delay} ms \u00b7 {rate} Hz"), "after 200 ms \u00b7 40 Hz");
    // A board's layers to move to: the one it's locked on let go, then
    // each above the first but that one locked on; held by a key, every
    // layer above the first.
    const keyboardsLayers = ["Coal", "Punk", "Numb"];
    machine("a locked board's layers", Keyboards.layerChoices(keyboardsLayers, "Numb", true), [
        { id: "keyboard:release", key: "keyboard:release", layer: "Numb", index: 0 },
        { id: "keyboard:lock", key: "keyboard:lock:Punk", layer: "Punk", index: 1 }
    ]);
    machine("a held board's layers", Keyboards.layerChoices(keyboardsLayers, "Numb", false)
        .map(choice => `${choice.layer} ${choice.index}`), ["Punk 1", "Numb 2"]);

    // What key-counts tallied of the board, as the machine daemon sums
    // it (modules/machine/src/counts.rs, whose tests sum it from the
    // file): its keys on two layers, some under modifiers.
    const counts = {
        key: "counts:ergodox", kind: "counts", board: "ergodox",
        presses: 20000, holds: 3240,
        most: [{ name: "e", presses: 9100 }, { name: "space", presses: 5000 },
            { name: "shift", presses: 3200 }, { name: "f23", presses: 1000 },
            { name: "a", presses: 900 }],
        layers: [{ name: "coal", presses: 18200 }, { name: "numb", presses: 1800 }],
        modified: 1900,
        modifiers: [{ name: "shift", presses: 1000 }, { name: "ctrl", presses: 900 }],
        began_s: 1793898549
    };
    // The theme's faces, by the key map's spelling, as Shortcuts.qml
    // gives them (modules/theme/keys.nix).
    const keyFaces = { shift: { text: "\u21e7" }, f23: { text: "Copilot", wide: false } };
    const tally = KeyCounts.board(counts, keyFaces);
    machine("a board's presses and holds", [tally.presses, tally.holds], [20000, 3240]);
    machine("the keys pressed most, as a chord draws them", tally.most,
        [{ key: "E", presses: 9100 }, { key: "Space", presses: 5000 },
            { key: "\u21e7", presses: 3200 }, { key: "Copilot", presses: 1000 },
            { key: "A", presses: 900 }]);
    const keyNamed = id => `[${id}]`;
    const keyWords = { since: "since {day}", began: "since counting began" };
    machine("a board's counts as facts", KeyCounts.facts(tally, keyNamed, "since 5 Nov"), [
        { label: "[presses]", value: "20,000 since 5 Nov" },
        { label: "[held]", value: "3,240" },
        { label: "[layers]", value: "coal 91% \u00b7 numb 9%" },
        { label: "[modified]", value: "9.5% \u00b7 shift 5% \u00b7 ctrl 4.5%" }
    ]);
    machine("the keys under their heading", KeyCounts.most(tally, keyNamed).slice(0, 2),
        [{ heading: "[most]" }, { keys: [["E"]], value: "9.1k" }]);
    machine("a board with nothing counted", [KeyCounts.board(null, keyFaces),
        KeyCounts.facts(null, keyNamed, ""), KeyCounts.most(null, keyNamed)], [null, [], []]);
    machine("counts short", [870, 9100, 12400].map(KeyCounts.short), ["870", "9.1k", "12k"]);
    machine("counting began", KeyCounts.since(counts.began_s, keyWords,
        new Date(2026, 10, 7)), "since 5 Nov");
    machine("counting began in another year", KeyCounts.since("1762362549", keyWords,
        new Date(2026, 10, 7)), "since 5 Nov 2025");
    machine("no start written", KeyCounts.since("", keyWords), "since counting began");

    // A keyboard's drawer: where it is, then what moves, what it
    // counted, how it's set, what describes it, its identifiers, and
    // its keys last; what it doesn't say is left out.
    const boardSaid = { usb: "on USB \u00b7 keyboard", builtIn: "built in \u00b7 keyboard",
        caps: "caps", num: "num", scroll: "scroll", none: "none", locked: "locked" };
    const boardDrawer = Keyboards.drawer({
        device: keyboardsErgodox,
        name: "ErgoDox EZ",
        layer: { name: "Numb", locked: true },
        locks: [{ id: "caps", on: false }, { id: "num", on: true }],
        counted: [{ label: "[presses]", value: "20,000 since 5 Nov" }],
        desk: [{ name: "firmware", value: "1.2" }, { name: "polling rate",
            value: "1000 a second" }, { name: "keys", value: "76" }],
        repeat: "after 200 ms, 40 Hz",
        lights: "",
        connection: "USB 2.0 at 12 Mb/s",
        maker: "ZSA Technology Labs",
        model: "ErgoDox EZ",
        usbId: "3297:4976",
        serial: "",
        most: [{ heading: "[most]" }]
    }, keyNamed, boardSaid);
    machine("a board's eyebrow", boardDrawer.eyebrow, "on USB \u00b7 keyboard");
    machine("a board's facts in order", boardDrawer.facts.map(fact => fact.label
        ?? fact.heading), ["[layer]", "[locks]", "[presses]", "[polling-rate]",
        "[repeat]", "[connection]", "[maker]", "[model]", "[keys]", "[firmware]",
        "[usb-id]", "[most]"]);
    machine("a layer locked", boardDrawer.facts[0].value, "Numb \u00b7 locked");
    machine("only the locks on", boardDrawer.facts[1].value, "num");
    machine("its polling rate in Hz", boardDrawer.facts[3].value, "1,000 Hz");
    machine("its USB id in the Name face", boardDrawer.facts[10].identifies, true);
    const bareBoard = Keyboards.drawer({ device: keyboardsErgodox, name: "", layer: null,
        locks: [{ id: "caps", on: false }], connection: "USB" }, keyNamed, boardSaid);
    machine("no lock on, and a bus the eyebrow says", bareBoard.facts.map(fact =>
        `${fact.label} ${fact.value}`), ["[locks] none"]);
    machine("the laptop's eyebrow", Keyboards.drawer({ device: keyboardsLaptop, name: "",
        layer: null }, keyNamed, boardSaid).eyebrow, "built in \u00b7 keyboard");
    return asked;
}

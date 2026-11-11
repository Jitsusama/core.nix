// How a thing's drawer says its parts, and what a screen's says of what
// niri gives, run in Qt's own JavaScript engine, the one the shell runs it
// in. The `a-things-drawer-says-its-parts` check copies
// modules/quickshell/launcher/Plate.js, Screens.js, Backlight.js,
// Numbers.js and Order.js beside this file and runs `qml -platform offscreen plate.qml`.
// A part's name is a fact's label and what it says the value, a level's
// bar a stroke after it; an act or a choice isn't said; a holder of
// figures is opened in place; and a screen's settings offer its rates
// with every digit niri has, its sizes and its turns. The machine
// daemon's backlight and light sensor records are read as Backlight.js
// reads them: the panel's steps and the lux the sensor hears, the room's
// light measured to three digits. A screen's drawer is the windows
// daemon's (modules/windows), so it's tested there.
import QtQuick
import "Backlight.js" as Backlight
import "Plate.js" as Plate
import "Screens.js" as Screens

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function said(facts) {
        return facts.map(fact => `${fact.label}=${fact.value}`).join("; ");
    }

    // The screen's words as the vocabulary gives them, each its own key.
    readonly property var words: ({
        named: id => id,
        said: (id, key) => ({ "preferred-mode": "preferred {mode}", at: "{size} at {rate}" })[key]
            ?? key,
        turned: transform => transform === "Normal" ? "upright" : transform
    })

    // A screen's white point and primaries, from desk-facts' figures for an
    // HP Z27k G3: the white a measure, three figures by Joel's settings,
    // each point to three places; nothing for figures that aren't whole.
    function colours(check) {
        const words = {
            said: (id, key) => ({ point: "x {x} y {y}", red: "red x {x} y {y}",
                green: "green x {x} y {y}", blue: "blue x {x} y {y}" })[key]
        };
        const z27k = "0.6396484375 0.330078125 0.2998046875 0.599609375 0.150390625 "
            + "0.0595703125";
        const z27kWhite = "6462 0.3134765625 0.3291015625";
        const white = Screens.whitePoint(z27kWhite, words, {});
        check("a white is its temperature, then its point",
            white === "6,460 K \u00b7 x 0.313 y 0.329", white);
        const pointed = Screens.whitePoint(z27kWhite, words, { group: ".", point: "," });
        check("a white is grouped and pointed as Joel says",
            pointed === "6.460 K \u00b7 x 0,313 y 0,329", pointed);
        const primaries = Screens.primaries(z27k, words, {});
        check("primaries name each colour and keep three places",
            primaries === "red x 0.640 y 0.330 \u00b7 green x 0.300 y 0.600 \u00b7 "
                + "blue x 0.150 y 0.060", primaries);
        const broken = [Screens.whitePoint("6462 0.3135", words, {}),
            Screens.whitePoint("", words, {}), Screens.primaries("0.64 0.33 x", words, {})];
        check("figures that aren't whole say nothing", broken.join("") === "", broken);
    }

    // The machine daemon's records of the panel's backlight and the room's
    // sensor (modules/machine's light.rs), as a step into a screen reads
    // them: what the kernel reads back only where it differs, and the room's
    // light measured to three digits.
    function lights(check) {
        const readings = [0, 5.2, 28.47, 999.4, 999.6, 1235, 40000]
            .map(each => Backlight.reading(each)).join("; ");
        check("the room's light measured to three digits, zeros kept", readings
            === "0 lx; 5.20 lx; 28.5 lx; 999 lx; 1.00 klx; 1.24 klx; 40.0 klx", readings);
        const panel = { key: "backlight:intel_backlight", kind: "backlight",
            device: "intel_backlight", brightness: 57_600, actual_brightness: 57_600,
            max_brightness: 96_000, type: "raw", scale: "non-linear", connector: "eDP-1" };
        const sensor = { key: "light:iio:device0", kind: "light", device: "iio:device0",
            name: "als", raw: 1234, scale: 0.1, offset: 0, lux: 123.4 };
        const lit = Backlight.lit([sensor, panel]);
        check("a backlight is its steps, its share and the screen it lights",
            lit.now === 57_600 && lit.most === 96_000 && lit.share === 60 && lit.reads === null
                && lit.connector === "eDP-1" && lit.device === "intel_backlight",
            JSON.stringify(lit));
        const moved = Backlight.lit([Object.assign({}, panel,
            { actual_brightness: 1200, scale: null, type: "platform" })]);
        check("steps read back otherwise are held, with what steps and drives it",
            lit.stepping === "non-linear" && lit.driven === "raw" && moved.reads === 1200
                && moved.stepping === null && moved.driven === "platform",
            JSON.stringify(moved));
        check("no backlight that says its most is none",
            Backlight.lit([Object.assign({}, panel, { max_brightness: 0 }), sensor]) === null
                && Backlight.lit([]) === null,
            JSON.stringify(Backlight.lit([Object.assign({}, panel, { max_brightness: 0 })])));
        const room = Backlight.sensor([panel, sensor]);
        check("a sensor is the lux it hears, its count and its device",
            room.lux === 123.4 && room.raw === 1234 && room.perCount === 0.1
                && room.offset === 0 && room.device === "iio:device0",
            JSON.stringify(room));
        const unread = Backlight.sensor([Object.assign({}, sensor, { raw: null, lux: null })]);
        check("a sensor not yet read holds no lux and no count",
            unread.lux === null && unread.raw === null && unread.perCount === 0.1,
            JSON.stringify(unread));
        check("no sensor is none", Backlight.sensor([panel]) === null,
            JSON.stringify(Backlight.sensor([panel])));
    }

    // The laptop's panel as niri says it: two modes, both preferred.
    function panel() {
        return {
            name: "eDP-1",
            make: "LG Display",
            model: "0x07C6",
            serial: null,
            physical_size: [300, 190],
            modes: [
                { width: 2880, height: 1800, refresh_rate: 120001, is_preferred: true },
                { width: 2880, height: 1800, refresh_rate: 60000, is_preferred: true }
            ],
            current_mode: 0,
            is_custom_mode: false,
            vrr_supported: true,
            vrr_enabled: false,
            logical: { x: 560, y: 1440, width: 1440, height: 900, scale: 2, transform: "Normal" }
        };
    }

    // A screen's settings' choices, each said by its value alone: Refresh's
    // every rate at the size it runs, with every digit, Resolution's each
    // size once, the mode it prefers and a custom one noted so; and
    // Rotation's eight turns, the mirrored after the four (VKCF3Z,
    // 2026-11-09).
    function settings(check) {
        const said = choices => choices.map(each => `${each.value}`
            + `${each.note === "" ? "" : ` (${each.note})`}${each.chosen ? "*" : ""}`).join(", ");
        const monitor = panel();
        monitor.modes = [
            { width: 2560, height: 1440, refresh_rate: 59951, is_preferred: true },
            { width: 2560, height: 1440, refresh_rate: 60000, is_preferred: false },
            { width: 1920, height: 1080, refresh_rate: 60000, is_preferred: false },
            { width: 1920, height: 1080, refresh_rate: 59940, is_preferred: false }
        ];
        monitor.current_mode = 1;
        check("Refresh offers each rate at its size, every digit, the preferred noted",
            said(Screens.refreshes(monitor, words)) === "59.951 Hz (preferred), 60 Hz*",
            said(Screens.refreshes(monitor, words)));
        check("Resolution offers each size once",
            said(Screens.sizes(monitor, words)) === "2560 \u00d7 1440 (preferred)*, "
            + "1920 \u00d7 1080", said(Screens.sizes(monitor, words)));
        monitor.current_mode = 3;
        monitor.is_custom_mode = true;
        check("a custom mode is noted where it's chosen",
            said(Screens.refreshes(monitor, words)) === "60 Hz, 59.94 Hz (custom)*"
            && said(Screens.sizes(monitor, words)) === "2560 \u00d7 1440 (preferred), "
            + "1920 \u00d7 1080 (custom)*", `${said(Screens.refreshes(monitor, words))}; `
            + said(Screens.sizes(monitor, words)));
        monitor.current_mode = null;
        check("a screen running no mode offers no rate",
            Screens.refreshes(monitor, words).length === 0
            && !Screens.sizes(monitor, words).some(each => each.chosen), "it did");
        check("Rotation offers the four turns, then the four mirrored",
            Screens.transforms().join(" ") === "Normal 90 180 270 "
            + "Flipped Flipped90 Flipped180 Flipped270", Screens.transforms().join(" "));
    }

    // A screen's settings typed on the : line (VKCF3Z, 2026-11-09): each
    // completing from its own choices, their words beside, the turns in
    // the vocabulary's typed words, a whole rate taken where one mode
    // rounds to it, and a value it doesn't offer saying what it does.
    function onTheLine(check) {
        const typed = { Normal: "upright", 90: "left", 180: "upside-down", 270: "right",
            Flipped: "mirrored", Flipped90: "mirrored-left",
            Flipped180: "mirrored-upside-down", Flipped270: "mirrored-right" };
        const lineWords = {
            named: id => id,
            said: (id, key) => key.startsWith("typed-") ? typed[key.slice("typed-".length)]
                : ({ "not-offered": "{value} not set \u00b7 {screen} offers {offers}",
                    "none-given": "no {setting} given \u00b7 {screen} offers {offers}",
                    "offers-range": "{first} to {last}" })[key] ?? key,
            turned: transform => transform === "Normal" ? "upright" : transform
        };
        const monitor = panel();
        monitor.modes = [
            { width: 2560, height: 1440, refresh_rate: 59951, is_preferred: true },
            { width: 2560, height: 1440, refresh_rate: 75000, is_preferred: false },
            { width: 1920, height: 1080, refresh_rate: 60000, is_preferred: false }
        ];
        monitor.current_mode = 0;
        monitor.logical.transform = "Flipped90";
        const values = setting => Screens.lineValues(monitor, setting, lineWords)
            .map(each => `${each.value} (${each.note})`).join(", ");
        const set = (setting, given) => JSON.stringify(Screens.lineSet(monitor,
            { setting: setting, typed: given }, lineWords, "HP"));
        check("the settings a running mode has",
            Screens.lineSettings(monitor).join(" ") === "scale refresh resolution rotation",
            Screens.lineSettings(monitor).join(" "));
        check("scale completes as typed, its multiple beside", values("scale").startsWith(
            "1 (1\u00d7), 1.25 (1.25\u00d7), 1.5 (1.5\u00d7)"), values("scale"));
        check("refresh completes every digit, its words beside",
            values("refresh") === "59.951 (59.951 Hz \u00b7 preferred), 75 (75 Hz)",
            values("refresh"));
        check("resolution completes by x", values("resolution")
            === "2560x1440 (2560 \u00d7 1440 \u00b7 preferred), 1920x1080 (1920 \u00d7 1080)",
            values("resolution"));
        check("rotation completes in its typed words", values("rotation").startsWith(
            "upright (upright), left (90)"), values("rotation"));
        const now = ["scale", "refresh", "resolution", "rotation"]
            .map(setting => Screens.lineNow(monitor, setting, lineWords)).join(" ");
        check("each says where it stands as it's typed",
            now === "2 59.951 2560x1440 mirrored-left", now);
        check("a scale is taken with its unit or without",
            set("scale", "1.5") === set("scale", "1.5\u00d7")
            && set("scale", "1.5") === '{"asks":{"Scale":{"scale":{"Specific":1.5}}}}',
            set("scale", "1.5"));
        check("a whole rate one mode rounds to sets it", set("refresh", "60")
            === '{"asks":{"Mode":{"mode":{"Specific":{"width":2560,"height":1440,'
            + '"refresh":59.951}}}}}', set("refresh", "60"));
        check("a size is taken by x or \u00d7, niri choosing its rate",
            set("resolution", "1920\u00d71080") === '{"asks":{"Mode":{"mode":{"Specific":'
            + '{"width":1920,"height":1080,"refresh":null}}}}}', set("resolution", "1920x1080"));
        check("a turn is taken by its typed word, in any case", set("rotation", "Mirrored-Left")
            === '{"asks":{"Transform":{"transform":"Flipped90"}}}', set("rotation", "Left"));
        check("a scale it doesn't offer says what it does", set("scale", "1.3")
            === '{"refused":"1.3\u00d7 not set \u00b7 HP offers 1\u00d7 to 3\u00d7"}',
            set("scale", "1.3"));
        check("a rate no one mode rounds to is refused, in hertz", set("refresh", "120")
            === '{"refused":"120 Hz not set \u00b7 HP offers 59.951 Hz to 75 Hz"}',
            set("refresh", "120"));
        check("a turn it doesn't know says each", set("rotation", "sideways").includes(
            "sideways not set \u00b7 HP offers upright, left, upside-down, right, mirrored, "),
            set("rotation", "sideways"));
        check("nothing after = says none was given", set("scale", " ")
            === '{"refused":"no scale given \u00b7 HP offers 1\u00d7 to 3\u00d7"}',
            set("scale", " "));
        // niri lists the preferred mode first, so the ends are found by
        // size, never by place, and two modes alike complete once.
        monitor.modes = [
            { width: 1920, height: 1080, refresh_rate: 60000, is_preferred: true },
            { width: 1920, height: 1080, refresh_rate: 144000, is_preferred: false },
            { width: 1920, height: 1080, refresh_rate: 144000, is_preferred: false },
            { width: 1920, height: 1080, refresh_rate: 59940, is_preferred: false },
            { width: 2560, height: 1440, refresh_rate: 60000, is_preferred: false },
            { width: 1280, height: 720, refresh_rate: 60000, is_preferred: false }
        ];
        check("a rate's ends are its least and its most", set("refresh", "100")
            === '{"refused":"100 Hz not set \u00b7 HP offers 59.94 Hz to 144 Hz"}',
            set("refresh", "100"));
        check("a size's ends are its least and its most", set("resolution", "800x600")
            === '{"refused":"800 \u00d7 600 not set \u00b7 HP offers 1280 \u00d7 720 to 2560 '
            + '\u00d7 1440"}', set("resolution", "800x600"));
        check("two modes alike complete once", Screens.lineValues(monitor, "refresh", lineWords)
            .map(each => each.value).join(" ") === "60 144 59.94", values("refresh"));
        monitor.current_mode = null;
        check("a screen running no mode takes only its scale and turn",
            Screens.lineSettings(monitor).join(" ") === "scale rotation",
            Screens.lineSettings(monitor).join(" "));
    }

    function run() {
        const failures = [];
        const check = (what, holds, got) => {
            if (!holds)
                failures.push(`${what}: ${got}`);
        };

        const parts = [
            { key: "verb:use", name: "Use It", side: "" },
            { key: "volume", name: "Volume", side: "62%", note: "-12.4 dB",
                graph: { shape: "bar", values: [0.62], warn: false } },
            { key: "level", name: "Level", side: "silent",
                graph: { shape: "spark", values: [0, 0.2] } },
            { key: "hardware", name: "Hardware", side: "CS35L57" },
            { key: "channels", name: "Channels", side: "stereo" },
            { key: "empty", name: "Port", side: "" },
            { key: "reading", name: "Clock", side: "\u2026" }
        ];
        const inside = [{ key: "hardware:chip", name: "Chip", side: "CS35L57" },
            { key: "verb:to-50", name: "Volume to 50%", side: "" },
            { key: "hardware:card", name: "Card", side: "MV7" }];
        const facts = Plate.facts(parts, row => row.key === "volume"
            ? { value: Plate.runs([row.side, row.note]) }
            : row.key === "hardware" ? { facts: inside }
            : row.key === "channels" ? { items: [{ name: "front left", side: "62%" },
                { name: "front right", side: "60%" }] }
            : null);
        check("acts and empty parts aren't said, a holder opens in place",
            said(facts) === "Volume=62% \u00b7 -12.4 dB; Level=silent; Chip=CS35L57; Card=MV7; "
            + "Channels=stereo \u00b7 front left 62% \u00b7 front right 60%; Clock=null",
            said(facts));
        check("a bar is a stroke", facts[0].figure?.shape === "stroke"
            && facts[0].figure.values[0] === 0.62, JSON.stringify(facts[0].figure));
        check("a spark stays one", facts[1].figure?.shape === "spark",
            JSON.stringify(facts[1].figure));
        check("a part only being read holds its place", facts[5].value === null,
            String(facts[5].value));
        check("a part can be left out", Plate.facts(parts, row => row.key === "level"
            ? { leave: true } : null).every(fact => fact.label !== "Level"), "it wasn't");
        check("a run leaves out what's empty", Plate.runs(["a", "", null, "b"]) === "a \u00b7 b",
            Plate.runs(["a", "", null, "b"]));
        const named = Plate.identified([{ label: "Serial", value: "0x1234" },
            { label: "Screen Mode", value: "preferred" }, { label: "Serial", value: "CN41" },
            { label: "Size", value: "300 \u00d7 190 mm" }], ["Serial"]);
        check("what identifies it ends the drawer, marked, and said once",
            said(named) === "Screen Mode=preferred; Size=300 \u00d7 190 mm; Serial=0x1234"
            && named.map(fact => fact.identifies === true).join() === "false,false,true",
            said(named));

        check("a rate keeps every digit niri has", [120001, 60000, 59940, 143999]
            .map(Screens.hertz).join(", ") === "120.001 Hz, 60 Hz, 59.94 Hz, 143.999 Hz",
            [120001, 60000, 59940, 143999].map(Screens.hertz).join(", "));
        check("a scale is a multiple", [2, 1.25, 1.5, 1.75].map(Screens.scale).join(" ")
            === "2\u00d7 1.25\u00d7 1.5\u00d7 1.75\u00d7",
            [2, 1.25, 1.5, 1.75].map(Screens.scale).join(" "));
        colours(check);
        lights(check);
        settings(check);
        onTheLine(check);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}

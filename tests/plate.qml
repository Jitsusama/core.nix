// How a thing's drawer says its parts, and what a screen's says of what
// niri gives, run in Qt's own JavaScript engine, the one the shell runs it
// in. The `a-things-drawer-says-its-parts` check copies
// modules/quickshell/launcher/Plate.js, Screens.js, Backlight.js,
// Numbers.js and Order.js beside this file and runs `qml -platform offscreen plate.qml`.
// A part's name is a fact's label and what it says the value, a level's
// bar a stroke after it; an act or a choice isn't said; a holder of
// figures is opened in place; and a screen says the mode it runs with
// every digit niri has, where it sits, what it offers and how fine it is,
// an off one only the last two. The backlight daemon's files are read as
// Backlight.js reads them: what sets the level, the room or why nothing
// does, and its curve, dimmest first;
// the room's light measured to three digits, the curve's lux every digit.
// The machine daemon's backlight and light sensor records are read as
// Backlight.js reads them too: the panel's steps, what the kernel says
// of them and what the sensor counts, each only where it's said.
import QtQuick
import "Backlight.js" as Backlight
import "Numbers.js" as Numbers
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

    // The backlight's words as the vocabulary gives them, filled.
    function backlightSaid(key, values) {
        return Backlight.fill(({ "the-room": "the room", "on-its-way": "on its way to {share}",
            "no-light": "no light heard", at: "{share} at {lux}" })[key] ?? key, values);
    }

    // What the machine daemon says of the level the backlight program
    // settled (modules/machine's steered.rs, which reads its files and
    // whose own tests hold how they're parsed): following the room, there,
    // resting a gap too slight to move, or still stepping to what it asks;
    // the lid shut, where the room is still said; no light heard, where
    // the Room Light says so; and nothing, for no record or one with no
    // level. Then the curve, as the daemon orders it.
    function backlight(check) {
        const lx = each => `${each} lx`;
        const said = (level, lux, asks, following, curve) => Backlight.settled([{
            key: "brightness:built-in", kind: "brightness", level_ratio: level, lux,
            asks_ratio: asks, following, curve: curve ?? []
        }]);
        const there = said(0.38, 28, 0.38, true);
        check("the daemon's level, what the curve asks and whether it follows",
            JSON.stringify(there)
                === JSON.stringify({ level: 0.38, lux: 28, asks: 0.38, following: true }),
            JSON.stringify(there));
        check("following the room, there", Backlight.follows(there, backlightSaid) === "the room"
            && Backlight.room(there, 28, backlightSaid, lx) === "28 lx",
            Backlight.follows(there, backlightSaid));
        const stepping = said(0.6, 28, 0.95, true);
        check("following the room, on its way",
            Backlight.follows(stepping, backlightSaid) === "the room \u00b7 on its way to 95%",
            Backlight.follows(stepping, backlightSaid));
        // The panel's steps put the level a point or two from what's asked,
        // under the daemon's 0.03, where it rests.
        const resting = said(0.38, 28, 0.4, true);
        check("a gap too slight to move is there",
            Backlight.follows(resting, backlightSaid) === "the room",
            Backlight.follows(resting, backlightSaid));
        const shut = said(0.6, 28, 0.95, false);
        check("the lid shut, the room still said",
            Backlight.follows(shut, backlightSaid) === ""
            && Backlight.room(shut, 30, backlightSaid, lx) === "30 lx"
            && Backlight.room(shut, null, backlightSaid, lx) === "28 lx",
            Backlight.room(shut, null, backlightSaid, lx));
        const unheard = said(0.6, null, null, false);
        check("no light heard, said where nothing hears the room", unheard?.asks === null
            && unheard?.lux === null
            && Backlight.follows(unheard, backlightSaid) === ""
            && Backlight.room(unheard, null, backlightSaid, lx) === "no light heard"
            && Backlight.room(unheard, 0, backlightSaid, lx) === "0 lx",
            JSON.stringify(unheard));
        const curveOnly = said(null, null, null, null, [{ lux: 6, level_ratio: 0.377 }]);
        check("no record, or one with no level settled, says nothing",
            Backlight.settled([]) === null && curveOnly === null
            && Backlight.follows(null, backlightSaid) === ""
            && Backlight.room(null, 28, backlightSaid, lx) === "28 lx"
            && Backlight.room(null, null, backlightSaid, lx) === "",
            JSON.stringify(curveOnly));
        const points = Backlight.curve([{ kind: "brightness", curve: [
            { lux: 6, level_ratio: 0.377 }, { lux: 28, level_ratio: 0.95 },
            { lux: 10000, level_ratio: 1 }] }]);
        const curve = Backlight.curveSaid(points, backlightSaid, each => Backlight.chosen(each));
        check("the curve, dimmest first",
            curve === "38% at 6 lx \u00b7 95% at 28 lx \u00b7 100% at 10 klx", curve);
        const readings = [0, 5.2, 28.47, 999.4, 999.6, 1235, 40000]
            .map(each => Backlight.reading(each)).join("; ");
        check("the room's light measured to three digits, zeros kept", readings
            === "0 lx; 5.20 lx; 28.5 lx; 999 lx; 1.00 klx; 1.24 klx; 40.0 klx", readings);
        const chosen = [0, 50, 400, 1234, 1500, 2000]
            .map(each => Backlight.chosen(each)).join("; ");
        check("a curve's lux every digit, none added, in a reading's units", chosen
            === "0 lx; 50 lx; 400 lx; 1.234 klx; 1.5 klx; 2 klx", chosen);
        check("no record is no curve", Backlight.curve([]).length === 0,
            JSON.stringify(Backlight.curve([])));
    }

    // The machine daemon's records of the panel's backlight and the room's
    // sensor (modules/machine's light.rs), as its drawer says them: what
    // the kernel reads back only where it differs, its stepping and its
    // driver only where the kernel says them, counts grouped in threes,
    // and an offset of none unsaid.
    function lights(check) {
        const named = id => id;
        const told = { "stepping-linear": "linear", "stepping-non-linear": "non-linear",
            "driven-firmware": "the firmware", "driven-raw": "the graphics chip",
            perCount: "{n} lx" };
        const count = each => Numbers.format(each, undefined, ",", ".");
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
        check("the kernel's say of its steps, read back the same",
            said(Backlight.kernelFacts(lit, named, told, count))
                === "stepping=non-linear; driven-by=the graphics chip",
            said(Backlight.kernelFacts(lit, named, told, count)));
        const moved = Backlight.lit([Object.assign({}, panel,
            { actual_brightness: 1200, scale: null, type: "platform" })]);
        check("steps read back otherwise are said, grouped; a driver with no word isn't",
            said(Backlight.kernelFacts(moved, named, told, count)) === "reads=1,200",
            said(Backlight.kernelFacts(moved, named, told, count)));
        const platform = Object.assign({ "driven-platform": "a platform driver" }, told);
        check("a platform driver is said as what drives it",
            said(Backlight.kernelFacts(moved, named, platform, count))
                === "reads=1,200; driven-by=a platform driver",
            said(Backlight.kernelFacts(moved, named, platform, count)));
        check("no backlight that says its most is none",
            Backlight.lit([Object.assign({}, panel, { max_brightness: 0 }), sensor]) === null
                && Backlight.lit([]) === null,
            JSON.stringify(Backlight.lit([Object.assign({}, panel, { max_brightness: 0 })])));
        const room = Backlight.sensor([panel, sensor]);
        check("a sensor is the lux it hears, its count and its device",
            room.lux === 123.4 && room.device === "iio:device0"
                && said(Backlight.sensorFacts(room, named, told, count))
                === "raw=1,234; per-count=0.1 lx",
            said(Backlight.sensorFacts(room, named, told, count)));
        const offset = Backlight.sensor([Object.assign({}, sensor, { offset: 6 })]);
        check("an offset is said where it isn't none",
            said(Backlight.sensorFacts(offset, named, told, count))
                === "raw=1,234; per-count=0.1 lx; offset=6",
            said(Backlight.sensorFacts(offset, named, told, count)));
        const unread = Backlight.sensor([Object.assign({}, sensor, { raw: null, lux: null })]);
        check("a sensor not yet read says no lux and no count",
            unread.lux === null && said(Backlight.sensorFacts(unread, named, told, count))
                === "per-count=0.1 lx",
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
        check("density takes three figures, its zeros kept",
            Screens.density(2880, 300) === "244 ppi" && Screens.density(1920, 527) === "92.5 ppi"
            && Screens.density(1920, 508) === "96.0 ppi" && Screens.density(1920, 0) === ""
            && Screens.density(1920, 527, ".", ",") === "92,5 ppi",
            `${Screens.density(2880, 300)}, ${Screens.density(1920, 527)}, `
            + `${Screens.density(1920, 508)}, ${Screens.density(1920, 527, ".", ",")}`);
        colours(check);

        const on = panel();
        check("a screen on says what it runs past its row, where it sits and what it offers",
            said(Screens.facts(on, words)) === "mode=preferred; "
            + "refresh=120.001 Hz; "
            + "logical=1440 \u00d7 900; position=560, 1440; rotation=upright; "
            + "modes=2 \u00b7 preferred 2880 \u00d7 1800 at 120.001 Hz; density=244 ppi",
            said(Screens.facts(on, words)));
        on.vrr_enabled = true;
        on.is_custom_mode = true;
        on.serial = "0x1234";
        on.modes[0].is_preferred = false;
        check("variable, custom and a serial say so",
            said(Screens.facts(on, words)).startsWith("mode=custom; "
                + "refresh=120.001 Hz; logical=")
            && said(Screens.facts(on, words)).endsWith("serial=0x1234"),
            said(Screens.facts(on, words)));
        on.modes[0].refresh_rate = 60000;
        on.is_custom_mode = false;
        check("a whole refresh and a mode neither preferred nor custom are the row's",
            said(Screens.facts(on, words)).startsWith("logical="), said(Screens.facts(on, words)));
        on.vrr_supported = false;
        check("variable refresh is its switch's to say, and only its lack the drawer's",
            said(Screens.facts(on, words)).startsWith("variable=unsupported; logical="),
            said(Screens.facts(on, words)));
        const off = panel();
        off.logical = null;
        off.current_mode = null;
        check("a screen off says only what it offers and is",
            said(Screens.facts(off, words)) === "modes=2 \u00b7 preferred 2880 \u00d7 1800 at "
            + "120.001 Hz; density=244 ppi", said(Screens.facts(off, words)));
        check("niri's size, in millimetres", Screens.physical(on) === "300 \u00d7 190 mm"
            && Screens.physical({ physical_size: [0, 0] }) === "", Screens.physical(on));

        backlight(check);
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

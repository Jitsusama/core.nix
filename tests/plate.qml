// How a thing's drawer says its parts, and what a screen's says of what
// niri gives, run in Qt's own JavaScript engine, the one the shell runs it
// in. The `a-things-drawer-says-its-parts` check copies
// modules/quickshell/launcher/Plate.js, Screens.js, Backlight.js,
// Numbers.js and Order.js beside this file and runs `qml -platform offscreen plate.qml`.
// A part's name is a fact's label and what it says the value, a level's
// bar a stroke after it; an act or a choice isn't said; a holder of
// figures is opened in place; and a screen's white point and primaries
// are said from desk-facts' figures. The machine daemon's backlight and
// light sensor records are read as Backlight.js reads them: the panel's
// steps and the lux the sensor hears, the room's light measured to three
// digits. A screen's drawer, its settings and what the : line types for
// them are the windows daemon's (modules/windows: screens.rs,
// screens/settings.rs and screens/line.rs), so they're tested there.
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

        colours(check);
        lights(check);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}

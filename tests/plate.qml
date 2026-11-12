// How a thing's drawer says its parts, and how a screen's light is read,
// run in Qt's own JavaScript engine, the one the shell runs it in. The
// `a-things-drawer-says-its-parts` check copies
// modules/quickshell/launcher/Plate.js, Backlight.js, Numbers.js and
// Order.js beside this file and runs `qml -platform offscreen plate.qml`.
// A part's name is a fact's label and what it says the value, a level's
// bar a stroke after it; an act or a choice isn't said; and a holder of
// figures is opened in place. The machine daemon's backlight and light
// sensor records are read as Backlight.js reads them: the panel's steps
// and the lux the sensor hears, the room's light measured to three
// digits. A screen's drawer, its settings, what its EDID says, its white
// point and primaries among it, and what the : line types for them are
// the windows daemon's (modules/windows: screens.rs and the files beside
// it), so they're tested there.
import QtQuick
import "Backlight.js" as Backlight
import "Plate.js" as Plate

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

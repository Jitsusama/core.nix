// How a thing's drawer says its parts, and what a screen's says of what
// niri gives, run in Qt's own JavaScript engine, the one the shell runs it
// in. The `a-things-drawer-says-its-parts` check copies
// modules/quickshell/launcher/Plate.js and Screens.js beside this file and
// runs `qml -platform offscreen plate.qml`. A part's name is a fact's
// label and what it says the value, a level's bar a stroke after it; an
// act or a choice isn't said; a holder of figures is opened in place; and a
// screen says the mode it runs with every digit niri has, where it sits,
// what it offers and how fine it is, an off one only the last two. The
// backlight daemon's files are read as Backlight.js reads them: what sets
// the level, and what its curve has learned, dimmest first.
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

    // The backlight's words as the vocabulary gives them, filled.
    function backlightSaid(key, fill) {
        const words = ({ "the-room": "the room", "your-pick": "your pick",
            asks: "the room asks {share}", at: "{share} at {lux}" })[key] ?? key;
        return Object.keys(fill).reduce((said, name) => said.replace(`{${name}}`, fill[name]),
            words);
    }

    // What backlight-now says, following the room, then holding a pick;
    // an empty file, as the daemon leaves it when it stops, says nothing;
    // and the learned points, dimmest first, a line it would refuse left out.
    function backlight(check) {
        const following = Backlight.now("level\t0.380\nlux\t28\nasks\t0.380\nfollowing\tyes\n");
        check("the daemon's level, its room and what the curve asks",
            JSON.stringify(following) === JSON.stringify({ level: 0.38, lux: 28, asks: 0.38,
                following: true }), JSON.stringify(following));
        check("following the room", Backlight.follows(following, backlightSaid) === "the room",
            Backlight.follows(following, backlightSaid));
        const picked = Backlight.now("level\t0.600\nlux\t28\nasks\t0.950\nfollowing\tno\n");
        check("a pick holding, and what the room asks",
            Backlight.follows(picked, backlightSaid) === "your pick \u00b7 the room asks 95%",
            Backlight.follows(picked, backlightSaid));
        check("an empty file says nothing", Backlight.now("") === null
            && Backlight.follows(null, backlightSaid) === "", JSON.stringify(Backlight.now("")));
        const points = Backlight.learned("28 0.950\n6 0.377\nnot a point\n10000 1.000\n");
        const learned = Backlight.learnedSaid(points, backlightSaid, lux => `${lux} lx`);
        check("the learned points, dimmest first",
            learned === "38% at 6 lx \u00b7 95% at 28 lx \u00b7 100% at 10000 lx", learned);
        check("nothing learned, nothing said", Backlight.learned("").length === 0, "some");
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

        check("a rate keeps every digit niri has", [120001, 60000, 59940, 143999]
            .map(Screens.hertz).join(", ") === "120.001 Hz, 60 Hz, 59.94 Hz, 143.999 Hz",
            [120001, 60000, 59940, 143999].map(Screens.hertz).join(", "));
        check("a scale is a multiple", [2, 1.25, 1.5, 1.75].map(Screens.scale).join(" ")
            === "2\u00d7 1.25\u00d7 1.5\u00d7 1.75\u00d7",
            [2, 1.25, 1.5, 1.75].map(Screens.scale).join(" "));
        check("density takes three figures", Screens.density(2880, 300) === "244 ppi"
            && Screens.density(1920, 527) === "92.5 ppi" && Screens.density(1920, 0) === "",
            `${Screens.density(2880, 300)}, ${Screens.density(1920, 527)}`);

        const on = panel();
        check("a screen on says what it runs past its row, where it sits and what it offers",
            said(Screens.facts(on, words)) === "mode=preferred; "
            + "refresh=120.001 Hz; variable=supported \u00b7 off; "
            + "logical=1440 \u00d7 900; position=560, 1440; rotation=upright; "
            + "modes=2 \u00b7 preferred 2880 \u00d7 1800 at 120.001 Hz; density=244 ppi",
            said(Screens.facts(on, words)));
        on.vrr_enabled = true;
        on.is_custom_mode = true;
        on.serial = "0x1234";
        on.modes[0].is_preferred = false;
        check("variable, custom and a serial say so",
            said(Screens.facts(on, words)).startsWith("mode=custom; "
                + "refresh=120.001 Hz; variable=supported \u00b7 on;")
            && said(Screens.facts(on, words)).endsWith("serial=0x1234"),
            said(Screens.facts(on, words)));
        on.modes[0].refresh_rate = 60000;
        on.is_custom_mode = false;
        check("a whole refresh and a mode neither preferred nor custom are the row's",
            said(Screens.facts(on, words)).startsWith("variable="), said(Screens.facts(on, words)));
        const off = panel();
        off.logical = null;
        off.current_mode = null;
        check("a screen off says only what it offers and is",
            said(Screens.facts(off, words)) === "modes=2 \u00b7 preferred 2880 \u00d7 1800 at "
            + "120.001 Hz; density=244 ppi", said(Screens.facts(off, words)));
        check("niri's size, in millimetres", Screens.physical(on) === "300 \u00d7 190 mm"
            && Screens.physical({ physical_size: [0, 0] }) === "", Screens.physical(on));

        backlight(check);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}

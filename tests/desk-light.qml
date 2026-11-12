// Drives the desk light (modules/quickshell/DeskLight.qml) and the
// launcher's answers for it (launcher/Light.qml) through Quickshell itself,
// over a stand-in litra that keeps a light's state and says each change on
// its watch, as the real one does. The answer says how the light is lit,
// and its drawer the light's name, its lumens, its node and its serial;
// "desk light 60", "desk light 4000 k", "desk light off" and "warmer" set
// it through litra; its answer holds its Brightness and Warmth, each a
// setting whose choices step it or set a level; the bar is told of a change
// of brightness or warmth, never of a camera lighting it or putting it out;
// and with no light its row stays, for a show to find, saying it isn't plugged in, or
// that litra isn't answering once its watch has ended, with nothing to set.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the first reading"
    property int brightened: 0
    property int warmed: 0

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    // Each case, in order: what it waits for, then what it checks and does.
    function step(name, test, then) {
        root.waiting = name;
        root.next = { test: test, then: then };
        // Each step has its own while to come, so a builder slow over every
        // step before it can't spend this one's.
        patience.restart();
        Qt.callLater(root.heard);
    }

    property var next: null

    function heard() {
        const now = root.next;
        if (now === null || !now.test())
            return;
        root.next = null;
        try {
            now.then();
        } catch (error) {
            root.fail(`${root.waiting}: ${error.message}`);
        }
    }

    // What a light's buttons or a camera do, or unplugging it, told to the
    // stand-in litra, which says it on the watch.
    function says(words) {
        teller.command = ["litra"].concat(words);
        teller.running = true;
    }

    Process {
        id: teller
    }

    QtObject {
        id: theLauncher

        property bool open: false
        // The rows the light said it will become, by key, and as what, as
        // the launcher holds them until the light says so (Launcher.qml's
        // becomesAll).
        property var held: ({})
        property var told: null

        function becomesAll(from, holds, said) {
            const now = Object.assign({}, held);
            for (const one of holds)
                now[one.key] = one.row;
            held = now;
            told = said;
        }
    }

    Launcher.Light {
        id: light

        launcher: theLauncher
        settings: ({ name: "light" })
    }

    Connections {
        target: DeskLight

        function onLightsChanged() {
            Qt.callLater(root.heard);
        }

        function onBrightened() {
            root.brightened++;
        }

        function onWarmed() {
            root.warmed++;
        }

        function onAnsweringChanged() {
            Qt.callLater(root.heard);
        }

        function onTriedChanged() {
            Qt.callLater(root.heard);
        }
    }

    Component.onCompleted: root.step("the first reading", () => DeskLight.light !== null, root.first)

    function lit(percent, kelvin) {
        return DeskLight.light !== null && DeskLight.light.on && DeskLight.light.percent === percent
            && DeskLight.light.kelvin === kelvin;
    }

    function first() {
        const told = light.answer("desk light");
        // Its side says only what its name doesn't (VKCF3Z, 2026-11-12).
        expect("how it's lit", told !== null && told.headline === "40% \u00b7 4,000 K",
            told?.headline);
        expect("its parts", told.rows.map(row => `${row.name} ${row.side}`).join(", ")
            === "Brightness 40%, Warmth 4,000 K, Turn It Off ",
            JSON.stringify(told.rows.map(row => [row.name, row.side])));
        // Its drawer: where it is and what, the light by its own name, then
        // its output in lumens of its range, its HID device and its serial,
        // marked as identifiers, never the warmth the answer says.
        const drawer = told.thing === true ? told.drawer() : null;
        expect("its drawer", drawer !== null && drawer.eyebrow === "on USB \u00b7 desk light"
            && drawer.title === "Litra Glow stand-in"
            && drawer.facts.map(fact => `${fact.label}=${fact.value}`).join("; ")
                === "Output=112 lm \u00b7 of 20 to 250 lm; HID Device=/dev/hidraw1; Serial=stand-in",
            JSON.stringify(drawer));
        expect("what identifies it", drawer?.facts.filter(fact => fact.identifies)
            .map(fact => fact.label).join(", ") === "HID Device, Serial", JSON.stringify(drawer));
        expect("never by the program's name", light.answer("litra") === null,
            light.answer("litra")?.headline);
        expect("nothing told the bar at first", root.brightened === 0 && root.warmed === 0,
            `${root.brightened} ${root.warmed}`);
        const set = light.answer("desk light 60");
        expect("a share set", set.headline === "set the desk light to 60%" && set.verb === "set", set.headline);
        set.run();
        // Told before litra answers, the rows as they'll be: its Brightness
        // at 60% and that level chosen in its step, but nothing it leaves as
        // it was (Joel, 2026-11-07: "it should show right away").
        const becoming = theLauncher.held;
        expect("the rows as they'll be, before litra answers",
            becoming["light:desk:brightness"]?.side === "60%"
                && becoming["light:desk:level:60"]?.side === "chosen"
                && becoming["light:desk:level:40"]?.side === ""
                && !("light:desk:warmth" in becoming) && !("verb:turn" in becoming)
                && theLauncher.told?.name === "Desk Light" && theLauncher.told?.act === "set",
            `${Object.keys(becoming).join(" ")} ${JSON.stringify(theLauncher.told)}`);
        root.step("brighter by the launcher", () => root.lit(60, 4000), root.brighter);
    }

    function brighter() {
        expect("the bar shows a brightness", root.brightened === 1 && root.warmed === 0,
            `${root.brightened} ${root.warmed}`);
        // How each row reads now, for the launcher to tell its hold is
        // done with, shown or not.
        expect("each row as it reads now",
            light.rowNow("light:desk:brightness")?.side === "60%"
                && light.rowNow("light:desk:level:60")?.side === "chosen",
            JSON.stringify([light.rowNow("light:desk:brightness")?.side,
                light.rowNow("light:desk:level:60")?.side]));
        const set = light.answer("desk light 3000 k");
        expect("a warmth set", set.headline === "set the desk light to 3,000 K", set.headline);
        set.run();
        root.step("warmer by the launcher", () => root.lit(60, 3000), root.warmer);
    }

    function warmer() {
        expect("the bar shows a warmth", root.brightened === 1 && root.warmed === 1,
            `${root.brightened} ${root.warmed}`);
        // The light's own button, cooler.
        root.says(["press", "true", "60", "4100"]);
        root.step("its button", () => root.lit(60, 4100), root.button);
    }

    function button() {
        expect("its button shows on the bar", root.warmed === 2, `${root.warmed}`);
        // Its parts: its brightness and its warmth, each a setting with
        // a bar, and putting it out.
        const parts = light.answer("desk light").rows;
        const partNames = parts.map(part => part.name).join(", ");
        expect("the parts", partNames === "Brightness, Warmth, Turn It Off", partNames);
        expect("how each is set", parts[0].side === "60%" && parts[1].side === "4,100 K"
            && parts[0].graph.shape === "bar" && parts[1].graph.shape === "bar",
            JSON.stringify(parts.slice(0, 2).map(part => [part.side, part.graph])));
        expect("each offers its choices beside it", parts[0].offers && parts[1].offers,
            `${parts[0].offers} ${parts[1].offers}`);
        // Their choices: a step either way first, then the levels, the
        // one it's at chosen.
        const brighter = parts[0].verbs();
        const warmer = parts[1].verbs();
        const steps = brighter.slice(0, 2).concat(warmer.slice(0, 2), [parts[2]]);
        const stepNames = steps.map(verb => verb.name).join(", ");
        expect("the steps", stepNames === "Brighter, Dimmer, Warmer, Cooler, Turn It Off",
            stepNames);
        expect("each its one word",
            steps.map(verb => verb.verb).join(" ") === "brighter dimmer warmer cooler off",
            steps.map(verb => verb.verb).join(" "));
        const chosen = brighter.concat(warmer).filter(choice => choice.side === "chosen")
            .map(choice => choice.name);
        // Each said by its value alone, as every choice is (VKCF3Z,
        // 2026-11-09).
        expect("the levels chosen", chosen.join(", ") === "60%", chosen.join(", "));
        expect("a warmth to choose", warmer.some(choice => choice.name === "2,700 K"),
            warmer.map(choice => choice.name).join(", "));
        brighter[0].run();
        root.step("a verb's step", () => root.lit(70, 4100), root.stepped);
    }

    function stepped() {
        const warmest = light.answer("warmer");
        expect("warmer by a step", warmest.headline === "make the desk light warmer \u00b7 3,600 K", warmest.headline);
        // A camera ends, and the light goes out by itself.
        root.says(["press", "false", "70", "4100"]);
        root.step("a camera putting it out",
            () => DeskLight.light !== null && !DeskLight.light.on, root.out);
    }

    function out() {
        expect("a camera tells the bar nothing", root.brightened === 2 && root.warmed === 2,
            `${root.brightened} ${root.warmed}`);
        expect("off", light.answer("desk light").headline === "off",
            light.answer("desk light").headline);
        theLauncher.held = ({});
        light.answer("desk light on").run();
        const lighting = theLauncher.held;
        expect("lit, it says so at once",
            lighting["verb:turn"]?.name === "Turn It Off"
                && lighting["light:desk:brightness"]?.side === "70%"
                && theLauncher.told?.act === "on",
            `${Object.keys(lighting).join(" ")} ${JSON.stringify(theLauncher.told)}`);
        root.step("lit by the launcher", () => DeskLight.light !== null && DeskLight.light.on,
            root.optioned);
    }

    // Its options, which :set sets as its rows do: desklight, deskbrightness
    // and deskwarmth, the warmth in kelvin, so Warmer's line takes from it
    // as Warmer's row does.
    function optioned() {
        const options = light.setOptions ?? {};
        const names = Object.keys(options).sort().join(" ");
        expect("its options", names === "deskbrightness desklight deskwarmth", names);
        expect("each as it stands", [options.desklight.now(), options.deskbrightness.now(),
            options.deskwarmth.now()].join(" ") === "true 70 4100",
            [options.desklight.now(), options.deskbrightness.now(), options.deskwarmth.now()]);
        expect("its warmth from the warmest to the coolest it takes",
            `${options.deskwarmth.least} ${options.deskwarmth.most}`
                === `${DeskLight.warmest} ${DeskLight.coolest}`,
            `${options.deskwarmth.least} ${options.deskwarmth.most}`);
        const warmer = Words.of("verb:warmer").argument;
        expect("Warmer's line lowers its kelvin", warmer === "deskwarmth-=500", warmer);
        options.deskbrightness.set(50, { option: "deskbrightness", how: "set", value: 50 });
        options.deskwarmth.set(3500, { option: "deskwarmth", how: "set", value: 3500 });
        root.step("set by its options", () => root.lit(50, 3500), root.optionedOut);
    }

    function optionedOut() {
        light.setOptions.desklight.set(false);
        root.step("put out by nodesklight", () => DeskLight.light !== null && !DeskLight.light.on,
            root.unplugged);
    }

    function unplugged() {
        root.says(["unplug"]);
        root.step("unplugged", () => DeskLight.light === null, root.none);
    }

    // With no light its row stays where Machine's head names it, saying
    // why, with nothing inside it to set (Y2YBTS, 2026-11-12: a place whose
    // source is away keeps its row). It's never gone, since a show skips a
    // gone row, and a terminal's show must find the row Machine names, as
    // Heat's held row is found (tests/heat-away.qml).
    function away(why) {
        const told = light.answer("desk light");
        expect(`no light, its row stays: ${why}`, told !== null && told.headline === ""
            && told.note === why && told.rows.length === 0 && told.thing !== true,
            JSON.stringify(told));
        expect("and is there for a show to find", told?.gone !== true, JSON.stringify(told));
        // A place holds an answer only with its parts, or whole
        // (Launcher.qml's resolved), so it says it has some, as Heat's
        // and Ethernet's held rows do, and Machine keeps it in its place.
        expect("held in Machine as it will be", told?.parted === true, JSON.stringify(told));
        expect("nothing to set", light.answer("desk light 60") === null
            && light.answer("warmer") === null, JSON.stringify(light.answer("desk light 60")));
        expect("and no options", Object.keys(light.setOptions ?? {}).length === 0,
            Object.keys(light.setOptions ?? {}).join(" "));
    }

    function none() {
        away("not plugged in");
        // litra's watch ends, and its one try again a little later ends too.
        root.says(["end"]);
        root.step("the watch ended", () => !DeskLight.answering && DeskLight.tried, root.ended);
    }

    function ended() {
        away("not answering");
        Qt.exit(0);
    }

    // Nothing here waits forever: a step that never comes fails, by name,
    // 30 s after it began. It was 30 s for the whole case, which a loaded
    // builder ran past in other sessions' changed-checks (RTOIMQ,
    // 2026-11-09, "never came: set by its options").
    Timer {
        id: patience
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }
}
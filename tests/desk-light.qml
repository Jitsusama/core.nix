// Drives the desk light (modules/quickshell/DeskLight.qml) and the
// launcher's answers for it (launcher/Light.qml) through Quickshell itself,
// over a stand-in litra that keeps a light's state and says each change on
// its watch, as the real one does. The answer says how the light is lit;
// "desk light 60", "desk light 4000 k", "desk light off" and "warmer" set
// it through litra; its answer holds its Brightness and Warmth, each a
// setting whose choices step it or set a level; the bar is told of a change
// of brightness or warmth, never of a camera lighting it or putting it out;
// and with no light there is no answer. It exits 0 once every case holds,
// and otherwise 1, saying which didn't.
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
        id: launcher

        property bool open: false
    }

    Launcher.Light {
        id: light

        launcher: launcher
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
    }

    Component.onCompleted: root.step("the first reading", () => DeskLight.light !== null, root.first)

    function lit(percent, kelvin) {
        return DeskLight.light !== null && DeskLight.light.on && DeskLight.light.percent === percent
            && DeskLight.light.kelvin === kelvin;
    }

    function first() {
        const told = light.answer("desk light");
        expect("how it's lit", told !== null && told.headline === "the desk light is 40%, 4000 K",
            told?.headline);
        expect("its parts", told.rows.map(row => `${row.name} ${row.side}`).join(", ")
            === "Brightness 40%, Warmth 4000 K, Turn it off ",
            JSON.stringify(told.rows.map(row => [row.name, row.side])));
        expect("never by the program's name", light.answer("litra") === null,
            light.answer("litra")?.headline);
        expect("nothing told the bar at first", root.brightened === 0 && root.warmed === 0,
            `${root.brightened} ${root.warmed}`);
        const set = light.answer("desk light 60");
        expect("a share set", set.headline === "set the desk light to 60%" && set.verb === "set", set.headline);
        set.run();
        root.step("brighter by the launcher", () => root.lit(60, 4000), root.brighter);
    }

    function brighter() {
        expect("the bar shows a brightness", root.brightened === 1 && root.warmed === 0,
            `${root.brightened} ${root.warmed}`);
        const set = light.answer("desk light 3000 k");
        expect("a warmth set", set.headline === "set the desk light to 3000 K", set.headline);
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
        expect("the parts", partNames === "Brightness, Warmth, Turn it off", partNames);
        expect("how each is set", parts[0].side === "60%" && parts[1].side === "4100 K"
            && parts[0].graph.shape === "bar" && parts[1].graph.shape === "bar",
            JSON.stringify(parts.slice(0, 2).map(part => [part.side, part.graph])));
        expect("each offers its choices beside it", parts[0].offers && parts[1].offers,
            `${parts[0].offers} ${parts[1].offers}`);
        // Their choices: a step either way first, then the levels, the
        // one it's at in use.
        const brighter = parts[0].verbs();
        const warmer = parts[1].verbs();
        const steps = brighter.slice(0, 2).concat(warmer.slice(0, 2), [parts[2]]);
        const stepNames = steps.map(verb => verb.name).join(", ");
        expect("the steps", stepNames === "Brighter, Dimmer, Warmer, Cooler, Turn it off",
            stepNames);
        expect("each its one word",
            steps.map(verb => verb.verb).join(" ") === "brighter dimmer warmer cooler off",
            steps.map(verb => verb.verb).join(" "));
        const inUse = brighter.concat(warmer).filter(choice => choice.side === "in use")
            .map(choice => choice.name);
        expect("the levels in use", inUse.join(", ") === "Brightness to 60%", inUse.join(", "));
        expect("a warmth to choose", warmer.some(choice => choice.name === "Warmth to 2700 K"),
            warmer.map(choice => choice.name).join(", "));
        brighter[0].run();
        root.step("a verb's step", () => root.lit(70, 4100), root.stepped);
    }

    function stepped() {
        const warmest = light.answer("warmer");
        expect("warmer by a step", warmest.headline === "make the desk light warmer, 3600 K", warmest.headline);
        // A camera ends, and the light goes out by itself.
        root.says(["press", "false", "70", "4100"]);
        root.step("a camera putting it out",
            () => DeskLight.light !== null && !DeskLight.light.on, root.out);
    }

    function out() {
        expect("a camera tells the bar nothing", root.brightened === 2 && root.warmed === 2,
            `${root.brightened} ${root.warmed}`);
        expect("off", light.answer("desk light").headline === "the desk light is off",
            light.answer("desk light").headline);
        light.answer("desk light on").run();
        root.step("lit by the launcher", () => DeskLight.light !== null && DeskLight.light.on, root.unplugged);
    }

    function unplugged() {
        root.says(["unplug"]);
        root.step("unplugged", () => DeskLight.light === null, root.none);
    }

    function none() {
        expect("no light, no answer", light.answer("desk light") === null, JSON.stringify(light.answer("desk light")));
        Qt.exit(0);
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }
}

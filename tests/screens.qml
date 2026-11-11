// How Screens says a screen stands, and which of the acts its row serves
// the : line's :setl power and unplugged do, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-launcher-turns-each-screen-off`
// check copies modules/quickshell/launcher/Screens.js beside this file and
// runs `qml -platform offscreen screens.qml`. Which ways a screen offers,
// darkening it before unplugging it where niri darkens one by name, are the
// windows daemon's, tested in modules/windows/src/screens/acts.rs (Joel,
// 2026-11-07: "i want to be able to control the screen that gets shut off,
// one, the other, all").
import QtQuick
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

    function check(failures, what, got, want) {
        const said = JSON.stringify(got);
        if (said !== JSON.stringify(want))
            failures.push(`${what}: got ${said}, wanted ${JSON.stringify(want)}`);
    }

    // A screen as niri lists it: in its outputs while `logical` isn't null,
    // and dark while `powered_off`, which an older niri never says.
    function screen(name, logical, dark) {
        const output = { name: name, logical: logical ? { scale: 1 } : null };
        if (dark !== undefined)
            output.powered_off = dark;
        return output;
    }

    // The acts a screen's row serves, as screens/acts.rs serves them, by
    // what each does, the ways a lit screen turns off after its Return.
    function served(name, ...does) {
        const key = { focus: "displays:focus", on: "verb:display-on", wake: "verb:display-on",
            dark: "verb:display-off", off: "verb:display-off", unplug: "verb:display-unplug" };
        return does.map(done => ({
            key: key[done],
            ask: { ask: "screen", do: done, output: name }
        }));
    }

    function run() {
        const failures = [];
        const panel = screen("eDP-1", true, false);
        const hp = screen("DP-2", true, false);
        const darkHp = screen("DP-2", true, true);
        const pulled = screen("DP-2", false, false);

        check(failures, "a lit screen is lit", Screens.standing(panel), "lit");
        check(failures, "one powered off by name is dark", Screens.standing(darkHp), "dark");
        check(failures, "one out of niri's outputs is off", Screens.standing(pulled), "off");
        check(failures, "an older niri's lit screen is lit",
            Screens.standing(screen("DP-2", true)), "lit");

        const besideLit = served("DP-2", "focus", "dark", "unplug");
        const lastLit = served("eDP-1", "focus", "dark");
        const dark = served("DP-2", "wake");
        const pulledActs = served("DP-2", "on");
        const olderBeside = served("DP-2", "focus", "off");
        const olderLast = served("eDP-1", "focus");

        // The : line's :setl power and :setl unplugged, each an act served
        // or nothing done in its place.
        const way = (output, acts, option, on) =>
            Screens.wayTo(output, acts, { option, on })?.ask.do ?? null;
        check(failures, ':setl nopower darkens a screen by name',
            way(hp, besideLit, "power", false), "dark");
        check(failures, ':setl power lights a dark one', way(darkHp, dark, "power", true), "wake");
        check(failures, ':setl unplugged takes it out of niri\'s outputs',
            way(hp, besideLit, "unplugged", true), "unplug");
        check(failures, 'and on an older niri, as its Turn It Off',
            way(hp, olderBeside, "unplugged", true), "off");
        check(failures, ':setl nounplugged puts an unplugged one back',
            way(pulled, pulledActs, "unplugged", false), "on");
        check(failures, 'but never lights a dark one',
            way(darkHp, dark, "unplugged", false), null);
        check(failures, 'the last lit screen is never unplugged',
            way(panel, lastLit, "unplugged", true), null);
        check(failures, 'nor powered on again while lit', way(panel, lastLit, "power", true), null);
        check(failures, 'what each option says now',
            [hp, darkHp, pulled].map(one => [Screens.optionNow(one, "power"),
                Screens.optionNow(one, "unplugged")]),
            [[true, false], [false, false], [false, true]]);
        // What the line says where it does nothing: what's true where the
        // screen is already as asked, as Vim's :set does, and a refused
        // act's "not {done} \u00b7 {why}" where the way isn't offered
        // (VKCF3Z, 2026-11-08).
        const told = (output, acts, option, on) => Screens.toldOf(output, acts, { option, on });
        check(failures, 'a lit screen powered on is on',
            told(panel, served("eDP-1", "focus", "dark", "unplug"), "power", true), "isOn");
        check(failures, 'a dark one powered off is dark',
            told(darkHp, dark, "power", false), "isDark");
        check(failures, 'an unplugged one powered off is unplugged',
            told(pulled, pulledActs, "power", false), "isUnplugged");
        check(failures, 'a dark one put back is plugged in',
            told(darkHp, dark, "unplugged", false), "isPluggedIn");
        check(failures, 'an unplugged one unplugged is unplugged',
            told(pulled, pulledActs, "unplugged", true), "isUnplugged");
        check(failures, 'the last lit screen is not unplugged',
            told(panel, lastLit, "unplugged", true), "not-unplugged");
        check(failures, 'nor turned off on an older niri',
            told(panel, olderLast, "power", false), "not-off");
        check(failures, 'and a way offered says nothing',
            told(hp, besideLit, "power", false), "");

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

// How the launcher's catalogue (modules/quickshell/launcher/Catalogue.js)
// tells what's asked of it from outside apart, run in Qt's own JavaScript
// engine, the one the shell runs it in. The
// `the-launcher-catalogues-what-it-does` check copies Catalogue.js beside
// this file and runs `qml -platform offscreen catalogue.qml`. Each case is
// a promise pi-ask's tool leans on: a place opens and an action is done,
// one that ends something is only offered, and what's only a figure, a
// copy or a command's start isn't listed at all. A step into a row, as the
// right arrow and the catalogue both show it, leads with Return's own act.
import QtQuick
import "Catalogue.js" as Catalogue

QtObject {
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    property int failed: 0

    function expect(what, got, wanted) {
        if (JSON.stringify(got) !== JSON.stringify(wanted)) {
            console.warn(`FAIL ${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
            failed += 1;
        }
    }

    function run() {
        const opens = () => [];
        const does = () => {};
        // What each is.
        expect("a place", Catalogue.kindOf({ category: {}, verbs: opens }), "place");
        expect("a collection", Catalogue.kindOf({ opens: "windows", verbs: opens }), "place");
        expect("a part holding more", Catalogue.kindOf({ verb: "more", run: does, verbs: opens }),
            "place");
        expect("a thing that's done and holds more", Catalogue.kindOf({ verb: "switch",
            run: does, verbs: opens }), "action");
        expect("an action", Catalogue.kindOf({ verb: "set", run: does }), "action");
        expect("one that takes words", Catalogue.kindOf({ verb: "say", given: does,
            verbs: opens }), "action");
        // What only Joel's Return does.
        expect("one asked to be sure of", Catalogue.disruptive({ verb: "reboot", confirm: true }),
            true);
        for (const verb of ["close", "quit", "leave", "lock", "sleep", "off", "turn off", "forget",
            "disconnect", "share", "stop"])
            expect(`${verb} ends something`, Catalogue.disruptive({ verb: verb }), true);
        for (const verb of ["open", "set", "brighter", "louder", "mute", "switch", "use", "pause"])
            expect(`${verb} is done at once`, Catalogue.disruptive({ verb: verb }), false);
        // What's listed.
        expect("a figure isn't", Catalogue.listed({ name: "Health", figure: true, run: does }),
            false);
        expect("a figure holding more is", Catalogue.listed({ name: "Battery", figure: true,
            verbs: opens }), true);
        expect("a copy isn't", Catalogue.listed({ name: "84", verb: "copy", run: does }), false);
        expect("a command's start isn't", Catalogue.listed({ name: "time in", start: "time in ",
            run: does }), false);
        expect("one gone isn't", Catalogue.listed({ name: "Headset", gone: true, run: does }),
            false);
        expect("an action is", Catalogue.listed({ name: "Brighter", verb: "brighter", run: does }),
            true);
        expect("a keyboard's key isn't", Catalogue.listed({ name: "Escape", verb: "show",
            key: "keyboard:key:Base:0", run: does }), false);
        // How far down it goes.
        expect("a list's things aren't", Catalogue.descends({ opens: "emoji" }, 1, null), false);
        expect("shortcuts aren't", Catalogue.descends({ key: "shortcut-level:shortcuts:kitty" },
            2, {}), false);
        expect("a place's are", Catalogue.descends({ opens: "windows" }, 1, null), true);
        const programs = { opens: "applications" };
        expect("Programs' things are", Catalogue.descends(programs, 0, null), true);
        expect("a program's own aren't", Catalogue.descends({ name: "Firefox", verbs: opens }, 1,
            programs), false);
        expect("a window's own aren't", Catalogue.descends({ name: "Notes", verbs: opens }, 2,
            { opens: "windows" }), false);
        expect("a quest's own aren't", Catalogue.descends({ name: "The Launcher", verbs: opens },
            2, { opens: "quests" }), false);
        expect("the runs aren't", Catalogue.descends({ opens: "runs" }, 1, null), false);
        expect("AI spend's findings aren't", Catalogue.descends({ opens: "spend" }, 1, null),
            false);
        expect("an orchestrator's are", Catalogue.descends({ name: "Lighthouse", verbs: opens },
            2, { opens: "orchestrators" }), true);
        expect("this window's are", Catalogue.descends({ key: "windows:this", verbs: opens }, 1,
            { category: {} }), true);
        expect("nothing below the deepest", Catalogue.descends({ category: {} }, 4, {}), false);
        // What an entry says.
        expect("an entry", Catalogue.entry({ name: "Reboot", verb: "reboot", confirm: true,
            run: does, side: "" }, "category:computer\u203acategory:energy\u203asession:Reboot",
            ["Machine", "Power"]), { id: "category:computer\u203acategory:energy\u203asession:Reboot",
            kind: "action", words: "Reboot", in: "Machine \u203a Power", disruptive: true });
        expect("one that takes words", Catalogue.entry({ name: "Talk", verb: "say", given: does,
            ask: "what to say?", side: "listening" }, "a", []), { id: "a", kind: "action",
            words: "Talk", in: "", disruptive: false, says: "listening",
            args: [{ name: "words", words: "what to say?" }] });
        // A step into a row leads with what Return does, unless it's there.
        const names = rows => rows.map(row => row.name).join(", ");
        let opened = 0;
        const notice = { name: "Build done", verb: "open", glyph: "n", run: () => {
            opened += 1;
        }, verbs: opens };
        const offers = [{ name: "Copy", verb: "copy", run: does },
            { name: "Forget", verb: "forget", run: does }];
        const led = Catalogue.leading(notice, offers);
        expect("Return's act leads what it left out", names(led), "Open, Copy, Forget");
        led[0].run();
        expect("and does what Return does", opened, 1);
        expect("as an act of its own", [led[0].key, led[0].verb, led[0].hue],
            ["return:open", "open", "action"]);
        // The list draws its chord and what's beside it, as a provider's.
        expect("drawn as any row is", [led[0].chord.length, led[0].side, led[0].what],
            [0, "", ""]);
        expect("never twice", names(Catalogue.leading({ name: "Slack", verb: "switch", run: does,
            verbs: opens }, [{ name: "Switch", verb: "switch", run: does }])), "Switch");
        // One already inside, named as it's done, is moved first, not doubled.
        const joined = { name: "Lantern", verb: "disconnect", run: does, verbs: opens };
        expect("found by its name", names(Catalogue.leading(joined, [{ name: "Sign in to it",
            verb: "open", run: does }, { name: "Disconnect it", run: does }])),
            "Disconnect it, Sign in to it");
        expect("or by how its name starts", names(Catalogue.leading({ name: "Quarry",
            verb: "copy", run: does, verbs: opens }, [{ name: "Play it", run: does },
            { name: "Copy what it said", run: does }])), "Copy what it said, Play it");
        expect("read without what it's done to", names(Catalogue.leading({ name: "Radio",
            verb: "turn off", run: does, verbs: opens }, [{ name: "Turn it off", run: does }])),
            "Turn it off");
        expect("not for a row that only steps in", names(Catalogue.leading({ name: "Space",
            verb: "more", run: does, verbs: opens }, offers)), "Copy, Forget");
        expect("not for one that asks first", names(Catalogue.leading({ name: "Reboot",
            verb: "reboot", confirm: true, run: does, verbs: opens }, offers)), "Copy, Forget");
        expect("not for one holding nothing", Catalogue.leading(notice, []).length, 0);
        // Every row has a Return act.
        expect("a row that steps in acts", Catalogue.acts({ name: "Power", category: {} }), true);
        expect("a value that copies acts", Catalogue.acts({ name: "Load", side: "34%",
            figure: true, verb: "copy", run: does }), true);
        expect("a value with nothing to copy doesn't", Catalogue.acts({ name: "Load", side: "",
            figure: true, verb: "close", run: does }), false);
        expect("nor one that does nothing", Catalogue.acts({ name: "Idle" }), false);
        expect("one still being read isn't held against it", Catalogue.acts({ name: "Load",
            side: "\u2026", figure: true, verb: "close", run: does }), true);
        expect("a window's Close acts", Catalogue.acts({ name: "Close", verb: "close",
            run: does }), true);
        expect("the long lists are named", ["emoji", "notices"].map(Catalogue.long),
            [true, false]);
        Qt.exit(failed === 0 ? 0 : 1);
    }
}

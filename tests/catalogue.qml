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
        const names = rows => rows.map(row => row.name).join(", ");
        // What each is.
        expect("a place", Catalogue.kindOf({ category: {}, verbs: opens }), "place");
        expect("a collection", Catalogue.kindOf({ opens: "windows", verbs: opens }), "place");
        expect("a part holding more", Catalogue.kindOf({ verb: "more", run: does, verbs: opens }),
            "place");
        expect("a thing that's done and holds more", Catalogue.kindOf({ verb: "go",
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
        for (const verb of ["open", "set", "brighter", "louder", "mute", "go", "use", "pause"])
            expect(`${verb} is done at once`, Catalogue.disruptive({ verb: verb }), false);
        // What Return typed in insert never does: what words.json marks as
        // ending something in use, a row's ending action or a disconnect.
        expect("an ending action ends", Catalogue.ends({ verb: "forget", act: "ending",
            ends: "forgets" }), true);
        expect("and so does a disconnect", Catalogue.ends({ verb: "disconnect",
            ends: "disconnects" }), true);
        expect("a close that only closes the launcher doesn't", Catalogue.ends({ verb: "close" }),
            false);
        expect("nor does a lock, typed to be done", Catalogue.ends({ verb: "lock" }), false);
        // What Return over visual's selection opens: a row Return takes
        // somewhere, never one that changes something, as one that stays
        // to show its new state does, nor one that ends or disrupts, nor a
        // place, which Return steps into.
        expect("a program opens", Catalogue.opens({ verb: "open", run: does }), true);
        expect("and a window goes", Catalogue.opens({ verb: "go", run: does }), true);
        expect("a change that stays doesn't", Catalogue.opens({ verb: "connect", stays: true,
            run: does }), false);
        expect("nor an ending", Catalogue.opens({ verb: "close", act: "ending", run: does }),
            false);
        expect("nor a lock", Catalogue.opens({ verb: "lock", run: does }), false);
        expect("nor a place", Catalogue.opens({ verb: "more", run: does }), false);
        expect("nor a row with nothing to run", Catalogue.opens({ verb: "open" }), false);
        // What Return from the find lands on rather than doing: whatever
        // changes how things stand, and never what only shows or opens.
        const changing = [
            ["an ending action", { verb: "forget", act: "ending", ends: "forgets", run: does }],
            ["a disconnect", { verb: "disconnect", ends: "disconnects", run: does }],
            ["a lock", { verb: "lock", run: does }],
            ["a turning off", { verb: "turn off", stays: true, run: does }],
            ["a setting's switch", { verb: "keep", stays: true, run: does }],
            ["a network's join", { verb: "connect", stays: true, run: does, verbs: opens }],
            ["one asked to be sure of", { verb: "reboot", confirm: true, run: does }],
            ["a setting's act, closing", { verb: "hold", changes: true, run: does }],
            ["a session's pause, closing", { verb: "pause", changes: true, run: does }]
        ];
        for (const [what, row] of changing)
            expect(`${what} changes how things stand`, Catalogue.changesState(row), true);
        const quietly = [
            ["a program", { verb: "open", run: does }],
            ["a window", { verb: "go", run: does, verbs: opens }],
            ["a copy", { verb: "copy", stays: true, run: does }],
            ["a See It", { verb: "see", stays: true, run: does }],
            ["a place", { category: {}, verbs: opens }],
            ["a spot to go to", { verb: "keep", stays: true, spot: "x", run: does }],
            ["a verb wanting targets", { verb: "more", stays: true, run: does, verbs: opens,
                targets: opens }],
            ["one gone", { verb: "disconnect", ends: "disconnects", gone: true, run: does }]
        ];
        for (const [what, row] of quietly)
            expect(`${what} changes nothing`, Catalogue.changesState(row), false);
        // The first row inside a thing never changes anything.
        const seen = { name: "See It", verb: "see", run: does };
        const cut = { name: "Disconnect It", verb: "disconnect", ends: "disconnects",
            run: does };
        const forget = { name: "Forget It", verb: "forget", act: "ending", run: does };
        expect("a disconnect first goes after the first quiet row",
            names(Catalogue.quietFirst([cut, forget, seen, { name: "Copy", verb: "copy",
                run: does }])), "See It, Disconnect It, Forget It, Copy");
        expect("a quiet first row stays first", names(Catalogue.quietFirst([seen, cut])),
            "See It, Disconnect It");
        expect("rows that all change something keep their order",
            names(Catalogue.quietFirst([cut, forget])), "Disconnect It, Forget It");
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
        expect("never twice", names(Catalogue.leading({ name: "Slack", verb: "go", run: does,
            verbs: opens }, [{ name: "Go To", verb: "go", run: does }])), "Go To");
        // One already inside, named as it's done, is moved first, not doubled.
        const joined = { name: "Lantern", verb: "disconnect", run: does, verbs: opens };
        expect("found by its name", names(Catalogue.leading(joined, [{ name: "Sign in to it",
            verb: "open", run: does }, { name: "Disconnect it", run: does }])),
            "Disconnect it, Sign in to it");
        // A copy is never first, and always last, after what can be done
        // and what's only said (docs/design.md's Return rule).
        expect("a copy last, found by how its name starts", names(Catalogue.leading({
            name: "Quarry", verb: "copy", run: does, verbs: opens }, [{ name: "Play it",
            run: does }, { name: "Copy what it said", run: does }, { name: "Said", figure: true,
            run: does }])), "Play it, Said, Copy what it said");
        expect("not a figure copying its own value", names(Catalogue.leading({ name: "Disk",
            verb: "copy", run: does, verbs: opens }, [{ name: "Space", verb: "copy",
            run: does }, { name: "Busy", verb: "copy", run: does }])), "Space, Busy, Copy");
        expect("a value holding only facts copies last", names(Catalogue.leading({
            name: "Battery", verb: "copy", run: does, verbs: opens, targets: opens },
            [{ name: "Charger", figure: true, run: does }])), "Charger, Copy");
        expect("read without what it's done to", names(Catalogue.leading({ name: "Radio",
            verb: "turn off", run: does, verbs: opens }, [{ name: "Turn it off", run: does }])),
            "Turn it off");
        expect("not for a row that only steps in", names(Catalogue.leading({ name: "Space",
            verb: "more", run: does, verbs: opens }, offers)), "Copy, Forget");
        expect("nor for one holding rows, its Return their step", names(Catalogue.leading({
            name: "See It", verb: "see", run: does, verbs: opens, targets: opens }, offers)),
            "Copy, Forget");
        expect("not for one that asks first", names(Catalogue.leading({ name: "Reboot",
            verb: "reboot", confirm: true, run: does, verbs: opens }, offers)), "Copy, Forget");
        expect("not for one holding nothing", Catalogue.leading(notice, []).length, 0);
        // A setting, which Return does in place, leads its step with an act
        // that closes, so the right arrow then Return does it and closes.
        let kept = 0;
        const awake = { name: "Keep Awake", verb: "keep", stays: true, run: () => {
            kept += 1;
        } };
        const closing = Catalogue.closingFirst(awake, []);
        expect("a setting holding nothing holds its act", [names(closing), closing[0].stays],
            ["Keep", false]);
        expect("which changes what the setting did", Catalogue.changesState(closing[0]), true);
        closing[0].run();
        expect("which does what Return does", kept, 1);
        const quiet = { name: "Do Not Disturb", verb: "hold", stays: true, run: does };
        const holding = Catalogue.closingFirst(quiet, [{ name: "Until It's Ended", verb: "end",
            stays: true, run: does }, { name: "For an Hour", verb: "hold", stays: true,
            run: does }]);
        expect("one already inside is moved first and closes",
            [names(holding), holding.map(row => row.stays)],
            ["For an Hour, Until It's Ended", [false, true]]);
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

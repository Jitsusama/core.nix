// How the launcher's catalogue (modules/quickshell/launcher/Catalogue.js)
// tells what's asked of it from outside apart, run in Qt's own JavaScript
// engine, the one the shell runs it in. The
// `the-launcher-catalogues-what-it-does` check copies Catalogue.js beside
// this file, with the key language's commands as commands.json, and runs `qml -platform offscreen catalogue.qml`. Each case is
// a promise pi-ask's tool leans on: a place opens and an action is done,
// one that ends something is only offered, and what's only a figure, a
// copy or a command's start isn't listed at all. A step into a row, as the
// right arrow and the catalogue both show it, leads with Return's own act.
// A place asked open is said opened only where the walk got
// (launcher/Reached.js, which the check copies beside it too).
import QtQuick
import "Catalogue.js" as Catalogue
import "Reached.js" as Reached

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

    // The key language's commands by their whole names, as the shell's
    // Options.commands holds them, which the check writes beside this file
    // from modules/keys/language.toml.
    function commands() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("commands.json"), false);
        request.send();
        return JSON.parse(request.responseText);
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
        // A session's row names the act its Return does, and holds its acts.
        expect("one whose Return is an act", Catalogue.kindOf({ verbs: opens,
            returns: { verb: "go", run: does } }), "action");
        expect("its verb is its act's", Catalogue.verbOf({ returns: { verb: "resume" } }),
            "resume");
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
        expect("one whose Return goes opens", Catalogue.opens({ verbs: opens,
            returns: { verb: "go", run: does } }), true);
        expect("but not one whose Return asks for words", Catalogue.opens({ verbs: opens,
            returns: { verb: "talk", run: does, given: does } }), false);
        // The key language decides as Return's landing does (changesState),
        // so Return on one row and over a selection never disagree.
        expect("nor one whose command the language says changes", Catalogue.opens(
            { verb: "move", run: does }, { command: "move", flags: { changes: true } }), false);
        expect("nor one that says it changes", Catalogue.opens({ verb: "go", changes: true,
            run: does }), false);
        expect("while one it says changes nothing opens", Catalogue.opens(
            { verb: "go", run: does }, { command: "go", flags: { changes: false } }), true);
        // What Return from the find lands on rather than doing: whatever
        // changes how things stand, as the key language flags the command a
        // row is named by (modules/keys/language.toml's changes), so d and
        // Return agree, and never what only shows or opens. Each row is
        // given its command as the launcher resolves it, and that command's
        // flags where the language names it.
        const language = commands();
        // A row's command, from its words (`command` here) or else as
        // Commands.js's commandOf reads one: its verb's first word, or its
        // name's where it has no verb.
        const verbal = row => {
            const verb = String(row.verb ?? "").trim().toLowerCase();
            const said = verb === "" ? String(row.name ?? "").trim().toLowerCase() : verb;
            return (said.match(/^[a-z]+/) ?? [""])[0];
        };
        const named = row => Catalogue.named(row.command, verbal(row), language);
        const changes = row => Catalogue.changesState(row, named(row));
        const changing = [
            ["an ending action", { verb: "forget", act: "ending", ends: "forgets", run: does }],
            ["a disconnect", { verb: "disconnect", ends: "disconnects", run: does }],
            ["a setting's switch, by :set", { verb: "keep", command: "set", run: does }],
            ["a window moved, by :move", { verb: "move", run: does }],
            ["one asked to be sure of", { verb: "reboot", confirm: true, run: does }],
            ["a setting's act, closing", { verb: "hold", changes: true, run: does }],
            ["a session's pause", { verb: "pause", run: does }],
            // A command the language doesn't name yet keeps the guess from
            // how the row is made, whatever its verb or name says, since a
            // row's command is read from its name where it has no verb.
            ["a lock, guessed", { verb: "lock", run: does }],
            ["a turning off, guessed", { verb: "turn off", stays: true, run: does }],
            ["a network's join, guessed", { verb: "connect", stays: true, run: does,
                verbs: opens }],
            ["Wi-Fi Off, guessed", { verb: "off", stays: true, run: does }],
            ["Do Not Disturb ended, guessed", { verb: "end", stays: true, run: does }],
            ["a track skipped, guessed", { verb: "next", stays: true, run: does }],
            ["a brightness set, guessed from its name", { name: "Brightness to 40%",
                stays: true, run: does }]
        ];
        for (const [what, row] of changing)
            expect(`${what} changes how things stand`, changes(row), true);
        expect("a window's Close It, by its words' :close, changes how things stand",
            changes({ verb: "close", command: "close", run: does }), true);
        const quietly = [
            ["a program", { verb: "open", run: does }],
            ["a program opened that stays, as the table says :open changes nothing",
                { verb: "open", stays: true, run: does }],
            ["a window", { verb: "go", run: does, verbs: opens }],
            ["a copy", { verb: "copy", command: "yank", stays: true, run: does }],
            ["a See It", { verb: "see", stays: true, run: does }],
            ["a verb the language names nothing for and the guess doesn't hold",
                { verb: "say", run: does }],
            ["a place", { category: {}, verbs: opens }],
            // A figure only says a value, so Return copies it, or closes
            // the launcher where it has nothing to copy yet, which isn't
            // the :close of a window that its verb would name.
            ["a figure with nothing to copy yet", { verb: "close", figure: true, run: does }],
            // A row whose verb alone says close only closes the launcher,
            // as :quit does, where a window's Close It is its words' :close.
            ["a row that only closes the launcher", { verb: "close", run: does }],
            // Return on a row that asks for words first only asks, whatever
            // the words then do, as a network's new password.
            ["a row asking its words first", { verb: "connect", command: "password",
                given: does, run: does }],
            ["a spot to go to", { verb: "start", stays: true, spot: "x", run: does }],
            ["a verb wanting targets", { verb: "more", stays: true, run: does, verbs: opens,
                targets: opens }],
            ["one gone", { verb: "disconnect", ends: "disconnects", gone: true, run: does }],
            ["a question asked", { verb: "ask", changes: false, stays: true, run: does }],
            ["a picker's mark, which only picks", { verb: "mark", changes: false, stays: true,
                run: does }],
            ["a place found again", { verb: "find", stays: true, run: does }],
            ["a search tried again", { verb: "retry", stays: true, run: does }]
        ];
        for (const [what, row] of quietly)
            expect(`${what} changes nothing`, changes(row), false);
        // What Return from the find does rather than landing on, though it
        // changes something: a row holding its own acts whose command
        // starts something, as a meeting's row joins from ten minutes
        // before it (VKCF3Z, 2026-11-05), the row saying when, not its act.
        const itself = row => Catalogue.startsItself(row, named(row));
        expect("a meeting's row starts itself", itself({ verb: "join", command: "meet",
            run: does, verbs: opens }), true);
        expect("while Join the Call, with no acts of its own, doesn't", itself({ verb: "join",
            command: "meet", run: does }), false);
        expect("nor a setting holding its steps", itself({ verb: "keep", command: "set",
            stays: true, run: does, verbs: opens }), false);
        expect("nor a network holding its acts", itself({ verb: "connect", stays: true,
            run: does, verbs: opens }), false);
        // The verbs the guess holds quiet, the language naming no command
        // for them yet, can only shrink: one joining fails here, and so does
        // one the language has come to name, whose rows the table decides.
        expect("the verbs the guess holds quiet", Catalogue.quiet,
            ["copy", "find", "go", "more", "see", "show"]);
        expect("none of which the language names as a command",
            Catalogue.quiet.filter(verb => language[verb] !== undefined), []);
        // The first row inside a thing never changes anything.
        const seen = { name: "See It", verb: "see", run: does };
        const cut = { name: "Disconnect It", verb: "disconnect", ends: "disconnects",
            run: does };
        const forget = { name: "Forget It", verb: "forget", act: "ending", run: does };
        expect("a disconnect first goes after the first quiet row",
            names(Catalogue.quietFirst([cut, forget, seen, { name: "Copy", verb: "copy",
                run: does }], named)), "See It, Disconnect It, Forget It, Copy");
        expect("a quiet first row stays first", names(Catalogue.quietFirst([seen, cut], named)),
            "See It, Disconnect It");
        expect("rows that all change something keep their order",
            names(Catalogue.quietFirst([cut, forget], named)), "Disconnect It, Forget It");
        // What never runs on Return, from any mode, its command put on the
        // : line instead: an act that ends, takes away or reaches someone,
        // as the language's ends flag says of its command, and :set's no
        // form, which turns something off (`off`, read from the command's
        // value as SetWords reads it, since the flag is the command's and
        // :set wifi turns it on).
        const ending = (row, off) => Catalogue.endsSomething(row, named(row), off ?? false);
        const endings = [
            ["a disconnect", { verb: "disconnect", command: "disconnect", run: does }],
            ["a forget", { verb: "forget", command: "forget", act: "ending", run: does }],
            ["a window's Close It, by :close", { verb: "close", command: "close", run: does }],
            ["a reply", { verb: "reply", command: "reply", run: does }],
            ["the session's Power Off", { verb: "off", command: "poweroff", run: does }],
            ["Join the Call in the Clipboard, by :meet", { verb: "join", command: "meet",
                run: does }],
            // Where the language names no command for it, an ending act, as
            // words.json marks one, still ends.
            ["an ending act whose command isn't named", { verb: "zap", act: "ending",
                run: does }]
        ];
        for (const [what, row] of endings)
            expect(`${what} ends something`, ending(row), true);
        expect("Wi-Fi Off, :set nowifi, ends something",
            ending({ verb: "off", command: "set", run: does }, true), true);
        // What calls someone or starts something new is the spec's other
        // rule for Return, the language's starts flag, and goes to the line
        // as an ending does (VKCF3Z, 2026-11-05).
        expect("what starts something, as :share, goes to the line",
            Catalogue.endsSomething({ verb: "share", run: does },
                { command: "share", flags: { starts: true } }), true);
        const lasting = [
            ["Wi-Fi On, :set wifi", { verb: "on", command: "set", run: does }],
            ["a copy", { verb: "copy", command: "yank", run: does }],
            ["a window moved", { verb: "move", run: does }],
            ["a place", { category: {}, verbs: opens }],
            // A thing holding acts of its own opens on Return, its ending
            // among its acts for d, so a meeting joins its call from ten
            // minutes before it starts, Joel's own rule, though its
            // command is :meet.
            ["a meeting, holding its acts", { verb: "join", command: "meet", verbs: opens,
                run: does }],
            ["one gone", { verb: "disconnect", command: "disconnect", gone: true, run: does }],
            ["a row asking its words first", { verb: "disconnect", command: "disconnect",
                given: does, run: does }],
            // A switch someone else holds is refused before it's asked, so
            // Return runs it and says why (VKCF3Z, 2026-11-08).
            ["a disconnect someone else holds", { verb: "disconnect", command: "disconnect",
                heldBy: "example", run: does }]
        ];
        for (const [what, row] of lasting)
            expect(`${what} ends nothing`, ending(row), false);
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
        // One naming its Return's act keeps its acts in their own order.
        expect("a session's acts stay in order", names(Catalogue.leading({ name: "A Quest",
            verbs: opens, returns: { verb: "talk", run: does } }, [{ name: "Go To",
            verb: "go", run: does }, { name: "Talk", verb: "talk", run: does }])),
            "Go To, Talk");
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
        // Return's own copy wears the copy mark, as every Copy row does,
        // never its row's, which may be a caution's (VKCF3Z, 2026-11-07:
        // Copy inside Budgets wore the budget's warning mark); any other
        // act of Return's wears its row's.
        const budget = { name: "Budgets", verb: "copy", glyph: "!", run: does, verbs: opens };
        const copied = Catalogue.leading(budget, [{ name: "Claude Pro", run: does }], "c");
        expect("Return's copy wears the copy mark", copied.map(one => [one.name, one.glyph]),
            [["Claude Pro", undefined], ["Copy", "c"]]);
        const opening = Catalogue.leading(notice, offers, "c");
        expect("another of Return's acts wears its row's mark", opening[0].glyph, "n");
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
        expect("which changes what the setting did", changes(closing[0]), true);
        closing[0].run();
        expect("which does what Return does", kept, 1);
        const profile = { name: "Balanced", verb: "use", stays: true, alwaysStays: true,
            run: does };
        expect("one that always stays keeps the launcher from its step too",
            Catalogue.closingFirst(profile, [])[0].stays, true);
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
        lives();
        Qt.exit(failed === 0 ? 0 : 1);
    }

    // Where a row lives, which Ctrl+] goes to as Vim follows a tag: the
    // shallowest place from home holding a row that is it, never Now, which
    // holds what lives elsewhere, nor inside a long list, a program or a
    // person, looked for on the press and only as far as it's found.
    function lives() {
        const row = (key, fields) => Object.assign({ key: key, name: key }, fields);
        const emoji = row("collection:emoji", { opens: "emoji" });
        const smile = row("emoji:smile");
        const quiet = row("session:quiet");
        const lock = row("session:Lock");
        const kitty = row("program:kitty");
        const window = row("window:kitty");
        const cpu = row("machine:cpu", { parted: true });
        const wifi = row("wifi:home");
        const ada = row("person:ada", { parted: true });
        const inside = {
            "collection:on": [quiet, lock],
            "category:desktop": [emoji, quiet, row("category:energy", { category: {} }), cpu,
                wifi],
            "machine:cpu": [row("machine:cpu:cores")],
            "category:energy": [lock],
            "collection:emoji": [smile],
            "collection:applications": [kitty],
            "program:kitty": [window],
            "wifi:home": [row("wifi:home:forget")],
            "session:quiet": [row("session:quiet:until")],
            "collection:people": [ada],
            "person:ada": [row("mail:1")]
        };
        const home = [row("collection:on", { opens: "on" }),
            row("collection:applications", { opens: "applications" }),
            row("category:desktop", { category: {} }),
            row("collection:people", { opens: "people" })];
        const asked = [];
        const ways = { idOf: one => one.key, holds: one => {
            asked.push(one.key);
            return inside[one.key] ?? [];
        } };
        const looked = (key, also) => {
            asked.length = 0;
            return Catalogue.livesOf(home, key, Object.assign({}, ways, also));
        };
        const at = key => looked(key).path;
        // Each kind of row Ctrl+] looks for rather than entering or being
        // told: a thing in a place (a Now row's, as Do Not Disturb), one
        // found from home deep in places (Lock), and a notice's program
        // (by its `follows`). A spot, a found act and a row with parts say
        // where they go themselves (Launcher.qml's followedTo and followed).
        expect("a thing lives in the place holding it, Now passed over", at("session:quiet"),
            ["category:desktop", "session:quiet"]);
        expect("a place inside a place, as deep as it is", at("session:Lock"),
            ["category:desktop", "category:energy", "session:Lock"]);
        expect("a collection lives in the place holding it", at("collection:emoji"),
            ["category:desktop", "collection:emoji"]);
        expect("a program, at home's Programs", at("program:kitty"),
            ["collection:applications", "program:kitty"]);
        expect("a place at home lives at home", at("category:desktop"), ["category:desktop"]);
        expect("what a long list holds isn't walked", at("emoji:smile"), null);
        expect("nor what a program holds", at("window:kitty"), null);
        expect("what lives only in Now lives nowhere else", at("on:waitingOnYou"), null);
        // A measured figure's parts are read as they're made, a CPU's
        // listing the processes, so a follow that finds no parts doesn't
        // make them on its way.
        expect("a figure lives where it's measured", at("machine:cpu"),
            ["category:desktop", "machine:cpu"]);
        expect("but what it measures isn't walked", at("machine:cpu:cores"), null);
        expect("nor made", asked.includes("machine:cpu"), false);
        expect("nor what a person holds", at("mail:1"), null);
        expect("nor are a person's insides made", asked.includes("person:ada"), false);
        expect("a thing's own row is found by the look into things", at("wifi:home:forget"),
            ["category:desktop", "wifi:home", "wifi:home:forget"]);
        // The look is the key's own work, so it stops where it finds the
        // row, and no place past it is asked.
        expect("a look asks each place once", looked("none:here").listed > 0
            && asked.length === new Set(asked).size, true);
        const kittyLook = looked("program:kitty");
        expect("the look stops where it's found", [kittyLook.path, asked],
            [["collection:applications", "program:kitty"], ["collection:applications"]]);
        expect("it says how many rows it listed, found", [kittyLook.listed, kittyLook.spent],
            [4, false]);
        expect("a row at home is found without asking a place",
            [looked("category:desktop").path, asked], [["category:desktop"], []]);
        // Places first, as the launcher looks: a thing's insides wait
        // until no place is left, and the look carries on from them
        // rather than starting again, so each row is listed once.
        const places = { first: one => Catalogue.isPlace(one) };
        expect("places first, Lock is found deep in them", looked("session:Lock", places).path,
            ["category:desktop", "category:energy", "session:Lock"]);
        expect("without a thing's insides asked", asked.includes("session:quiet"), false);
        expect("a thing's own row is found after the places",
            looked("wifi:home:forget", places).path,
            ["category:desktop", "wifi:home", "wifi:home:forget"]);
        expect("a place asked before a thing set aside",
            asked.indexOf("category:energy") < asked.indexOf("session:quiet"), true);
        expect("each asked once", asked.length === new Set(asked).size, true);
        expect("a miss lists each row once, as many as a look in one order",
            looked("none:here", places).listed, looked("none:here").listed);
        expect("a place is a category or a collection",
            [row("a", { category: {} }), emoji, wifi].map(Catalogue.isPlace), [true, true, false]);
        // A look past its budget gives up, saying so, rather than holding
        // the key: here once more than five rows are listed, the sixth
        // being the first Desktop holds.
        const spentLook = looked("wifi:home:forget", { spent: listed => listed > 5 });
        expect("a look past its budget gives up, unfound", [spentLook.path, spentLook.spent],
            [null, true]);
        expect("as soon as it's listed past it", spentLook.listed, 6);
        // A provider makes a list whole, so the budget is asked once it's
        // made: the look gives up partway down Desktop's rows, asking no
        // place after it.
        expect("a list made whole, then given up at once", asked,
            ["collection:applications", "category:desktop"]);
        // One budget runs across places and the things set aside after
        // them: the places list 12 rows, so a look allowed 12 gives up in
        // the first thing it goes into, and one allowed 13 finds the row.
        const across = most => looked("wifi:home:forget",
            Object.assign({ spent: listed => listed > most }, places));
        const short = across(12);
        expect("a budget runs on from the places into things",
            [short.path, short.spent, short.listed, asked.includes("session:quiet")],
            [null, true, 13, true]);
        expect("counting the places once", across(13).path,
            ["category:desktop", "wifi:home", "wifi:home:forget"]);
        expect("a miss within it isn't given up", looked("none:here").spent, false);
        // Where Ctrl+] goes from where the launcher stands: the place that
        // holds it and the row there, or nowhere when it's held here.
        expect("found from home, it goes into its place, onto it",
            Catalogue.followed(["category:desktop", "session:quiet"], []),
            { trail: ["category:desktop"], spot: "session:quiet" });
        expect("from Now, the same",
            Catalogue.followed(["category:desktop", "session:quiet"], ["collection:on"]),
            { trail: ["category:desktop"], spot: "session:quiet" });
        expect("in its own place, it goes nowhere",
            Catalogue.followed(["category:desktop", "session:quiet"], ["category:desktop"]),
            null);
        expect("a place at home, from home, goes nowhere",
            Catalogue.followed(["category:desktop"], []), null);
        expect("nor does what lives nowhere", Catalogue.followed(null, []), null);
        // A place asked open from outside is said opened only where the
        // walk got: one that refused its step, as Passwords does while
        // it's walked from outside, leaves the launcher short of it.
        const passwords = ["category:secrets", "collection:passwords"];
        expect("walked all the way, it's reached",
            Reached.reached(["category:secrets", "collection:passwords"], passwords), true);
        expect("stopped short, it isn't", Reached.reached(["category:secrets"], passwords),
            false);
        expect("left at home, it isn't", Reached.reached([], passwords), false);
        expect("somewhere else as deep, it isn't",
            Reached.reached(["category:secrets", "collection:keys"], passwords), false);
    }
}

// The launcher's command line, the : line, held in Qt's own JavaScript
// engine, the one the shell runs it in: what its menu lists for a row and
// a surface, how typing narrows it, what Tab and the arrows choose and
// write in, and what Return runs, or the error it says, as Vim's command
// line would. The `the-command-line-lists-every-command` check copies
// modules/quickshell/Commands.js beside this file and runs
// `qml -platform offscreen command-line.qml`.
import QtQuick
import "Commands.js" as Commands

QtObject {
    // What failed, said as the test ends, or as it throws.
    property var failures: []

    // A test that throws ends as a failure, rather than leaving qml running,
    // saying what had failed before it.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // The launcher's words for the line's own commands, as words.json
    // gives them.
    readonly property var said: ({
        quit: "Close the Launcher",
        help: "List the Keys",
        row: "Go to a Row"
    })

    // A network's acts, as Wireless.qml makes them: Return's first, the
    // ending act d does, and its copy.
    function network() {
        return [
            { name: "See It", verb: "see" },
            { name: "Disconnect It", verb: "disconnect", ends: "disconnects" },
            { name: "Forget It", verb: "forget", act: "ending", ends: "forgets" },
            { name: "Copy the Name", verb: "copy", copy: "Home-5G" }
        ];
    }

    // The menu for the network's row and a surface with :sort, by what each
    // line says: its notation, its title and its key.
    function menuOf(acts, surface) {
        return Commands.listed({
            acts: acts ?? network(),
            surface: surface ?? [],
            table: Commands.standIn,
            said: said,
            row: 3
        });
    }

    function lines(entries) {
        return entries.map(entry => [Commands.notation(entry.command, entry.short), entry.title,
            entry.key]);
    }

    function run() {
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        // Vim's help notation: the shortest form outside the brackets.
        holds("a short form leads its name", Commands.notation("disconnect", "disc"),
            "disc[onnect]");
        holds("a name with no shorter form stands whole", Commands.notation("dial", "dial"),
            "dial");
        holds("one letter is a short form too", Commands.notation("quit", "q"), "q[uit]");

        // The menu: the row's acts, Return's first, then the surface's, then
        // what every surface has.
        const sort = { command: "sort", short: "sor", title: "Sort the Rows", key: "",
            run: () => {} };
        const menu = menuOf(network(), [sort]);
        holds("the menu lists the row's acts, the surface's, then every surface's", lines(menu), [
            ["see", "See It", "\u21b5"],
            ["disc[onnect]", "Disconnect It", ""],
            ["forg[et]", "Forget It", "d"],
            ["copy", "Copy the Name", "y"],
            ["sor[t]", "Sort the Rows", ""],
            ["q[uit]", "Close the Launcher", "ZZ"],
            ["h[elp]", "List the Keys", "F1"],
            ["{n}", "Go to a Row", ""]
        ]);
        holds("each act's line holds the act it runs", menu[1]?.act?.verb, "disconnect");
        holds("an act named twice is listed once",
            lines(menuOf(network().concat([{ name: "Disconnect Now", verb: "disconnect" }])))
                .filter(line => line[0] === "disc[onnect]").length, 1);
        holds("an act with no verb is named by its name's first word",
            menuOf([{ name: "Pause Music" }])[0].command, "pause");
        holds("an act whose verb is more words is named by its first",
            menuOf([{ name: "Sign In", verb: "sign in" }])[0].command, "sign");
        holds("a row with no acts lists only what every surface has",
            lines(menuOf([])).map(line => line[0]), ["q[uit]", "h[elp]", "{n}"]);

        // Typing narrows it: by the command's start, then by the act's words.
        const shown = typed => Commands.narrowed(menu, typed).map(entry => entry.command);
        holds("nothing typed shows the whole menu", shown("").length, menu.length);
        holds("typing narrows by the command's start", shown("d"), ["disconnect"]);
        holds("then by the act's words", shown("it"),
            ["see", "disconnect", "forget"]);
        holds("an act's words find it where its command doesn't", shown("r"),
            ["sort"]);
        holds("each line is listed once", shown("s"), ["see", "sort"]);
        holds("case is ignored, as his ignorecase", shown("DISC"), ["disconnect"]);
        holds("a count shows where it goes", shown("12"), ["{n}"]);
        holds("a name and a space show that command alone", shown("disc "), ["disconnect"]);
        holds("a word nothing holds shows nothing", shown("wifi"), []);
        holds("the colons and spaces before a command are passed over", shown(": :d"),
            ["disconnect"]);
        holds("! after a name still finds it", shown("forg!"), ["forget"]);

        // Tab and the arrows choose, writing the chosen line in, and come
        // back round to what was typed, as his cmp and Vim's wildmenu do.
        const narrowed = Commands.narrowed(menu, "");
        let choosing = Commands.typing("");
        const steps = [];
        for (let step = 0; step < 3; step++) {
            choosing = Commands.chosen(choosing, 1, narrowed);
            steps.push([choosing.at, choosing.line]);
        }
        holds("Tab chooses the next line, writing its command in", steps,
            [[0, "see"], [1, "disconnect"], [2, "forget"]]);
        choosing = Commands.chosen(choosing, -1, narrowed);
        holds("Shift+Tab chooses the one before", [choosing.at, choosing.line],
            [1, "disconnect"]);
        let round = Commands.typing("d");
        const one = Commands.narrowed(menu, "d");
        round = Commands.chosen(round, 1, one);
        holds("past the last line it comes back to what was typed",
            Commands.chosen(round, 1, one), { typed: "d", at: -1, line: "d" });
        holds("before the first it goes to the last",
            Commands.chosen(Commands.typing("d"), -1, one), { typed: "d", at: 0,
                line: "disconnect" });
        holds("the {n} line writes the cursor row's number",
            Commands.chosen(Commands.typing("1"), 1, Commands.narrowed(menu, "1")).line, "3");
        holds("a chosen line keeps its ! and what follows its name",
            Commands.chosen(Commands.typing("forg!"), 1, Commands.narrowed(menu, "forg!")).line,
            "forget!");
        holds("a menu with nothing in it chooses nothing",
            Commands.chosen(Commands.typing("wifi"), 1, []), { typed: "wifi", at: -1,
                line: "wifi" });
        holds("Ctrl+E puts back what was typed", Commands.dropped({ typed: "", at: 2,
            line: "forget" }), { typed: "", at: -1, line: "" });
        holds("Ctrl+Y takes what's chosen as typed", Commands.taken({ typed: "", at: 2,
            line: "forget" }), { typed: "forget", at: -1, line: "forget" });

        // Return runs exactly what's typed.
        const ran = line => {
            const read = Commands.ran(line, menu);
            return read.error ?? read.go ?? `${read.entry.command}${read.bang ? "!" : ""}`
                + (read.arg !== "" ? ` ${read.arg}` : "");
        };
        holds("a whole name runs its act", ran("disconnect"), "disconnect");
        holds("so does its short form", ran("disc"), "disconnect");
        holds("and anything between", ran("discon"), "disconnect");
        holds("shorter than its short form is no command", ran("dis"),
            "E492: Not an editor command: dis");
        holds("past its name is no command", ran("disconnected"),
            "E492: Not an editor command: disconnected");
        holds("a word that isn't a command says so, whatever the menu shows", ran("wifi"),
            "E492: Not an editor command: wifi");
        holds("the whole line is said", ran("wifi off"), "E492: Not an editor command: wifi off");
        holds("a command off the menu is no command here", ran("pair"),
            "E492: Not an editor command: pair");
        holds(":q closes", ran("q"), "quit");
        holds(":q! closes too", ran("q!"), "quit!");
        holds("the colons and spaces before it are passed over", ran(" : q"), "quit");
        holds(":help runs with or without a topic", [ran("h"), ran("help dd")],
            ["help", "help dd"]);
        holds("a number goes to that row", ran("12"), 12);
        holds(":$ goes to the last", ran("$"), "$");
        holds("a time goes there, as a player's does", [ran("12:30"), ran("1:02:03")],
            ["12:30", "1:02:03"]);
        holds("what only starts as a time is no command", ran("12:"),
            "E492: Not an editor command: 12:");
        holds("nothing typed does nothing", ran(""), "");
        holds("! where it isn't taken says so", ran("see!"), "E477: No ! allowed");
        holds("words after an act that takes none say so", ran("disc now"),
            "E488: Trailing characters: now");
        holds("an act that asks takes !, which doesn't", ran("forg!"), "forget!");
        holds("a surface's command takes what follows it", ran("sort name"), "sort name");

        // Ctrl+W deletes the word before the caret, as Vim's command line's.
        holds("Ctrl+W takes the word before the caret", [Commands.wordStart("set volume", 10),
            Commands.wordStart("set volume+=", 12), Commands.wordStart("q  ", 3)], [4, 10, 0]);

        // Each key's work is a fraction of the 4 ms a key has, here over a
        // menu of 400 commands, timed as the fastest of twenty batches of
        // fifty, since a check shares its machine: a slower batch is its
        // other work or the collector, not the line's own cost. The menu is
        // listed once, as : opens it, which is that key's work with the
        // narrowing that follows; every key after narrows and reads it, held
        // to half its 4 ms, since the line has more to draw than this. Both
        // summed to half once, which a busy builder broke by a tenth of a
        // millisecond, the colon's listing counted against keys that never
        // list.
        const many = [];
        for (let at = 0; at < 400; at++)
            many.push({ command: `cmd${at}`, short: `cmd${at}`, title: `Command ${at}`, key: "",
                run: () => {} });
        const timed = work => {
            let fastest = Infinity;
            for (let batch = 0; batch < 20; batch++) {
                const began = Date.now();
                for (let at = 0; at < 50; at++)
                    work();
                fastest = Math.min(fastest, (Date.now() - began) / 50);
            }
            return fastest;
        };
        const listing = timed(() => menuOf(network(), many));
        const big = menuOf(network(), many);
        const keyed = timed(() => {
            Commands.narrowed(big, "cmd3");
            Commands.ran("cmd399", big);
        });
        console.log(`400 commands listed in ${listing.toFixed(3)} ms, narrowed and read in`
            + ` ${keyed.toFixed(3)} ms`);
        holds("400 commands listed and narrowed within the colon's 4 ms", listing + keyed < 4,
            true);
        holds("and narrowed and read in half a key's 4 ms", keyed < 2, true);

        // :set, read as Vim reads its words (:help :set): each an option
        // on, off, the other way, set, moved by so much, or shown.
        const options = {
            wifi: { kind: "switch", takes: true },
            bluetooth: { kind: "switch" },
            quiet: { kind: "switch" },
            mute: { kind: "switch" },
            volume: { kind: "number", least: 0, most: 100 },
            brightness: { kind: "number", least: 0, most: 100 },
            background: { kind: "choice", values: ["light", "dark"] }
        };
        const set = arg => Commands.setting(arg, options);
        const first = arg => (set(arg).changes ?? [])[0] ?? null;
        holds(":set wifi turns it on", first("wifi"), { option: "wifi", how: "on", value: "" });
        holds(":set nowifi turns it off", first("nowifi"),
            { option: "wifi", how: "off", value: "" });
        holds(":set wifi! and invwifi turn it the other way",
            [first("wifi!")?.how, first("invwifi")?.how], ["toggle", "toggle"]);
        holds(":set wifi=Home-5G joins a network", first("wifi=Home-5G"),
            { option: "wifi", how: "set", value: "Home-5G" });
        holds("a network's spaces are escaped, as Vim's are", first("wifi=Home\\ 5G")?.value,
            "Home 5G");
        holds(":set volume=40 sets it", first("volume=40"),
            { option: "volume", how: "set", value: 40 });
        holds(":set volume+=5 and -=5 move it",
            [first("volume+=5"), first("volume-=5")],
            [{ option: "volume", how: "add", value: 5 },
                { option: "volume", how: "subtract", value: 5 }]);
        holds(":set volume? and :set volume say it", [first("volume?")?.how, first("volume")?.how],
            ["show", "show"]);
        holds(":set wifi? says it too", first("wifi?")?.how, "show");
        holds(":set background=dark", first("background=dark")?.value, "dark");
        holds(":set all says every one", first("all"), { option: "all", how: "show", value: "" });
        holds("several at once, in order", (set("nowifi mute").changes ?? []).map(c => c.option),
            ["wifi", "mute"]);
        holds("an option it hasn't says E518", set("wfii"), { error: "E518: Unknown option: wfii" });
        holds("and so does one after a good one, changing none", set("mute wfii"),
            { error: "E518: Unknown option: wfii" });
        holds("a word for a number says E521", set("volume=loud"),
            { error: "E521: Number required after =: volume=loud" });
        holds("a number past its ends says E474", set("volume=140"),
            { error: "E474: Invalid argument: volume=140" });
        holds("a choice it hasn't says E474", set("background=blue"),
            { error: "E474: Invalid argument: background=blue" });
        holds("no before what isn't a switch says E474", set("novolume"),
            { error: "E474: Invalid argument: novolume" });
        holds("a switch given a value it takes none of says E474", set("mute=1"),
            { error: "E474: Invalid argument: mute=1" });
        holds("+= on a switch says E474", set("wifi+=1"),
            { error: "E474: Invalid argument: wifi+=1" });
        holds("nothing after :set says every one changed, as Vim's does", first(""),
            { option: "all", how: "show", value: "" });

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

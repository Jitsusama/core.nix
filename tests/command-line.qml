// The launcher's command line, the : line, held in Qt's own JavaScript
// engine, the one the shell runs it in: what its menu lists for a row and
// a surface, how typing narrows it, what Tab and the arrows choose and
// write in, and what Return runs, or the error it says, as Vim's command
// line would, and how its :set reads and sets the launcher's options. The
// `the-command-line-lists-every-command` check copies
// modules/quickshell/Commands.js, SetWords.js, Ex.js, CommandBinding.js and
// Operators.js beside this file and runs
// `qml -platform offscreen command-line.qml`.
import QtQuick
import "Commands.js" as Commands
import "SetWords.js" as SetWords
import "CommandBinding.js" as CommandBinding

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
        set: "Set an Option",
        setlocal: "Set an Option of This",
        history: "List Earlier Lines",
        row: "Go to a Row"
    })

    // The key language's commands as the shell has them (Options.commands,
    // from modules/keys/language.toml), by their whole names, a flag left
    // out where the language leaves it out.
    readonly property var commands: ({
        quit: { act: "close", short: "q" },
        help: { act: "help", short: "h" },
        set: { act: "set", short: "se", changes: true, undoable: true },
        setlocal: { act: "setlocal", short: "setl", changes: true, undoable: true,
            range: true },
        history: { act: "history", short: "his" },
        sort: { act: "sort", short: "sor", changes: true, undoable: true, range: true },
        open: { act: "open-it", short: "ope" },
        disconnect: { act: "disconnect", short: "disc", changes: true, undoable: true,
            range: true },
        forget: { act: "forget", short: "forg", changes: true, range: true },
        suspend: { act: "suspend", short: "sus", changes: true },
        close: { act: "close-window", short: "clo", changes: true, guarded: true, range: true }
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
            table: commands,
            said: said,
            row: 3
        });
    }

    function holds(what, got, want) {
        if (JSON.stringify(got) !== JSON.stringify(want))
            failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
    }

    function lines(entries) {
        return entries.map(entry => [Commands.called(entry), entry.title, entry.key]);
    }

    function run() {
        // Vim's help notation: the shortest form outside the brackets.
        holds("a short form leads its name", Commands.notation("disconnect", "disc"),
            "disc[onnect]");
        holds("a name with no shorter form stands whole", Commands.notation("dial", "dial"),
            "dial");
        holds("one letter is a short form too", Commands.notation("quit", "q"), "q[uit]");

        // The menu: the row's acts, Return's first, then the surface's, then
        // what every surface has.
        const sort = { command: "sort", short: "sor", title: "Sort the Rows", key: "",
            args: [{ value: "name", note: "A to Z" }, { value: "changed", note: "Newest First" }],
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
            ["se[t]", "Set an Option", ""],
            ["setl[ocal]", "Set an Option of This", ""],
            ["his[tory]", "List Earlier Lines", ""],
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
            lines(menuOf([])).map(line => line[0]),
            ["q[uit]", "h[elp]", "se[t]", "setl[ocal]", "his[tory]", "{n}"]);

        // Typing narrows it: by the command's start, then by the act's words.
        const shown = typed => Commands.narrowed(menu, typed).map(entry => entry.command);
        holds("nothing typed shows the whole menu", shown("").length, menu.length);
        holds("typing narrows by the command's start", shown("d"), ["disconnect"]);
        holds("then by the act's words", shown("it"),
            ["see", "disconnect", "forget"]);
        holds("an act's words find it where its command doesn't", shown("r"),
            ["sort"]);
        holds("each line is listed once", shown("s"), ["see", "sort", "set", "setlocal"]);
        holds(":set's short form is its own", Commands.ran("se nowifi", menu).entry?.kind, "set");
        // A line asking for words, as a talk line, leaves on what the command
        // line reads as :quit, and gives every other line as it's typed.
        const leaving = words => CommandBinding.leaves(words, commands);
        holds(":q typed in a line asking words leaves it",
            [":q", ":q!", ":quit", " :q "].map(leaving), [true, true, true, true]);
        holds("words that only hold :q are given",
            ["q", "hello :q", ":quiet please", ":qa", ":clo", ":", ""].map(leaving),
            [false, false, false, false, false, false, false]);
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
        holds("an act that changes nothing asks nothing, so takes no !",
            Commands.ran("ope!", menuOf([{ name: "Open It", verb: "open" }])).error,
            "E477: No ! allowed");
        // A window's Close, whose words name it :close, the language's
        // close-window.
        const closing = Commands.listed({ acts: [{ name: "Close It", verb: "close" }],
            table: commands, said: said, called: () => ({ command: "close" }) });
        holds("a guarded act asks nothing on one thing, once on more, so takes !",
            ["undoable", "guarded", "bang"].map(flag => closing[0][flag]), [false, true, true]);
        // As d asks (Operators.asks): what changes asks unless the launcher
        // can put it back, the language's undoable or not.
        const back = { name: "Disconnect It", verb: "disconnect", undo: () => {} };
        holds("what asks is what changes and nothing puts back",
            menuOf([{ name: "Open It", verb: "open" }, { name: "Forget It", verb: "forget" },
                { name: "Disconnect It", verb: "disconnect" }, { name: "See It", verb: "see" }])
                .slice(0, 4).map(entry => entry.undoable), [true, false, false, true]);
        holds("so an undoable act takes ! until it says how it's put back",
            [ran("disc!"), Commands.ran("disc!", menuOf([back])).error],
            ["disconnect!", "E477: No ! allowed"]);
        // A bare close, its words naming no command, only closes the
        // launcher, as Provider's row with nothing to copy does: it's :quit,
        // never the language's close-window (1JQ9TO's finding, 2026-11-05),
        // and stands for the :q every surface has.
        const quitting = menuOf([{ name: "Close", verb: "close" }]);
        holds("a bare close is the launcher's :quit, listed once",
            lines(quitting).filter(line => line[0] === "q[uit]"), [["q[uit]", "Close", "\u21b5"]]);
        holds("and asks nothing", [quitting[0].undoable, quitting[0].bang], [true, true]);
        holds("a surface's command takes what follows it", ran("sort name"), "sort name");

        // Ctrl+W deletes the word before the caret, as Vim's command line's.
        holds("Ctrl+W takes the word before the caret", [Commands.wordStart("set volume", 10),
            Commands.wordStart("set volume+=", 12), Commands.wordStart("q  ", 3)], [4, 10, 0]);

        // Each key's work over a menu of 400 commands, held to a few times
        // the least that work could be: listing it against copying each
        // command into a line with its title lowered and a set of names
        // seen, and narrowing and reading it against one pass that sorts
        // the lines by start and word and finds one by name. A key's 4 ms
        // can't be held in milliseconds on a shared builder, where the same
        // listing took from 1.5 to 3.6 ms; its cost against a floor timed in
        // turn with it, in the same batch, meets the same moment's load, and
        // stayed within twice the floor at load averages of 40 to 95. What
        // it catches is the line doing several times the least it could:
        // a pattern made for each word of each title as it's listed came to
        // about three times the copy, and a seen-check by search ran past
        // the check's minute.
        const most = 3;
        const many = [];
        for (let at = 0; at < 400; at++)
            many.push({ command: `cmd${at}`, short: `cmd${at}`, title: `Command ${at}`, key: "",
                run: () => {} });
        // The floor and the work, timed in turn in each of twenty batches,
        // each by the fastest batch, since a slower one is the builder's
        // other work or the collector, not the line's own cost.
        const against = (floor, work, size) => {
            let least = Infinity;
            let took = Infinity;
            for (let batch = 0; batch < 20; batch++) {
                let began = Date.now();
                for (let at = 0; at < size; at++)
                    floor();
                least = Math.min(least, (Date.now() - began) / size);
                began = Date.now();
                for (let at = 0; at < size; at++)
                    work();
                took = Math.min(took, (Date.now() - began) / size);
            }
            return { least, took };
        };
        const copied = () => {
            const seen = new Set();
            const menu = [];
            for (const own of many) {
                if (seen.has(own.command))
                    continue;
                seen.add(own.command);
                menu.push({ command: own.command, short: own.short, title: own.title,
                    key: own.key, own, lowered: ` ${own.title.toLowerCase()}` });
            }
            return menu;
        };
        const big = menuOf(network(), many);
        const sorted = () => {
            const starting = [];
            const worded = [];
            for (const entry of big) {
                if (entry.command.startsWith("cmd3"))
                    starting.push(entry);
                else if (entry.lowered.includes(" cmd3"))
                    worded.push(entry);
            }
            return [starting.concat(worded), big.filter(one => one.command === "cmd399")];
        };
        // Narrowing is a tenth of listing, so it's timed over batches of a
        // hundred, for a floor the clock's millisecond can tell.
        const listing = against(copied, () => menuOf(network(), many), 50);
        const keyed = against(sorted, () => {
            Commands.narrowed(big, "cmd3");
            Commands.ran("cmd399", big);
        }, 100);
        console.log(`400 commands listed in ${listing.took.toFixed(3)} ms against a copy's`
            + ` ${listing.least.toFixed(3)} ms, narrowed and read in ${keyed.took.toFixed(3)} ms`
            + ` against a pass's ${keyed.least.toFixed(3)} ms`);
        holds(`400 commands listed within ${most} times a copy of them`,
            listing.took < most * listing.least, true);
        holds(`and narrowed and read within ${most} times a pass over them`,
            keyed.took < most * keyed.least, true);

        // :set, read as Vim reads its words (:help :set): each an option
        // on, off, the other way, set, moved by so much, or shown.
        const options = {
            wifi: { kind: "switch", takes: true },
            bluetooth: { kind: "switch" },
            dnd: { kind: "switch", takes: "number", least: 1 },
            mute: { kind: "switch" },
            volume: { kind: "number", least: 0, most: 100 },
            brightness: { kind: "number", least: 0, most: 100 },
            background: { kind: "choice", values: ["light", "dark"] }
        };
        const set = arg => SetWords.setting(arg, options);
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
        const clocked = Object.assign({ position: { kind: "number", least: 0, clock: true } },
            options);
        const placed = arg => (SetWords.setting(arg, clocked).changes ?? [])[0]?.value;
        holds(":set position=1:30 reads a clock, in seconds, and 1:02:05 past the hour",
            [placed("position=1:30"), placed("position=1:02:05"), placed("position+=10")],
            [90, 3725, 10]);
        holds("but only where the option keeps time", set("volume=1:30"),
            { error: "E521: Number required after =: volume=1:30" });
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

        // What a change sets, from where the option stands now.
        const toward = (arg, now) => {
            const change = first(arg);
            return SetWords.toward(change, now, options[change.option]);
        };
        holds("on and off set a switch", [toward("wifi", false), toward("nowifi", true)],
            [{ to: true }, { to: false }]);
        holds("toggling turns it the other way", [toward("wifi!", true), toward("wifi!", false)],
            [{ to: false }, { to: true }]);
        holds("a joined network toggles off", toward("wifi!", "Home-5G"), { to: false });
        holds("a network is set by its name", toward("wifi=Home-5G", true), { to: "Home-5G" });
        holds("+= and -= move a number from where it is",
            [toward("volume+=5", 40), toward("volume-=5", 40)], [{ to: 45 }, { to: 35 }]);
        holds("and stop at its ends", [toward("volume+=20", 90), toward("volume-=20", 10)],
            [{ to: 100 }, { to: 0 }]);
        holds("= sets it", toward("volume=40", 70), { to: 40 });
        holds("a choice is set", toward("background=light", "dark"), { to: "light" });
        holds("an option that can't be set says E519",
            SetWords.toward(first("background=light"), "dark",
                { kind: "choice", values: ["light", "dark"], fixed: true }),
            { error: "E519: Option not supported: background=light" });

        // A space after a name lists its arguments, each replacing the word
        // being typed: :set's options with where each stands, the values
        // after one's =, and a surface command's own.
        const standing = Object.assign({}, options, {
            wifi: Object.assign({}, options.wifi, { now: () => "Home-5G",
                choices: () => ["Home-5G", "Cafe Wifi"] }),
            volume: Object.assign({}, options.volume, { now: () => 40 }),
            mute: Object.assign({}, options.mute, { now: () => false }),
            dnd: Object.assign({}, options.dnd, { now: () => true }),
            bluetooth: Object.assign({}, options.bluetooth, { now: () => true }),
            brightness: Object.assign({}, options.brightness, { now: () => 60 }),
            background: Object.assign({}, options.background, { now: () => "dark" })
        });
        const argued = (entry, arg) => entry.kind === "set"
            ? SetWords.setArguments(arg, standing) : Commands.ownArguments(entry, arg);
        const listedAs = typed => Commands.narrowed(menu, typed, argued)
            .map(line => [line.command, line.title]);
        holds(":set and a space lists every option and where it stands", listedAs("set "), [
            ["background", "background=dark"], ["bluetooth", "bluetooth"],
            ["brightness", "brightness=60"], ["dnd", "dnd"], ["mute", "nomute"],
            ["volume", "volume=40"], ["wifi", "wifi=Home-5G"]
        ]);
        holds("typing narrows them", listedAs("set b").map(line => line[0]),
            ["background", "bluetooth", "brightness"]);
        holds("a word typed after another narrows on its own",
            listedAs("set nomute v").map(line => line[0]), ["volume"]);
        holds("an = lists the values it takes", listedAs("set background="),
            [["background=light", ""], ["background=dark", ""]]);
        holds("and a network's, escaped", listedAs("se wifi=").map(line => line[0]),
            ["wifi=Home-5G", "wifi=Cafe\\ Wifi"]);
        holds("narrowed by what's typed after it", listedAs("set wifi=c").map(line => line[0]),
            ["wifi=Cafe\\ Wifi"]);
        holds("a number lists nothing to choose", listedAs("set volume="), []);
        holds("a surface command lists its own", listedAs("sort "),
            [["name", "A to Z"], ["changed", "Newest First"]]);
        // An argument that reads as a row, as :filter's do, keeps its count,
        // mark and hue; one with none, as :set's, stays bare.
        const rowed = (whole, typed) => Commands.narrowed(whole, typed, argued)
            .map(line => [line.command, line.key, line.glyph, line.hue]);
        const filter = { command: "filter", short: "filt", title: "Show Only", key: "",
            args: [{ value: "look", note: "To Look At", side: "4", glyph: "E", hue: "accent" }],
            run: () => {} };
        holds("an argument read as a row keeps its count, mark and hue",
            rowed(menuOf(network(), [filter]), "filter "), [["look", "4", "E", "accent"]]);
        holds("and one with none of them stays bare", rowed(menu, "set m"),
            [["mute", "", "", ""]]);
        holds("choosing one writes it in place of the word",
            Commands.chosen(Commands.typing("set wifi=c"), 1,
                Commands.narrowed(menu, "set wifi=c", argued)).line, "set wifi=Cafe\\ Wifi");
        holds("and keeps what came before it",
            Commands.chosen(Commands.typing("set nomute v"), 1,
                Commands.narrowed(menu, "set nomute v", argued)).line, "set nomute volume");
        holds("a command that takes none shows its own line alone",
            listedAs("disc ").map(line => line[0]), ["disconnect"]);

        // An act is named by the command its words give it, and two on a
        // row that share one differ by argument, as :new and :new . do.
        const words = {
            "apps:new": { command: "new" },
            "folder:new": { command: "new", argument: "." },
            "window:third": { command: "resize", argument: "third" },
            "bluetooth:connect": { command: "set", argument: "bluetooth+=" }
        };
        const calledMenu = acts => Commands.listed({ acts, table: commands, said,
            called: act => words[act.key] ?? {}, row: 1 });
        const windows = calledMenu([{ key: "apps:new", name: "New Window", verb: "start" },
            { key: "folder:new", name: "New Window Here", verb: "start" }]);
        holds("an act is named by its words' command, its verb where they give none",
            lines(windows).slice(0, 2).concat(lines(menuOf([{ name: "See It", verb: "see" }]))
                .slice(0, 1)),
            [["new", "New Window", "\u21b5"], ["new .", "New Window Here", ""],
                ["see", "See It", "\u21b5"]]);
        const titled = (typed, menu) => {
            const read = Commands.ran(typed, menu);
            return read.error ?? read.entry.title;
        };
        holds("a command alone runs its act with no argument, with one the act holding it",
            [titled("new", windows), titled("new .", windows), titled("new  . ", windows)],
            ["New Window", "New Window Here", "New Window Here"]);
        holds("an argument no act holds is trailing", titled("new x", windows),
            "E488: Trailing characters: x");
        holds("an act that needs its argument says so without it",
            titled("resize", calledMenu([{ key: "window:third", name: "A Third" }])),
            "E471: Argument required");
        holds("choosing writes an act's argument in",
            Commands.chosen(Commands.typing("new"), 2, Commands.narrowed(windows, "new")).line,
            "new .");
        holds("a space after it narrows them by their arguments",
            lines(Commands.narrowed(windows, "new .", argued)).map(line => line[0]),
            ["new ."]);
        const paired = calledMenu([{ key: "bluetooth:connect", name: "Connect It" }]);
        holds("a row's own act on :set runs ahead of :set, which runs the rest",
            [titled("set bluetooth+=", paired), Commands.ran("set nowifi", paired).entry?.kind],
            ["Connect It", "set"]);
        holds("and :set still lists its options after a space",
            Commands.narrowed(paired, "set w", argued).map(line => line.command), ["wifi"]);

        // A bang its words give is the act's own, as :skip! goes back where
        // :skip goes on; it's written in and only a bang typed runs it.
        const said2 = {
            "track:next": { command: "skip" },
            "track:previous": { command: "skip", bang: true },
            "notice:forget": { command: "forget" },
            "notices:forget": { command: "forget", range: "%" },
            "wifi:join": { command: "set", argument: "wifi={name}" },
            "sound:port": { command: "set", argument: "port={port}" },
            "player:other": { command: "pause", argument: "{player}" }
        };
        const filledMenu = (acts, thing) => Commands.listed({ acts, table: commands, said,
            called: act => said2[act.key] ?? {}, row: 1, thing });
        const tracks = filledMenu([{ key: "track:next", name: "Next" },
            { key: "track:previous", name: "Previous" }]);
        holds("an act's own bang is written on its line",
            lines(tracks).slice(0, 2).map(line => line[0]), ["skip", "skip!"]);
        holds("and runs only where a bang is typed",
            [titled("skip", tracks), titled("skip!", tracks), titled("skip !", tracks)],
            ["Next", "Previous", "E488: Trailing characters: !"]);
        holds("choosing writes its bang in",
            Commands.chosen(Commands.typing("ski"), 2, Commands.narrowed(tracks, "ski")).line,
            "skip!");
        const forgetting = filledMenu([{ key: "notice:forget", name: "Forget It" },
            { key: "notices:forget", name: "Forget Them All" }]);
        holds("an act on every row is written with its range",
            lines(forgetting).slice(0, 2).map(line => line[0]), ["forg[et]", "%forg[et]"]);
        holds("and runs only with every row typed",
            [titled("forg", forgetting), titled("%forg", forgetting),
                titled("%forget", forgetting)], ["Forget It", "Forget Them All",
                "Forget Them All"]);
        const joining = filledMenu([{ key: "wifi:join", name: "Connect" }],
            { name: "Home 5G" });
        holds("what's in braces is filled from the row, a space escaped as Vim's",
            [lines(joining)[0][0], titled("set wifi=Home\\ 5G", joining)],
            ["se[t] wifi=Home\\ 5G", "Connect"]);
        holds("the act's own fill comes first",
            lines(filledMenu([{ key: "sound:port", name: "Speakers", fill: { port: "Line" } }],
                { name: "Card", port: "Jack" }))[0][0], "se[t] port=Line");
        holds("words it can't fill give way to its verb",
            lines(filledMenu([{ key: "player:other", name: "Pause the Other", verb: "pause" }],
                { name: "Playing" }))[0][0], "pause");

        // An answer after the line is done, as a screen's setting niri
        // refused: said on the line while it holds the keys that ran it,
        // in the journal once Joel has typed on, and nothing where it
        // has no words.
        const refused = "1.3\u00d7 not set \u00b7 HP offers 1\u00d7 to 3\u00d7";
        holds("a late refusal is said while the keys are still the line's",
            SetWords.later("scale", refused, 7, 7), { say: refused });
        holds("a late refusal after Joel typed on goes to the journal alone",
            SetWords.later("scale", refused, 7, 9),
            { journal: `launcher: scale refused after Joel typed on: ${refused}` });
        holds("a late answer with no words does nothing", SetWords.later("scale", "", 7, 9),
            null);

        // Each option as Vim's :set volume? says it.
        holds("a switch on says its name, off its no",
            [SetWords.shown("dnd", true, options.dnd),
                SetWords.shown("dnd", false, options.dnd)], ["dnd", "nodnd"]);
        holds("a network joined says its name", SetWords.shown("wifi", "Home-5G", options.wifi),
            "wifi=Home-5G");
        holds("a number and a choice say theirs",
            [SetWords.shown("volume", 40, options.volume),
                SetWords.shown("background", "dark", options.background)],
            ["volume=40", "background=dark"]);

        holdMinutes(options);
        holdSteps();
        holdRanges();
        holdWords();

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // An act's words for the line by its key, as words.json has them: a
    // verb's own, or the verb's for one of several made from it; a row that
    // is an act itself, as Lock, its own, and the step it asks first, as
    // Power Off's sure, its row's; one in a state, as Wi-Fi On, its state's,
    // so the line says what runs, never a toggle; a row whose key never walks
    // back to its words, as a display's Warmth, by the id it names them by;
    // none for a key whose words name none.
    function holdWords() {
        const vocabulary = {
            "wifi:radio": { id: "wifi:radio", command: "set", argument: "nowifi",
                states: { off: { argument: "wifi" } } },
            "verb:sound-port": { id: "verb:sound-port", command: "set",
                argument: "port={port}" },
            "displays:warmth": { id: "displays:warmth", command: "set", argument: "nowarmth",
                states: { off: { argument: "warmth" } } },
            "keyboards:lights:to": { id: "keyboards:lights:to", command: "set",
                argument: "kbdlevel={n}" },
            "session:Lock": { id: "session:Lock", command: "lockscreen" },
            "session:Power Off": { id: "session:Power Off", command: "poweroff" },
            "session:quiet": { id: "session:quiet" }
        };
        const of = (id, state) => Object.assign({ id }, vocabulary[id] ?? {},
            ((vocabulary[id] ?? {}).states ?? {})[state] ?? {});
        const named = key => CommandBinding.wordsFor({ key }, of).command ?? "";
        const set = state => CommandBinding.wordsFor({ key: "wifi:radio", wordState: state },
            of).argument ?? "";
        holds("a row in a state names its state's", [set(""), set("off")], ["nowifi", "wifi"]);
        holds("a verb made one of several reads the verb's", named("verb:sound-port-2"), "set");
        holds("a row that's an act names its own", named("session:Lock"), "lockscreen");
        holds("the step a row asks first names the row's", named("session:Power Off:sure"),
            "poweroff");
        const warmth = state => CommandBinding.wordsFor({ key: "displays:eDP-1:warmth",
            wordsId: "displays:warmth", wordState: state }, of).argument ?? "";
        holds("a row names its words by their id", [warmth(""), warmth("off"),
            CommandBinding.wordsFor({ key: "verb:keyboards-lights-2",
                wordsId: "keyboards:lights:to" }, of).argument ?? ""],
        ["nowarmth", "warmth", "kbdlevel={n}"]);
        holds("a row whose words name none has none",
            [named("session:quiet"), named("session:quiet:60"), named("")], ["", "", ""]);
    }

    // A range before a command, read as Vim's (Ex.js): Return says what it
    // names for the launcher to bind to things, narrowing and choosing keep
    // it, and a command that takes no rows says E481; :g and :v name a
    // pattern and the command run on each row it finds, or doesn't.
    function holdRanges() {
        const menu = menuOf();
        const ran = typed => {
            const read = Commands.ran(typed, menu);
            return read.error ?? [read.entry.command, JSON.stringify(read.range ?? null)];
        };
        const from = (one, two) => JSON.stringify({ from: one, to: two ?? one });
        holds("a range is read off before the command",
            [ran("3,5disc"), ran("%forg"), ran("'<,'>disc"), ran(".,+2 disc")],
            [["disconnect", from({ at: 3, offset: 0 }, { at: 5, offset: 0 })],
                ["forget", JSON.stringify("%")],
                ["disconnect", from({ at: "'<", offset: 0 }, { at: "'>", offset: 0 })],
                ["disconnect", from({ at: ".", offset: 0 }, { at: ".", offset: 2 })]]);
        holds("no range is none, :{n} still goes to its row",
            [ran("disc"), Commands.ran("12", menu).go], [["disconnect", "null"], 12]);
        holds("a command that takes no rows says so given a range",
            [ran("3,5see"), ran("%q"), ran("%set nowifi")],
            ["E481: No range allowed", "E481: No range allowed", "E481: No range allowed"]);
        holds("narrowing reads past the range",
            Commands.narrowed(menu, "3,5dis").map(entry => entry.command), ["disconnect"]);
        holds("choosing keeps it",
            Commands.chosen(Commands.typing("3,5dis"), 1, Commands.narrowed(menu, "3,5dis"))
                .line, "3,5disconnect");

        const global = typed => {
            const read = Commands.ran(typed, menu);
            return read.error ?? [read.global.pattern, read.global.invert, read.entry.command,
                JSON.stringify(read.range)];
        };
        holds(":g runs a command on each row it finds, every row with no range",
            [global("g/Slack/disc"), global("3,5g/Slack/disc")],
            [["Slack", false, "disconnect", JSON.stringify("%")],
                ["Slack", false, "disconnect", from({ at: 3, offset: 0 }, { at: 5, offset: 0 })]]);
        holds(":v and :g! run it on each it doesn't",
            [global("v/Slack/forg")[1], global("g!/Slack/forg")[1]], [true, true]);
        holds("and say Vim's errors for what they can't take",
            [global("g/Slack/wat"), global("g/a/g/b/disc"), global("g")],
            ["E492: Not an editor command: wat", "E147: Cannot do :global recursive",
                "E35: No previous regular expression"]);

        // The rows a line names, as the launcher binds them to things as :
        // is pressed: the cursor row, a range's, or those :g finds by name,
        // ignoring case where the pattern has no capital, as smartcase does.
        const at = { names: ["Slack", "kitty", "Slack Huddle", "Files", "slack notes"],
            cursor: 1, marks: { "<": 2, ">": 3 } };
        const bound = typed => {
            const read = CommandBinding.bound(Commands.ran(typed, menu), at);
            return read.error ?? read.rows;
        };
        holds("no range binds the cursor row, a range the rows it names",
            [bound("disc"), bound("3,5disc"), bound("%forg"), bound("'<,'>disc"),
                bound(".,+2disc")], [[1], [2, 3, 4], [0, 1, 2, 3, 4], [2, 3], [1, 2, 3]]);
        holds(":g binds those it finds, :v those it doesn't, within its range",
            [bound("g/Slack/disc"), bound("g/slack/disc"), bound("v/slack/disc"),
                bound("2,3g/Slack/disc")], [[0, 2], [0, 2, 4], [1, 3], [2]]);
        holds("a range it can't bind says Vim's error",
            [bound("'a,'bdisc"), bound("7,9disc")], ["E20: Mark not set", "E16: Invalid range"]);

        // The act a line names on each thing it's bound to: by its words'
        // id where it has words, else by its command and title.
        const wordsOf = { "wifi:leave": { id: "verb:wifi-leave", command: "disconnect" } };
        const worded = Commands.listed({ acts: [{ key: "wifi:leave", name: "Leave It" }],
            table: commands, said, called: act => wordsOf[act.key] ?? {} });
        holds("each thing's own act is found by the line's words, else command and title",
            [CommandBinding.actFor(worded[0],
                [{ key: "x" }, { key: "wifi:leave", name: "Leave" }],
                act => wordsOf[act.key] ?? {})?.name,
                CommandBinding.actFor(Commands.ran("disc", menu).entry, network(),
                    () => ({}))?.name,
                CommandBinding.actFor(Commands.ran("disc", menu).entry,
                    [{ name: "See It", verb: "see" }], () => ({}))],
            ["Leave", "Disconnect It", undefined]);
        holdEarlier();
        holdAsking();
        holdLocal();
    }

    // :setl, as Vim's :setlocal sets the current buffer's option rather
    // than the global one (VKCF3Z, 2026-11-05): the option of the thing
    // under the cursor, or of the nearest thing it's inside that has one,
    // as Muted inside a program sets the program's; over a range, each
    // thing's own. A thing with none has none, so :setl there says E518
    // rather than setting the machine's.
    function holdLocal() {
        const menu = menuOf([]);
        const ran = typed => {
            const read = Commands.ran(typed, menu);
            return read.error ?? [read.entry.kind, read.arg, JSON.stringify(read.range ?? null)];
        };
        holds(":setl is its own command, taking a range, and :set still none",
            [ran("setl mute"), ran("setlocal nomute"), ran("%setl mute"), ran("%set mute")],
            [["setlocal", "mute", "null"], ["setlocal", "nomute", "null"],
                ["setlocal", "mute", JSON.stringify("%")], "E481: No range allowed"]);
        const mute = { kind: "switch", now: () => false };
        const program = { name: "mpv", options: { mute, volume: { kind: "number" } } };
        const device = { name: "Speakers", options: { mute, port: { kind: "choice" } } };
        const muted = { name: "Muted" };
        const files = { name: "Files" };
        const whose = found => found.map(options =>
            [program, device].find(thing => thing.options === options)?.name ?? "none");
        holds("one thing sets its own",
            whose(CommandBinding.localOptions([program], [device])), ["mpv"]);
        holds("an act inside a thing sets the thing's, the nearest first",
            whose(CommandBinding.localOptions([muted], [device, program])), ["mpv"]);
        holds("over a range, each thing its own, none for one that has none",
            whose(CommandBinding.localOptions([program, files, device], [device])),
            ["mpv", "none", "Speakers"]);
        holds("a thing with none, or none at all, says E518, never the machine's",
            [SetWords.setting("mute", CommandBinding.localOptions([files], [])[0]),
                SetWords.setting("port", CommandBinding.localOptions([], [])[0] ?? {})],
            [{ error: "E518: Unknown option: mute" }, { error: "E518: Unknown option: port" }]);
    }

    // A switch that takes minutes, as Do Not Disturb's dnd does (VKCF3Z,
    // 2026-11-05): dnd holds notices until it's ended, dnd=60 for an hour,
    // dnd+=60 an hour more, and nodnd ends it; held for some minutes, it
    // says how many are left.
    function holdMinutes(options) {
        const first = arg => (SetWords.setting(arg, options).changes ?? [])[0] ?? null;
        const toward = (arg, now) => {
            const change = first(arg);
            return change === null ? null : SetWords.toward(change, now, options[change.option]);
        };
        holds(":set dnd=60 holds it an hour, dnd+=60 an hour more",
            [first("dnd=60"), first("dnd+=60")],
            [{ option: "dnd", how: "set", value: 60 }, { option: "dnd", how: "add", value: 60 }]);
        holds("its minutes are a number", SetWords.setting("dnd=soon", options),
            { error: "E521: Number required after =: dnd=soon" });
        holds("of one minute at least", SetWords.setting("dnd=0", options),
            { error: "E474: Invalid argument: dnd=0" });
        holds("an hour more is added to the minutes left", toward("dnd+=60", 30), { to: 90 });
        holds("held until it's ended, it stays so; off, it holds for that long",
            [toward("dnd+=60", true), toward("dnd+=60", false)], [{ to: true }, { to: 60 }]);
        holds("an hour less never ends it, nor holds what's off",
            [toward("dnd-=60", 30), toward("dnd-=60", true), toward("dnd-=60", false)],
            [{ to: 1 }, { to: true }, { to: false }]);
        holds("nodnd and dnd! end it, whatever's left", [toward("nodnd", 30), toward("dnd!", 30)],
            [{ to: false }, { to: false }]);
        holds("held for some minutes, it says how many are left",
            SetWords.shown("dnd", 45, options.dnd), "dnd=45");
    }

    // A level only stepped, as the keyboards' lights where only a board
    // that can't be asked has them: kbdlevel+=1 steps it, kbdlevel=2 is
    // refused, since nothing there takes a level, and kbdlevel? can't say,
    // for the launcher to say so in its own words.
    function holdSteps() {
        const options = {
            kbdlevel: { kind: "number", least: 0, stepsOnly: true, now: () => null },
            level: { kind: "number", least: 0, most: 3, now: () => 2 }
        };
        const first = arg => (SetWords.setting(arg, options).changes ?? [])[0] ?? null;
        const toward = (arg, now) => {
            const change = first(arg);
            return change === null ? null : SetWords.toward(change, now, options[change.option]);
        };
        holds("a level only stepped takes no =", SetWords.setting("kbdlevel=2", options),
            { error: "E474: Invalid argument: kbdlevel=2" });
        holds("but steps either way, from where it can't say",
            [toward("kbdlevel+=1", null), toward("kbdlevel-=1", null)],
            [{ to: null }, { to: null }]);
        holds("a level read steps from where it stands", toward("level+=1", 2), { to: 3 });
        holds("one that can't be read says nothing of itself",
            SetWords.shown("kbdlevel", null, options.kbdlevel), null);
        holds("nor where it stands in the list",
            SetWords.setArguments("kb", options), [{ value: "kbdlevel", note: "" }]);
        const turns = { rotation: { kind: "choice", now: () => "upright",
            choices: () => [{ value: "upright", note: "upright" },
                { value: "mirrored-left", note: "mirrored, a quarter turn left" }] } };
        holds("a choice given with its words says them beside it",
            SetWords.setArguments("rotation=mi", turns),
            [{ value: "rotation=mirrored-left", note: "mirrored, a quarter turn left" }]);
    }

    // What a line asks before it runs, once, by the rule d asks by
    // (Operators.asks), so d, the line and the menu are one: an act that
    // changes asks unless the launcher can put it back (its undo) or it's
    // guarded and alone; once, on one thing or many, unless a ! says don't;
    // and where only some of what it's bound to have the act, once, naming
    // how many can, ! or not. None that can says so; what changes nothing
    // asks nothing.
    function holdAsking() {
        const menu = menuOf(network());
        const entry = typed => Commands.ran(typed, menu).entry;
        const acts = (count, act) => Array.from({ length: count }, () => act ?? {});
        const asking = (typed, count, all, bang, act) =>
            CommandBinding.asking(entry(typed), acts(count, act), { all, bang }, commands);
        const back = { undo: () => {} };
        holds("an act that changes asks, the language's undoable or not, on one or many",
            [asking("disc", 1, 1, false), asking("disc", 3, 3, false)],
            [{ ask: "one" }, { ask: "many" }]);
        holds("unless the launcher can put it back",
            [asking("disc", 1, 1, false, back), asking("disc", 3, 3, false, back)],
            [{ ask: "" }, { ask: "" }]);
        holds("one that can't asks once, on one or many, unless its ! says don't",
            [asking("forg", 1, 1, false), asking("forg", 3, 3, false),
                asking("forg", 3, 3, true)],
            [{ ask: "one" }, { ask: "many" }, { ask: "" }]);
        const guarded = { command: "close", undoable: false, guarded: true };
        holds("a guarded one asks only over more than one",
            [CommandBinding.asking(guarded, acts(1), { all: 1, bang: false }, commands),
                CommandBinding.asking(guarded, acts(2), { all: 2, bang: false }, commands)],
            [{ ask: "" }, { ask: "many" }]);
        holds("what changes nothing asks nothing", asking("see", 3, 3, false), { ask: "" });
        holds("where only some have it, it asks once, naming how many, ! or not",
            [asking("disc", 2, 3, false, back), asking("forg", 2, 3, true)],
            [{ ask: "some" }, { ask: "some" }]);
        holds("where none has it, it says so and asks nothing",
            asking("disc", 0, 3, false), { ask: "", none: true });
        // A row's :set runs its own act, as Return on it does, and asks
        // nothing, as a typed :set never asks (2V74KO, 2026-11-05): a
        // display's Warmth, by :set warmth, with no undo of its own.
        const warmth = Commands.listed({ acts: [{ key: "displays:warmth", name: "Warmth" }],
            table: commands, said, called: () => ({ command: "set", argument: "warmth" }),
            row: 1 });
        holds("a row's :set asks nothing, on one or many, and its line isn't banged",
            [CommandBinding.asking(warmth[0], acts(1), { all: 1, bang: false }, commands),
                CommandBinding.asking(warmth[0], acts(2), { all: 2, bang: false }, commands),
                warmth[0].bang],
            [{ ask: "" }, { ask: "" }, false]);
        holds("though where only some have it, it asks once all the same",
            CommandBinding.asking(warmth[0], acts(1), { all: 2, bang: false }, commands),
            { ask: "some" });
        // :setl never asks either, as Vim's :setlocal doesn't: a program's
        // Muted, by :setl mute.
        const hushing = Commands.listed({ acts: [{ key: "verb:program:mute", name: "Muted" }],
            table: commands, said, called: () => ({ command: "setlocal", argument: "mute" }),
            row: 1 });
        holds("a row's :setl asks nothing, on one or many, and its line isn't banged",
            [CommandBinding.asking(hushing[0], acts(1), { all: 1, bang: false }, commands),
                CommandBinding.asking(hushing[0], acts(2), { all: 2, bang: false }, commands),
                hushing[0].bang],
            [{ ask: "" }, { ask: "" }, false]);
    }

    // Earlier lines, as Vim's :his lists them: each line Return ran
    // remembered once, the newest last, and listed newest first above the
    // rest once :his opens them, each written in by choosing it and run as
    // typed; none is ever named as a command.
    function holdEarlier() {
        let earlier = [];
        for (const line of ["disc", "set nowifi", "disc", "12"])
            earlier = Commands.remembered(earlier, line);
        holds("each line is remembered once, the newest last", earlier,
            ["set nowifi", "disc", "12"]);
        const many = Array.from({ length: 60 }, (_, at) => `${at}`)
            .reduce((kept, line) => Commands.remembered(kept, line), []);
        holds("fifty are kept, as Vim's history keeps", [many.length, many[0]], [50, "10"]);
        const menu = Commands.listed({ acts: network(), table: commands, said, row: 1, earlier });
        holds(":his lists them newest first, before the rest",
            lines(menu).slice(0, 4).map(line => line[0]), ["12", "disc", "set nowifi", "see"]);
        holds("and lists itself with what every surface has",
            lines(Commands.listed({ acts: [], table: commands, said })).map(line => line[0]),
            ["q[uit]", "h[elp]", "se[t]", "setl[ocal]", "his[tory]", "{n}"]);
        holds("typing narrows them by their start",
            Commands.narrowed(menu, "se").map(line => Commands.called(line)),
            ["set nowifi", "see", "se[t]", "setl[ocal]"]);
        holds("choosing one writes it in, and Return runs it as typed",
            [Commands.chosen(Commands.typing("se"), 1, Commands.narrowed(menu, "se")).line,
                Commands.ran("set nowifi", menu).entry?.kind,
                Commands.ran("his", menu).entry?.kind],
            ["set nowifi", "set", "history"]);
    }
}

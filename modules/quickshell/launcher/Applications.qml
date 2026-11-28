// Every application with a desktop entry, found by its name and by what it
// is: "browser" finds Chrome. A program is one row whether it's running or
// not, saying how many windows it has open: Return switches to the one
// used last, or starts it when none is, and Shift+Return opens a new one
// below the focused window. A desktop action that opens just the program,
// as Super+Return opens kitty, is its row too: the row answers to the
// action's words ("open a terminal") and the foot teaches its chord, so
// the two aren't listed side by side.
//
// A program whose desktop entry hides itself, having no window to open, is
// listed all the same when another provider says it gives it acts
// (`hiddenPrograms`, Provider.qml), as Calls.qml does for Tuple. Starting it
// would only start its daemon, so its Return steps in, as a device's does,
// and inside are only those acts and what follows them, never starting it.
//
// The right arrow steps into everything the program can be told, the same
// for every program, each row there only when it applies: what can be done
// first (Go To, New window, New window below, its desktop entry's own
// actions, what it has open now, each the thing it is, its other windows
// by title, Recent where it keeps what it had open lately, its Shortcuts
// where it has keys of its own, Quit), then its parts (Media while it
// has a player, Sound while it plays or records, Notifications while the
// history holds any of its), then any part another provider gives it, such
// as Slack's theme (ByHand.qml). All of it is made as the arrow is
// pressed, and again as its player or its streams change while it's open,
// never as letters are typed, and nothing is read from a file.
//
// Programs is a collection too, home's row for them: every program, those
// open first, each by name. It adds no row of its own to a search, where
// each program is found by itself.
//
// A program's own acts are found from the top as well, each by its name
// alone (Joel, 2026-10-31: "new meeting" finds Meet's New Meeting): its
// desktop entry's actions, as its step lists them, and those another
// provider opts in (`foundActs`, Provider.qml), as Meet's Join by Code
// and Copy a New Meeting's Link are. Each wears its program's mark, its
// note says where it lives ("Programs \u203a Meet"), and it ranks below
// its program, a later word of its name being a lower tier (Rank.js).
// Return does what it does inside the program, and the right arrow steps
// into the program onto it, so the left arrow climbs back out (VKCF3Z,
// 52da2a10).
//
// Inside Programs, only programs are listed and searched.
//
// Long because a program's row, its step, its drawer, its Sound and
// its notices, the tabs and files it has open, the acts found from the
// top and the answers held between keys for whether it starts only all
// read one another and the same hooks of the other providers, and the
// held answers let go on turns each of them moves. Its windows, what
// plays for it and its Recent hold state of their own, so they are
// ApplicationsWindows.qml, ApplicationsPlaying.qml and
// ApplicationsRecents.qml, each made once below, with every name
// another file reads of them still read here.
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import qs
import QtQuick
import KeyTimes
import "../FileFacts.js" as FileFacts
import "../Fuzzy.js" as Fuzzy
import "Captured.js" as Captured
import "Order.js" as Order
import "Program.js" as Program
import "Recent.js" as Recent

Provider {
    id: provider

    what: "program"

    collection: Words.collection("applications", { searched: false })

    // Open programs first, then the rest, each A to Z, so a program moves
    // only as it opens or closes, which Joel did himself
    // (collection-order.md): as the programs daemon serves them, placed,
    // counted and worded (modules/programs/src/program_rows.rs), each made
    // here into its program's row by its id, once its record changes, and
    // searched there too (search.rs), by its name or what else it's
    // called, its key binds' words among them as the watch says them.
    function everything() {
        return ready.rows();
    }
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/programs.sock`
        place: "programs"
        // The hidden programs other providers give acts to are listed too,
        // and what each program's binds do is what it's found by too.
        settings: ({ shown: provider.declared, also: provider.bound })
        make: row => provider.served(row)
        drawn: provider.launcher?.open ?? false
        // The answer to what's typed in Programs now takes its order, the
        // selection staying on its row (Launcher.qml's reshow).
        onSearched: words => {
            const launcher = provider.launcher;
            const step = launcher?.held[launcher.held.length - 1];
            if (launcher?.open && launcher.typed === words && step?.row.from === provider)
                launcher.reshow(launcher.held.length, true);
        }
    }
    // What each program's key binds do, by its entry's id, as its row's
    // keywords say them, for the daemon to find it by.
    readonly property var bound: {
        const said = {};
        for (const entry of DesktopEntries.applications.values) {
            const binds = opening(provider.command(entry));
            if (binds.length > 0)
                said[entry.id] = binds.map(bind => `${bind.does} ${bind.also}`).join(" ");
        }
        return said;
    }
    // A served row made into its program's, or nothing where Quickshell
    // doesn't know the entry yet.
    function served(row) {
        const entry = DesktopEntries.byId(row.id);
        return entry ? programRow(entry, row) : null;
    }
    function returned() {
        const open = row => (running[row.key.slice("program:".length)] ?? []).length > 0;
        return Order.returned({ first: open, name: row => row.name });
    }

    // Inside Programs, as the programs daemon ranks them for what's typed.
    // While its answer is asked, the rows that still hold every letter
    // typed stand: the last answer's, in its order, when what's typed goes
    // on from it, so a key takes rows away and puts none in a place of its
    // own, else every row. The answer, a moment later, takes its order
    // (ReadyRows' searched). A daemon that can't search leaves
    // them matched here, as they were (`matched`), saying so once.
    function search(typed) {
        const answer = ready.search(typed);
        if (answer !== null)
            return { rows: ready.foundRows(answer), grade: answer.keys.length > 0 ? 0 : 3 };
        if (ready.searchless)
            return matched(typed);
        const need = Fuzzy.letters(Fuzzy.words(typed));
        const holds = item => !!item && (item.mask & need) === need;
        const said = Fuzzy.words(typed).join(" ");
        const last = ready.found.words !== null
            && said.startsWith(Fuzzy.words(ready.found.words).join(" "))
            ? ready.foundRows(ready.found) : ready.rows();
        const rows = last.filter(row => holds(row.prepared) || holds(row.keywords));
        return { rows: rows, grade: rows.length > 0 ? 0 : 3 };
    }

    // Inside Programs, by its name or what it is, the best grade first.
    function matched(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of poolFor(programs, typed)) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : { score: -1 };
            const byWord = row.keywords && (row.keywords.mask & need) === need
                ? Fuzzy.match(row.keywords, words, false) : { score: -1 };
            const best = byName.score >= byWord.score ? byName : byWord;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        matchedIn(programs, typed, found.map(f => f.row));
        const grade = Math.min(3, ...found.map(f => f.grade));
        // Those alike in score keep the order nothing typed lists them in.
        const order = returned();
        const kept = Order.sorted(found.filter(f => f.grade === grade),
            (a, b) => b.score - a.score || order(a.row, b.row));
        return { rows: kept.map(f => f.row), grade: grade };
    }

    // What plays for each program, as its Media and its drawer say it
    // (ApplicationsPlaying.qml). Who plays, as last heard, is read here
    // too: by the held answers below, by this provider's start and by
    // the tests.
    ApplicationsPlaying {
        id: playingPart

        provider: provider
    }
    property alias playingHeard: playingPart.playingHeard
    property alias playingWhoNow: playingPart.playingWhoNow

    // How far Louder and Quieter go, as a share of the whole.
    readonly property real step: 0.05

    // And a program found from the top, which lists what it holds under
    // it (`under`), is drawn again as that changes, as a Chrome's tabs
    // read once it's found.
    function redrawHeld() {
        if (launcher?.open && (launcher.held.length > 0 || launcher.typed.trim() !== ""))
            changed();
    }

    // What a program found from the top by its name lists under it, so
    // what it holds shows without a key (VKCF3Z, 2026-11-07: Return still
    // switches at once): what it has open, then its Recent, as Chrome's
    // open tabs and then the sites it visited, kitty's folders or Docs'
    // documents, of which the launcher shows the first few. The same rows
    // its step holds, so Return does there what it does inside. Given
    // apart, { open, recent }, since what it holds now, its parts as
    // Calendar's today's meetings among it, comes before its Recent
    // (VKCF3Z, c7d9e2bd).
    function under(entry) {
        const held = recent(entry)[0];
        return { open: openRows(entry), recent: held ? held.targets() : [] };
    }

    // The desktop's actions that run exactly this command.
    function opening(command) {
        const run = Array.from(command).join("\u0000");
        return Shortcuts.actions.filter(action => action.run.join("\u0000") === run);
    }

    draws: programs.reduce((all, row) => all.concat(row.stands), [])

    // Each program's open windows and what its rows say of them, heard
    // from the windows daemon (ApplicationsWindows.qml). Read here too:
    // by this provider's rows, by Provider.qml's noted and by the tests,
    // which tell the daemon's stand-in what to say.
    ApplicationsWindows {
        id: windowsPart

        provider: provider
    }
    property alias running: windowsPart.running
    property alias windowsDaemon: windowsPart.windowsDaemon
    function noted() {
        return windowsPart.noted();
    }
    function windowsQuietFor() {
        windowsPart.windowsQuietFor();
    }
    function focusedAt(window) {
        return windowsPart.focusedAt(window);
    }
    function windowCount(entry) {
        return windowsPart.windowCount(entry);
    }
    function titled(window, entry) {
        return windowsPart.titled(window, entry);
    }
    function isRunning(entry) {
        return windowsPart.isRunning(entry);
    }
    function windowsOf(entry, held) {
        return windowsPart.windowsOf(entry, held);
    }
    function windowRow(window, entry) {
        return windowsPart.windowRow(window, entry);
    }

    // An act or a part as the vocabulary says it (Words.qml), in the state
    // it's in. One the vocabulary gives no glyph wears what it's inside.
    function said(key, state, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state),
            glyph: Words.glyph(key, state),
            hue: "action",
            // Its words' state, so the : line names the command for it.
            wordState: state
        }, Words.ending(key), fields));
    }

    // A figure inside a part, whose name, side and any graph are read as
    // they're asked for, so the foot follows what Louder or Next just did.
    // The list shows a row as it was when it was given, so a program
    // stepped into is made again as its player and streams change
    // (`playingNow`). Return copies nothing.
    function figure(key, name, side, graph) {
        const getters = { name: name, side: side };
        if (graph)
            getters.graph = graph;
        return gotten(copyRow({ key: key, name: name(), side: side(), copy: "" }), getters);
    }

    // Whether a name another program gives, a player's desktop entry or
    // identity, a stream's application or a notice's sender, is this
    // program: its id or its name, or the entry the name leads to, each
    // name looked up once a step (`looked`).
    function names(entry, given, looked) {
        if (!given)
            return false;
        const name = String(given).toLowerCase();
        if (name === entry.id.toLowerCase() || name === entry.name.toLowerCase())
            return true;
        if (!(name in looked))
            looked[name] = DesktopEntries.heuristicLookup(String(given))?.id ?? "";
        return looked[name] === entry.id;
    }

    // An action's name from its desktop entry, without the program's name
    // leading it ("Spotify: Play" is "Play"), since the step already says
    // whose it is.
    function shortened(name, program) {
        if (!name.toLowerCase().startsWith(program.toLowerCase()))
            return name;
        const after = name.slice(program.length);
        const rest = after.replace(/^[\s:\u2013\u2014-]+/, "");
        if (rest === "" || rest === after)
            return name;
        return rest[0].toUpperCase() + rest.slice(1);
    }

    // The desktop entry's own actions, as its menu lists them, each run as
    // the entry runs: in a terminal when it asks for one, and from its own
    // directory. One that's another act here already (Chrome's own "New
    // Window") is left to that act, so nothing reads twice.
    function menu(entry, taken) {
        return ownActions(entry, taken).map(one => thing({
            key: `verb:desktop:${one.action.id}`,
            // A new window is started; anything else is run.
            verb: /^new\b/i.test(one.name) ? "start" : "run",
            name: one.name,
            keywords: one.name === one.action.name ? "" : one.action.name,
            hue: "program",
            run: () => Niri.spawn(provider.command({
                command: Array.from(one.action.command),
                runInTerminal: entry.runInTerminal,
                workingDirectory: entry.workingDirectory
            }))
        }));
    }

    // Quit asks once, as ending the session does (Session.qml): Return
    // steps in to a row saying what it closes, and a second Return closes
    // each of its windows, as they are then, through niri, so a program
    // asks about anything unsaved as its window's close button would.
    function quit(entry, windows) {
        const key = "verb:program:quit";
        const closing = windows.length === 1 ? "Its Window" : `${windows.length} Windows`;
        return said(key, "", {
            targets: () => [provider.thing({
                key: `${key}:sure`,
                // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
                sure: true,
                verb: Words.verb(key),
                name: `${Words.of(key).sure}, Closing ${closing}`,
                glyph: Words.glyph(key),
                hue: "alert",
                run: quitting(entry)
            })]
        });
    }

    // What d takes of a program: its Quit while it runs, done at once, as
    // d's own ask stands for Quit's step in to be sure; as plain fields
    // rather than its every act made a row, since d over a place of
    // hundreds asks every row (Launcher.qml's actionsOf).
    function operands(entry) {
        const key = "verb:program:quit";
        if ((provider.running[entry.id] ?? []).length === 0)
            return [];
        return [Object.assign({ key: key, verb: Words.verb(key) }, Words.ending(key),
            { run: quitting(entry) })];
    }

    function quitting(entry) {
        return () => {
            for (const window of provider.running[entry.id] ?? [])
                Niri.act({ CloseWindow: { id: window.id } });
        };
    }

    // A program's player, what it plays, its Media row and that row's
    // drawer, which Sound.qml asks of this provider by name
    // (ApplicationsPlaying.qml).
    function playerOf(same) {
        return playingPart.playerOf(same);
    }
    function nowPlaying(player) {
        return playingPart.nowPlaying(player);
    }
    function media(player) {
        return playingPart.media(player);
    }
    function mediaDrawer(player) {
        return playingPart.mediaDrawer(player);
    }

    // A program's drawer (Program.js): the workspaces its windows are on
    // and what it is, what it plays, what other providers say of it
    // (their programFacts), the kinds of file and links it opens, its
    // categories, and its command and id last; its comment is its words.
    // What its entry says that Quickshell's doesn't hold, the kinds of
    // file it opens, its keywords and its package, is the programs
    // daemon's record of it (`entryOf`). A slow one says in the journal
    // what each part spent (timedMaking).
    function programDrawer(entry) {
        if (entry.id === undefined || entry.name === undefined)
            return null;
        return timedMaking(`${entry.id}'s drawer`, () => programDrawerMade(entry));
    }

    function programDrawerMade(entry) {
        const part = (name, work) => Program.timed(making.parts, name, provider.now, work);
        const looked = {};
        const player = part("the player",
            () => playerOf(given => provider.names(entry, given, looked)));
        // Its memory whole, with agents' share beside it, and where only
        // Joel's own windows are.
        const split = Program.split(running[entry.id]);
        const trees = part("its processes", () => treesOf(split.pids));
        const given = hooked("programFacts", entry.id);
        const record = part("its entry", () => entryOf(entry.id));
        return part("its facts", () => Program.drawer({
            name: entry.name,
            usage: Program.usage(trees, split.agents),
            asking: split.pids.length > 0,
            places: split.own
                .map(window => Niri.workspaces[window.workspace_id]?.idx)
                .filter(place => place !== undefined),
            playing: player && player.trackTitle !== "" ? nowPlaying(player) : "",
            given,
            record,
            categories: entry.categories,
            command: entry.command,
            id: entry.id,
            comment: entry.comment,
            marks: { group: settings.group, point: settings.point }
        }, id => Words.name(`applications:${id}`), Words.of("applications").said));
    }

    // The parts of a program's drawer or acts being made, { name, ms }
    // each, while one is timed, and null otherwise: each other provider's
    // answer to a hook is kept by its name then (hooked). Five of 26
    // drawers took 43 to 134 ms making in Joel's journal, against 4 to 17
    // for the rest, with nothing saying which part (2026-11-13). A plain
    // field in an object, as Provider's tally, so timing notifies nothing
    // while a drawer's binding is being worked out.
    readonly property var making: ({ parts: null })

    // How long program rows' notes and counts have spent reading what each
    // program holds, as they're drawn, which the launcher zeroes as each
    // showing begins and reads as it ends (Launcher.qml's readingSpent):
    // Programs showed 20 new rows in 132 ms under load with 78 of them
    // untold (2026-11-14). Plain, as making is, so timing notifies nothing.
    readonly property var reading: ({ ms: 0 })

    // A row's getter that reads `take`, timed into reading.
    function readTimed(take) {
        return () => {
            const began = KeyTimes.now();
            try {
                return take();
            } finally {
                reading.ms += KeyTimes.now() - began;
            }
        };
    }

    // The answer of `work`, a making of `what` timed by its parts, and
    // said in the journal by them when it's slow (Program.makingSaid).
    // One made inside another's is timed as part of it.
    function timedMaking(what, work) {
        if (making.parts !== null)
            return work();
        const parts = [];
        const began = KeyTimes.now();
        making.parts = parts;
        try {
            const answer = work();
            const said = Program.makingSaid(what, KeyTimes.now() - began, parts);
            if (said !== "")
                console.info(`applications: ${said}`);
            return answer;
        } finally {
            making.parts = null;
        }
    }

    // The clock a making is timed by, the one keys and drawers are.
    function now() {
        return KeyTimes.now();
    }

    // A program's entry as the programs daemon holds it (modules/programs),
    // by its desktop entry's id, or null until it says. The daemon reads an
    // entry only while it's watched, so the drawer names its id just after
    // it first asks, watched over a connection of its own only while the
    // launcher is open, as its trees are.
    property string entryWanted: ""

    function entryOf(id) {
        if (id !== entryWanted)
            Qt.callLater(() => {
                provider.entryWanted = id;
            });
        const watch = entries.item;
        return watch && watch.revision >= 0 ? watch.held[`program:${id}`] ?? null : null;
    }

    LazyLoader {
        id: entries

        active: provider.entryWanted !== "" && (provider.launcher?.open ?? false)

        Daemon {
            socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/programs.sock`
            kind: "program"
            live: false
            ahead: [`program:${provider.entryWanted}`]
        }
    }

    // Its streams open to play sound through, by the name each gives, as
    // the Sound collection finds them. A filter's own stream, such as the
    // speakers' tuning, is no program's: it carries its filter's link group.
    function streamsOf(same, from) {
        return (from ?? Playing.streams).filter(node => !!node.audio
            && !node.properties["node.link-group"]
            && (same(node.properties["application.name"])
                || same(node.properties["application.process.binary"]) || same(node.name)));
    }

    // Its streams open to record from a microphone, the same way; a level
    // meter's monitor records nothing.
    function recordersOf(same) {
        return streamsOf(same, Playing.recorders ?? [])
            .filter(node => node.properties["stream.monitor"] !== "true");
    }

    // What its Sound says, as the vocabulary words it: "now {level}".
    function soundSaid(id, level) {
        return ((Words.of("program:sound").said ?? {})[id] ?? id).replace("{level}", level ?? "");
    }

    // Those of its streams still open: a row read once its program has
    // closed a stream, as the launcher's last rows are after it closes,
    // finds the stream without its audio.
    function live(streams) {
        return streams.filter(node => !!node?.audio);
    }

    // How loud it is: its loudest stream, as a share of the whole.
    function level(streams) {
        return Math.max(0, ...live(streams).map(node => node.audio.volume));
    }

    function muted(streams) {
        return live(streams).every(node => node.audio.muted);
    }

    function loudness(streams) {
        return muted(streams) ? soundSaid("muted") : `${Math.round(level(streams) * 100)}%`;
    }

    // Its volume as a bar, empty while muted. Loud wants nothing, so it
    // never warns.
    function bar(streams) {
        return {
            shape: "bar",
            values: [muted(streams) ? 0 : Math.min(1, level(streams))],
            warn: false
        };
    }

    // Every stream a step louder or quieter, from the loudest, so they move
    // as one; louder is meant to be heard, so it unmutes.
    function turn(streams, by) {
        const to = Math.min(1, Math.max(0, Math.round((level(streams) + by) / step) * step));
        for (const node of live(streams)) {
            node.audio.volume = to;
            if (by > 0)
                node.audio.muted = false;
        }
    }

    // Its own options, which :setl sets as Vim's :setlocal sets a buffer's
    // (VKCF3Z, 2026-11-05): whether its streams are muted, and how loud,
    // on them alone, never the machine's output, which :set's are
    // (Sound.qml's setOptions). A level set or raised is meant to be
    // heard, so it unmutes, as its rows' do.
    function muteOption(streams) {
        return {
            kind: "switch",
            now: () => provider.muted(streams),
            set: to => {
                for (const node of provider.live(streams))
                    node.audio.muted = to;
                return "";
            }
        };
    }

    function streamOptions(streams) {
        return {
            mute: muteOption(streams),
            volume: {
                kind: "number",
                least: 0,
                most: 100,
                now: () => Math.round(provider.level(streams) * 100),
                set: (to, change) => {
                    for (const node of provider.live(streams)) {
                        node.audio.volume = to / 100;
                        if (change?.how !== "subtract")
                            node.audio.muted = false;
                    }
                    return "";
                }
            }
        };
    }

    // Its Microphone, while it records: whether it's heard, saying so, and
    // a step in whether what it records is Muted, one switch saying yes or
    // no as a device's is (Sound.qml's mutedSwitch).
    function microphone(recorders) {
        const hears = () => provider.muted(recorders) ? provider.soundSaid("muted")
            : provider.soundSaid("recording");
        const row = said("program:microphone", muted(recorders) ? "muted" : "", {
            hue: "program",
            options: { mute: provider.muteOption(recorders) },
            targets: () => [provider.said("verb:program:mute",
                provider.muted(recorders) ? "muted" : "", {
                    key: "verb:program:mute:microphone",
                    side: provider.soundSaid(provider.muted(recorders) ? "yes" : "no"),
                    stays: true,
                    run: () => {
                        const hushed = provider.muted(recorders);
                        for (const node of provider.live(recorders))
                            node.audio.muted = !hushed;
                    }
                })]
        });
        return gotten(row, { side: hears });
    }

    // Sound: whether it's Muted, one switch saying yes or no, Louder and
    // Quieter, which keep the launcher to be pressed again, and its volume,
    // while it plays; then its Microphone, while it records.
    function sound(streams, recorders) {
        const row = said("program:sound", "", {
            hue: "program",
            options: streams.length === 0 ? undefined : provider.streamOptions(streams),
            targets: () => {
                const stepping = by => ({
                    stays: true,
                    state: () => provider.soundSaid("now", provider.loudness(streams)),
                    run: () => provider.turn(streams, by)
                });
                const rows = streams.length === 0 ? [] : [provider.said("verb:program:mute",
                    provider.muted(streams) ? "muted" : "", {
                        side: provider.soundSaid(provider.muted(streams) ? "yes" : "no"),
                        stays: true,
                        run: () => {
                            const hushed = provider.muted(streams);
                            for (const node of provider.live(streams))
                                node.audio.muted = !hushed;
                        }
                    }), provider.said("verb:program:louder", "", stepping(step)),
                provider.said("verb:program:quieter", "", stepping(-step)),
                provider.figure("program:volume", () => Words.name("program:volume"),
                    () => provider.loudness(streams), () => provider.bar(streams))];
                return recorders.length === 0 ? rows
                    : rows.concat([provider.microphone(recorders)]);
            }
        });
        // A program only recording says so, where one playing says how loud.
        return gotten(row, {
            side: () => streams.length > 0 ? provider.loudness(streams)
                : provider.muted(recorders) ? provider.soundSaid("muted")
                : provider.soundSaid("recording"),
            graph: () => streams.length > 0 ? provider.bar(streams) : undefined
        });
    }

    // Notifications: those it sent, newest first, each as the collection
    // of them has it (PastNotices.qml), so Return on one does what it does
    // there, and the right arrow steps into what else can be done to one.
    // Without that collection in the launcher, there's none to hold.
    function notices(entry, same) {
        const past = provider.hooking.sentBy[0] ?? null;
        const { count, unread } = past ? sentCount(past, entry, same) : { count: 0, unread: 0 };
        if (count === 0)
            return [];
        return [said("program:notices", "", {
            hue: "program",
            // Its figures at its right, where a row's stand (VKCF3Z,
            // 2026-11-05: one place for each kind of fact).
            side: unread > 0 ? `${count} \u00b7 ${unread} unread` : String(count),
            ask: "which?",
            nested: true,
            targets: () => past.sentBy(same)
        })];
    }

    // How many notices each program sent, and how many of them are unread,
    // kept while the notices' counts are one (PastNotices.qml's sentCounts
    // gives the same until they change), so making a program's acts again
    // counts none of them; `read` counts the countings, for the check.
    // Summed from each sender's count, never the notices' rows or their
    // copies (sentBy's, lined for a list): copying Slack's for its count
    // each time took 142 ms of a drawer's 151 making on Joel's
    // (2026-11-13), and making every row past the page for it 97 ms of
    // Google Calendar's acts (2026-11-14).
    readonly property var sentCounted: ({ of: null, by: Object.create(null), read: 0 })
    function sentCount(past, entry, same) {
        const senders = past.sentCounts();
        const kept = provider.sentCounted;
        if (kept.of !== senders) {
            kept.of = senders;
            kept.by = Object.create(null);
        }
        if (!(entry.id in kept.by)) {
            kept.read += 1;
            let count = 0;
            let unread = 0;
            for (const sender of Object.keys(senders)) {
                if (!same(sender))
                    continue;
                count += senders[sender].count;
                unread += senders[sender].unread;
            }
            kept.by[entry.id] = { count, unread };
        }
        return kept.by[entry.id];
    }

    // Shortcuts, which says which layer is whose, or null where there's none.
    function keyMap() {
        return provider.hooking.layerOf[0] ?? null;
    }

    // The name of the layer of its own keys, or "" for none.
    function ownLayer(entry) {
        const map = keyMap();
        const own = map ? map.ofProgram(entry.id) : "desktop";
        return own === "desktop" ? "" : own;
    }

    // What each program had open lately and has open now, its Recent and
    // its Tabs (ApplicationsRecents.qml). Read here too: by
    // this provider's rows and notes, by Screenshots.qml, which finds the
    // recents by name among the launcher's providers, and by the tests.
    ApplicationsRecents {
        id: recentsPart

        provider: provider
    }
    property alias recentSources: recentsPart.recentSources
    property alias recents: recentsPart.recents
    function recent(entry) {
        return recentsPart.recent(entry);
    }
    function sourceOf(entry) {
        return recentsPart.sourceOf(entry);
    }
    function keepsRecent(entry, given) {
        return recentsPart.keepsRecent(entry, given);
    }
    function kept(entry, asked, opens) {
        return recentsPart.kept(entry, asked, opens);
    }
    function openRows(entry) {
        return recentsPart.openRows(entry);
    }
    function openGroup(entry) {
        return recentsPart.openGroup(entry);
    }

    // A program's notices past the notices place's page come once they're
    // asked for (PastNotices.qml's every), so its row of them, counting
    // them, is drawn again when they do.
    Connections {
        target: (provider.launcher?.sources ?? [])
            .find(source => typeof source.sentBy === "function") ?? null
        function onChanged() {
            provider.redrawHeld();
        }
    }

    // A source read only as its program is stepped into is read again the
    // next time the launcher opens.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open) {
                provider.recents.forget();
                provider.treesWanted = [];
                provider.entryWanted = "";
            }
        }

        // Off a program's row, nothing shows its figures, so the daemon
        // stops reading its trees rather than reading on until the
        // launcher closes.
        function onSelectedChanged() {
            const is = String(provider.launcher.selected?.is ?? "");
            if (!is.startsWith("program:") && provider.treesWanted.length > 0)
                provider.treesWanted = [];
        }
    }

    // What a program's processes use is the machine daemon's (modules/
    // machine), a `tree:<pid>` for each of its windows' processes, with
    // all each started counted in. The daemon reads a tree only while it's
    // watched, so the drawer names its pids just after it first asks, and
    // they're watched over a connection of their own only while the
    // launcher is open; closing it closes the connection, which ends the
    // watch. Moving to another program's drawer watches its pids instead.
    property var treesWanted: []

    function treesOf(pids) {
        if (JSON.stringify(pids) !== JSON.stringify(treesWanted))
            Qt.callLater(() => {
                provider.treesWanted = pids;
            });
        const watch = trees.item;
        return watch && watch.revision >= 0
            ? pids.map(pid => watch.held[`tree:${pid}`] ?? null) : [];
    }

    LazyLoader {
        id: trees

        active: provider.treesWanted.length > 0 && (provider.launcher?.open ?? false)

        Daemon {
            socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
            kind: "tree"
            live: false
            ahead: provider.treesWanted.map(pid => `tree:${pid}`)
            keptThroughGap: false
            answersWhole: true
        }
    }

    // The keys, as keyOf makes them, of what other providers list as open
    // in a program among its acts (`programOpen(program)`), as Neovim's
    // files open now are (NeovimProgram.qml): its Recent leaves them out
    // and its note counts them, so no file shows twice in its step
    // (RTOIMQ, 2026-11-09).
    function givenOpen(program) {
        return hooked("programOpen", program);
    }

    // The key a site, file or folder's row has (`site:`, `file:`,
    // `folder:` and its address or path), the one way Recent and what's
    // open name a thing, so one is never listed twice under two keys.
    function keyOf(kind, thing) {
        return `${kind}:${thing}`;
    }

    // The windows what it has open now stands for, by id: each window whose
    // front tab is a page it has open, as niri titles Chrome's windows by
    // it, and each window a file it has open is in.
    function heldWindows(entry) {
        const source = provider.sourceOf(entry);
        if (!provider.recents.tellsOpen(source) || !provider.isRunning(entry))
            return [];
        const lines = provider.recents.openIn(entry.id).map(line => line.split("\t"));
        if (source.kind === "site") {
            const fronts = new Set(lines.filter(tab => tab[2] === "true" && (tab[4] ?? "") !== "")
                .map(tab => tab[5] ?? ""));
            return (running[entry.id] ?? []).filter(window => fronts.has(titled(window, entry)))
                .map(window => window.id);
        }
        return lines.filter(([, path]) => (path ?? "") !== "")
            .map(([who]) => provider.fileWindow(entry, who)).filter(window => window !== null)
            .map(window => window.id);
    }

    // The window a file open now is in, by what its source said holds it:
    // the process that has it, or `@` and niri's window where one process
    // has many, as kitty does for each Neovim; null once it's gone.
    function fileWindow(entry, who) {
        return !who.startsWith("@")
            ? (running[entry.id] ?? []).find(one => one.pid === Number(who)) ?? null
            : Niri.windows[who.slice(1)] ?? null;
    }

    // What other providers give a program's Recent.
    function givenRecent(program) {
        return hooked("recentOf", program);
    }

    // The acts other providers give a program beside its desktop entry's
    // own (`programActs(program)`), as Meet's Join by Code is given
    // (Meet.qml), listed after its desktop entry's actions and before its
    // Recent.
    // Whether each gave any is told to `gave`, a Map, when one's given.
    function givenActs(program, gave) {
        return hooked("programActs", program, gave);
    }

    // What other providers say of a program as its row's note
    // (`programNote(program)`), the first that says anything, as Slack's
    // says why it can't be checked (SlackProgram.qml); a program has no
    // note of its own, so it's empty while none does.
    function givenNote(program) {
        for (const other of noting) {
            const note = other.programNote(program);
            if (note !== "")
                return note;
        }
        return "";
    }
    // What a program holds, said by its row on the cursor where no other
    // provider says its note, so what's inside shows without a key
    // (VKCF3Z, 2026-11-07): what it holds now, how many it has open,
    // Chrome's as tabs, and its next meeting, today's or how much of its
    // mail is new, then how many its Recent keeps, as "3 tabs \u00b7 9
    // recent" or "3 today \u00b7 1 recent", in the order its rows are
    // listed under it (VKCF3Z, c7d9e2bd). A part that's no news, as
    // Slack's theme, isn't counted. Read only from what's already kept,
    // so reading it asks no program anything: Chrome's tabs are counted
    // once its Recent was asked for, as typing its name does.
    function holding(entry) {
        const words = Words.of("program:recent").said ?? {};
        const fill = (said, count) => (said ?? "").replace("{count}", String(count));
        const source = recentSources.find(each => each.app === entry.id) ?? null;
        const open = provider.recents.openIn(entry.id);
        const site = source?.kind === "site";
        const addresses = new Set(open.map(line => line.split("\t")[4] ?? "")
            .filter(address => address !== ""));
        const pages = addresses.size;
        // A closed browser's saved tabs are counted as recent, as listed.
        const live = isRunning(entry);
        // What other providers list open in it and give its Recent, asked
        // once for all it counts: a row that doesn't step in reads this for
        // its count, on every row a step into Programs draws.
        const openGiven = givenOpen(entry.id);
        const recentGiven = givenRecent(entry.id);
        const opened = site ? (live ? pages : 0) : open.length + openGiven.length;
        // As many as its Recent lists, so what's open, listed above it, isn't
        // counted twice.
        const kept = provider.keepsRecent(entry, recentGiven)
            ? provider.kept(entry, recentGiven, openGiven).length : 0;
        const parts = provider.launcher?.partsOfRows[`program:${entry.id}`] ?? [];
        const meetings = parts.filter(part => part.what === "meeting");
        const mail = parts.filter(part => part.what === "mail").length;
        // Its next meeting by name only where none is listed under it, and
        // while they are, how many are today, so the say never names a row
        // just below it (VKCF3Z, 2026-11-07).
        const listed = provider.launcher?.underListed === `program:${entry.id}`;
        const today = meetings.filter(part => part.today === true).length;
        const next = meetings.length > 0
            ? listed ? (today > 0 ? fill(words.today, today) : "")
                : (words.next ?? "").replace("{when}", meetings[0].side ?? "")
                    .replace("{name}", meetings[0].name)
            : mail > 0 ? fill(words.parts, mail) : "";
        const tabs = opened === 1 ? words.tab : words.tabs;
        return [opened > 0 ? fill(site ? tabs : words.open, opened) : "", next,
            kept > 0 ? fill(words.recent, kept) : ""]
            .filter(said => said !== "").join(" \u00b7 ");
    }

    // The other providers holding each hook a program's making asks, found
    // as the launcher's sources change rather than on every making: a step
    // into Programs looked through every source for each hook of each
    // program it asked, about 1,500 lookups a step in the keys lab
    // (2026-11-15).
    readonly property var hooking: ["keepsRecentOf", "layerOf", "programActs", "programFacts",
        "programOpen", "recentOf", "sentBy"]
        .reduce((held, hook) => Object.assign(held, { [hook]: provider.holders(hook) }), {})

    // The other providers with the hook `hook`, looked for now.
    function holders(hook) {
        return (provider.launcher?.sources ?? [])
            .filter(other => other !== provider && typeof other[hook] === "function");
    }

    // The providers that say a program's note, found once rather than as
    // each program's row is read.
    readonly property var noting: (provider.launcher?.sources ?? [])
        .filter(other => other !== provider && typeof other.programNote === "function")

    // What every other provider with the hook `hook` gives `program`, each
    // provider's answer timed by its name while a making is (`making`),
    // and whether each gave anything told to `each`, a Map, when it's
    // given, so startsOnly asks no giver twice.
    function hooked(hook, program, each) {
        return (provider.hooking[hook] ?? provider.holders(hook)).reduce((rows, other) => {
            const given = Program.timed(making.parts, other.settings?.name ?? hook, provider.now,
                () => other[hook](program));
            each?.set(other, (given ?? []).length > 0);
            return rows.concat(given);
        }, []);
    }

    // One path a program had open lately, as Recent's row, made as every
    // file's and folder's is (Files.qml), with one act of Recent's own
    // first: a file opens in the program, and a folder, as kitty's are,
    // gets a new window there. Not named opened, which is the hook the
    // launcher calls as Programs is stepped into (Provider.qml). A file
    // is said as a download is (ChromeProgram.qml): when it was last used
    // (`at`), where its source said, at its right, before its folder but
    // the home's, which a lone "~" says nothing of (VKCF3Z, 2026-11-09);
    // a picture's mark or a video's, as Capture's wear them; and a drawer
    // saying where it is and what kind, its name, and a picture as it is.
    function lately(entry, source, path, at) {
        const kind = source?.kind === "folder" ? "folder" : "file";
        const key = kind === "folder" ? "verb:path:folder:window" : "verb:path:file:open";
        const act = thing({
            key: `${key}:${path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            run: () => {
                if (source === null) {
                    Niri.spawn(/%[fFuU]/.test(entry.execString)
                        ? provider.command(entry).concat([path]) : ["xdg-open", path]);
                    return;
                }
                const window = (provider.running[entry.id] ?? [])[0] ?? null;
                const command = window ? source.open : source.start;
                Niri.spawn(command.map(part => part.replace("{path}", path)
                    .replace("{pid}", String(window?.pid ?? ""))));
            }
        });
        if (kind === "folder")
            return Files.folder(path, thing, [act]);
        return provider.saidWhole(Files.file(path, thing, [act]), path, at);
    }

    // What a file's row says beside what every file's does, as lately
    // gives it, its drawer made only as it's opened.
    function saidWhole(row, path, at) {
        if (Number.isFinite(at)) {
            row.side = [provider.usedWhen(at), Files.beside(path)]
                .filter(part => part !== "").join(" \u00b7 ");
            row.at = at;
        }
        const mark = Captured.mark(path);
        if (mark !== "")
            row.glyph = Theme.glyph[mark];
        row.drawer = () => {
            const drawer = Object.assign(FileFacts.placed(Files.within(path),
                FileFacts.kindByName(row.name, provider.fileWords), provider.fileWords), {
                title: row.name,
                facts: []
            });
            const preview = FileFacts.pictured(path);
            if (preview)
                drawer.preview = preview;
            return drawer;
        };
        return row;
    }

    // When a file was last used, as its row says it, by now.
    function usedWhen(at) {
        return FileFacts.when(at, Date.now(), provider.fileWords);
    }

    // What a file's row and drawer say, in the launcher's words, and days
    // and months as the locale names them, as the file chooser says them.
    readonly property var fileWords: FileFacts.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    // One site a program visited lately, a line of its source's: the page's
    // address, a tab and its title. Keyed by the page, as a site is wherever
    // it shows.
    function visited(entry, line) {
        const tab = line.indexOf("\t");
        const address = tab < 0 ? line : line.slice(0, tab);
        const title = tab < 0 ? "" : line.slice(tab + 1);
        const open = said("verb:recent:site:open", "", {
            key: `verb:recent:site:open:${address}`,
            hue: "action",
            run: () => Niri.spawn(provider.command(entry).concat([address]))
        });
        return site(address, title, hostOf(address), [open]);
    }

    // A page's host, as said beside it.
    function hostOf(address) {
        return address.replace(/^[a-z]+:\/\//, "").replace(/[/?#].*$/, "");
    }

    // A site's row, made here alone for every page wherever it shows (the
    // launcher's components), keyed by the page: Return does the first of
    // `acts`, and the right arrow lists them, then Copy the Link. One
    // `gone` stays dimmed where it was, as the launcher keeps a row whose
    // thing has gone.
    function site(address, title, side, acts, gone) {
        const all = acts.concat([said("verb:recent:site:link", "", {
            key: `verb:recent:site:link:${address}`,
            hue: "action",
            copy: address,
            copies: address,
            run: () => Quickshell.clipboardText = address
        })]);
        return thing({
            key: keyOf("site", address),
            what: "site",
            component: "site",
            verb: all[0].verb,
            name: title !== "" ? title : hostOf(address),
            glyph: Words.glyph("recent:site"),
            hue: "program",
            side: side,
            gone: gone === true,
            run: all[0].run,
            verbs: () => all
        });
    }

    // What a program has open now, as its source's `opened` prints it, a
    // tab a line: its id, its window, whether it's in front there, when it
    // was last looked at, its address and its title. One row a page, newest
    // looked at first; two tabs on one page are one row, going to the one
    // looked at last and saying how many. A tab with no address, one the
    // source leaves out, has no row, but its title still finds the window
    // it's in front of.
    function openSites(entry, source) {
        const all = provider.recents.openIn(entry.id).map(line => {
            const [id, window, front, last, address, title] = line.split("\t");
            return { id, window, front: front === "true", last: Number(last), address, title };
        });
        const tabs = all.filter(tab => (tab.address ?? "") !== "")
            .sort((a, b) => b.last - a.last);
        const pages = [];
        for (const tab of tabs) {
            const page = pages.find(one => one[0].address === tab.address);
            if (page)
                page.push(tab);
            else
                pages.push([tab]);
        }
        return pages.map(page => openSite(entry, source, page, all));
    }

    // One page open now: Return goes to the tab looked at last, inside Go
    // To, Close It, its window, then Copy the Link. Its window is the one
    // niri titles by what's in front of it, as Chrome titles its windows,
    // of two the one used last. Go To asks the source to bring the tab
    // forward (`go`) and brings its window forward, and Close It closes
    // the page's tabs (`close`), each only where the source reaches them by
    // ids it gave (Recent.js's reaches); otherwise its window's row leads,
    // as Go To would only have done what it does. Close It keeps the
    // launcher, its foot saying how many tabs it closes when it's more
    // than one, and the page stays where it was, said closed and holding
    // only Copy the Link, until the launcher opens again.
    function openSite(entry, source, page, tabs) {
        const [tab] = page;
        const words = Words.of("recent:site").said ?? {};
        const counted = said => (said ?? "").replace("{count}", String(page.length));
        const host = hostOf(tab.address);
        if ((provider.recents.closed[entry.id] ?? []).includes(tab.address))
            return site(tab.address, tab.title, `${words.closed ?? ""} \u00b7 ${host}`, [], true);
        const front = tabs.find(one => one.window === tab.window && one.front) ?? tab;
        const window = (running[entry.id] ?? [])
            .find(one => titled(one, entry) === front.title) ?? null;
        const open = page.length > 1 ? counted(words.tabs) : words.open ?? "";
        const ask = (command, id) => (command ?? []).map(part => part.replace("{id}", id));
        const go = () => {
            Niri.spawn(ask(source.go, tab.id));
            if (window)
                Niri.act({ FocusWindow: { id: window.id } });
        };
        const goes = !Recent.reaches(source.go, [tab]) ? [] : [said("verb:recent:site:go", "", {
            key: `verb:recent:site:go:${tab.address}`,
            hue: "action",
            side: window ? titled(window, entry) : "",
            run: go
        })];
        const closes = !Recent.reaches(source.close, page) ? [] : [said(
            "verb:recent:site:close", "", {
            key: `verb:recent:site:close:${tab.address}`,
            hue: "action",
            stays: true,
            note: page.length > 1 ? counted(words.closes) : "",
            run: () => {
                for (const one of page)
                    Niri.spawn(ask(source.close, one.id));
                provider.recents.closing(entry.id, tab.address);
            }
        })];
        const acts = goes.concat(closes, window ? [windowRow(window, entry)] : []);
        // The state before the host, as a file's before its folder
        // (Files.held), so a side cut short keeps it.
        return site(tab.address, tab.title, `${open} \u00b7 ${host}`, acts);
    }

    // What a program has open now, as its file source's `opened` prints
    // it, a file a line: the process that has it, or `@` and niri's window
    // it's in where one process has many windows, as kitty does for each
    // Neovim, then its whole path, and after another tab what `go` is
    // given ({id}) to bring it forward inside that window. One row a file,
    // the file's own wherever it shows, and only while its window is open,
    // newest looked at first; two windows on one file are one row, said
    // open in both, going to the one looked at last. Return goes to its
    // window, and inside are Go To, Close It and Copy the Path. Close It is
    // there only where the process is the window's own, so closing it
    // closes nothing else; it keeps the launcher, and the file stays where
    // it was, said closed and holding only Copy the Path, until the
    // launcher opens again.
    function openFiles(entry, source) {
        const words = Words.of("path:file").said ?? {};
        const held = [];
        for (const line of provider.recents.openIn(entry.id)) {
            const [who, path, go] = line.split("\t");
            if (!path)
                continue;
            const window = provider.fileWindow(entry, who);
            if (window)
                held.push({ window: window, path: path, go: go ?? "", own: !who.startsWith("@") });
        }
        held.sort((a, b) => provider.focusedAt(b.window) - provider.focusedAt(a.window));
        const closed = provider.recents.closed[entry.id] ?? [];
        const rows = [];
        const paths = [];
        for (const open of held) {
            if (paths.includes(open.path))
                continue;
            paths.push(open.path);
            if (closed.includes(open.path)) {
                rows.push(Files.held(open.path, thing, [], words.closed ?? "", true));
                continue;
            }
            const windows = held.filter(one => one.path === open.path).length;
            const goes = said("verb:path:file:go", "", {
                key: `verb:path:file:go:${open.path}`,
                hue: "action",
                run: () => {
                    if (source?.go && open.go !== "")
                        Niri.spawn(source.go.map(part => part.replace("{id}", open.go)));
                    Niri.act({ FocusWindow: { id: open.window.id } });
                }
            });
            const closes = !open.own ? [] : [said("verb:path:file:close", "", {
                key: `verb:path:file:close:${open.path}`,
                hue: "action",
                stays: true,
                run: () => {
                    Niri.act({ CloseWindow: { id: open.window.id } });
                    provider.recents.closing(entry.id, open.path);
                }
            })];
            rows.push(Files.held(open.path, thing, [goes].concat(closes), windows > 1
                ? (words.windows ?? "").replace("{count}", String(windows)) : words.open ?? ""));
        }
        return rows;
    }

    // Its own keys, where Shortcuts has a layer of them (KeyMap.qml's
    // layerOf), as Shortcuts lists the layer, named Shortcuts here since
    // the program already says whose they are.
    function keysOf(entry) {
        const map = keyMap();
        const layer = map ? map.layerOf(entry.id) : null;
        return layer ? [Object.assign({}, layer, { name: Words.name("shortcuts") })] : [];
    }

    // The hidden programs other providers give acts to, by id.
    readonly property var declared: (provider.launcher?.sources ?? [])
        .filter(other => other !== provider)
        .reduce((ids, other) => ids.concat(other.hiddenPrograms ?? []), [])

    // Whether a program steps in on Return rather than starting: only one
    // another provider declares, never one that's simply closed.
    function stepsIn(entry) {
        return entry.noDisplay && declared.includes(entry.id);
    }

    // Everything the right arrow holds for a program, as it stands now, in
    // one order for every program: Go To or Open, what's given it to lead,
    // New Window, New Window Below, its own actions, what it has open, its
    // other windows, Recent, its notices, Shortcuts and Quit, then Media
    // and Sound while it plays. What it does comes first and what ends it
    // near the bottom, so Return on a program never lands in its Recent,
    // and what's open comes before what was, so Recent sits straight after
    // them in every program. One that steps in has only what's given it,
    // what it has open, its Recent, its notices and Shortcuts, in that
    // order, never starting.
    // Either way, what's given it to copy comes after everything, as Meet's
    // Copy a New Meeting's Link does, since Copy is last. Each made is
    // counted (stepsCounted). What it read that can change while its entry
    // stays is told to `told` when it's given, as startsOnly does: whether
    // each giver gave it any (`gave`) and whether anything plays (`plays`).
    function controls(entry, start, below, told) {
        // A program held open as its desktop entries are read again, as on
        // a switch, keeps the entry that's gone, which has no id or name;
        // it offers nothing until the step is drawn from the new one.
        if (entry.id === undefined || entry.name === undefined)
            return [];
        stepsCounted.made += 1;
        return timedMaking(`${entry.id}'s acts`, () => controlsMade(entry, start, below, told));
    }

    // Each part timed by its name, as a drawer's are: Google Calendar's
    // acts took 122 ms on Joel's shell with every part but other
    // providers' left in the rest (2026-11-13).
    function controlsMade(entry, start, below, told) {
        const part = (name, work) => Program.timed(making.parts, name, provider.now, work);
        const looked = {};
        const same = given => provider.names(entry, given, looked);
        const given = givenActs(entry.id, told?.gave);
        const copying = row => row.verb === "copy";
        // An act given to lead, as signing in again does while a program
        // is signed out, comes straight after Return's own, since it's
        // what the program wants first.
        const leading = given.filter(row => row.leads === true && !copying(row));
        const doing = given.filter(row => row.leads !== true && !copying(row));
        const copies = given.filter(copying);
        const open = () => part("its open tabs", () => openGroup(entry));
        const lately = () => part("its recent", () => recent(entry));
        const sent = () => part("its notices", () => notices(entry, same));
        const keys = () => part("its keys", () => keysOf(entry));
        if (stepsIn(entry))
            return leading.concat(doing, open(), lately(), sent(), keys(), copies);
        const windows = running[entry.id] ?? [];
        const latest = windows[0] ?? null;
        const acts = (latest ? [said("verb:program:switch", "", {
                side: titled(latest, entry),
                run: () => Niri.act({ FocusWindow: { id: latest.id } })
            })] : []).concat([said("verb:start", "", { run: start }),
            said("verb:below", "", { chord: provider.launcher?.belowChord ?? [], run: below })]);
        const taken = provider.taken();
        const { player, streams, recorders, plays } = heardOf(same, part);
        if (told)
            told.plays = plays;
        return acts.slice(0, 1).concat(leading, acts.slice(1),
            part("its menu", () => menu(entry, taken)), doing, open(),
            part("its windows", () => windowsOf(entry, heldWindows(entry))), lately(), sent(),
            keys(), latest ? [part("its quit", () => quit(entry, windows))] : [],
            player ? [media(player)] : [],
            streams.length > 0 || recorders.length > 0 ? [sound(streams, recorders)] : [],
            copies);
    }

    // A program's player, streams and recorders, and whether any is there,
    // which its Media and Sound are made of (controlsMade) and a held
    // answer asks again (playsOf), each part timed as `part` says.
    function heardOf(same, part) {
        const player = part("the player", () => playerOf(same));
        const streams = part("its sound", () => streamsOf(same));
        const recorders = part("its sound", () => recordersOf(same));
        return {
            player,
            streams,
            recorders,
            plays: !!player || streams.length > 0 || recorders.length > 0
        };
    }

    // Whether a program's step would hold only New Window and New Window
    // Below, as a closed Foot's does, so Return starts it rather than
    // opening a list of those two and its drawer stays beside it (Y2YBTS,
    // 2026-11-12 [39f55fff]: a row holding only acts never opens a
    // submenu). Only the launcher's holdsMore reads it, on a key or the
    // selection, never a row drawn; what's cheap is asked first, so a
    // program with windows or a Recent asks nothing of its step, nor
    // has its Recent read.
    function startsOnly(entry, start, below) {
        if ((provider.running[entry.id] ?? []).length > 0 || provider.keepsRecent(entry))
            return false;
        // Keys or an action of its own are in its step whatever else is,
        // and cheap to see where making the step isn't: Programs' keys
        // took 44 of an 84 ms showing under load, each closed program
        // making its step to be asked this (vm-desktop-keeps-pace,
        // 2026-11-14).
        if (provider.keysOf(entry).length > 0
            || !provider.stepsIn(entry) && provider.ownActions(entry, provider.taken()).length > 0)
            return false;
        // Read so a binding that asked is asked again as the answers held
        // are let go.
        void provider.startsTurn;
        const held = provider.startsHeld.by.get(entry.id);
        // holdsSafely is read here too, for the moment between a giver with
        // no changed joining and onHookingChanged letting every answer go.
        if (held !== undefined && held.entry === entry && provider.holdsSafely
            && provider.givesAsHeld(held, entry.id) && provider.sendsAsHeld(held, entry)
            && provider.playsAsHeld(held, entry))
            return held.only;
        const told = { gave: new Map(), plays: false };
        const only = provider.controls(entry, start, below, told)
            .every(one => one.key === "verb:start" || one.key === "verb:below");
        // Held only once every giver was asked, which a step made of nothing
        // didn't do.
        if (provider.holdsSafely && told.gave.size === provider.hooking.programActs.length)
            provider.startsHeld.by.set(entry.id, {
                entry,
                only,
                gave: provider.givesTurned(told.gave),
                sent: provider.sentTurned(entry),
                plays: told.plays,
                playsTurn: provider.playingHeard.turn
            });
        return only;
    }

    // Whether each giver of acts gave a program any, with the turn it had
    // said it changed by: every act given is in the program's step
    // (controls), so that's all of a giver its answer reads.
    function givesTurned(gave) {
        return new Map(Array.from(gave, ([other, gives]) => [other, {
            turn: provider.givesTurns.by.get(other) ?? 0,
            gives
        }]));
    }

    // Whether the notices' collection holds any its program sent, with
    // the turn the collection had said it changed by: a Notifications
    // row is in its step only while it does (notices), and sentCount
    // keeps each program's count until the counts change, so asking
    // after the step was made counts nothing again.
    function sentTurned(entry) {
        const past = provider.hooking.sentBy[0] ?? null;
        return {
            turn: past ? provider.givesTurns.by.get(past) ?? 0 : 0,
            sends: provider.sendsOf(past, entry)
        };
    }
    function sendsOf(past, entry) {
        sendsCounted.asked += 1;
        if (!past)
            return false;
        const looked = {};
        const same = given => provider.names(entry, given, looked);
        return provider.sentCount(past, entry, same).count > 0;
    }

    // Whether a held program's notices are there or not as they were,
    // asked again only once the collection said it changed since.
    function sendsAsHeld(held, entry) {
        const past = provider.hooking.sentBy[0] ?? null;
        const turn = past ? provider.givesTurns.by.get(past) ?? 0 : 0;
        if (turn === held.sent.turn)
            return true;
        if (provider.sendsOf(past, entry) !== held.sent.sends)
            return false;
        held.sent.turn = turn;
        return true;
    }

    // Whether a player, a stream or a recorder is a program's, as its
    // Media and Sound are found (controlsMade); one that steps in has
    // neither.
    function playsOf(entry) {
        playsCounted.asked += 1;
        if (stepsIn(entry))
            return false;
        const looked = {};
        const same = given => provider.names(entry, given, looked);
        return heardOf(same, (named, find) => find()).plays;
    }
    // How often a held answer asked whether its program plays, for the
    // check that each asks once a change of who plays; plain, as
    // stepsCounted is.
    readonly property var playsCounted: ({ asked: 0 })
    // And whether its notices are there, for the same check of the notices.
    readonly property var sendsCounted: ({ asked: 0 })

    // Whether anything plays for a held program as it did, asked again
    // only once who plays changed since.
    function playsAsHeld(held, entry) {
        const turn = provider.playingHeard.turn;
        if (held.playsTurn === turn)
            return true;
        if (provider.playsOf(entry) !== held.plays)
            return false;
        held.playsTurn = turn;
        return true;
    }

    // Whether every giver that said it changed since an answer was held
    // still gives its program acts or none as it did, each asked again
    // only then.
    function givesAsHeld(held, program) {
        for (const [other, was] of held.gave) {
            const turn = provider.givesTurns.by.get(other) ?? 0;
            if (turn === was.turn)
                continue;
            if (((other.programActs(program) ?? []).length > 0) !== was.gives)
                return false;
            was.turn = turn;
        }
        return true;
    }

    // What startsOnly found of each closed program, by its desktop entry's
    // id with the entry it was asked of, held until something its step
    // reads says it changed (startsAgain). The foot weighs every row
    // listed on each key, and a closed program made its whole step each
    // time to say the same, about 1 ms a program: 160 of 536 ms of the
    // keys lab's weighing over c, m and s, profiled (2026-11-18). Plain,
    // so holding one wakes no binding; `startsTurn` counts the lettings go.
    // Only a program with no windows, Recent, keys or actions of its own
    // is held, each asked fresh above, so the windows and the Recents
    // never let go: a closed program's step reads neither, and a title
    // changing as Joel types would empty it between keys.
    readonly property var startsHeld: ({ by: new Map() })
    property int startsTurn: 0
    // Lets go of every answer held, since no source says which program its
    // change was for.
    function startsAgain() {
        startsHeld.by = new Map();
        startsTurn += 1;
    }
    // A giver of acts saying it changed has only the answers it gives
    // differently asked again (givesAsHeld), by a turn of its own: kitty
    // says changed as any of its windows is retitled, which an agent's
    // does with each command, and letting every answer go then brought
    // each key's cost back. The notices' collection has a turn here too,
    // each held answer asking only whether its program's notices are
    // there (sendsAsHeld): it says changed as each notice comes, and
    // they came about every 48 s through Joel's working day (2026-11-17).
    readonly property var givesTurns: ({ by: new Map() })
    function gaveAgain(other) {
        givesTurns.by.set(other, (givesTurns.by.get(other) ?? 0) + 1);
        startsTurn += 1;
    }
    // What a closed program's step reads that can change while its desktop
    // entry stays: the acts and notices other providers give it, each
    // provider's `changed` saying its own did (Provider.qml); the programs
    // others declare hidden; a walk from outside, which some providers
    // give less to; and who plays, above, each held answer asking only
    // whether anything plays for it. Each opening starts afresh
    // too, so the hold saves only the asks of one opening's keys: a giver
    // may read only as it's asked, as Spotify does once an opening, and a
    // change one forgot to say lasts no longer than the opening it came in.
    onHookingChanged: startsAgain()
    onDeclaredChanged: startsAgain()
    Connections {
        target: provider.launcher
        ignoreUnknownSignals: true

        function onOpenChanged() {
            provider.feltOpening(() => provider.startsAgain());
        }

        function onCataloguingChanged() {
            provider.startsAgain();
        }
    }
    // Whether every giver can say it changed; a source made by hand with
    // no `changed` couldn't let its answers go, so none is held.
    readonly property bool holdsSafely: provider.hooking.programActs
        .concat(provider.hooking.sentBy)
        .every(other => typeof other.changed === "function")
    Variants {
        model: (provider.launcher?.sources ?? []).filter(other => other !== provider
            && typeof other.changed === "function"
            && (provider.hooking.programActs.includes(other)
                || provider.hooking.sentBy.includes(other)))

        Connections {
            required property var modelData

            target: modelData

            function onChanged() {
                provider.gaveAgain(modelData);
            }
        }
    }

    // The desktop entry's actions its menu lists, each with the name it's
    // listed by.
    function ownActions(entry, taken) {
        const own = Array.from(entry.command).join("\u0000");
        return Array.from(entry.actions).map(action => ({
            action: action,
            name: provider.shortened(action.name, entry.name)
        })).filter(one => one.name !== "" && !taken.includes(one.name.toLowerCase())
            && Array.from(one.action.command).join("\u0000") !== own);
    }

    // How many programs' steps were made, for the check that asking
    // whether one starts only makes none where it's plain it doesn't. A
    // plain field, as making's, since a counting property notified inside
    // the foot's binding that asked, a binding loop on niri (2026-11-14).
    readonly property var stepsCounted: ({ made: 0 })

    // The names a program's own acts have here, which its desktop entry's
    // actions of the same name leave to them.
    function taken() {
        return ["verb:program:switch", "verb:start", "verb:below", "verb:program:quit"]
            .map(key => Words.name(key).toLowerCase());
    }

    // The acts other providers opt in to being found from the top, each
    // { program, key, from }, and their rows (optedActs), made again only
    // as one's list says another act, never on a key. Not as a provider's
    // binding makes the same list anew, nor as what its acts read changes:
    // bound, the rows asked each provider's acts inside the binding, so
    // swayimg's galleries or Chrome's pages changing made every act found
    // from the top again, on Joel's shut launcher about every 2.5 s, 1,216
    // rows in two minutes (2026-11-12).
    readonly property string optedSaid: JSON.stringify((provider.launcher?.sources ?? [])
        .map(other => other === provider ? [] : other.foundActs ?? []))
    property var optedIn: []
    onOptedSaidChanged: optedAgain()
    Component.onCompleted: {
        playingHeard.who = playingWhoNow;
        optedAgain();
    }
    // And as the desktop entries are read, which a program's own acts are
    // made again on, since an act's program is found by its entry. Held
    // answers go then too: an entry read again as a new object is already
    // refused, but a player's, a stream's or a notice's sender is found
    // by its name among all the entries (names()), so one installed can
    // make what plays or sends a held program's while its entry stays.
    // No case reads the entries again, as DesktopEntries can't be stood in.
    onOwnActsChanged: {
        startsAgain();
        optedAgain();
    }

    function optedAgain() {
        optedIn = (provider.launcher?.sources ?? [])
            .filter(other => other !== provider && (other.foundActs ?? []).length > 0)
            .reduce((all, other) => all.concat(other.foundActs
                .map(one => Object.assign({ from: other }, one))), []);
        optedActs = optedMade();
    }

    // Every program's acts found from the top: its desktop entry's own,
    // made again only as the entries are read, then those opted in, made
    // again as another provider's change. Made together, every program's
    // own were made again with each gallery swayimg gives, twice an
    // opening on Joel's shell (2026-11-09).
    readonly property var ownActs: {
        const taken = provider.taken();
        return DesktopEntries.applications.values
            .reduce((all, entry) => all.concat(provider.menu(entry, taken)
                .map(act => provider.foundAct(entry, act))), []);
    }
    // Each provider's acts for a program are asked once, however many of
    // them it opts in, as swayimg opts in a gallery a folder, and only
    // those it finds from the top (Provider.foundActsOf).
    property var optedActs: []
    function optedMade() {
        const asked = new Map();
        return provider.optedIn.map(one => {
            const entry = DesktopEntries.byId(one.program);
            if (!entry)
                return null;
            const by = asked.get(one.from) ?? new Map();
            asked.set(one.from, by);
            if (!by.has(one.program))
                by.set(one.program, one.from.foundActsOf(one.program));
            const act = by.get(one.program).find(row => row.key === one.key);
            return act ? provider.foundAct(entry, act) : null;
        }).filter(row => row !== null);
    }
    readonly property var acts: ownActs.concat(optedActs)

    // One of a program's acts as a row found from the top, by its own name
    // only, so typing a program's name doesn't list every act it has. It
    // sinks below anything else found, whatever its tier (Rank.js): a
    // call's link just copied comes before Meet's Join by Code for "join",
    // and the Clipboard's picture before GIMP's New Image for "image".
    // What Joel picks still lifts it, as it does any row.
    function foundAct(entry, act) {
        const trail = ["collection:applications", `program:${entry.id}`];
        const row = thing({
            key: `program-act:${entry.id}:${act.key}`,
            name: act.name,
            verb: act.verb,
            glyph: programGlyph(entry.id),
            hue: "program",
            note: `${provider.collection.title} \u203a ${entry.name}`,
            stays: act.stays === true,
            state: act.state
        });
        row.weight = provider.weight / 2;
        row.sinks = true;
        // One that asks, as Join by Code does for words and Neovim's Open
        // a File by Name for a choice, goes to its ask inside the program
        // on Return and the right arrow alike, since what it asks for is
        // typed or chosen there. Going there changes nothing yet, so
        // Return from the find goes at once rather than landing, though
        // its verb, as join, may be one Catalogue.js guesses at.
        if (act.given || act.targets)
            return Object.assign(row, {
                trail: trail.concat([act.key]),
                spot: "",
                stays: true,
                changes: false,
                run: () => provider.launcher?.goTo(row)
            });
        return Object.assign(row, { inside: { trail: trail, spot: act.key }, run: act.run });
    }

    things: programs.concat(acts)

    // Quickshell lists no entry that hides itself, so a hidden one another
    // provider declares is looked up by its id, as its list changes.
    readonly property var programs: DesktopEntries.applications.values
        .concat(provider.declared.map(id => DesktopEntries.byId(id))
            .filter(entry => entry?.noDisplay))
        .map(entry => programRow(entry, null))

    // A program's row, from its entry, and what Programs serves of it
    // (`served`, its ready row) where it's made for Programs: then what it
    // is, its windows and the window Return goes to are the daemon's, and
    // a search's row asks the windows held as it's read.
    function programRow(entry, served) {
        const start = () => Niri.spawn(provider.command(entry));
        const below = () => provider.launcher.grid.under(Niri.current, provider.command(entry));
        const binds = provider.opening(provider.command(entry));
        const row = thing({
            thing: true,
            key: `program:${entry.id}`,
            is: `program:${entry.id}`,
            // A program says more than any one of its windows.
            rich: 1,
            name: entry.name,
            keywords: [entry.genericName, entry.comment]
                .concat(Array.from(entry.keywords), binds.map(bind => `${bind.does} ${bind.also}`))
                .join(" "),
            glyph: programGlyph(entry.id),
            hue: "program",
            // What it is follows its name, in its words, on every row,
            // in a place as at home, as the row fields rule has it: the
            // far end keeps its chord (VKCF3Z, 2026-11-06).
            count: entry.genericName.toLowerCase(),
            chord: binds.length > 0 ? binds[0].chord : [],
            stands: binds.map(bind => bind.does),
            // Then the action's words are what it's called, and count as
            // its name, as a collection's do (Launcher.qml).
            named: binds.length > 0,
            // Everything it is and does, under the row (Program.js).
            drawer: () => provider.programDrawer(entry),
            below: below,
            verbs: () => provider.controls(entry, start, below),
            under: () => provider.under(entry),
            operands: () => provider.operands(entry)
        });
        // Each row is made again with its getters rather than a field
        // turned into one (Provider's gotten), its note among them.
        const noted = provider.readTimed(() => provider.noteOf(entry));
        if (provider.stepsIn(entry)) {
            const stepping = provider.gotten(row, { note: noted });
            return Object.assign(stepping, {
                verb: "more",
                stays: true,
                nested: true,
                run: () => provider.launcher?.enter(stepping),
                targets: stepping.verbs
            });
        }
        if (served)
            return provider.madeServed(row, served, entry, start);
        // Whether it's running, and so what Return does, is asked as
        // the row is read, never kept.
        const latest = () => (provider.running[entry.id] ?? [])[0] ?? null;
        return provider.gotten(row, {
            note: noted,
            verb: () => latest() ? "go to" : "start",
            // What it is, then how many windows it has open, after its
            // name; only what it is while its say counts what it holds,
            // which says it's open already, so the say isn't cut for
            // saying it twice (VKCF3Z, 2026-11-07).
            actsOnly: () => provider.startsOnly(entry, start, below),
            // A closed one has no windows to count, so it asks nothing
            // of what it holds: a step into Programs reads every row it
            // draws (2026-11-10).
            count: provider.readTimed(() => {
                const windows = provider.windowCount(entry);
                return [entry.genericName.toLowerCase(),
                    windows !== "" && provider.holding(entry) === "" ? windows : ""]
                    .filter(part => part !== "").join(" \u00b7 ");
            }),
            // The layer of its own keys it holds as Shortcuts, so the
            // layer, named as the program is, isn't listed beside it
            // (Rank.js's keysHeld).
            ownLayer: () => provider.ownLayer(entry),
            run: () => {
                const window = latest();
                return window ? () => Niri.act({ FocusWindow: { id: window.id } }) : start;
            }
        });
    }

    // What another provider says of a program, read as its row is, so it's
    // never kept past being true.
    function noteOf(entry) {
        return provider.givenNote(entry.id) || provider.holding(entry);
    }

    // A program's row for Programs, saying what the daemon served: what it
    // is, then its windows unless another provider's say counts what it
    // holds, which says it's open already (VKCF3Z, 2026-11-07).
    function madeServed(row, served, entry, start) {
        const window = served.window ?? null;
        // Made again with its getters (Provider's gotten); its verb and
        // run stay fields, so they're only written.
        return Object.assign(provider.gotten(row, {
            note: provider.readTimed(() => provider.noteOf(entry)),
            // A closed one asks nothing of what it holds.
            count: provider.readTimed(() => served.count === served.what
                || provider.holding(entry) === "" ? served.count : served.what),
            actsOnly: () => provider.startsOnly(entry, start, row.below),
            ownLayer: () => provider.ownLayer(entry)
        }), {
            verb: served.verb,
            run: window === null ? start : () => Niri.act({ FocusWindow: { id: window } })
        });
    }
}

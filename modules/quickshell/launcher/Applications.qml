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
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import qs
import QtQuick
import "../FileFacts.js" as FileFacts
import "../Fuzzy.js" as Fuzzy
import "HeldWindows.js" as HeldWindows
import "Captured.js" as Captured
import "Media.js" as Media
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
    // here into its program's row by its id, once its record changes.
    // A search ranks the programs here still, until the programs daemon
    // ranks them on ask too, due Fri 13 Nov 12:00 (PLAN-20261110-XGN3ZV).
    function everything() {
        return ready.rows();
    }
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/programs.sock`
        place: "programs"
        // The hidden programs other providers give acts to are listed too.
        watching: ({ shown: provider.declared })
        make: row => provider.served(row)
        drawn: provider.launcher?.open ?? false
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

    // Inside Programs, by its name or what it is, the best grade first.
    function search(typed) {
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

    // Held from the start, so the streams it binds say which program each
    // is before a program is first stepped into.
    readonly property var playing: Playing

    // How far Louder and Quieter go, as a share of the whole.
    readonly property real step: 0.05

    // How every player and stream stands, in what a program's Media and
    // Sound say of them, as one string. Mpris and PipeWire say when any of
    // it changes, so nothing is polled, and while something is stepped
    // into the launcher hears that this provider changed, which makes a
    // program held from the top again (Launcher.qml's refreshed): Pause
    // turns to Play when a media key pauses its player, Previous comes
    // once there's a track before, and the volume's bar moves with Louder.
    readonly property string playingNow: Playing.players.map(player => [player.dbusName,
        player.isPlaying, player.canPlay, player.canPause, player.canGoNext,
        player.canGoPrevious, player.trackTitle, player.trackArtist].join("\u241f"))
        .concat((Playing.streams.concat(Playing.recorders ?? [])).map(node => node.audio
            ? `${node.id} ${node.audio.volume} ${node.audio.muted}` : String(node.id)))
        .join("\u241e")

    onPlayingNowChanged: redrawHeld()

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

    // Each program's open windows by its desktop entry's id, the one used
    // last first, as the windows daemon says them (modules/windows), which
    // names each window's owner: its entry, or for a browser's window of
    // its own opened deeper than its program's own page, that program.
    // Read when a row is drawn or run, so a window opening never prepares
    // every program again. Heard only while it's wanted: every event niri
    // sent once walked all of Joel's windows here, launcher shut or open
    // (2026-11-10).
    property var running: ({})
    readonly property var windowsHeld: HeldWindows.kept()
    readonly property var windowsDaemon: daemon

    // With the daemon down its row says so, since the counts it keeps
    // through the gap may no longer be true. It never says it's still
    // reading (Provider.stillReading), though its counts are the last
    // opening's until this watch's first answer: Programs is a home row,
    // so every terminal's show from home would ask twice, for counts the
    // daemon answers within a frame.
    function noted() {
        return windowsUnanswered ? Words.of("windows").said.silentFrom : "";
    }
    // Whether the windows daemon has been silent a second, as Windows.qml
    // waits too, so a restart doesn't blink it (VKCF3Z, 2026-11-10).
    property bool windowsUnanswered: false
    Connections {
        target: daemon
        function onStateChanged() {
            if (daemon.state === "not answering") {
                windowsQuiet.restart();
            } else {
                windowsQuiet.stop();
                provider.windowsUnanswered = false;
            }
        }
    }
    Timer {
        id: windowsQuiet

        interval: 1000
        onTriggered: provider.windowsQuietFor()
    }
    // The second's silence over: said if the daemon is silent still.
    function windowsQuietFor() {
        windowsUnanswered = daemon.state === "not answering";
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        kind: "window"
        live: provider.wanted
        paced: provider.launcher
        answersWhole: true
        onHeard: (changed, gone) => provider.hearWindows(changed, gone)
    }

    // Takes what the daemon said, and lists each program's windows again
    // only when a window's record differs, so a whole answer as the
    // launcher opens that reads the same changes nothing. The rows read
    // their counts and Go To as they're drawn, so they're drawn again
    // when a program's windows came or went or another was used last;
    // a title alone changes only what a row stepped into holds.
    function hearWindows(changed, gone) {
        provider.tally.looked += changed.length + gone.length;
        const held = provider.windowsHeld;
        const records = changed.map(key => daemon.held[key]).filter(record => record);
        if (HeldWindows.take(held, records, gone).length === 0 && gone.length === 0)
            return;
        const by = {};
        for (const key of held.order) {
            const owner = held.records[key].owner;
            if (owner !== null)
                (by[owner] ?? (by[owner] = [])).push(held.windows[key]);
        }
        const was = provider.running;
        provider.running = by;
        if (HeldWindows.ownersMoved(was, by))
            provider.changed();
        else
            redrawHeld();
    }

    // When a window was last looked at, in nanoseconds, or 0 for never.
    function focusedAt(window) {
        const stamp = window.focus_timestamp;
        return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
    }

    // How many windows a program has open, as the row says it beside the
    // name, before what it is.
    function windowCount(entry) {
        const open = (running[entry.id] ?? []).length;
        return open === 0 ? "" : open === 1 ? "1 window" : `${open} windows`;
    }

    // A window's title as its rows say it, without the program's name.
    function titled(window, entry) {
        return provider.tidy(window.title ?? "", entry.name) || entry.name;
    }

    // Its windows but the one used last, which Go To already goes to, as
    // rows Return goes to, each by its own title, saying where it is as the
    // Windows collection does: nothing on the workspace in front, and a
    // column when two are titled alike. Each holds what Windows' row for it
    // does (Windows.qml's actsOf), so a window has one set of insides
    // wherever it shows.
    // Whether a program has a window open now.
    function isRunning(entry) {
        return (running[entry.id] ?? []).length > 0;
    }

    // The pages a closed browser's saved session holds, each as a line of
    // the sites it visited (`line`, its address, a tab and its title) and
    // when it was last looked at (`at`, in milliseconds), newest first,
    // each once.
    function savedSites(entry) {
        const seen = new Set();
        return provider.recents.openIn(entry.id).map(line => line.split("\t"))
            .filter(tab => (tab[4] ?? "") !== "")
            .sort((a, b) => Number(b[3]) - Number(a[3]))
            .filter(tab => !seen.has(tab[4]) && seen.add(tab[4]))
            .map(tab => ({ line: `${tab[4]}\t${tab[5] ?? ""}`, at: Number(tab[3]) || undefined }));
    }

    // Its windows but the one used last and those a row it has open
    // already stands for (`held`, window ids), so a window and its front
    // tab or its file are never two rows.
    function windowsOf(entry, held) {
        return (running[entry.id] ?? []).slice(1).filter(window => !(held ?? []).includes(window.id))
            .map(window => windowRow(window, entry));
    }

    // One of a program's windows, as its row, by its title.
    function windowRow(window, entry) {
        const listing = (provider.launcher?.sources ?? [])
            .find(source => typeof source.actsOf === "function") ?? null;
        const alike = (running[entry.id] ?? [])
            .filter(other => titled(other, entry) === titled(window, entry)).length;
        return thing({
            thing: true,
            key: `window:${window.app_id}:${provider.tidy(window.title ?? "", entry.name)}`,
            is: `window:${window.id}`,
            verb: "go to",
            name: titled(window, entry),
            glyph: programGlyph(entry.id),
            hue: "program",
            side: provider.placeOf(window, Niri.workspaces[window.workspace_id], Niri.workspace(), alike),
            run: () => Niri.act({ FocusWindow: { id: window.id } }),
            verbs: listing ? () => listing.actsOf(window, entry) : null
        });
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
        const own = Array.from(entry.command).join("\u0000");
        return Array.from(entry.actions).map(action => ({
            action: action,
            name: provider.shortened(action.name, entry.name)
        })).filter(one => one.name !== "" && !taken.includes(one.name.toLowerCase())
            && Array.from(one.action.command).join("\u0000") !== own)
            .map(one => thing({
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

    // Its player that matters: the one playing, or else the first.
    function playerOf(same) {
        const players = Playing.players.filter(player => same(player.desktopEntry)
            || same(player.identity));
        return players.find(player => player.isPlaying) ?? players[0] ?? null;
    }

    // A track and who plays it, a dot between them, as every part of a
    // value is (VKCF3Z, 2026-11-08).
    function nowPlaying(player) {
        return player.trackArtist === "" ? player.trackTitle
            : `${player.trackTitle} \u00b7 ${player.trackArtist}`;
    }

    // Media: playing or pausing, one row by how it stands, then the next
    // and the last track, then what's playing. Next and Previous keep the
    // launcher, since they're pressed again and again, and the foot and
    // the last row say what's on now; a toggle is pressed once and goes.
    function media(player) {
        const row = said("program:media", "", {
            hue: "program",
            // Everything the player says, under the row (Media.js).
            thing: true,
            drawer: () => provider.mediaDrawer(player),
            targets: () => {
                const rows = [];
                const going = player.isPlaying;
                if (going ? player.canPause : player.canPlay)
                    rows.push(provider.said("verb:program:play", going ? "playing" : "", {
                        stays: true,
                        run: () => {
                            if (player.isPlaying)
                                player.pause();
                            else
                                player.play();
                        }
                    }));
                if (player.canGoNext)
                    rows.push(provider.said("verb:program:next", "", {
                        stays: true,
                        state: () => provider.nowPlaying(player),
                        run: () => player.next()
                    }));
                if (player.canGoPrevious)
                    rows.push(provider.said("verb:program:previous", "", {
                        stays: true,
                        state: () => provider.nowPlaying(player),
                        run: () => player.previous()
                    }));
                if (player.trackTitle !== "")
                    rows.push(provider.figure("program:playing", () => player.trackTitle,
                        () => player.trackArtist, null));
                return rows;
            }
        });
        return gotten(row, { side: () => player.isPlaying ? "playing" : "paused" });
    }

    // A player's drawer, where it is in its track read again each second
    // while one plays and the launcher shows, since MPRIS says a position
    // only when asked. Its art may be the network's, so only with lookups.
    function mediaDrawer(player) {
        void playedClock.date;
        return Media.drawer(player, id => Words.name(`program:media:${id}`),
            Words.of("program:media").said, settings.lookups !== false);
    }

    SystemClock {
        id: playedClock
        enabled: !!provider.launcher && provider.launcher.open
            && Playing.players.some(player => player.isPlaying)
        precision: SystemClock.Seconds
    }

    // A program's drawer (Program.js): the workspaces its windows are on
    // and what it is, what it plays, what other providers say of it
    // (their programFacts), the kinds of file and links it opens, its
    // categories, and its command and id last; its comment is its words.
    // What its entry says that Quickshell's doesn't hold, the kinds of
    // file it opens, its keywords and its package, is the programs
    // daemon's record of it (`entryOf`).
    function programDrawer(entry) {
        if (entry.id === undefined || entry.name === undefined)
            return null;
        const looked = {};
        const player = playerOf(given => provider.names(entry, given, looked));
        // Each process once, as two windows are often one process's.
        const pids = Array.from(new Set((running[entry.id] ?? []).map(window => window.pid)
            .filter(pid => Number.isInteger(pid) && pid > 0))).sort((a, b) => a - b);
        return Program.drawer({
            name: entry.name,
            usage: Program.usage(treesOf(pids)),
            asking: pids.length > 0,
            places: (running[entry.id] ?? [])
                .map(window => Niri.workspaces[window.workspace_id]?.idx)
                .filter(place => place !== undefined),
            playing: player && player.trackTitle !== "" ? nowPlaying(player) : "",
            given: hooked("programFacts", entry.id),
            record: entryOf(entry.id),
            categories: entry.categories,
            command: entry.command,
            id: entry.id,
            comment: entry.comment,
            marks: { group: settings.group, point: settings.point }
        }, id => Words.name(`applications:${id}`), Words.of("applications").said);
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
    function notices(same) {
        const past = (provider.launcher?.sources ?? [])
            .find(source => typeof source.sentBy === "function") ?? null;
        const sent = past ? past.sentBy(same) : [];
        if (sent.length === 0)
            return [];
        const unread = sent.filter(row => row.unread).length;
        return [said("program:notices", "", {
            hue: "program",
            // Its figures at its right, where a row's stand (VKCF3Z,
            // 2026-11-05: one place for each kind of fact).
            side: unread > 0 ? `${sent.length} \u00b7 ${unread} unread` : String(sent.length),
            ask: "which?",
            nested: true,
            targets: () => past.sentBy(same)
        })];
    }

    // Shortcuts, which says which layer is whose, or null where there's none.
    function keyMap() {
        return (provider.launcher?.sources ?? [])
            .find(source => typeof source.layerOf === "function") ?? null;
    }

    // The name of the layer of its own keys, or "" for none.
    function ownLayer(entry) {
        const map = keyMap();
        const own = map ? map.ofProgram(entry.id) : "desktop";
        return own === "desktop" ? "" : own;
    }

    // The sources of each program's Recent: a page's own list, one with a
    // `name` as Chrome's downloads, is its page's to read, not Recent's.
    readonly property var recentSources: (settings.recents ?? []).filter(each => !each.name)

    // What each program had open lately, from the files they write.
    readonly property Recents recents: Recents {
        sources: provider.recentSources
        showing: provider.launcher?.open ?? false
    }

    // A list read after a program was stepped into, or changing while it's
    // shown, draws its Recent again, as Media does with what's playing.
    Connections {
        target: provider.recents

        function onLatelyChanged() {
            provider.redrawHeld();
        }

        function onSharedChanged() {
            provider.redrawHeld();
        }

        function onOpenNowChanged() {
            provider.redrawHeld();
        }

        function onClosedChanged() {
            provider.redrawHeld();
        }
    }

    // What another provider gives a program (recentOf, programActs)
    // changing while it's shown draws it again too: the launcher remakes a
    // step only for the provider its row came from, and a program's row
    // is this one's.
    Variants {
        model: (provider.launcher?.sources ?? []).filter(other => other !== provider
            && (typeof other.recentOf === "function" || typeof other.programActs === "function"))

        Connections {
            required property var modelData

            target: modelData

            function onChanged() {
                provider.redrawHeld();
            }
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

    // Recent: what it had open lately and has open no more (Recents.qml),
    // laid out as every Recent is (Recent.js): newest first, each once,
    // never what it has open now, which its step lists above it
    // (`openRows`), nine at most. Each is a folder or a file by its name,
    // beside the folder it's in.
    // Return opens it through the program as its own list's settings say:
    // `open` while a window of it is open, through the process behind the
    // window used last ({pid}), and `start` while none is. A file from the
    // desktop's list programs share opens in the program, by its own
    // command given the file, when its entry takes files; one given to a
    // program that opens none, such as a chat's attachment, opens as the
    // desktop opens it, which is what the list itself says of those.
    // A site Chrome visited, its source's `site` kind, is the page's title
    // beside its host, opened by the program's own command given the page
    // when its source names no command of its own. A closed browser's
    // saved session is the sites it visited last, before its history. A
    // file source may say what's open now, as mpv's says the file each mpv
    // plays, and one saying no more than that keeps the list programs
    // share for the rest. Any other provider may give a program's Recent
    // its own things after these, each its one row, by
    // `recentOf(program)`, newest first, as Docs' documents are given
    // (Documents.qml).
    //
    // It stands wherever the program has a source of what it had, its own
    // list's settings or a provider that says it keeps some
    // (`keepsRecentOf(program)`), saying "none" while it holds nothing, and
    // an ellipsis until a source read only as it's stepped into first
    // answers, so it never comes late at the bottom of a step already
    // drawn. Its side says how many, and its drawer their names, as a
    // place's does (VKCF3Z, dafc2199).
    function recent(entry) {
        if (!provider.keepsRecent(entry))
            return [];
        provider.recents.ask(entry.id);
        const words = Words.of("program:recent");
        const kept = provider.kept(entry);
        // Named from the rows themselves, kept while each reads the same,
        // and only those it holds now are kept on.
        const keeping = provider.recentKeeper(entry);
        keepingBegun(keeping);
        const names = kept.map(one => provider.made(entry, one).name);
        keptOnly(keeping);
        const source = provider.sourceOf(entry);
        const waiting = source?.asked === true && !(entry.id in provider.recents.lately);
        const stands = Recent.said(names, waiting, words.said);
        return [said("program:recent", "", {
            hue: "program",
            side: stands.side,
            heldNames: stands.held,
            ask: "which?",
            nested: true,
            empty: waiting ? "\u2026" : words.empty,
            // Its verbs are the same rows as its targets, as People's
            // Recent's are, so it steps in inside a program whose step
            // lists a thing's verbs, as Tuple's does, holding only acts.
            verbs: () => provider.kept(entry).map(one => provider.made(entry, one)),
            targets: () => provider.kept(entry).map(one => provider.made(entry, one))
        })];
    }

    // Its list's settings, or null where it has none.
    function sourceOf(entry) {
        return recentSources.find(each => each.app === entry.id) ?? null;
    }

    // Whether it has a source of what it had: its own list's settings, the
    // list programs share naming it, or another provider keeping some or
    // giving some now.
    // `given`, what other providers give it, is asked here when not passed,
    // as it is by a note that reads it once for all it counts.
    function keepsRecent(entry, given) {
        return provider.sourceOf(entry) !== null || provider.recents.of(entry.id).length > 0
            || (provider.launcher?.sources ?? []).some(other => other !== provider
                && typeof other.keepsRecentOf === "function" && other.keepsRecentOf(entry.id))
            || (given ?? givenRecent(entry.id)).length > 0;
    }

    // What its Recent holds, as Recent.js lays them out, each { key, at,
    // row } with `row` what its row is made from (`made`), so counting
    // them makes no rows: a closed browser's saved pages, what its list
    // keeps and what other providers give, merged by when each was last
    // used where its source says (a dated source, recents.nix), none of
    // what it has open now.
    // `asked` and `opens`, what other providers give it and list as open in
    // it, are asked here when not passed, as keepsRecent's `given` is.
    function kept(entry, asked, opens) {
        const source = provider.sourceOf(entry);
        const site = source?.kind === "site";
        const kind = site ? "site" : source?.list && source.kind === "folder" ? "folder" : "file";
        const keyed = line => {
            const tab = line.indexOf("\t");
            return keyOf(kind, site && tab >= 0 ? line.slice(0, tab) : line);
        };
        const saved = site && !provider.isRunning(entry) ? provider.savedSites(entry)
            .map(one => ({ key: keyed(one.line), at: one.at, row: { line: one.line } })) : [];
        const listed = provider.recents.of(entry.id).map(line => ({
            key: keyed(line),
            at: provider.recents.atOf(entry.id, line),
            row: { line: line }
        }));
        const given = (asked ?? givenRecent(entry.id))
            .map(row => ({ key: row.key, at: row.at, row: { made: row } }));
        return Recent.listed([saved, listed, given], provider.openKeys(entry, opens),
            provider.recents.most);
    }

    // One of what its Recent holds as its row, kept while its line is, so
    // a step held, drawn again at every change of Programs (redrawHeld),
    // makes none of its Recent again: Chrome's step on Joel's shell was
    // made again whole as its downloads were read (2026-11-10, 136 ms).
    function made(entry, one) {
        if (one.row.made)
            return one.row.made;
        const source = provider.sourceOf(entry);
        const line = one.row.line;
        // Kept while it says the same when, too, so a file used again, or
        // an hour on, never keeps saying "just now".
        const when = Number.isFinite(one.at) ? provider.usedWhen(one.at) : "";
        return keptOr(provider.recentKeeper(entry), line, `${line}\t${when}`,
            () => source?.kind === "site" ? provider.visited(entry, line)
                : provider.lately(entry, source?.list ? source : null, line, one.at));
    }

    // Each program's keeper of its Recent's rows, by its desktop entry's
    // id, made anew when its entry or its list's settings are, as when
    // the desktop entries are read again, so no kept row holds one gone.
    // A plain object, so keeping notifies nothing.
    readonly property var recentKept: ({})
    function recentKeeper(entry) {
        const source = provider.sourceOf(entry);
        const had = recentKept[entry.id];
        if (had && had.entry === entry && had.source === source)
            return had.keeping;
        const keeping = keeper();
        recentKept[entry.id] = { entry: entry, source: source, keeping: keeping };
        return keeping;
    }

    // What it has open now, where its source says (`opened`), each the
    // thing it is, listed in its step before its Recent: every page a
    // browser has open while it runs (openSites), each file mpv plays
    // (openFiles). A browser's tabs are open only while it runs: one its
    // saved session still holds once it's closed is a site it visited,
    // in its Recent, never "open" beside a Return that starts it (VKCF3Z,
    // 2026-11-07). Not named opened, the hook the launcher calls as
    // Programs is stepped into (Provider.qml).
    function openRows(entry) {
        const source = provider.sourceOf(entry);
        if (!source?.opened || !provider.isRunning(entry))
            return [];
        provider.recents.ask(entry.id);
        return source.kind === "site" ? provider.openSites(entry, source)
            : provider.openFiles(entry, source);
    }

    // The keys of what it has open now, read from what its source said
    // without making a row, and of what other providers list as open in
    // it (givenOpen), so its Recent never lists one again.
    function openKeys(entry, given) {
        return ownOpenKeys(entry).concat(given ?? provider.givenOpen(entry.id));
    }

    function ownOpenKeys(entry) {
        const source = provider.sourceOf(entry);
        if (!source?.opened || !provider.isRunning(entry))
            return [];
        const lines = provider.recents.openIn(entry.id).map(line => line.split("\t"));
        return source.kind === "site"
            ? lines.filter(tab => (tab[4] ?? "") !== "").map(tab => keyOf("site", tab[4]))
            : lines.filter(([who, path]) => (path ?? "") !== ""
                && provider.fileWindow(entry, who) !== null).map(([, path]) => keyOf("file", path));
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
        if (!source?.opened || !provider.isRunning(entry))
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
    function givenActs(program) {
        return hooked("programActs", program);
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

    // The providers that say a program's note, found once rather than as
    // each program's row is read.
    readonly property var noting: (provider.launcher?.sources ?? [])
        .filter(other => other !== provider && typeof other.programNote === "function")

    // What every other provider with the hook `hook` gives `program`.
    function hooked(hook, program) {
        return (provider.launcher?.sources ?? [])
            .filter(other => other !== provider && typeof other[hook] === "function")
            .reduce((rows, other) => rows.concat(other[hook](program)), []);
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
    // forward (`go`) when it can, and brings its window forward either
    // way. Close It is there only when the source can close a tab
    // (`close`) and every tab of the page has an id to close it by, as a
    // tab Chrome's session file says has none, since a running Chrome may
    // give that run's ids to other tabs; it keeps the launcher, its foot saying how many tabs it
    // closes when it's more than one, and the page stays where it was,
    // said closed and holding only Copy the Link, until the launcher opens
    // again.
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
        const named = one => String(one.id ?? "") !== "";
        const go = () => {
            if (source.go && named(tab))
                Niri.spawn(ask(source.go, tab.id));
            if (window)
                Niri.act({ FocusWindow: { id: window.id } });
        };
        const goes = said("verb:recent:site:go", "", {
            key: `verb:recent:site:go:${tab.address}`,
            hue: "action",
            side: window ? titled(window, entry) : "",
            run: go
        });
        const closes = !source.close || !page.every(named) ? [] : [said("verb:recent:site:close", "", {
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
        const acts = [goes].concat(closes, window ? [windowRow(window, entry)] : []);
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
    // Copy a New Meeting's Link does, since Copy is last.
    function controls(entry, start, below) {
        // A program held open as its desktop entries are read again, as on
        // a switch, keeps the entry that's gone, which has no id or name;
        // it offers nothing until the step is drawn from the new one.
        if (entry.id === undefined || entry.name === undefined)
            return [];
        const looked = {};
        const same = given => provider.names(entry, given, looked);
        const given = givenActs(entry.id);
        const copying = row => row.verb === "copy";
        // An act given to lead, as signing in again does while a program
        // is signed out, comes straight after Return's own, since it's
        // what the program wants first.
        const leading = given.filter(row => row.leads === true && !copying(row));
        const doing = given.filter(row => row.leads !== true && !copying(row));
        const copies = given.filter(copying);
        if (stepsIn(entry)) {
            return leading.concat(doing, openRows(entry), recent(entry), notices(same), keysOf(entry),
                copies);
        }
        const windows = running[entry.id] ?? [];
        const latest = windows[0] ?? null;
        const acts = (latest ? [said("verb:program:switch", "", {
                side: titled(latest, entry),
                run: () => Niri.act({ FocusWindow: { id: latest.id } })
            })] : []).concat([said("verb:start", "", { run: start }),
            said("verb:below", "", { chord: provider.launcher?.belowChord ?? [], run: below })]);
        const taken = provider.taken();
        const player = playerOf(same);
        const streams = streamsOf(same);
        const recorders = recordersOf(same);
        return acts.slice(0, 1).concat(leading, acts.slice(1), menu(entry, taken), doing,
            openRows(entry), windowsOf(entry, heldWindows(entry)), recent(entry),
            notices(same), keysOf(entry),
            latest ? [quit(entry, windows)] : [],
            player ? [media(player)] : [],
            streams.length > 0 || recorders.length > 0 ? [sound(streams, recorders)] : [],
            copies);
    }

    // The names a program's own acts have here, which its desktop entry's
    // actions of the same name leave to them.
    function taken() {
        return ["verb:program:switch", "verb:start", "verb:below", "verb:program:quit"]
            .map(key => Words.name(key).toLowerCase());
    }

    // The acts other providers opt in to being found from the top, each
    // { program, key, from }, read again only as one's list changes, never
    // on a key.
    readonly property var optedIn: (provider.launcher?.sources ?? [])
        .filter(other => other !== provider && (other.foundActs ?? []).length > 0)
        .reduce((all, other) => all.concat(other.foundActs
            .map(one => Object.assign({ from: other }, one))), [])

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
    // those it finds from the top (Provider.foundActsOf), which write
    // nothing this reads.
    readonly property var optedActs: {
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
    // Those opted in may be read before their own binding has run, as
    // Applications is made beside the program that opts them in, and
    // concat would list undefined as a row.
    readonly property var acts: ownActs.concat(optedActs ?? [])

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
        const noted = () => provider.noteOf(entry);
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
            // A closed one has no windows to count, so it asks nothing
            // of what it holds: a step into Programs reads every row it
            // draws (2026-11-10).
            count: () => {
                const windows = provider.windowCount(entry);
                return [entry.genericName.toLowerCase(),
                    windows !== "" && provider.holding(entry) === "" ? windows : ""]
                    .filter(part => part !== "").join(" \u00b7 ");
            },
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
            note: () => provider.noteOf(entry),
            // A closed one asks nothing of what it holds.
            count: () => served.count === served.what || provider.holding(entry) === ""
                ? served.count : served.what,
            ownLayer: () => provider.ownLayer(entry)
        }), {
            verb: served.verb,
            run: window === null ? start : () => Niri.act({ FocusWindow: { id: window } })
        });
    }
}

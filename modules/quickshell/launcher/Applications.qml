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
// first (Go To, New window, New window below, its other windows by title,
// its desktop entry's own actions, Recent where it keeps what it had open
// lately, its Shortcuts where it has keys of its own, Quit), then its parts (Media while it
// has a player, Sound while it plays or records, Notifications while the
// history holds any of its), then any part another provider gives it, such
// as Slack's theme (ByHand.qml). All of it is made as the arrow is
// pressed, and again as its player or its streams change while it's open,
// never as letters are typed, and nothing is read from a file.
//
// Programs is a collection too, home's row for them: every program, those
// open first, each by name. It adds no row of its own to a search, where
// each program is found by itself.
pragma ComponentBehavior: Bound

import Quickshell
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "program"

    collection: Words.collection("applications", { searched: false })

    // Open programs first, then the rest, each A to Z, so a program moves
    // only as it opens or closes, which Joel did himself
    // (collection-order.md).
    function everything() {
        const shut = row => (running[row.key.slice("program:".length)] ?? []).length === 0;
        return things.slice().sort((a, b) => Number(shut(a)) - Number(shut(b))
            || a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }));
    }

    // Inside Programs, by its name or what it is, the best grade first.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of poolFor(things, typed)) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : { score: -1 };
            const byWord = row.keywords && (row.keywords.mask & need) === need
                ? Fuzzy.match(row.keywords, words, false) : { score: -1 };
            const best = byName.score >= byWord.score ? byName : byWord;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        matchedIn(things, typed, found.map(f => f.row));
        const grade = Math.min(3, ...found.map(f => f.grade));
        const kept = found.filter(f => f.grade === grade).sort((a, b) => b.score - a.score);
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

    function redrawHeld() {
        if (launcher?.open && launcher.held.length > 0)
            changed();
    }

    // The desktop's actions that run exactly this command.
    function opening(command) {
        const run = Array.from(command).join("\u0000");
        return Shortcuts.actions.filter(action => action.run.join("\u0000") === run);
    }

    draws: things.reduce((all, row) => all.concat(row.stands), [])

    // Each program's open windows by its desktop entry's id, the one used
    // last first, followed as niri's windows change. Read when a row is
    // drawn or run, so a window opening never prepares every program again.
    readonly property var running: {
        Niri.revision;
        const when = window => {
            const stamp = window.focus_timestamp;
            return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
        };
        const by = {};
        for (const window of Object.values(Niri.windows).sort((a, b) => when(b) - when(a))) {
            const entry = window.app_id ? DesktopEntries.heuristicLookup(window.app_id) : null;
            if (entry)
                (by[entry.id] ?? (by[entry.id] = [])).push(window);
        }
        return by;
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
    function windowsOf(entry) {
        return (running[entry.id] ?? []).slice(1).map(window => windowRow(window, entry));
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
            verb: "go",
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
            hue: "action"
        }, fields));
    }

    // A figure inside a part, whose name, side and any graph are read as
    // they're asked for, so the foot follows what Louder or Next just did.
    // The list shows a row as it was when it was given, so a program
    // stepped into is made again as its player and streams change
    // (`playingNow`). Return copies nothing.
    function figure(key, name, side, graph) {
        const row = copyRow({ key: key, name: name(), side: side(), copy: "" });
        Object.defineProperties(row, {
            name: { enumerable: true, get: name },
            side: { enumerable: true, get: side }
        });
        if (graph)
            Object.defineProperty(row, "graph", { enumerable: true, get: graph });
        return row;
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
                verb: Words.verb(key),
                name: `${Words.of(key).sure}, Closing ${closing}`,
                glyph: Words.glyph(key),
                hue: "alert",
                run: () => {
                    for (const window of provider.running[entry.id] ?? [])
                        Niri.act({ CloseWindow: { id: window.id } });
                }
            })]
        });
    }

    // Its player that matters: the one playing, or else the first.
    function playerOf(same) {
        const players = Playing.players.filter(player => same(player.desktopEntry)
            || same(player.identity));
        return players.find(player => player.isPlaying) ?? players[0] ?? null;
    }

    function nowPlaying(player) {
        return player.trackArtist === "" ? player.trackTitle
            : `${player.trackTitle}, ${player.trackArtist}`;
    }

    // Media: playing or pausing, one row by how it stands, then the next
    // and the last track, then what's playing. Next and Previous keep the
    // launcher, since they're pressed again and again, and the foot and
    // the last row say what's on now; a toggle is pressed once and goes.
    function media(player) {
        const row = said("program:media", "", {
            hue: "program",
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
        Object.defineProperty(row, "side", {
            enumerable: true,
            get: () => player.isPlaying ? "playing" : "paused"
        });
        return row;
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

    // Its Microphone, while it records: whether it's heard, saying so, and
    // a step in whether what it records is Muted, one switch saying yes or
    // no as a device's is (Sound.qml's mutedSwitch).
    function microphone(recorders) {
        const hears = () => provider.muted(recorders) ? provider.soundSaid("muted")
            : provider.soundSaid("recording");
        const row = said("program:microphone", muted(recorders) ? "muted" : "", {
            hue: "program",
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
        Object.defineProperty(row, "side", { enumerable: true, get: hears });
        return row;
    }

    // Sound: whether it's Muted, one switch saying yes or no, Louder and
    // Quieter, which keep the launcher to be pressed again, and its volume,
    // while it plays; then its Microphone, while it records.
    function sound(streams, recorders) {
        const row = said("program:sound", "", {
            hue: "program",
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
        Object.defineProperties(row, {
            side: {
                enumerable: true,
                get: () => streams.length > 0 ? provider.loudness(streams)
                    : provider.muted(recorders) ? provider.soundSaid("muted")
                    : provider.soundSaid("recording")
            },
            graph: {
                enumerable: true,
                get: () => streams.length > 0 ? provider.bar(streams) : undefined
            }
        });
        return row;
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
            count: String(sent.length),
            side: unread > 0 ? `${unread} unread` : "",
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

    // What each program had open lately, from the files they write.
    readonly property Recents recents: Recents {
        sources: provider.settings.recents ?? []
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
            if (!provider.launcher.open)
                provider.recents.forget();
        }
    }

    // Recent: what it had open lately (Recents.qml), in the order it keeps
    // them, each a folder or a file by its name, beside the folder it's in.
    // Return opens it through the program as its own list's settings say:
    // `open` while a window of it is open, through the process behind the
    // window used last ({pid}), and `start` while none is. A file from the
    // desktop's list programs share opens in the program, by its own
    // command given the file, when its entry takes files; one given to a
    // program that opens none, such as a chat's attachment, opens as the
    // desktop opens it, which is what the list itself says of those.
    // A site Chrome visited, its source's `site` kind, is the page's title
    // beside its host, opened by the program's own command given the page
    // when its source names no command of its own. What it has open now,
    // where its source says (`opened`), leads, every one of it, and up to
    // nine closed follow, a page open now never listed again as closed
    // (openSites). Any other provider may
    // give a program's Recent its own things after these, each its one
    // row, by `recentOf(program)`, newest first, as Docs' documents are
    // given (Documents.qml).
    function recent(entry) {
        const source = (settings.recents ?? []).find(each => each.app === entry.id) ?? null;
        provider.recents.ask(entry.id);
        const listed = () => {
            const open = source?.kind === "site" ? provider.openSites(entry, source) : [];
            const shown = open.map(row => row.key);
            return open.concat(provider.recents.of(entry.id)
                .map(path => source?.kind === "site"
                    ? provider.visited(entry, path) : provider.lately(entry, source, path))
                .filter(row => !shown.includes(row.key))
                .concat(givenRecent(entry.id)).slice(0, provider.recents.most));
        };
        const count = listed().length;
        if (count === 0)
            return [];
        return [said("program:recent", "", {
            hue: "program",
            count: String(count),
            ask: "which?",
            nested: true,
            targets: listed
        })];
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
    // launcher calls as Programs is stepped into (Provider.qml).
    function lately(entry, source, path) {
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
        return kind === "folder" ? Files.folder(path, thing, [act]) : Files.file(path, thing, [act]);
    }

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
            run: () => Quickshell.clipboardText = address
        })]);
        return thing({
            key: `site:${address}`,
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
    // (`close`); it keeps the launcher, its foot saying how many tabs it
    // closes when it's more than one, and the page stays where it was,
    // said closed and holding only Copy the Link, until the launcher opens
    // again.
    function openSite(entry, source, page, tabs) {
        const [tab] = page;
        const words = Words.of("recent:site").said ?? {};
        const counted = said => (said ?? "").replace("{count}", String(page.length));
        const host = hostOf(tab.address);
        if ((provider.recents.closed[entry.id] ?? []).includes(tab.address))
            return site(tab.address, tab.title, `${host} \u00b7 ${words.closed ?? ""}`, [], true);
        const front = tabs.find(one => one.window === tab.window && one.front) ?? tab;
        const window = (running[entry.id] ?? [])
            .find(one => titled(one, entry) === front.title) ?? null;
        const open = page.length > 1 ? counted(words.tabs) : words.open ?? "";
        const ask = (command, id) => (command ?? []).map(part => part.replace("{id}", id));
        const go = () => {
            if (source.go)
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
        const closes = !source.close ? [] : [said("verb:recent:site:close", "", {
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
        return site(tab.address, tab.title, `${host} \u00b7 ${open}`, acts);
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
    // one order for every program: Go To or Open, New Window, New Window
    // Below, its own actions, Recent, its windows, its notices, Shortcuts
    // and Quit, then Media and Sound while it plays. What it does comes
    // first and what ends it near the bottom, so Return on a program never
    // lands in its Recent. One that steps in has only what's given it,
    // its Recent, its notices and Shortcuts, in that order, never starting.
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
        const doing = given.filter(row => !copying(row));
        const copies = given.filter(copying);
        if (stepsIn(entry)) {
            return doing.concat(recent(entry), notices(same), keysOf(entry), copies);
        }
        const windows = running[entry.id] ?? [];
        const latest = windows[0] ?? null;
        const acts = (latest ? [said("verb:program:switch", "", {
                side: titled(latest, entry),
                run: () => Niri.act({ FocusWindow: { id: latest.id } })
            })] : []).concat([said("verb:start", "", { run: start }),
            said("verb:below", "", { chord: provider.launcher?.belowChord ?? [], run: below })]);
        const taken = ["verb:program:switch", "verb:start", "verb:below", "verb:program:quit"]
            .map(key => Words.name(key).toLowerCase());
        const player = playerOf(same);
        const streams = streamsOf(same);
        const recorders = recordersOf(same);
        return acts.concat(menu(entry, taken), doing, recent(entry), windowsOf(entry),
            notices(same), keysOf(entry),
            latest ? [quit(entry, windows)] : [],
            player ? [media(player)] : [],
            streams.length > 0 || recorders.length > 0 ? [sound(streams, recorders)] : [],
            copies);
    }

    // Quickshell lists no entry that hides itself, so a hidden one another
    // provider declares is looked up by its id, as its list changes.
    things: DesktopEntries.applications.values
        .concat(provider.declared.map(id => DesktopEntries.byId(id))
            .filter(entry => entry?.noDisplay))
        .map(entry => {
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
                side: entry.genericName.toLowerCase(),
                chord: binds.length > 0 ? binds[0].chord : [],
                stands: binds.map(bind => bind.does),
                // Then the action's words are what it's called, and count as
                // its name, as a collection's do (Launcher.qml).
                named: binds.length > 0,
                below: below,
                verbs: () => provider.controls(entry, start, below)
            });
            if (provider.stepsIn(entry))
                return Object.assign(row, {
                    verb: "more",
                    stays: true,
                    nested: true,
                    run: () => provider.launcher?.enter(row),
                    targets: row.verbs
                });
            // Whether it's running, and so what Return does, is asked as
            // the row is read, never kept.
            const latest = () => (provider.running[entry.id] ?? [])[0] ?? null;
            Object.defineProperties(row, {
                verb: { enumerable: true, get: () => latest() ? "go" : "start" },
                count: { enumerable: true, get: () => provider.windowCount(entry) },
                // The layer of its own keys it holds as Shortcuts, so the
                // layer, named as the program is, isn't listed beside it
                // (Rank.js's keysHeld).
                ownLayer: { enumerable: true, get: () => provider.ownLayer(entry) },
                run: {
                    enumerable: true,
                    get: () => {
                        const window = latest();
                        return window ? () => Niri.act({ FocusWindow: { id: window.id } }) : start;
                    }
                }
            });
            return row;
        })
}

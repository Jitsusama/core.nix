// Drives a program's step in the launcher (modules/quickshell/launcher/
// Applications.qml) through Quickshell itself, over desktop entries written
// beside this, with niri, the desktop's shortcuts, the notices' history and
// what's playing (launcher/Playing.qml) stood in. A running program with a
// player, a stream and notices holds, in order, Switch to the window used
// last, New window, New window below, its desktop entry's own actions (one
// shortened, one left to New window), Recent, its other window by title,
// Notices, Quit, then Media and Sound, each doing what it says: Quit asks once
// and closes each window, the media and sound acts reach the player and
// the program's streams only, and its figures follow what they did. A
// program with nothing of the sort holds only New window and New window
// below, and one whose player is found by its identity holds Media with
// only what that player can do, and Sound with only its Microphone while
// it records and plays nothing, Mute there reaching what it records alone.
// A program with keys of its own holds them as its Shortcuts. One that
// keeps what it had open lately holds Recent, each path by its name beside
// its folder, opened by its start command while none of its windows is
// open and through the process behind its last window while one is, and
// read again as its file changes, nine at most, a program held then drawn
// again. A program's files on the
// list programs share are its Recent too, found by its desktop entry's id
// or name, and opened by its own command when it takes files, else as the
// desktop opens them. A program whose sites are read only as it's stepped
// into reads them then, once each time the launcher opens, each site by
// its title beside its host, opened by its own command. A program whose
// entry hides itself is listed only when another provider gives it acts,
// and then steps in on Return rather than starting. Typing on
// looks only through what the last key found, and finds what looking
// through everything would. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the programs"

    readonly property string home: Quickshell.env("HOME")
    // What quiet had open lately, one path a line, as the check writes it.
    readonly property string recentFile: `${home}/recent`

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: launcher

        property bool open: false
        property var held: []
        property var belowChord: []
        property var sources: [notices, keys, giver]
        property var entered: null

        function enter(row) {
            entered = row;
        }
    }

    // Another provider giving a program's Recent its own things, as Docs'
    // documents are given, and the acts of a program whose entry hides
    // itself, as Tuple's are.
    QtObject {
        id: giver

        signal changed

        property var hiddenPrograms: ["pair"]

        function programActs(program) {
            return program !== "pair" ? [] : [{ key: "pair:room", name: "Open the Room" }];
        }

        function recentOf(program) {
            return program !== "tester" ? [] : [{ key: "gdoc:plan", name: "Plan Doc" }];
        }
    }

    // Shortcuts as a program's row asks it: Quiet has keys of its own.
    QtObject {
        id: keys

        function layerOf(app) {
            return app !== "quiet" ? null : {
                key: "shortcut-level:shortcuts:quiet",
                name: "Quiet",
                count: "4",
                side: "Volume",
                verbs: () => []
            };
        }
    }

    Launcher.PastNotices {
        id: notices

        launcher: launcher
        settings: ({ name: "notices" })
    }

    Launcher.Applications {
        id: applications

        launcher: launcher
        settings: ({
            name: "applications",
            recents: [{
                app: "quiet",
                kind: "folder",
                watch: root.recentFile,
                list: [Quickshell.env("RECENT_LIST")],
                open: ["quiet", "@", "--to", "unix:@quiet-{pid}", "launch", "{path}"],
                start: ["quiet", "--directory", "{path}"]
            }, {
                // The list programs share, as recent-files prints it.
                watch: `${root.home}/shared`,
                list: [Quickshell.env("SHARED_LIST")]
            }, {
                // Sites, read as the browser is stepped into.
                app: "browser",
                kind: "site",
                asked: true,
                list: [Quickshell.env("SITES_LIST")]
            }]
        })
        // Desktop entries and what quiet had open are read in the
        // background, so the cases wait for all three and both lists.
        onThingsChanged: root.ready()
    }

    Connections {
        target: applications.recents

        function onLatelyChanged() {
            if (root.waiting === "the recents to change")
                root.changed();
            else if (root.waiting === "the sites")
                root.browsed(1);
            else if (root.waiting === "the sites again")
                root.browsed(2);
            else
                root.ready();
        }

        function onSharedChanged() {
            root.ready();
        }
    }

    // Writes quiet's list anew, as the program would.
    FileView {
        id: writer

        path: root.recentFile
        printErrors: false
    }

    function ready() {
        if (root.waiting === "the programs"
                && ["tester", "quiet", "radio", "browser", "pair"]
                    .every(id => root.program(id) !== undefined)
                && applications.recents.of("quiet").length === 2
                && applications.recents.of("tester").length === 2
                && applications.recents.of("radio").length === 1)
            Qt.callLater(root.cases);
    }

    // How often the launcher heard the programs changed, as it would redraw.
    property int redrawn: 0

    Connections {
        target: applications

        function onChanged() {
            root.redrawn += 1;
        }
    }

    // Once its file changes, Recent says what's in it now, nine at most.
    function changed() {
        const now = applications.recents.of("quiet");
        if (now.length !== 9)
            return;
        try {
            const recent = program("quiet").verbs()[2];
            same("Recent follows its file, nine at most", `${recent.count} ${names(recent.targets())}`,
                "9 folder 1, folder 2, folder 3, folder 4, folder 5, folder 6, folder 7, folder 8, folder 9");
            same("each in the folder it's in", recent.targets()[0].side, "~");
            // Read once every handler of the change has run.
            Qt.callLater(() => {
                try {
                    expect("a program held as its list changes is drawn again",
                        root.redrawn > 0, root.redrawn);
                    root.redrawn = 0;
                    giver.changed();
                    Qt.callLater(() => {
                        try {
                            expect("a program held as another provider's Recent for it changes is drawn again",
                                root.redrawn > 0, root.redrawn);
                            root.browse();
                        } catch (error) {
                            fail(error.message);
                        }
                    });
                } catch (error) {
                    fail(error.message);
                }
            });
        } catch (error) {
            fail(error.message);
        }
    }

    // A program whose sites are read as it's stepped into: not before,
    // once an opening, and again the next time the launcher opens.
    function browse() {
        launcher.open = false;
        launcher.open = true;
        same("sites aren't read before their program is stepped into",
            applications.recents.of("browser").length, 0);
        root.waiting = "the sites";
        program("browser").verbs();
    }

    function browsed(reading) {
        if (applications.recents.of("browser").length === 0)
            return;
        try {
            const rows = program("browser").verbs();
            same("stepped into, it reads its sites as its Recent", names(rows),
                "New Window, New Window Below, Recent");
            const sites = rows[2].targets();
            same("each site by its title beside its host, keyed by the page, newest first",
                `${names(sites)} ${sites.map(row => row.side).join(" ")} ${sites[0].key}`,
                `Page ${reading}, Older page example.org other.example.net site:https://example.org/${reading}`);
            same("read once an opening, however often it's stepped into",
                JSON.stringify(applications.recents.asked), '{"browser":true}');
            if (reading === 2) {
                Qt.exit(0);
                return;
            }
            sites[0].run();
            same("Return opens the page by the program's own command", JSON.stringify(Niri.spawned),
                JSON.stringify(["browser", "https://example.org/1"]));
            launcher.open = false;
            same("closing the launcher lets the next opening read them again",
                JSON.stringify(applications.recents.asked), "{}");
            launcher.open = true;
            root.waiting = "the sites again";
            program("browser").verbs();
        } catch (error) {
            fail(error.message);
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    function program(id) {
        return applications.things.find(row => row.key === `program:${id}`);
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function near(value, wanted) {
        return Math.abs(value - wanted) < 1e-9;
    }

    // What a case found, as said when it isn't what was wanted.
    function same(what, got, wanted) {
        expect(what, got === wanted, got);
    }

    function acted() {
        return JSON.stringify(Niri.acted);
    }

    function cases() {
        if (root.waiting !== "the programs")
            return;
        root.waiting = "the cases";
        // The windows are matched to programs as niri's change, and these
        // were there before the entries were read.
        Niri.revision += 1;
        try {
            // The launcher says Programs was stepped into, as it does.
            applications.opened();
            const tester = program("tester").verbs();
            same("in the program's order, each part where it applies", names(tester),
                "Switch, New Window, New Window Below, Private window, Open the logbook, "
                + "Recent, Draft, Notices, Quit, Media, Sound");
            const [switching, , , secret, logbook, shared, draft, told, quit, media, sound] = tester;
            const files = shared.targets();
            same("its files from the list programs share, by its name or its id, newest first, then what another provider gives it",
                `${names(files)} ${files.map(row => row.key).join(" ")}`,
                `notes.md, plan.md, Plan Doc file:${root.home}/notes.md file:${root.home}/plan.md gdoc:plan`);
            files[0].run();
            same("one opens by the program's own command", JSON.stringify(Niri.spawned),
                JSON.stringify(["tester", `${root.home}/notes.md`]));
            same("a hidden program nobody gives acts to isn't listed", program("ghost"), undefined);
            const pair = program("pair");
            same("a hidden program another gives acts to steps in rather than starting",
                `${pair.verb} ${pair.stays} ${names(pair.verbs())} ${names(pair.targets())}`,
                "more true Open the Room Open the Room");
            const before = JSON.stringify(Niri.spawned);
            pair.run();
            expect("and Return steps in, starting nothing", launcher.entered === pair
                && JSON.stringify(Niri.spawned) === before, JSON.stringify(Niri.spawned));
            same("one closed still starts", program("radio").verb, "start");
            same("Switch names the window used last", switching.side, "Logbook");
            switching.run();
            same("Switch goes to it", acted(), '[{"FocusWindow":{"id":11}}]');
            same("the other window says where it is", draft.side, "workspace 2");
            same("an action is run", `${secret.verb} ${logbook.verb}`, "run run");
            expect("its full name still finds it", secret.keywords !== null, secret.keywords);
            secret.run();
            same("an action runs as its entry says", JSON.stringify(Niri.spawned),
                '["tester","--private"]');

            const sure = quit.targets();
            same("Quit asks once, saying what it closes", names(sure),
                "Quit Now, Closing 2 Windows");
            same("as an alert", sure[0].hue, "alert");
            Niri.acted = [];
            sure[0].run();
            same("Quit closes each window through niri", acted(),
                '[{"CloseWindow":{"id":11}},{"CloseWindow":{"id":12}}]');

            const player = Launcher.Playing.players[0];
            same("Media holds more, saying how it stands", `${media.verb} ${media.side}`,
                "more playing");
            const played = media.targets();
            same("Media's acts, then what's playing", names(played),
                "Pause, Next, Previous, Song of the test");
            same("what's playing says by whom", played[3].side, "The Testers");
            same("as a figure", played[3].figure, true);
            same("Next keeps the launcher", played[1].stays, true);
            same("saying what's on", played[1].state(), "Song of the test, The Testers");
            played[1].run();
            played[2].run();
            same("Next and Previous reach its player", `${player.nexts} ${player.previouses}`,
                "1 1");
            played[0].run();
            same("Pause pauses it", `${player.isPlaying} ${media.side}`, "false paused");

            const [first, second, filter, other] = Launcher.Playing.streams;
            const volumes = () => [first, second, filter, other]
                .map(node => node.audio.volume.toFixed(2)).join(" ");
            same("Sound holds more, saying how loud", `${sound.verb} ${sound.side}`, "more 40%");
            expect("as a bar", near(sound.graph.values[0], 0.4), sound.graph.values[0]);
            const heard = sound.targets();
            same("Sound's acts, then its volume", names(heard), "Mute, Louder, Quieter, Volume");
            const volume = heard[3];
            same("its volume is a figure", `${volume.side} ${volume.figure}`, "40% true");
            same("with a bar", volume.graph.shape, "bar");
            same("Louder keeps the launcher", heard[1].stays, true);
            same("saying how loud", heard[1].state(), "now 40%");
            heard[1].run();
            same("Louder turns up its streams together, and no one else's", volumes(),
                "0.45 0.45 0.90 0.70");
            same("its volume follows", `${volume.side} ${heard[1].state()}`, "45% now 45%");
            heard[2].run();
            heard[2].run();
            same("Quieter turns it down", volumes(), "0.35 0.35 0.90 0.70");
            heard[0].run();
            same("Mute mutes its streams, and no one else's",
                [first, second, other].map(node => node.audio.muted).join(" "), "true true false");
            same("its volume says so", `${volume.side} ${sound.graph.values[0]}`, "muted 0");
            same("then it says Unmute", sound.targets()[0].name, "Unmute");

            same("Notices counts its own", `${told.count} ${told.side}`, "2 1 unread");
            same("and holds more", told.nested, true);
            const sent = told.targets();
            same("its notices, newest first", names(sent), "Newest, Oldest \u00b7 said");
            Niri.acted = [];
            sent[0].run();
            same("Return on one reads it", Notices.marked.join(" "), "3");
            same("and goes to its window, as the collection's does", acted(),
                '[{"FocusWindow":{"id":11}}]');

            const quiet = program("quiet").verbs();
            same("a program with keys of its own holds them, and only its new windows besides",
                names(quiet), "New Window, New Window Below, Recent, Shortcuts");
            same("its Shortcuts counts its keys", quiet[3].count, "4");

            const recent = quiet[2];
            same("Recent holds more, counting what it had open",
                `${recent.verb} ${recent.count} ${recent.nested}`, "more 2 true");
            const lately = recent.targets();
            same("each by its name, in the order it keeps them", names(lately), "core.nix, etc");
            same("beside the folder it's in", lately.map(row => row.side).join(" "), "~/src /");
            same("Return opens it", lately.map(row => row.verb).join(" "), "open open");
            lately[0].run();
            same("with none of its windows open, by starting it there",
                JSON.stringify(Niri.spawned),
                JSON.stringify(["quiet", "--directory", `${root.home}/src/core.nix`]));
            const before = Niri.windows;
            Niri.windows = Object.assign({}, before, {
                "13": { id: 13, app_id: "quiet", pid: 4242, title: "Quiet", workspace_id: 1,
                    focus_timestamp: { secs: 50, nanos: 0 } }
            });
            Niri.revision += 1;
            lately[0].run();
            same("with one open, through the process behind it", JSON.stringify(Niri.spawned),
                JSON.stringify(["quiet", "@", "--to", "unix:@quiet-4242", "launch",
                    `${root.home}/src/core.nix`]));
            Niri.windows = before;
            Niri.revision += 1;

            const radio = program("radio").verbs();
            same("a player found by its identity, and a program recording", names(radio),
                "New Window, New Window Below, Recent, Media, Sound");
            radio[2].targets()[0].run();
            same("a file given to a program that opens none opens as the desktop opens it",
                JSON.stringify(Niri.spawned), JSON.stringify(["xdg-open", `${root.home}/song.ogg`]));
            same("Media holds only what the player can do", names(radio[3].targets()), "Play");
            const recording = radio[4];
            same("Sound says it records, with nothing to draw",
                `${recording.side} ${recording.graph}`, "recording undefined");
            const [microphone] = recording.targets();
            same("its Microphone alone, as it plays nothing", names(recording.targets()),
                "Microphone");
            same("saying it's heard", microphone.side, "recording");
            const [hush] = microphone.targets();
            same("its Microphone holds Mute", hush.name, "Mute");
            hush.run();
            const [listening, meter] = Launcher.Playing.recorders;
            same("Mute mutes what it records, and not a level meter",
                `${listening.muted} ${meter.muted}`, "true false");
            same("its Microphone says so", `${microphone.side} ${recording.side}`, "muted muted");
            same("then it says Unmute", microphone.targets()[0].name, "Unmute");

            // Typing on looks only through what the last key found
            // (Provider.qml's poolFor), and finds just what looking
            // through everything would; going back, or another word,
            // looks through everything again.
            const found = typed => names(applications.search(typed).rows);
            const whole = typed => {
                applications.matchedIn(null, "", []);
                return found(typed);
            };
            applications.matchedIn(null, "", []);
            for (const [typed, narrowed] of [["r", false], ["ra", true], ["rad", true],
                ["ra", false], ["q", false], ["qu", true], ["qu r", true]]) {
                const pool = applications.poolFor(applications.things, typed);
                same(`"${typed}" looked ${narrowed ? "through the last key's" : "whole"}`,
                    pool !== applications.things, narrowed);
                const got = found(typed);
                same(`"${typed}" found as looking whole does`, got, whole(typed));
                found(typed);
            }
            root.waiting = "the recents to change";
            // Held inside a program, as Joel would be, while its list changes.
            launcher.open = true;
            launcher.held = [{ row: program("quiet"), rows: program("quiet").verbs() }];
            root.redrawn = 0;
            writer.setText(Array.from({ length: 11 }, (_, n) => `${root.home}/folder ${n + 1}`)
                .join("\n") + "\n");
        } catch (error) {
            fail(error.message);
        }
    }
}
// Drives a program's step in the launcher (modules/quickshell/launcher/
// Applications.qml) through Quickshell itself, over desktop entries written
// beside this, with niri, the desktop's shortcuts, the notices' history and
// what's playing (launcher/Playing.qml) stood in. A running program with a
// player, a stream and notices holds, in order, Switch to the window used
// last, New window, New window below, its other window by title, its desktop
// entry's own actions (one shortened, one left to New window), Quit, then
// Media, Sound and Notifications, each doing what it says: Quit asks once
// and closes each window, the media and sound acts reach the player and
// the program's streams only, and its figures follow what they did. A
// program with nothing of the sort holds only New window and New window
// below, and one whose player is found by its identity holds Media with
// only what that player can do. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the programs"

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
        property var sources: [notices]
    }

    Launcher.PastNotices {
        id: notices

        launcher: launcher
        settings: ({ name: "notices" })
    }

    Launcher.Applications {
        id: applications

        launcher: launcher
        settings: ({ name: "applications" })
        // Desktop entries are read in the background, so the cases wait for
        // all three.
        onThingsChanged: {
            if (root.waiting === "the programs"
                    && ["tester", "quiet", "radio"].every(id => root.program(id) !== undefined))
                Qt.callLater(root.cases);
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
            const tester = program("tester").verbs();
            same("acts first, then parts, each where it applies", names(tester),
                "Switch, New window, New window below, Draft, Private window, "
                + "Open the logbook, Quit, Media, Sound, Notifications");
            const [switching, , , draft, secret, logbook, quit, media, sound, told] = tester;
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
                "Quit now, closing 2 windows");
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

            same("Notifications counts its own", `${told.count} ${told.side}`, "2 1 unread");
            same("and holds more", told.nested, true);
            const sent = told.targets();
            same("its notices, newest first", names(sent), "Newest, Oldest \u00b7 said");
            Niri.acted = [];
            sent[0].run();
            same("Return on one reads it", Notices.marked.join(" "), "3");
            same("and goes to its window, as the collection's does", acted(),
                '[{"FocusWindow":{"id":11}}]');

            same("a program with nothing more holds only its new windows",
                names(program("quiet").verbs()), "New window, New window below");

            const radio = program("radio").verbs();
            same("a player found by its identity", names(radio),
                "New window, New window below, Media");
            same("Media holds only what the player can do", names(radio[2].targets()), "Play");
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            fail(error.message);
        }
    }
}

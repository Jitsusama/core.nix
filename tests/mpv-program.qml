// Drives mpv's step in the launcher (modules/mpv/MpvProgram.qml, through
// modules/quickshell/launcher/Applications.qml) through Quickshell itself,
// over mpv's desktop entry written beside this, with niri, the desktop's
// shortcuts, what's playing and what was copied last (Copied.qml) stood in,
// and mpv-stopped by a script printing what it would. First, in Qt's own
// engine, Mpv.js's cases: which copies are one address mpv plays, a video
// site's page or a media file, and which aren't, how far into a file is
// said, and the lines mpv-stopped prints, read.
//
// Then mpv's Recent: led by the file an mpv plays now, then each file mpv
// stopped part way through, newest first, saying where it stopped before
// its folder, as "12:04 of 41:10 · ~/Videos", when it was left in its
// note and as `at`, each listed once though the list programs share has it
// too, the rest of that list after them, and mpv's row counting each once.
// Return on one plays on from where it stopped, and inside are Play On,
// saying from where, Play From the Start and Copy the Path. mpv-stopped is
// run once mpv is stepped into, never before, once each opening.
//
// And Play the Copied Link: there only while what was copied is one address
// mpv plays, straight after Go To, saying its site, found from the top as
// well, and gone once something else is copied; Return plays it. It exits
// 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/Mpv.js" as Mpv

ShellRoot {
    id: root

    property string waiting: "the programs"

    readonly property string home: Quickshell.env("HOME")
    readonly property string videos: `${home}/Videos`

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    function same(what, got, wanted) {
        expect(what, got === wanted, got);
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function program(id) {
        return applications.things.find(row => row.key === `program:${id}`);
    }

    // How many times mpv-stopped ran, as the script counts them.
    function listings() {
        return Number(counted.text().trim() || "0");
    }

    QtObject {
        id: launcher

        property bool open: false
        property var held: []
        property var belowChord: []
        property var sources: [mpv]
        // What a program's row says it holds asks these too.
        property var partsOfRows: ({})
        property string underListed: ""
    }

    Launcher.MpvProgram {
        id: mpv

        launcher: launcher
        settings: ({ name: "mpv", stopped: [Quickshell.env("STOPPED_LIST")] })
    }

    Launcher.Applications {
        id: applications

        launcher: launcher
        settings: ({
            name: "applications",
            recents: [{
                // The list programs share, as recent-files prints it.
                watch: `${root.home}/shared`,
                list: [Quickshell.env("SHARED_LIST")]
            }, {
                // What each mpv plays now, as mpv's own source says.
                app: "mpv",
                kind: "file",
                opened: [Quickshell.env("PLAYING_LIST")]
            }]
        })
        onThingsChanged: root.ready()
    }

    Connections {
        target: applications.recents

        function onSharedChanged() {
            root.ready();
        }

        function onOpenNowChanged() {
            root.ready();
        }
    }

    Connections {
        target: mpv

        function onStoppedChanged() {
            root.ready();
        }
    }

    FileView {
        id: counted

        path: `${root.home}/listed`
        printErrors: false
        blockLoading: true
    }

    function ready() {
        if (root.waiting === "the programs" && root.program("mpv") !== undefined
                && applications.recents.of("mpv").length === 3)
            Qt.callLater(root.cases);
        else if (root.waiting === "mpv's files" && mpv.stopped.length === 3
                && applications.recents.openIn("mpv").length === 1)
            Qt.callLater(root.stepped);
    }

    function cases() {
        if (root.waiting !== "the programs")
            return;
        root.waiting = "mpv's files";
        try {
            sums();
            Niri.revision += 1;
            launcher.open = true;
            program("mpv").verbs();
            counted.reload();
            counted.waitForJob();
            same("nothing is listed before mpv is stepped into", listings(), 0);
            launcher.held = [{ row: program("mpv"), rows: [] }];
        } catch (error) {
            fail(error.message);
        }
    }

    // Mpv.js's own cases, as Qt's engine runs them.
    function sums() {
        // [what was copied, whether mpv plays it]
        const links = [
            ["https://www.youtube.com/watch?v=dQw4w9WgXcQ", true],
            ["https://youtube.com/watch?feature=share&v=dQw4w9WgXcQ&t=42", true],
            ["https://m.youtube.com/shorts/abc_DEF-1", true],
            ["https://www.youtube.com/playlist?list=PL0123", true],
            ["https://music.youtube.com/watch?v=abc", true],
            ["https://youtu.be/dQw4w9WgXcQ?t=10", true],
            ["  https://vimeo.com/123456789\n", true],
            ["https://vimeo.com/channels/staffpicks/123456789", true],
            ["https://www.twitch.tv/videos/987654", true],
            ["https://www.dailymotion.com/video/x8abc", true],
            ["https://soundcloud.com/someone/a-track", true],
            ["https://www.tiktok.com/@some.one/video/7301234567890", true],
            ["https://example.com/talks/keynote.MP4", true],
            ["http://192.168.1.20:8000/films/one.mkv?token=x#t=5", true],
            ["https://cdn.example.net/live/index.m3u8", true],
            ["https://example.com/podcast/episode.mp3", true],
            ["https://www.youtube.com/", false],
            ["https://www.youtube.com/watch", false],
            ["https://www.youtube.com/@channel", false],
            ["https://vimeo.com/about", false],
            ["https://soundcloud.com/someone", false],
            ["https://example.com/page", false],
            ["https://github.com/someone/thing/blob/main/index.ts", false],
            ["https://example.com/keynote.mp4.html", false],
            ["ftp://example.com/one.mkv", false],
            ["https://user:secret@example.com/one.mkv", false],
            ["watch https://youtu.be/dQw4w9WgXcQ", false],
            ["https://youtu.be/a https://youtu.be/b", false],
            ["/home/someone/one.mkv", false],
            ["", false]
        ];
        for (const [copied, plays] of links)
            same(`whether mpv plays ${JSON.stringify(copied)}`, Mpv.playable(copied),
                plays ? copied.trim() : "");
        same("a link's site", Mpv.hostOf("https://m.youtube.com/shorts/abc"), "youtube.com");

        // [seconds, as said]
        const clocks = [[0, "0:00"], [9.9, "0:09"], [724.5, "12:04"], [2470.2, "41:10"],
            [3600, "1:00:00"], [3723, "1:02:03"], [-4, "0:00"]];
        for (const [seconds, said] of clocks)
            same(`${seconds} seconds in`, Mpv.clock(seconds), said);

        const read = Mpv.stopped(["/a.mkv\t724.5\t2470.2\t1700000000", "relative.mkv\t1\t2\t3",
            "/b.mkv\t\t60\t1700000000", "/c.mkv\t5\tN/A\t1700000000", "/d.mkv\t5\t60"]);
        same("only whole lines are read", JSON.stringify(read),
            JSON.stringify([{ path: "/a.mkv", start: 724.5, length: 2470.2, at: 1700000000000 }]));
        same("where it stopped", Mpv.where(read[0], "{at} of {length}"), "12:04 of 41:10");
        same("playing on", JSON.stringify(Mpv.play("/a.mkv", false)),
            JSON.stringify(["mpv", "--player-operation-mode=pseudo-gui", "--", "/a.mkv"]));
        same("playing from the start", JSON.stringify(Mpv.play("/a.mkv", true)),
            JSON.stringify(["mpv", "--player-operation-mode=pseudo-gui", "--no-resume-playback",
                "--", "/a.mkv"]));
    }

    // Stepped into mpv, with its files listed and what it plays now read.
    function stepped() {
        if (root.waiting !== "mpv's files")
            return;
        root.waiting = "the link";
        try {
            counted.reload();
            counted.waitForJob();
            same("mpv-stopped runs as mpv is stepped into", listings(), 1);
            const recent = program("mpv").verbs().find(row => row.name === "Recent");
            const files = recent.targets();
            same("what plays now, then where it stopped, newest first, then the rest, each once",
                files.map(row => row.key).join(" "),
                ["now", "long", "talk", "song"].map(name => `file:${root.videos}/${name}.mkv`)
                    .join(" "));
            same("Recent counts each once", recent.side, "4");
            same("so does mpv's row", program("mpv").note, "1 open \u00b7 3 recent");
            const talk = files[2];
            same("where it stopped, before its folder", talk.side,
                "12:04 of 41:10 \u00b7 ~/Videos");
            same("past an hour", files[1].side, "1:02:03 of 1:40:00 \u00b7 ~/Videos");
            same("when it was left", talk.note, "stopped 2 h ago");
            expect("when it was left, to be ordered by", Math.abs(talk.at
                - (Number(Quickshell.env("NOW")) - 7200) * 1000) < 1, talk.at);
            same("the file playing now, as it plays", files[0].side, "open \u00b7 ~/Videos");
            talk.run();
            same("Return plays on", JSON.stringify(Niri.spawned),
                JSON.stringify(Mpv.play(`${root.videos}/talk.mkv`, false)));
            const inside = talk.verbs();
            same("inside, Play On first, then Play From the Start", names(inside),
                "Play On, Play From the Start, Copy the Path");
            same("Play On says from where", inside[0].side, "from 12:04");
            inside[1].run();
            same("Play From the Start leaves where it stopped unread",
                JSON.stringify(Niri.spawned),
                JSON.stringify(Mpv.play(`${root.videos}/talk.mkv`, true)));

            same("nothing copied, nothing to play", names(program("mpv").verbs()).includes("Link"),
                false);
            same("nor found from the top", mpv.foundActs.length, 0);
            Copied.text = "https://youtu.be/dQw4w9WgXcQ ";
            const acts = program("mpv").verbs();
            same("a link copied leads, straight after Go To",
                `${acts[0].name}, ${acts[1].name} ${acts[1].side}`,
                "Go To, Play the Copied Link youtu.be");
            acts[1].run();
            same("Return plays it", JSON.stringify(Niri.spawned),
                JSON.stringify(Mpv.play("https://youtu.be/dQw4w9WgXcQ", false)));
            same("found from the top too", JSON.stringify(mpv.foundActs),
                JSON.stringify([{ program: "mpv", key: "verb:mpv:link" }]));
            Copied.text = "see https://youtu.be/dQw4w9WgXcQ";
            same("gone once something else is copied",
                names(program("mpv").verbs()).includes("Link"), false);

            // Stepped out and in again, nothing's listed again until the
            // launcher opens afresh.
            launcher.held = [];
            launcher.held = [{ row: program("mpv"), rows: [] }];
            counted.reload();
            counted.waitForJob();
            same("once an opening", listings(), 1);
            root.waiting = "listed again";
            launcher.held = [];
            launcher.open = false;
            launcher.open = true;
            again.start();
        } catch (error) {
            fail(error.message);
        }
    }

    // Long enough for a listing started on opening to have run.
    Timer {
        id: again

        interval: 1000
        onTriggered: {
            try {
                counted.reload();
                counted.waitForJob();
                root.same("nothing is listed on opening while mpv isn't held",
                    root.listings(), 1);
                launcher.held = [];
                launcher.held = [{ row: root.program("mpv"), rows: [] }];
                listedAgain.start();
            } catch (error) {
                root.fail(error.message);
            }
        }
    }

    Timer {
        id: listedAgain

        interval: 1000
        onTriggered: {
            try {
                counted.reload();
                counted.waitForJob();
                root.same("and listed again once it's stepped into", root.listings(), 2);
                Qt.exit(0);
            } catch (error) {
                root.fail(error.message);
            }
        }
    }
}

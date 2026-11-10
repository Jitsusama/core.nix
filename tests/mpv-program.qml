// Drives mpv's step in the launcher (modules/mpv/MpvProgram.qml, through
// modules/quickshell/launcher/Applications.qml) through Quickshell itself,
// over mpv's desktop entry written beside this, with niri, the desktop's
// shortcuts and what's playing stood in, the shell's own watch of the
// clipboard (Copied.qml) with wl-paste stood in, so a copy is set by
// hand, and mpv-stopped by a script printing what it would. First, in
// Qt's own engine, Mpv.js's cases: which copies are one address mpv plays,
// a video site's page or a media file, and which aren't, how far into a
// file is said, which files were watched whole, the lines mpv-stopped
// prints, read, and what mpv kept of a file, ordered and said.
//
// Then mpv's Recent: what an mpv plays now listed above it and not in it,
// then each file mpv left merged by when it left it, newest first, with
// the list programs share, which gives no times, after them, each file
// listed once, the shared list's own row giving way to mpv's, and none
// another provider says is open in mpv, which its row counts open. One stopped
// part way says "12:04 of 41:10 · 12 min ago" beside it and Return
// resumes it, with Resume, saying from where, Play From the Start and Copy
// the Path inside; one watched whole says its length and when, and Return
// plays it from its start. Each one's drawer says where it is and its
// kind, then how much is still to play, what mpv kept, its size and its
// path, in the Name face. mpv-stopped is run once mpv is stepped into,
// never before, once each opening.
//
// And Play the Copied Link: there only while what was copied is one address
// mpv plays, after New Window Below, saying its site, found from the top
// as well, and gone, never dimmed, once something else is copied; Return
// plays it. It exits 0 once every case holds, and otherwise 1, saying
// which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "launcher/Mpv.js" as Mpv
import "Copied.js" as Copying

ShellRoot {
    id: root

    property string waiting: "the programs"

    readonly property string home: Quickshell.env("HOME")
    readonly property string videos: `${home}/Videos`
    readonly property real now: Number(Quickshell.env("NOW")) * 1000

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

    function file(name) {
        return `file:${root.videos}/${name}.mkv`;
    }

    // How many times mpv-stopped ran, as the script counts them.
    function listings() {
        return Number(counted.text().trim() || "0");
    }

    // A drawer's facts as they read, a label and its value each.
    function facts(row) {
        return row.drawer.facts.map(fact => `${fact.label} ${fact.value}`).join(", ");
    }

    QtObject {
        id: theLauncher

        property bool open: false
        // The row chosen, whose drawer shows; Applications ends a program's
        // watch once it's off the program's row.
        property var selected: null
        property string typed: ""
        property var held: []
        property var belowChord: []
        property var sources: [mpv, elsewhere]
        // What a program's row says it holds asks these too.
        property var partsOfRows: ({})
        property string underListed: ""
    }

    // Another provider saying what's open in mpv, by Recent's keys, as
    // Neovim's says the files open in it (NeovimProgram.qml).
    QtObject {
        id: elsewhere

        property var open: []

        function programOpen(program) {
            return program === "mpv" ? open : [];
        }
    }

    Launcher.MpvProgram {
        id: mpv

        launcher: theLauncher
        // The files it left, which the programs daemon lists by this key.
        settings: ({ name: "mpv", stopped: "listed:mpv:stopped" })
    }

    Launcher.Applications {
        id: applications

        launcher: theLauncher
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
                && applications.recents.of("mpv").length === 4)
            Qt.callLater(root.cases);
        else if (root.waiting === "mpv's files" && mpv.stopped.length === 4
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
            theLauncher.open = true;
            program("mpv").verbs();
            counted.reload();
            counted.waitForJob();
            same("nothing is listed before mpv is stepped into", listings(), 0);
            theLauncher.held = [{ row: program("mpv"), rows: [] }];
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
            ["https://youtu.be/a\nhttps://youtu.be/b", false],
            ["/home/someone/one.mkv", false],
            ["", false]
        ];
        // Each as mpv reads it, through the shell's watch of the
        // clipboard's one web address, and by itself.
        for (const [copied, plays] of links) {
            const wanted = plays ? copied.trim() : "";
            same(`whether mpv plays ${JSON.stringify(copied)} as copied`,
                Mpv.playable(Copying.link(copied)), wanted);
            same(`whether mpv plays ${JSON.stringify(copied)}`, Mpv.playable(copied), wanted);
        }
        same("a link's site", Mpv.hostOf("https://m.youtube.com/shorts/abc"), "youtube.com");

        // [seconds, as said]
        const clocks = [[0, "0:00"], [9.9, "0:09"], [724.5, "12:04"], [2470.2, "41:10"],
            [3600, "1:00:00"], [3723, "1:02:03"], [-4, "0:00"]];
        for (const [seconds, said] of clocks)
            same(`${seconds} seconds in`, Mpv.clock(seconds), said);

        const read = Mpv.stopped([
            "/a.mkv\t724.5\t2470.2\t1700000000\t1234567890\tvideo\tspeed=1.500000\tvf=%9%lavfi=[x]",
            "relative.mkv\t1\t2\t3\t4\tvideo", "/b.mkv\t\t60\t1700000000\t4\tvideo",
            "/c.mkv\t5\tN/A\t1700000000\t4\tvideo", "/d.mkv\t5\t60\t1700000000",
            "/e.mkv\t5\t60\t1700000000\t4\tpicture", "/f.mkv\t5\t60\t1700000000\t\taudio"]);
        same("only whole lines are read, with what mpv kept", JSON.stringify(read),
            JSON.stringify([{ path: "/a.mkv", start: 724.5, length: 2470.2, at: 1700000000000,
                size: 1234567890, kind: "video", kept: [{ option: "speed", value: "1.500000" },
                    { option: "vf", value: "%9%lavfi=[x]" }] }]));
        same("still to play", Mpv.clock(Mpv.toGo(read[0])), "29:05");

        // Its drawer in Programs: how many videos and audio files it left
        // part way, one watched whole not among them.
        const counts = { video1: "1 video", videos: "{count} videos", audio1: "1 audio file",
            audios: "{count} audio files" };
        const left = Mpv.stopped(["/a.mkv\t724\t2470\t1\t1\tvideo",
            "/b.mkv\t5\t60\t1\t1\tvideo", "/c.mp3\t60\t60\t1\t1\taudio",
            "/d.mp3\t5\t60\t1\t1\taudio"]);
        same("what's left to finish", JSON.stringify(Mpv.held(left, counts)),
            JSON.stringify([{ id: "mpv:to-finish", value: "2 videos \u00b7 1 audio file" }]));
        same("one video alone", Mpv.held(left.slice(1, 3), counts)[0].value, "1 video");
        same("nothing left part way, nothing said", Mpv.held(left.slice(2, 3), counts).length, 0);

        // [when it was left, before now, in seconds, as its side says it]
        const words = mpv.words;
        const sides = [[30, "12:04 of 41:10 \u00b7 just now"],
            [720, "12:04 of 41:10 \u00b7 12\u00a0min ago"]];
        for (const [before, said] of sides)
            same(`its side, left ${before} s ago`,
                Mpv.side(Object.assign({}, read[0], { at: root.now - before * 1000 }), root.now,
                    words), said);
        same("one watched whole says its length and when",
            Mpv.side({ start: 60, length: 60, at: root.now }, root.now, words),
            "1:00 \u00b7 just now");

        // What mpv kept, out of its order, as the drawer orders it.
        const kept = ["sub-delay", "sid", "speed", "aid", "volume"]
            .map(option => ({ option, value: "1" }));
        same("speed, volume and the tracks first, then the rest",
            Mpv.kept({ kept }).map(each => each.option).join(" "),
            "speed volume aid sid sub-delay");
        // [its option, as mpv wrote it, as the drawer says it]
        const values = [["speed", "1.500000", "1.5\u00d7"], ["speed", "2.000000", "2\u00d7"],
            ["volume", "80.000000", "80%"], ["volume", "33.333333", "33.333%"],
            ["aid", "no", "off"], ["sid", "2", "2"], ["sid", "auto", "auto"],
            ["sub-delay", "-0.500000", "-0.500 s"], ["audio-delay", "0.1", "0.100 s"],
            ["mute", "yes", "yes"],
            ["vf", "%9%lavfi=[x]", "lavfi=[x]"]];
        for (const [option, value, said] of values)
            same(`${option}=${value} as said`, Mpv.keptValue({ option, value }, words.said), said);
        // [where it stopped, its length, whether that's watched whole]
        const ends = [[724.5, 2470.2, false], [2469.1, 2470.2, false], [2469.7, 2470.2, true],
            [60, 60, true]];
        for (const [start, length, whole] of ends)
            same(`${start} of ${length} watched whole`, Mpv.whole({ start, length }), whole);
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
            const acts = program("mpv").verbs();
            const playing = acts.find(row => row.key === file("now"));
            same("the file playing now, above Recent, as it plays",
                playing?.side, "open \u00b7 ~/Videos");
            const recent = acts.find(row => row.name === "Recent");
            const files = recent.targets();
            same("what mpv left by when, then the shared list's rest, each once, none open",
                files.map(row => row.key).join(" "),
                ["long", "talk", "song", "clip"].map(name => file(name)).join(" "));
            same("Recent counts each once", recent.side, "4");
            same("so does mpv's row", program("mpv").note, "1 open \u00b7 4 recent");
            elsewhere.open = [file("talk")];
            same("what another provider has open in it leaves its Recent",
                program("mpv").verbs().find(row => row.name === "Recent").targets()
                    .map(row => row.key).join(" "),
                ["long", "song", "clip"].map(name => file(name)).join(" "));
            same("and its row counts it open", program("mpv").note, "2 open \u00b7 3 recent");
            elsewhere.open = [];

            const [long, talk, song, clip] = files;
            same("where it stopped, its length and when, beside it", talk.side,
                "12:04 of 41:10 \u00b7 12\u00a0min ago");
            same("and nothing again under it", talk.note ?? "", "");
            same("past an hour", long.side, "1:02:03 of 1:40:00 \u00b7 10\u00a0min ago");
            same("Return resumes it", talk.verb, "resume");
            expect("when it was left, to be ordered by", Math.abs(talk.at - (root.now - 720000))
                < 1, talk.at);
            same("its drawer says where it is and its kind", talk.drawer.eyebrow,
                "in ~/Videos \u00b7 video");
            same("as written, since its folder is a name", talk.drawer.eyebrowNames, true);
            same("then what's still to play, what mpv kept, its size and its path", facts(talk),
                "Still to Play 29:05, Speed 1.5\u00d7, Volume 80%, Audio 2, Subtitles off, "
                + "sub-delay 0.500 s, Size 1.23\u00a0GB, Path ~/Videos/talk.mkv");
            same("its path marked an identifier", JSON.stringify(talk.drawer.facts.map(fact =>
                fact.identifies === true)), JSON.stringify([false, false, false, false, false, false,
                false, true]));
            same("one mpv kept nothing more of", facts(long),
                "Still to Play 37:57, Size 5.00\u00a0MB, Path ~/Videos/long.mkv");
            talk.run();
            same("Return resumes", JSON.stringify(Niri.spawned),
                JSON.stringify(Mpv.play(`${root.videos}/talk.mkv`, false)));
            const inside = talk.verbs();
            same("inside, Resume first, then Play From the Start", names(inside),
                "Resume, Play From the Start, Copy the Path");
            same("Resume says from where", inside[0].side, "from 12:04");
            inside[1].run();
            same("Play From the Start leaves where it stopped unread",
                JSON.stringify(Niri.spawned),
                JSON.stringify(Mpv.play(`${root.videos}/talk.mkv`, true)));

            same("one watched whole says play", song.verb, "play");
            same("and its length and when", song.side,
                `41:10 \u00b7 ${Qt.formatDate(new Date(root.now - 172800000), "dddd")}`);
            same("its drawer the same shape, audio alone", song.drawer.eyebrow,
                "in ~/Videos \u00b7 audio");
            same("with nothing still to play", facts(song),
                "Volume 50%, Size 940\u00a0kB, Path ~/Videos/song.mkv");
            same("inside, only from the start", names(song.verbs()),
                "Play From the Start, Copy the Path");
            song.run();
            same("Return plays it from its start", JSON.stringify(Niri.spawned),
                JSON.stringify(Mpv.play(`${root.videos}/song.mkv`, true)));
            same("one mpv kept nothing of is as the shared list has it", clip.note, "");

            const playable = () => names(program("mpv").verbs()).includes("Play the Copied Link");
            same("nothing copied, nothing to play", playable(), false);
            same("nor found from the top", mpv.foundActs.length, 0);
            Copied.text = "https://example.com/page";
            same("a page with nothing to play, nothing to play", playable(), false);
            Copied.text = "https://youtu.be/dQw4w9WgXcQ ";
            const offered = program("mpv").verbs();
            const at = offered.findIndex(row => row.name === "Play the Copied Link");
            same("a link copied is offered after New Window Below, saying its site",
                `${offered[at - 1]?.name}, ${offered[at]?.name} ${offered[at]?.side}`,
                "New Window Below, Play the Copied Link youtu.be");
            offered[at].run();
            same("Return plays it", JSON.stringify(Niri.spawned),
                JSON.stringify(Mpv.play("https://youtu.be/dQw4w9WgXcQ", false)));
            same("found from the top too", JSON.stringify(mpv.foundActs),
                JSON.stringify([{ program: "mpv", key: "verb:mpv:link" }]));
            Copied.text = "see https://youtu.be/dQw4w9WgXcQ";
            same("gone once something else is copied", playable(), false);
            same("from the top too", mpv.foundActs.length, 0);

            // Stepped out and in again, nothing's listed again until the
            // launcher opens afresh.
            theLauncher.held = [];
            theLauncher.held = [{ row: program("mpv"), rows: [] }];
            counted.reload();
            counted.waitForJob();
            same("once an opening", listings(), 1);
            root.waiting = "listed again";
            theLauncher.held = [];
            theLauncher.open = false;
            theLauncher.open = true;
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
                theLauncher.held = [];
                theLauncher.held = [{ row: root.program("mpv"), rows: [] }];
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

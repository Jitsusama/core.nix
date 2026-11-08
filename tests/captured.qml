// What Capture's Recent says of each thing taken, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-capture-says-what-it-took` check copies launcher/Captured.js, with the
// FileFacts.js and When.js it reads, beside this file and runs `qml
// -platform offscreen captured.qml` over files and file-preview's answers
// shaped as Taken.qml and file-preview give them.
import QtQuick
import "launcher/Captured.js" as Captured

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // The vocabulary's words, as words.json's capture:recent gives them,
    // and each fact named by its id, so a fact's order is read off.
    readonly property var said: ({
        eyebrow: "in {where} \u00b7 {kind}",
        within: "in {where}",
        kinds: { screenshot: "screenshot", recording: "recording" },
        today: "{time} today",
        yesterday: "{time} yesterday",
        on: "{time} on {day}",
        date: (date, format) => Qt.formatDate(date, format)
    })

    function named(id) {
        return `[${id}]`;
    }

    property var failures: []
    property int asked: 0

    function expect(what, got, wanted) {
        asked += 1;
        if (JSON.stringify(got) !== JSON.stringify(wanted))
            failures.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
    }

    function run() {
        const now = new Date(2026, 10, 8, 9, 30, 0).getTime();
        const told = { now, named, said };
        const shot = "/home/joel/Pictures/Screenshots/Screenshot from 2026-11-08 04-39-15.png";
        const video = "/home/joel/Videos/Recordings/2026-11-07 22-10-04.mp4";

        expect("a picture wears the picture's mark, a video the film's, else its own",
            [shot, video, "/tmp/notes.txt"].map(Captured.mark), ["picture", "film", ""]);

        expect("taken to the second, today, yesterday, this week and before",
            [new Date(2026, 10, 8, 4, 39, 15), new Date(2026, 10, 7, 23, 5, 9),
                new Date(2026, 10, 5, 12, 0, 0), new Date(2026, 9, 28, 8, 1, 2)]
                .map(at => Captured.taken(at.getTime(), now, said)),
            ["04:39:15 today", "23:05:09 yesterday",
                `12:00:00 on ${Qt.formatDate(new Date(2026, 10, 5), "dddd")}`,
                `08:01:02 on ${Qt.formatDate(new Date(2026, 9, 28), "d MMM")}`]);

        const file = {
            path: shot,
            name: "Screenshot from 2026-11-08 04-39-15.png",
            where: "~/Pictures/Screenshots",
            kind: "screenshot",
            at: new Date(2026, 10, 8, 4, 39, 15).getTime(),
            size: 2412345
        };
        const before = Captured.drawer(file, null, told);
        expect("before file-preview says, it's where, what, the picture and what Taken read",
            before, {
                eyebrow: "in ~/Pictures/Screenshots \u00b7 screenshot",
                title: "Screenshot from 2026-11-08 04-39-15.png",
                preview: { picture: shot, across: 0, down: 0 },
                facts: [
                    { label: "[taken]", value: "04:39:15 today" },
                    { label: "[size]", value: "2.4\u00a0MB" }
                ]
            });
        const shown = Captured.drawer(file, { size: 2412345, kind: "PNG", picture: shot,
            across: 2880, down: 1800 }, told);
        expect("a picture says its dimensions once file-preview has", shown.facts,
            [
                { label: "[taken]", value: "04:39:15 today" },
                { label: "[size]", value: "2.4\u00a0MB" },
                { label: "[dimensions]", value: "2880\u00a0\u00d7\u00a01800" }
            ]);
        expect("its preview takes file-preview's sides", shown.preview,
            { picture: shot, across: 2880, down: 1800 });

        const frame = "/home/joel/.cache/file-preview/abc.png";
        const recording = Captured.drawer({
            path: video,
            name: "2026-11-07 22-10-04.mp4",
            where: "~/Videos/Recordings",
            kind: "recording",
            at: new Date(2026, 10, 7, 22, 10, 4).getTime(),
            size: 48e6
        }, { size: 48e6, kind: "MP4", seconds: 221.4, picture: frame, across: 1040, down: 650 },
        told);
        expect("a recording says its length, its frame previewed, never the frame's sides",
            [recording.eyebrow, recording.preview?.picture]
                .concat(recording.facts.map(fact => `${fact.label} ${fact.value}`)),
            ["in ~/Videos/Recordings \u00b7 recording", frame, "[taken] 22:10:04 yesterday",
                "[size] 48\u00a0MB", "[length] 3:41"]);
        expect("a recording says nothing of a picture before file-preview has",
            Captured.drawer({ path: video, name: "v.mp4", where: "~/Videos/Recordings",
                kind: "recording" }, null, told).preview, null);
        expect("a folder with no kind of its own says where alone",
            Captured.drawer(Object.assign({}, file, { kind: "other" }), null, told).eyebrow, "in ~/Pictures/Screenshots");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}

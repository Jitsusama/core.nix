// How long the launcher's selection glides after a key, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-selection-lands-within-50-ms-of-its-key` check copies Glide.js and
// Expect.js beside this file and runs `qml -platform offscreen glide.qml`.
import QtQuick
import "Expect.js" as Expect
import "Glide.js" as Glide

QtObject {
    // A frame of the 60 Hz panel, and of a 120 Hz one.
    readonly property real sixty: 1000 / 60
    readonly property real twice: 1000 / 120

    Component.onCompleted: {
        try {
            // A key drawn at once glides the launcher's own 30 ms, not the
            // theme's 150.
            Expect.same("a key just pressed", Glide.lasting(0, sixty), 30);
            Expect.same("a key of unknown time", Glide.lasting(-1, sixty), 30);
            // A key already late glides only what's left of 50 ms, its
            // first frame's wait counted.
            Expect.same("a key 10 ms late", Glide.lasting(10, sixty), 23);
            Expect.same("a key 20 ms late on 120 Hz", Glide.lasting(20, twice), 22);
            // Too late for a frame of glide, it jumps.
            Expect.same("a key 17 ms late", Glide.lasting(17, sixty), 0);
            Expect.same("a key past the bar", Glide.lasting(80, sixty), 0);
            // A key held to repeat jumps in its own frame, however early,
            // and a single press still glides.
            Expect.same("a held key", Glide.lasting(0, sixty, true), 0);
            Expect.same("a held key on 120 Hz", Glide.lasting(0, twice, true), 0);
            Expect.same("a key pressed once", Glide.lasting(0, twice, false), 30);
            // However late, a glide lands within 50 ms of its key.
            for (let since = 0; since <= 60; since += 1) {
                const lasts = Glide.lasting(since, sixty);
                Expect.expect(`a key ${since} ms late`, lasts === 0 || since + sixty + lasts <= 50.5,
                    `glides ${lasts} ms, landing at ${since + sixty + lasts}`);
            }
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}

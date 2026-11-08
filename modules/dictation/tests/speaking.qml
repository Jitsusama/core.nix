// What dictation's surface says in each state, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `dictation-says-what-its-doing`
// check copies modules/dictation/Speaking.js beside this file and runs
// `qml -platform offscreen speaking.qml`.
import QtQuick
import "Speaking.js" as Speaking

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

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        const face = (glyph, hue, words, time, wave, quiet) => ({ glyph, hue, words, time, wave, quiet });

        // [what, the state, how long it's listened, what it said after, typing seen, the face]
        const cases = [
            ["idle shows nothing", "idle", 0, "", false, null],
            ["listening counts up", "listening", 8400, "", false,
                face("dictating", "voice", "listening", "0:08", true, false)],
            ["seconds stay whole until the next", "listening", 24999, "", false,
                face("dictating", "voice", "listening", "0:24", true, false)],
            ["from 25 s it counts down what's left of 30", "listening", 25000, "", false,
                face("dictating", "warning", "listening", "5 s", true, false)],
            ["the last second still says one", "listening", 29600, "", false,
                face("dictating", "warning", "listening", "1 s", true, false)],
            ["past 30 it never counts below nothing", "listening", 31000, "", false,
                face("dictating", "warning", "listening", "0 s", true, false)],
            ["typing too quick to see keeps listening's face", "transcribing", 8400, "", false,
                face("dictating", "voice", "listening", "0:08", true, false)],
            ["typing that lasts says so", "transcribing", 8400, "", true,
                face("dictating", "voice", "typing", "", false, false)],
            ["a press while typing says it's still writing", "transcribing", 8400,
                "still-writing", true, face("dictating", "voice", "still writing", "", false, false)],
            ["the microphone muted says so, with its glyph", "idle", 2000, "muted", false,
                face("muted", "machine", "the microphone is muted", "", false, true)],
            ["too short heard nothing", "idle", 300, "short", false,
                face("dictating", "muted", "heard nothing", "", false, true)],
            ["silence heard nothing", "idle", 4000, "silent", false,
                face("dictating", "muted", "heard nothing", "", false, true)],
            ["nothing it could make out heard nothing", "idle", 4000, "unclear", false,
                face("dictating", "muted", "heard nothing", "", false, true)],
            ["a word it doesn't know is no answer", "idle", 4000, "something", false, null]
        ];
        for (const [what, state, elapsed, after, typing, want] of cases)
            expect(what, Speaking.said(state, elapsed, after, typing), want);

        // The wave: a bar for each step of time, the loudest peak within it,
        // a step with no peak silent, and only as many as there's room for.
        let levels = [];
        levels = Speaking.waved(levels, 0.2, 10, 4);
        levels = Speaking.waved(levels, 0.5, 10, 4);
        levels = Speaking.waved(levels, 0.1, 10, 4);
        expect("peaks in one step keep the loudest", levels, [{ step: 10, level: 0.5 }]);
        levels = Speaking.waved(levels, 0.3, 13, 4);
        expect("steps with no peak are silent", levels, [
            { step: 10, level: 0.5 }, { step: 11, level: 0 }, { step: 12, level: 0 },
            { step: 13, level: 0.3 }
        ]);
        levels = Speaking.waved(levels, 0.4, 14, 4);
        expect("the oldest gives way", levels.map(bar => bar.step), [11, 12, 13, 14]);
        expect("a long silence leaves only its room", Speaking.waved(levels, 0.1, 100, 4)
            .map(bar => bar.step), [97, 98, 99, 100]);
        expect("a step back in time starts again", Speaking.waved(levels, 0.1, 2, 4), [
            { step: 2, level: 0.1 }
        ]);

        // A bar's height, from the least to the most, louder bars rising
        // faster than the level, since speech peaks low.
        expect("silence is the least", Speaking.height(0, 2, 16), 2);
        expect("full is the most", Speaking.height(1, 2, 16), 16);
        expect("past full is still the most", Speaking.height(1.5, 2, 16), 16);
        expect("a quarter is half way", Speaking.height(0.25, 2, 16), 9);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

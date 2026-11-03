// How the shell puts a Tuple call in words, and finds a call's link in
// text, run in Qt's own JavaScript engine, the one the shell runs it in.
// The `the-shell-follows-tuple-calls` check copies Tuple.js beside this
// file and runs `qml -platform offscreen tuple.qml`.
import QtQuick
import "Tuple.js" as Tuple

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
        let asked = 0;

        // [names, "" for one not known, said]
        const companies = [
            [[], "on your own"],
            [["Aadi"], "with Aadi"],
            [[""], "with someone"],
            [["Aadi", "Ben"], "with Aadi and Ben"],
            [["Aadi", ""], "with Aadi and one other"],
            [["", "Ben", "Cy"], "with Ben and 2 others"],
            [["", "", ""], "with 3 others"]
        ];
        for (const [names, expected] of companies) {
            asked++;
            const got = Tuple.company(names);
            if (got !== expected)
                failures.push(`${JSON.stringify(names)} said "${got}", not "${expected}"`);
        }

        // [text, the link in it]
        const links = [
            ["can you pair? https://tuple.app/c/abcDEF123 thanks", "https://tuple.app/c/abcDEF123"],
            ["https://production.tuple.app/c/x_y-z", "https://production.tuple.app/c/x_y-z"],
            ["personal url: https://tuple.app/c/room1\r", "https://tuple.app/c/room1"],
            ["https://tuple.app/pricing", ""],
            ["http://tuple.app/c/abc", ""],
            ["", ""],
            [undefined, ""]
        ];
        for (const [text, expected] of links) {
            asked++;
            const got = Tuple.link(text);
            if (got !== expected)
                failures.push(`"${text}" gave "${got}", not "${expected}"`);
        }

        // [minutes, said]
        const lengths = [
            [0, "under a minute"],
            [1, "a minute"],
            [12, "12 minutes"],
            [59, "59 minutes"],
            [60, "1 h"],
            [125, "2 h 5 min"]
        ];
        for (const [minutes, expected] of lengths) {
            asked++;
            const got = Tuple.length(minutes);
            if (got !== expected)
                failures.push(`${minutes} minutes said "${got}", not "${expected}"`);
        }

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}

// Whether JSON.parse holds each string value at its own length, in the Qt the
// shell is built with (modules/quickshell/qtdeclarative-json-strings-at-their-length.patch),
// and still reads what the language says. Qt 6.12.0 grew each value one
// character at a time, to the next power of two, and kept the room it grew
// to. The `json-strings-at-their-length` check runs this as the shell: 20,000
// distinct values of 248 characters need 514 bytes each, which jemalloc holds
// in 640, where grown they took 1,024. They grow its memory by under 17 MB,
// where grown they took about 21. What's parsed reads as before: escapes,
// an escaped quote inside a value, empty values, a character past the basic
// plane, and an unterminated value refused. It exits 1, saying what was
// wrong, for any.
import Quickshell
import QtQuick
import "Memory.js" as Memory

ShellRoot {
    Component.onCompleted: {
        const wrong = [];
        const expect = (what, got, wanted) => {
            if (got !== wanted)
                wrong.push(`${what}: ${String(got)}, not ${String(wanted)}`);
        };

        // Distinct values, so keeping each once per parse shares none of them.
        const length = 248;
        const values = Array.from({ length: 20000 },
            (_, at) => String(at).padStart(6, "0") + "x".repeat(length - 6));
        const said = JSON.stringify(values);
        gc();
        const before = Memory.anonymous();
        const parsed = JSON.parse(said);
        gc();
        const grew = Memory.anonymous() - before;
        expect("every value", parsed.every((value, at) => value === values[at]), true);
        if (grew >= 17 * 1024 * 1024)
            wrong.push(`20,000 values of ${length} characters grew memory by `
                + `${Math.round(grew / 1024)} KB`);

        expect("escapes", JSON.parse('["\\u00e9t\\u00e9", "\\n\\t\\\\", "\\/"]').join("|"),
            "été|\n\t\\|/");
        expect("an escaped quote", JSON.parse('["say \\"hi\\" twice", "after"]').join("|"),
            'say "hi" twice|after');
        expect("a value ending in an escaped backslash", JSON.parse('["end\\\\", "x"]').join("|"),
            "end\\|x");
        expect("empty values", JSON.parse('["", "", "a"]').join("|"), "||a");
        expect("past the basic plane", JSON.parse('["\\ud83c\\udf0a"]')[0], "\u{1F30A}");
        expect("a key and a value", JSON.parse('{"row": "a row"}').row, "a row");
        let refused = false;
        try {
            JSON.parse('["never closed');
        } catch (error) {
            refused = error instanceof SyntaxError;
        }
        expect("an unterminated value refused", refused, true);

        if (wrong.length > 0) {
            console.error(wrong.join("\n"));
            Qt.callLater(() => Qt.exit(1));
            return;
        }
        console.info(`json strings at their length: 20,000 values of ${length} characters `
            + `grew memory by ${Math.round(grew / 1024)} KB`);
        Qt.callLater(() => Qt.exit(0));
    }
}

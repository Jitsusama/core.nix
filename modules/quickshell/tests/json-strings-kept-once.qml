// Whether JSON.parse keeps each string value once per parse, in the Qt the
// shell is built with (modules/quickshell/qtdeclarative-json-strings-kept-once.patch),
// and still means what the language says. The `json-strings-kept-once` check
// runs this as the shell: 20,000 equal values parsed grow its memory by under
// 1 MB, where a string apiece took about 7; and what's parsed reads as before,
// a repeated key's last value winning, escapes read, and a value dropped by a repeated key still whole when an equal one comes
// after the parse has collected. It exits 1, saying what was wrong, for any.
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

        // A daemon's answer says the same words over and over.
        const word = "a word every row says, long enough to weigh more than its slot";
        const said = JSON.stringify(Array.from({ length: 20000 }, () => word));
        gc();
        const before = Memory.anonymous();
        const parsed = JSON.parse(said);
        gc();
        const grew = Memory.anonymous() - before;
        expect("every value", parsed.every(value => value === word), true);
        if (grew >= 1024 * 1024)
            wrong.push(`20,000 equal values grew memory by ${Math.round(grew / 1024)} KB`);

        const repeated = JSON.parse('{"a": "first", "a": "last", "b": "last"}');
        expect("a repeated key's last value", repeated.a, "last");
        expect("an equal value under another key", repeated.b, "last");
        expect("escapes", JSON.parse('["\\u00e9t\\u00e9", "\\n", "", ""]').join("|"), "été|\n||");
        expect("a key and a value alike", JSON.parse('{"row": "row"}').row, "row");

        // Enough strings between the dropped value and its twin that the
        // parse collects, so a string held by the table alone would be gone.
        const filler = Array.from({ length: 200000 }, (_, at) => `filler ${at}`);
        const late = JSON.parse(`[{"x": "dropped once", "x": "kept"}, ${JSON.stringify(filler)},`
            + ' "dropped once"]');
        expect("a dropped value's twin after collecting", late[2], "dropped once");
        expect("the filler", late[1][199999], "filler 199999");

        if (wrong.length > 0) {
            console.error(wrong.join("\n"));
            Qt.callLater(() => Qt.exit(1));
            return;
        }
        console.info(`json strings kept once: 20,000 equal values grew memory by `
            + `${Math.round(grew / 1024)} KB`);
        Qt.callLater(() => Qt.exit(0));
    }
}

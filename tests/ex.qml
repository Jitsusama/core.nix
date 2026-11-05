// Vim's command line, read in Qt's own JavaScript engine, the one the shell
// runs it in. The `ex-reads-vims-command-lines` check copies
// modules/quickshell/Ex.js beside this file and runs
// `qml -platform offscreen ex.qml`.
import QtQuick
import "Ex.js" as Ex

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

    // A surface's commands, as the held notices have them.
    readonly property var commands: [
        { command: "delete", short: "d", ranged: true },
        { command: "forget", short: "forg", ranged: true },
        { command: "reply", short: "rep", ranged: false },
        { command: "quit", short: "q", ranged: false }
    ]

    function run() {
        const failures = [];
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }
        const here = { at: ".", offset: 0 };
        const line = (at, offset) => ({ at, offset: offset ?? 0 });
        const asked = (range, name, bang, args) => ({ range, name, bang, args });

        // [what, the line, what it asks]
        const reading = [
            ["a name in full", "forget", asked(null, "forget", false, "")],
            ["its short form", "forg", asked(null, "forget", false, "")],
            ["cut between the two", "forge", asked(null, "forget", false, "")],
            ["shorter than its short form is no command", "for",
                { error: "E492: Not an editor command: for" }],
            ["Vim's d[elete]", "d", asked(null, "delete", false, "")],
            ["a word that's no command, said whole", "wifi off",
                { error: "E492: Not an editor command: wifi off" }],
            ["a colon and spaces before it are let go", " :q", asked(null, "quit", false, "")],
            ["a bang", "forget!", asked(null, "forget", true, "")],
            ["what follows", "reply on my way", asked(null, "reply", false, "on my way")],
            ["a bang and what follows", "reply! soon", asked(null, "reply", true, "soon")],
            ["one row", "3d", asked({ from: line(3), to: line(3) }, "delete", false, "")],
            ["two rows", "3,5d", asked({ from: line(3), to: line(5) }, "delete", false, "")],
            ["from here on two", ".,+2d",
                asked({ from: here, to: line(".", 2) }, "delete", false, "")],
            ["offsets add up", ".+1-3,$-1d",
                asked({ from: line(".", -2), to: line("$", -1) }, "delete", false, "")],
            ["a bare sign is one", ".,++d",
                asked({ from: here, to: line(".", 2) }, "delete", false, "")],
            ["every row", "%d", asked("%", "delete", false, "")],
            ["the selection", "'<,'>forg",
                asked({ from: line("'<"), to: line("'>") }, "forget", false, "")],
            ["a semicolon as a comma", "2;4d",
                asked({ from: line(2), to: line(4) }, "delete", false, "")],
            ["a range alone goes to its row", "12", asked({ from: line(12), to: line(12) }, "",
                false, "")],
            ["a space between the range and the name", "2,3 d",
                asked({ from: line(2), to: line(3) }, "delete", false, "")],
            ["nothing does nothing", "", asked(null, "", false, "")],
            ["a range where none is taken", "3reply",
                { error: "E481: No range allowed" }],
            ["a range with what's no command", "2x",
                { error: "E492: Not an editor command: 2x" }]
        ];
        for (const [what, typed, want] of reading)
            holds(what, Ex.parse(typed, commands), want);

        const each = (pattern, invert, command) => ({ pattern, invert, command });
        const globals = [
            ["every row holding it", "g/Slack/d",
                { range: "%", global: each("Slack", false, asked(null, "delete", false, "")) }],
            ["in full, another mark", "global#Mail#forg!",
                { range: "%", global: each("Mail", false, asked(null, "forget", true, "")) }],
            ["every row not holding it", "v/Slack/d",
                { range: "%", global: each("Slack", true, asked(null, "delete", false, "")) }],
            ["g! is v", "g!/Slack/d",
                { range: "%", global: each("Slack", true, asked(null, "delete", false, "")) }],
            ["within a range", "1,3g/a\\/b/d",
                { range: { from: line(1), to: line(3) },
                    global: each("a/b", false, asked(null, "delete", false, "")) }],
            ["a command that's no command", "g/x/wifi",
                { error: "E492: Not an editor command: wifi" }],
            ["no pattern", "g//d", { error: "E35: No previous regular expression" }],
            ["a figure as the mark", "g1x1d",
                { error: "E146: Regular expressions can't be delimited by letters" }],
            ["no mark", "g", { error: "E35: No previous regular expression" }],
            ["within itself", "g/a/g/b/d", { error: "E147: Cannot do :global recursive" }]
        ];
        for (const [what, typed, want] of globals)
            holds(what, Ex.parse(typed, commands), want);

        holds("a count of three writes from here on two", Ex.counted(3), ".,.+2");
        holds("a count of one is the row", Ex.counted(1), ".");
        holds("visual writes the selection", Ex.selected, "'<,'>");

        // Five rows, the cursor on the second, the selection the third and
        // fourth, and a mark on the last.
        const at = { cursor: 1, count: 5, marks: { "<": 2, ">": 3, a: 4 } };
        const rows = typed => Ex.resolve(Ex.parse(typed, commands).range, at);
        const resolving = [
            ["no range is the cursor row", "d", { rows: [1] }],
            ["one row, from 1", "3d", { rows: [2] }],
            ["two rows", "3,5d", { rows: [2, 3, 4] }],
            ["from here on two", ".,+2d", { rows: [1, 2, 3] }],
            ["every row", "%d", { rows: [0, 1, 2, 3, 4] }],
            ["the selection", "'<,'>d", { rows: [2, 3] }],
            ["to a mark", ".,'ad", { rows: [1, 2, 3, 4] }],
            ["the last", "$d", { rows: [4] }],
            ["backwards, turned round", "4,2d", { rows: [1, 2, 3] }],
            ["0 is the first", "0,1d", { rows: [0] }],
            ["past the last", "4,6d", { error: "E16: Invalid range" }],
            ["before the first", ".-2d", { error: "E16: Invalid range" }],
            ["a mark not set", "'bd", { error: "E20: Mark not set" }]
        ];
        for (const [what, typed, want] of resolving)
            holds(what, rows(typed), want);
        holds("no rows, no cursor row", Ex.resolve(null, { cursor: 0, count: 0 }),
            { error: "E16: Invalid range" });

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

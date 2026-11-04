// What a file says of itself in the launcher's chooser of files, run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `a-file-says-its-size-and-when-it-changed` check copies
// modules/quickshell/FileFacts.js and the launcher's words.json beside
// this file and runs `qml -platform offscreen file-facts.qml`. Each case
// is how VKCF3Z set a file's row and its plate (2026-11-03): a size in SI
// units, one decimal under 10; when it changed by how long ago; a plate of
// its kind and numbers, its name, its preview and its facts, every number
// kept to its unit, and every word said as words.json says it.
import QtQuick
import "FileFacts.js" as FileFacts

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

    property var failures: []

    function expect(what, got, wanted) {
        if (JSON.stringify(got) !== JSON.stringify(wanted))
            failures.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
    }

    // Words with their no-break spaces written as underscores, to read.
    function kept(words) {
        return words.replace(/_/g, "\u00a0");
    }

    function at(year, month, day, hours, minutes) {
        return new Date(year, month - 1, day, hours, minutes).getTime();
    }

    // The words as the chooser gives them: each thing as words.json says
    // it, and the months as a locale in English names them.
    function launcherWords() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("words.json"), false);
        request.send();
        const things = {};
        for (const thing of JSON.parse(request.responseText).things)
            things[thing.id] = thing;
        return FileFacts.wordsOf(id => things[id] ?? {}, ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
            "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]);
    }

    function run() {
        const words = launcherWords();
        expect("bytes are bytes", FileFacts.weighed(12), kept("12_B"));
        expect("under 10 of a unit keeps a decimal", FileFacts.weighed(2_400_000), kept("2.4_MB"));
        expect("10 and over doesn't", FileFacts.weighed(940_000), kept("940_kB"));
        expect("a size that rounds up to the next unit says it", FileFacts.weighed(999_960),
            kept("1.0_MB"));
        expect("9.96 of a unit is 10, not 10.0", FileFacts.weighed(9_960), kept("10_kB"));

        expect("a folder counts its items", FileFacts.items(14, words), kept("14_items"));
        expect("one item is one", FileFacts.items(1, words), kept("1_item"));
        expect("a folder with none is empty", FileFacts.items(0, words), "empty");

        const now = at(2026, 11, 3, 16, 30);
        expect("under a minute is just now", FileFacts.when(now - 20_000, now, words), "just now");
        expect("under an hour is minutes ago", FileFacts.when(now - 12 * 60_000, now, words),
            kept("12_min ago"));
        expect("earlier today is its time", FileFacts.when(at(2026, 11, 3, 9, 5), now, words),
            "09:05");
        expect("this year is its day", FileFacts.when(at(2026, 10, 28, 14, 32), now, words),
            kept("28_Oct"));
        expect("before this year is its day and year",
            FileFacts.when(at(2024, 10, 28, 1, 0), now, words), kept("28_Oct_2024"));
        expect("a moment ahead of now is said as today's",
            FileFacts.when(now + 3_600_000, now, words), "17:30");

        const file = { folder: false, size: 2_400_000, modified: at(2026, 10, 28, 14, 32) };
        expect("a file's row says its size and its day", FileFacts.side(file, undefined, now, words),
            kept("2.4_MB \u00b7 28_Oct"));
        const folder = { folder: true, size: 4096, modified: at(2026, 10, 28, 14, 32) };
        expect("a folder's row says its count once it's known",
            FileFacts.side(folder, 14, now, words), kept("14_items \u00b7 28_Oct"));
        expect("and only its day until then", FileFacts.side(folder, undefined, now, words),
            kept("28_Oct"));

        expect("a video's length", [42, 221, 3723].map(seconds => FileFacts.length(seconds)),
            ["0:42", "3:41", "1:02:03"]);

        const picture = "/home/j/shot.png";
        const eyebrow = (said, path) => FileFacts.eyebrow(said, path, words);
        expect("a picture's eyebrow is its kind and size", eyebrow(
            { kind: "PNG", across: 2400, down: 1600, picture: picture }, picture),
            kept("PNG \u00b7 2400_\u00d7_1600"));
        expect("a PDF's, its pages", eyebrow({ kind: "PDF", pages: 12, picture: "/c/p.png",
            across: 1040, down: 1346 }, "/home/j/a.pdf"), kept("PDF \u00b7 12_pages"));
        expect("one page is one", eyebrow({ kind: "PDF", pages: 1 }, "/a.pdf"),
            kept("PDF \u00b7 1_page"));
        expect("a PDF with no count says no picture's size", eyebrow(
            { kind: "PDF", picture: "/c/p.png", across: 1040, down: 1346 }, "/home/j/a.pdf"), "PDF");
        expect("a video's, its length", eyebrow({ kind: "MP4", seconds: 221 }, "/v.mp4"),
            "MP4 \u00b7 3:41");
        expect("text with no ending is text", eyebrow({ lines: ["a"] }, "/notes"), "Text");
        expect("text is its kind by its own name, with its lines", eyebrow(
            { kind: "MD", lines: ["# Plan"], count: 4 }, "/a.md"), kept("Markdown \u00b7 4_lines"));
        expect("one line is one", eyebrow({ kind: "TXT", lines: ["a"], count: 1 }, "/a.txt"),
            kept("Text \u00b7 1_line"));
        expect("text of an ending with no name of its own is its ending", eyebrow(
            { kind: "CONF", lines: ["a"], count: 120 }, "/a.conf"), kept("CONF \u00b7 120_lines"));
        expect("text too long to count whole says it's over what was counted", eyebrow(
            { kind: "LOG", lines: ["a"], count: 900_000, more: true }, "/a.log"),
            kept("LOG \u00b7 over 900000_lines"));
        expect("an ending that names text names nothing else", eyebrow({ kind: "MD" }, "/a.md"),
            "MD");
        expect("anything else with none is a file", eyebrow({ size: 1 }, "/blob"), "File");
        expect("a folder is a folder", eyebrow({ items: 3, names: [] }, "/f"), "Folder");

        const shot = { path: picture, name: "shot.png", where: "~", entry: file };
        const before = FileFacts.plate(shot, null, now, words);
        expect("until it's said, a plate is its facts alone, from its folder", before, {
            eyebrow: "", title: "shot.png", facts: [{ label: "Size", value: kept("2.4_MB") },
                { label: "Changed", value: kept("28_Oct") }, { label: "Where", value: "~" }]
        });
        const said = { kind: "PNG", size: 2_400_000, changed: file.modified, across: 2400,
            down: 1600, picture: picture };
        const shown = FileFacts.plate(shot, said, now, words);
        expect("once said, its eyebrow and its picture, its own shape",
            [shown.eyebrow, shown.preview], [kept("PNG \u00b7 2400_\u00d7_1600"),
                { picture: picture, across: 2400, down: 1600 }]);
        const text = FileFacts.plate({ path: "/n/a.md", name: "a.md", where: "~/n", entry: null },
            { kind: "MD", size: 9, changed: now, lines: ["# Plan", "one"] }, now, words);
        expect("a file of text previews its first lines, its facts from what's said",
            [text.preview, text.facts[0].value], [{ lines: ["# Plan", "one"] }, kept("9_B")]);
        expect("its change said as its row says it", text.facts[1].value,
            FileFacts.when(now, now, words));
        const sub = { path: "/n/sub", name: "sub", where: "~/n", entry: folder };
        const listed = FileFacts.plate(sub,
            { size: 4096, changed: now, items: 2, names: ["deep/", "z.md"] }, now, words);
        expect("a folder previews its first items, quiet, and counts them",
            [listed.eyebrow, listed.preview, listed.facts[0]], ["Folder",
                { lines: ["deep/", "z.md"], quiet: true }, { label: "Items", value: "2" }]);
        const waiting = FileFacts.plate(sub, null, now, words);
        expect("a folder not yet counted shows the placeholder", waiting.facts[0],
            { label: "Items", value: null });
        const plain = FileFacts.plate({ path: "/n/a.zip", name: "a.zip", where: "~/n", entry: null },
            { kind: "ZIP", size: 7, changed: now }, now, words);
        expect("anything else shows its facts alone, no stand-in", plain.preview, undefined);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}

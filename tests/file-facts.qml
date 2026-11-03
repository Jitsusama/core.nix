// What a file says of itself in the launcher's chooser of files, run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `a-file-says-its-size-and-when-it-changed` check copies
// modules/quickshell/FileFacts.js beside this file and runs `qml -platform
// offscreen file-facts.qml`. Each case is how VKCF3Z set a file's row and
// its plate (2026-11-03): a size in SI units, one decimal under 10; when it
// changed by how long ago; a plate of its kind and numbers, its name, its
// preview and its facts, every number kept to its unit.
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

    function run() {
        expect("bytes are bytes", FileFacts.weighed(12), kept("12_B"));
        expect("under 10 of a unit keeps a decimal", FileFacts.weighed(2_400_000), kept("2.4_MB"));
        expect("10 and over doesn't", FileFacts.weighed(940_000), kept("940_kB"));
        expect("a size that rounds up to the next unit says it", FileFacts.weighed(999_960),
            kept("1.0_MB"));
        expect("9.96 of a unit is 10, not 10.0", FileFacts.weighed(9_960), kept("10_kB"));

        expect("a folder counts its items", FileFacts.items(14), kept("14_items"));
        expect("one item is one", FileFacts.items(1), kept("1_item"));
        expect("a folder with none is empty", FileFacts.items(0), "empty");

        const now = at(2026, 11, 3, 16, 30);
        expect("under a minute is just now", FileFacts.when(now - 20_000, now), "just now");
        expect("under an hour is minutes ago", FileFacts.when(now - 12 * 60_000, now),
            kept("12_min ago"));
        expect("earlier today is its time", FileFacts.when(at(2026, 11, 3, 9, 5), now), "09:05");
        expect("this year is its day", FileFacts.when(at(2026, 10, 28, 14, 32), now),
            kept("28_Oct"));
        expect("before this year is its day and year", FileFacts.when(at(2024, 10, 28, 1, 0), now),
            kept("28_Oct_2024"));
        expect("a moment ahead of now is said as today's", FileFacts.when(now + 3_600_000, now),
            "17:30");
        expect("the drawer says it to the minute, today",
            FileFacts.whenExactly(at(2026, 11, 3, 14, 32), now), "today, 14:32");
        expect("the drawer says it to the minute, another day",
            FileFacts.whenExactly(at(2024, 10, 28, 14, 32), now), kept("28_Oct_2024, 14:32"));

        const file = { folder: false, size: 2_400_000, modified: at(2026, 10, 28, 14, 32) };
        expect("a file's row says its size and its day", FileFacts.side(file, undefined, now),
            kept("2.4_MB \u00b7 28_Oct"));
        const folder = { folder: true, size: 4096, modified: at(2026, 10, 28, 14, 32) };
        expect("a folder's row says its count once it's known", FileFacts.side(folder, 14, now),
            kept("14_items \u00b7 28_Oct"));
        expect("and only its day until then", FileFacts.side(folder, undefined, now),
            kept("28_Oct"));

        expect("a video's length", [42, 221, 3723].map(seconds => FileFacts.length(seconds)),
            ["0:42", "3:41", "1:02:03"]);

        const picture = "/home/j/shot.png";
        expect("a picture's eyebrow is its kind and size", FileFacts.eyebrow(
            { kind: "PNG", across: 2400, down: 1600, picture: picture }, picture),
            kept("PNG \u00b7 2400_\u00d7_1600"));
        expect("a PDF's, its pages", FileFacts.eyebrow({ kind: "PDF", pages: 12, picture: "/c/p.png",
            across: 1040, down: 1346 }, "/home/j/a.pdf"), kept("PDF \u00b7 12_pages"));
        expect("one page is one", FileFacts.eyebrow({ kind: "PDF", pages: 1 }, "/a.pdf"),
            kept("PDF \u00b7 1_page"));
        expect("a PDF with no count says no picture's size", FileFacts.eyebrow(
            { kind: "PDF", picture: "/c/p.png", across: 1040, down: 1346 }, "/home/j/a.pdf"), "PDF");
        expect("a video's, its length", FileFacts.eyebrow({ kind: "MP4", seconds: 221 }, "/v.mp4"),
            "MP4 \u00b7 3:41");
        expect("text with no ending is text", FileFacts.eyebrow({ lines: ["a"] }, "/notes"), "Text");
        expect("anything else with none is a file", FileFacts.eyebrow({ size: 1 }, "/blob"), "File");
        expect("a folder is a folder", FileFacts.eyebrow({ items: 3, names: [] }, "/f"), "Folder");

        const shot = { path: picture, name: "shot.png", where: "~", entry: file };
        const before = FileFacts.plate(shot, null, now);
        expect("until it's said, a plate is its facts alone, from its folder", before, {
            eyebrow: "", title: "shot.png", facts: [{ label: "Size", value: kept("2.4_MB") },
                { label: "Changed", value: kept("28_Oct, 14:32") }, { label: "Where", value: "~" }]
        });
        const said = { kind: "PNG", size: 2_400_000, changed: file.modified, across: 2400,
            down: 1600, picture: picture };
        const shown = FileFacts.plate(shot, said, now);
        expect("once said, its eyebrow and its picture, its own shape",
            [shown.eyebrow, shown.preview], [kept("PNG \u00b7 2400_\u00d7_1600"),
                { picture: picture, across: 2400, down: 1600 }]);
        const text = FileFacts.plate({ path: "/n/a.md", name: "a.md", where: "~/n", entry: null },
            { kind: "MD", size: 9, changed: now, lines: ["# Plan", "one"] }, now);
        expect("a file of text previews its first lines, its facts from what's said",
            [text.preview, text.facts[0].value], [{ lines: ["# Plan", "one"] }, kept("9_B")]);
        const sub = { path: "/n/sub", name: "sub", where: "~/n", entry: folder };
        const listed = FileFacts.plate(sub,
            { size: 4096, changed: now, items: 2, names: ["deep/", "z.md"] }, now);
        expect("a folder previews its first items, quiet, and counts them",
            [listed.eyebrow, listed.preview, listed.facts[0]], ["Folder",
                { lines: ["deep/", "z.md"], quiet: true }, { label: "Items", value: kept("2_items") }]);
        const waiting = FileFacts.plate(sub, null, now);
        expect("a folder not yet counted shows the placeholder", waiting.facts[0],
            { label: "Items", value: null });
        const plain = FileFacts.plate({ path: "/n/a.zip", name: "a.zip", where: "~/n", entry: null },
            { kind: "ZIP", size: 7, changed: now }, now);
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

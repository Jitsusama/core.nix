// What a file says of itself in the launcher's chooser of files, run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `a-file-says-its-size-and-when-it-changed` check copies
// modules/quickshell/FileFacts.js, the When.js it says a day by, and the
// launcher's words.json beside
// this file and runs `qml -platform offscreen file-facts.qml`. Each case
// is how VKCF3Z set a file's row and its plate (2026-11-03): a size in SI
// units, one decimal under 10; when it changed by how long ago; a plate of
// its kind and numbers, its name, its preview and its facts, every number
// kept to its unit, and every word said as words.json says it. A folder
// lists by name, change or size, its folders first (VKCF3Z, 2026-11-04).
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
    // it, and days and months as Qt names them, in the check's English.
    function launcherWords() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("words.json"), false);
        request.send();
        const things = {};
        for (const thing of JSON.parse(request.responseText).things)
            things[thing.id] = thing;
        return FileFacts.wordsOf(id => things[id] ?? {}, (at, format) => Qt.formatDate(at, format));
    }

    function run() {
        const words = launcherWords();
        expect("bytes are bytes", FileFacts.weighed(12), kept("12_B"));
        expect("a size keeps three figures", FileFacts.weighed(2_412_345), kept("2.41_MB"));
        expect("its zeros too", FileFacts.weighed(2_400_000), kept("2.40_MB"));
        expect("tens keep one place", FileFacts.weighed(19_340_000_000), kept("19.3_GB"));
        expect("hundreds keep none", FileFacts.weighed(940_000), kept("940_kB"));
        expect("a size that rounds up to the next unit says it", FileFacts.weighed(999_960),
            kept("1.00_MB"));

        expect("a folder counts its items", FileFacts.items(14, words), kept("14_items"));
        expect("one item is one", FileFacts.items(1, words), kept("1_item"));
        expect("a folder with none is empty", FileFacts.items(0, words), "empty");

        // A Tuesday.
        const now = at(2026, 11, 10, 16, 30);
        expect("under a minute is just now", FileFacts.when(now - 20_000, now, words), "just now");
        expect("under an hour is minutes ago", FileFacts.when(now - 12 * 60_000, now, words),
            kept("12_min ago"));
        expect("earlier today is its time", FileFacts.when(at(2026, 11, 10, 9, 5), now, words),
            "09:05");
        expect("the day before is yesterday, however late",
            FileFacts.when(at(2026, 11, 9, 23, 59), now, words), "yesterday");
        expect("within six days is its weekday",
            [at(2026, 11, 8, 9, 0), at(2026, 11, 4, 9, 0)]
                .map(day => FileFacts.when(day, now, words)), ["Sunday", "Wednesday"]);
        expect("a week before is its day", FileFacts.when(at(2026, 11, 3, 9, 0), now, words),
            kept("3_Nov"));
        expect("this year is its day", FileFacts.when(at(2026, 10, 28, 14, 32), now, words),
            kept("28_Oct"));
        expect("before this year is its day and year",
            FileFacts.when(at(2024, 10, 28, 1, 0), now, words), kept("28_Oct_2024"));
        expect("a moment ahead of now is said as today's",
            FileFacts.when(now + 3_600_000, now, words), "17:30");
        expect("and a day ahead as its day", FileFacts.when(at(2026, 11, 12, 9, 0), now, words),
            kept("12_Nov"));

        const file = { folder: false, size: 2_400_000, modified: at(2026, 10, 28, 14, 32) };
        expect("a file's row says its size and its day", FileFacts.side(file, undefined, now, words),
            kept("2.40_MB \u00b7 28_Oct"));
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
            eyebrow: "", title: "shot.png", facts: [{ label: "Size", value: kept("2.40_MB") },
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

        // One kept elsewhere, as in Google's Drive, reads as one here does.
        const away = { name: "menu.pdf", folder: false, size: 2_400_000,
            modified: at(2026, 10, 28, 14, 32) };
        expect("a file kept elsewhere says its size and its day, as one here does",
            FileFacts.sideElsewhere(away, now, words), FileFacts.side(away, undefined, now, words));
        expect("a folder kept elsewhere, uncounted, says its day alone",
            FileFacts.sideElsewhere({ name: "Trips", folder: true, modified: away.modified }, now,
                words), kept("28_Oct"));
        expect("and with nothing said of it, nothing",
            FileFacts.sideElsewhere({ name: "Team", folder: true }, now, words), "");
        const owner = { label: "Owner", value: "Ada Lovelace" };
        expect("its plate is a file's here, by its ending, its place's facts after",
            FileFacts.plateElsewhere({ name: "menu.pdf", where: "My Drive/Trips", entry: away,
                facts: [owner] }, now, words), {
                eyebrow: "PDF", title: "menu.pdf", facts: [
                    { label: "Size", value: kept("2.4_MB") },
                    { label: "Changed", value: kept("28_Oct") },
                    { label: "Where", value: "My Drive/Trips" }, owner]
            });
        expect("what its place doesn't say is left out, never awaited",
            FileFacts.plateElsewhere({ name: "Trips", where: "My Drive",
                entry: { name: "Trips", folder: true } }, now, words), {
                eyebrow: "Folder", title: "Trips", facts: [{ label: "Where", value: "My Drive" }]
            });
        expect("where its place says it is leads its eyebrow, in place of Where",
            FileFacts.plateElsewhere({ name: "menu.pdf", where: "My Drive/Trips",
                within: "in My Drive/Trips", entry: away, facts: [owner] }, now, words), {
                eyebrow: "in My Drive/Trips \u00b7 PDF", eyebrowNames: true, title: "menu.pdf",
                facts: [{ label: "Size", value: kept("2.4_MB") },
                    { label: "Changed", value: kept("28_Oct") }, owner]
            });
        expect("and a folder's, its kind its folder", FileFacts.plateElsewhere({ name: "Trips",
            where: "My Drive", within: "in My Drive", entry: { name: "Trips", folder: true } },
            now, words).eyebrow, "in My Drive \u00b7 Folder");
        expect("a file with no ending is a file", FileFacts.plateElsewhere({ name: "notes",
            where: "", entry: { name: "notes", size: 3 } }, now, words).eyebrow, "File");

        const entries = [
            { name: "note 10.md", path: "/n/note 10.md", size: 5, modified: 3 },
            { name: "Note 2.md", path: "/n/Note 2.md", size: 900, modified: 1 },
            { name: "b", path: "/n/b", folder: true, modified: 2 },
            { name: "a", path: "/n/a", folder: true, modified: 1 },
            { name: "apple.md", path: "/n/apple.md", size: 40, modified: 2 }
        ];
        const order = by => FileFacts.ordered(entries, by, { "/n/a": 1, "/n/b": 7 })
            .map(entry => entry.name);
        expect("by name: folders first, then A to Z, case aside, 2 before 10", order("name"),
            ["a", "b", "apple.md", "Note 2.md", "note 10.md"]);
        expect("by when it changed: folders first, newest first", order("changed"),
            ["b", "a", "note 10.md", "apple.md", "Note 2.md"]);
        expect("by size: folders first by how many they hold, then largest first", order("size"),
            ["b", "a", "Note 2.md", "apple.md", "note 10.md"]);
        const backward = by => FileFacts.ordered(entries, by, { "/n/a": 1, "/n/b": 7 }, true)
            .map(entry => entry.name);
        expect("reversed by name: folders still first, each order turned round", backward("name"),
            ["b", "a", "note 10.md", "Note 2.md", "apple.md"]);
        expect("reversed by when it changed: oldest first", backward("changed"),
            ["a", "b", "Note 2.md", "apple.md", "note 10.md"]);
        expect("reversed by size: smallest first", backward("size"),
            ["a", "b", "note 10.md", "apple.md", "Note 2.md"]);
        expect("a folder not yet counted is the least", FileFacts.ordered(entries, "size", {})
            .slice(0, 2).map(entry => entry.name), ["a", "b"]);
        expect("the entries given are left as they were", entries[0].name, "note 10.md");

        // [what, a name, where the caret starts in it]
        const ends = [
            ["the caret starts before the extension", "notes.md", 5],
            ["before the last of several", "notes.tar.gz", 9],
            ["at the end of a name with none", "notes", 5],
            ["at the end of a hidden name, whose dot begins it", ".bashrc", 7],
            ["at the end of a name ending in a dot", "notes.", 6],
            ["at the start of nothing", "", 0]
        ];
        for (const [what, name, want] of ends)
            expect(what, FileFacts.nameEnd(name), want);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}

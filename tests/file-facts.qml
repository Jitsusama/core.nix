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
// A file in the Nix store goes by its own name, past its hash (VKCF3Z,
// 2026-11-08).
import QtQuick
import "FileFacts.js" as FileFacts
import "When.js" as When

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
        expect("and a day ahead as days behind are, its weekday within six",
            [at(2026, 11, 11, 9, 0), at(2026, 11, 12, 9, 0), at(2026, 11, 24, 9, 0)]
                .map(day => FileFacts.when(day, now, words)),
            ["Wednesday", "Thursday", kept("24_Nov")]);
        // When.js on its own, given a word for tomorrow, as For You's
        // deadlines are said (VKCF3Z, 2026-11-08).
        const days = { yesterday: "yesterday", tomorrow: "tomorrow",
            date: (date, format) => Qt.formatDate(date, format) };
        expect("tomorrow, then a weekday, then a date, as yesterday's run back",
            [at(2026, 11, 11, 9, 0), at(2026, 11, 16, 9, 0), at(2026, 11, 17, 9, 0),
                at(2026, 11, 10, 20, 0)].map(day => When.day(day, now, days)),
            ["tomorrow", "Monday", "17 Nov", ""]);

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
        // A plate's eyebrow, then each fact as its label and value.
        const plated = (said, path) => {
            const shown = FileFacts.plate({ path, name: "n", where: "~/n", entry: null }, said,
                words);
            return [shown.eyebrow].concat(shown.facts.map(fact => `${fact.label} ${fact.value}`));
        };
        expect("a picture's eyebrow is where, then its kind, its dimensions a fact", plated(
            { kind: "PNG", across: 2400, down: 1600, picture: picture }, picture),
            ["in ~/n \u00b7 PNG", kept("Dimensions 2400_\u00d7_1600")]);
        expect("a PDF's pages are a fact, the number alone", plated({ kind: "PDF", pages: 12,
            picture: "/c/p.png", across: 1040, down: 1346 }, "/home/j/a.pdf"),
            ["in ~/n \u00b7 PDF", "Pages 12"]);
        expect("a PDF with no count says no picture's size", plated(
            { kind: "PDF", picture: "/c/p.png", across: 1040, down: 1346 }, "/home/j/a.pdf"),
            ["in ~/n \u00b7 PDF"]);
        expect("a video's length is a fact, never its eyebrow's", plated(
            { kind: "MP4", seconds: 20 }, "/v.mp4"), ["in ~/n \u00b7 MP4", "Length 0:20"]);
        expect("text with no ending is text", plated({ lines: ["a"] }, "/notes"),
            ["in ~/n \u00b7 Text"]);
        expect("text is its kind by its own name, its lines a fact", plated(
            { kind: "MD", lines: ["# Plan"], count: 4 }, "/a.md"),
            ["in ~/n \u00b7 Markdown", "Lines 4"]);
        expect("plain text is said as Text", plated({ kind: "TXT", lines: ["a"], count: 1 },
            "/a.txt"), ["in ~/n \u00b7 Text", "Lines 1"]);
        expect("text of an ending with no name of its own is its ending", plated(
            { kind: "CONF", lines: ["a"], count: 120 }, "/a.conf"),
            ["in ~/n \u00b7 CONF", "Lines 120"]);
        expect("text too long to count whole says it's over what was counted", plated(
            { kind: "LOG", lines: ["a"], count: 900_000, more: true }, "/a.log"),
            ["in ~/n \u00b7 LOG", kept("Lines over 900000")]);
        expect("an ending that names text names nothing else", plated({ kind: "MD" }, "/a.md"),
            ["in ~/n \u00b7 MD"]);
        expect("anything else with none is a file", plated({ size: 1 }, "/blob"),
            ["in ~/n \u00b7 File"]);
        expect("a folder is a folder, its count the row's", plated({ items: 3, names: [] }, "/f"),
            ["in ~/n \u00b7 Folder"]);

        const shot = { path: picture, name: "shot.png", where: "~", entry: file };
        const before = FileFacts.plate(shot, null, words);
        expect("until it's said, a plate is where alone, its size and change the row's", before,
            {
                eyebrow: "in ~",
                eyebrowItems: [{ said: "in ~", keep: 2, path: "~" }],
                eyebrowNames: true,
                title: "shot.png",
                facts: []
            });
        const said = { kind: "PNG", size: 2_400_000, changed: file.modified, across: 2400,
            down: 1600, picture: picture };
        const shown = FileFacts.plate(shot, said, words);
        expect("once said, where and what, the kind giving way first, and its picture",
            [shown.eyebrowItems, shown.preview], [[{ said: "in ~", keep: 2, path: "~" },
                { said: "PNG", keep: 1 }], { picture: picture, across: 2400, down: 1600 }]);
        const text = FileFacts.plate({ path: "/n/a.md", name: "a.md", where: "~/n", entry: null },
            { kind: "MD", size: 9, changed: now, lines: ["# Plan", "one"] }, words);
        expect("a file of text previews its first lines", text.preview,
            { lines: ["# Plan", "one"] });
        const sub = { path: "/n/sub", name: "sub", where: "~/n", entry: folder };
        const listed = FileFacts.plate(sub,
            { size: 4096, changed: now, items: 2, names: ["deep/", "z.md"] }, words);
        expect("a folder previews its first items, quiet, its count the row's",
            [listed.eyebrow, listed.preview, listed.facts], ["in ~/n \u00b7 Folder",
                { lines: ["deep/", "z.md"], quiet: true }, []]);
        expect("a folder not yet read is a folder at once", FileFacts.plate(sub, null, words)
            .eyebrow, "in ~/n \u00b7 Folder");
        const plain = FileFacts.plate({ path: "/n/a.zip", name: "a.zip", where: "~/n", entry: null },
            { kind: "ZIP", size: 7, changed: now }, words);
        expect("anything else is where and what alone, no stand-in", [plain.eyebrow,
            plain.preview, plain.facts], ["in ~/n \u00b7 ZIP", undefined, []]);

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
        expect("its plate is a file's here, where then its kind by its ending, its size and"
            + " day the row's, its place's facts after",
            FileFacts.plateElsewhere({ name: "menu.pdf", where: "My Drive/Trips", entry: away,
                facts: [owner] }, words), {
                eyebrow: "in My Drive/Trips \u00b7 PDF",
                eyebrowItems: [
                    { said: "in My Drive/Trips", keep: 2, path: "My Drive/Trips" },
                    { said: "PDF", keep: 1 }
                ],
                eyebrowNames: true,
                title: "menu.pdf",
                facts: [owner]
            });
        expect("what its place doesn't say is left out, never awaited",
            FileFacts.plateElsewhere({ name: "Trips", where: "My Drive",
                entry: { name: "Trips", folder: true } }, words).facts, []);
        expect("where its place says it is, in its own words, leads its eyebrow",
            FileFacts.plateElsewhere({ name: "menu.pdf", where: "My Drive/Trips",
                within: "in My Drive/Trips", entry: away, facts: [owner] }, words).eyebrow,
            "in My Drive/Trips \u00b7 PDF");
        expect("and a folder's, its kind its folder", FileFacts.plateElsewhere({ name: "Trips",
            where: "My Drive", within: "in My Drive", entry: { name: "Trips", folder: true } },
            words).eyebrow, "in My Drive \u00b7 Folder");
        expect("a file with no ending is a file", FileFacts.plateElsewhere({ name: "notes",
            where: "", entry: { name: "notes", size: 3 } }, words).eyebrow, "File");

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

        // A file in the Nix store goes by its own name past its hash, but
        // kept whole where the store's own listing holds it (VKCF3Z,
        // 2026-11-08), and every other path by its last part.
        const hash = "bdmylmvw562fyhfd39fkn89zcw7qn66w";
        const names = [
            ["a store file goes by its own name", `/nix/store/${hash}-track.mp3`, false, "track.mp3"],
            ["one listed in the store keeps its hash", `/nix/store/${hash}-track.mp3`, true,
                `${hash}-track.mp3`],
            ["one deeper in the store is its last part", `/nix/store/${hash}-pkg/share/a.mp3`, false,
                "a.mp3"],
            ["a store-like name elsewhere keeps its hash", `~/Music/${hash}-track.mp3`, false,
                `${hash}-track.mp3`],
            ["a hash Nix can't write isn't one", "/nix/store/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-track.mp3",
                false, "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-track.mp3"],
            ["a file at home is its last part", "~/src/core.nix", false, "core.nix"],
            ["the root is itself", "/", false, "/"],
            ["home is itself", "~", false, "~"]
        ];
        for (const [what, said, listed, want] of names)
            expect(what, FileFacts.named(said, listed), want);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}

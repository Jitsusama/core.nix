// How the launcher's chooser of files knows a file's kind and whether a
// program's filter passes it, run in Qt's own JavaScript engine, the one
// the shell runs it in. The `a-filter-passes-a-file-by-its-kind` check
// copies modules/open-with/FileKinds.js and tests/lib/Expect.js beside this
// file and runs `qml -platform offscreen file-kinds.qml`. The chooser's own
// check (file-chooser.qml) reads a few real lines of shared-mime-info's
// through it; these are the table's edges, which no folder there reaches:
// the heaviest of two globs for an ending, the longest ending, a whole name
// and a glob that's neither, kinds broader in a loop, and a glob's dot, its
// wildcards, its letters and a lone ] taken as a program writes them.
import QtQuick
import "FileKinds.js" as FileKinds
import "Expect.js" as Expect

QtObject {
    // Each case holds, or the first that doesn't fails the test.
    Component.onCompleted: Expect.run(run)

    // A few lines as globs2 writes them, the heaviest first, with a
    // comment, a line naming no glob, and a glob neither an ending nor a
    // whole name.
    readonly property string globs: [
        "# shared-mime-info's globs2, as a stand-in",
        "80:text/markdown:*.md",
        "50:text/x-readme:*.md",
        "60:application/x-compressed-tar:*.tar.gz",
        "50:application/gzip:*.gz",
        "50:image/png:*.PNG",
        "50:text/x-makefile:Makefile",
        "50:text/x-readme:[Rr][Ee][Aa][Dd][Mm][Ee]*",
        "50:text/broken",
        ""
    ].join("\n")

    // And as subclasses writes them, with two kinds each the other's.
    readonly property string subclasses: [
        "text/markdown text/plain",
        "text/x-makefile text/plain",
        "text/x-makefile text/x-script",
        "a/loop b/loop",
        "b/loop a/loop",
        ""
    ].join("\n")

    function run() {
        const empty = { names: {}, endings: {}, broader: {} };
        const named = FileKinds.kindsFrom(empty, globs);
        const kinds = FileKinds.broaderFrom(named, subclasses);
        const same = Expect.same;

        same("an ending's kind is its heaviest glob's", kinds.endings[".md"], "text/markdown");
        same("an ending is kept in small letters", kinds.endings[".png"], "image/png");
        same("an ending of two parts is one", kinds.endings[".tar.gz"],
            "application/x-compressed-tar");
        same("only a whole name is a name; a comment, a line with no glob and a wild glob "
            + "name nothing", kinds.names, { Makefile: "text/x-makefile" });
        same("the kinds read keep what the table held beside them",
            FileKinds.kindsFrom({ names: {}, endings: {}, broader: { "a/b": ["c/d"] } }, globs)
                .broader, { "a/b": ["c/d"] });
        same("a kind is one of each broader one said", kinds.broader["text/x-makefile"],
            ["text/plain", "text/x-script"]);
        same("the broader kinds read keep the names beside them", kinds.names, named.names);

        same("a file is its ending's kind", FileKinds.kindOf(kinds, "notes.md"),
            "text/markdown");
        same("in any letters", FileKinds.kindOf(kinds, "NOTES.MD"), "text/markdown");
        same("its longest ending first", FileKinds.kindOf(kinds, "backup.tar.gz"),
            "application/x-compressed-tar");
        same("a whole name before an ending", FileKinds.kindOf(kinds, "Makefile"),
            "text/x-makefile");
        same("a file nothing names is no kind", FileKinds.kindOf(kinds, "LICENSE"), "");

        same("a group holds its kinds", FileKinds.isKind(kinds, "image/png", "image/*", 0),
            true);
        same("a name only begun like a group's isn't in it",
            FileKinds.isKind(kinds, "imagery/png", "image/*", 0), false);
        same("a kind is a broader one it's said to be",
            FileKinds.isKind(kinds, "text/markdown", "text/plain", 0), true);
        same("kinds broader in a loop end, as none wanted",
            FileKinds.isKind(kinds, "a/loop", "c/none", 0), false);

        const text = { name: "Text", globs: ["*.txt"], kinds: [] };
        same("no filter passes every file", FileKinds.wants(kinds, null, "LICENSE", ""), true);
        same("a glob passes in any letters", FileKinds.wants(kinds, text, "A.TXT", ""), true);
        same("a glob's dot is a dot", FileKinds.wants(kinds, text, "a_txt", ""), false);
        same("a glob is the whole name", FileKinds.wants(kinds, text, "a.txt.bak", ""), false);
        const one = { name: "C", globs: ["?.c"], kinds: [] };
        same("a ? is one letter", FileKinds.wants(kinds, one, "a.c", ""), true);
        same("never two", FileKinds.wants(kinds, one, "ab.c", ""), false);
        const either = { name: "Logs", globs: ["[ab].log"], kinds: [] };
        same("a glob's brackets are a choice", FileKinds.wants(kinds, either, "b.log", ""),
            true);
        same("of only what they hold", FileKinds.wants(kinds, either, "c.log", ""), false);
        // A Kelvin sign is a k only to Unicode's folding, which a
        // program's glob was never read by.
        same("a glob's letters fold as they always did",
            FileKinds.wants(kinds, { name: "K", globs: ["*.k"], kinds: [] }, "a.\u212a", ""),
            false);
        same("a lone ] is itself",
            FileKinds.wants(kinds, { name: "Odd", globs: ["x].txt"], kinds: [] }, "x].txt", ""),
            true);

        const plain = { name: "Plain Text", globs: [], kinds: ["text/plain"] };
        same("a kind passes the kinds it's one of",
            FileKinds.wants(kinds, plain, "notes.md", ""), true);
        same("and no other", FileKinds.wants(kinds, plain, "photo.png", ""), false);
        same("a file whose kind is said goes by it, not its name",
            FileKinds.wants(kinds, plain, "notes.md", "image/png"), false);
        same("one with no ending too",
            FileKinds.wants(kinds, { name: "Pictures", globs: [], kinds: ["image/*"] }, "scan",
                "image/png"), true);
        same("a filter naming nothing passes nothing",
            FileKinds.wants(kinds, { name: "None" }, "notes.md", ""), false);
    }
}

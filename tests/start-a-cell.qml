// Start a Cell, Agents' act that begins new work from a link or words, run
// in Qt's own JavaScript engine, the one the shell runs it in. The
// `start-a-cell-asks-only-a-sprout` check copies
// modules/quickshell/launcher/Sprouting.js and tests/lib/Expect.js beside
// this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Sprouting.js" as Sprouting

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: {
        try {
            cases();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message ?? String(error));
        }
    }

    readonly property var said: ({
        unanswered: "its node didn't answer",
        unread: "its node's answer couldn't be read"
    })

    function cases() {
        // What it asks the node.
        Expect.same("a link alone is sprouted where its repository is checked out",
            Sprouting.command("harbour", "https://github.com/owner/repo/pull/7", ""),
            ["mycelium", "--realm", "harbour", "sprout", "-o", "json", "--",
                "https://github.com/owner/repo/pull/7"]);
        Expect.same("words are sprouted in the repository they're said in, as one",
            Sprouting.command("harbour", "  tidy the launcher's rows ", "/home/j/src/core"),
            ["mycelium", "--realm", "harbour", "sprout", "-o", "json", "--in",
                "/home/j/src/core", "--", "tidy the launcher's rows"]);
        Expect.same("words opening with a dash are words, never an option",
            Sprouting.command("harbour", "--join now", "/home/j/src/core"),
            ["mycelium", "--realm", "harbour", "sprout", "-o", "json", "--in",
                "/home/j/src/core", "--", "--join now"]);
        Expect.same("nothing said starts nothing",
            Sprouting.command("harbour", "   ", "/home/j/src/core"), null);
        Expect.same("no realm to ask starts nothing",
            Sprouting.command("", "tidy it", "/home/j/src/core"), null);
        Expect.same("a realm that isn't a name starts nothing",
            Sprouting.command("har bour; rm", "tidy it", "/home/j/src/core"), null);
        Expect.same("a folder that isn't a whole path is never named",
            Sprouting.command("harbour", "tidy it", "src/core"), null);
        Expect.same("it never asks for a window: no --join",
            Sprouting.command("harbour", "tidy it", "/home/j/src/core").includes("--join"),
            false);

        // Where words can be said: the repositories the open quests work in.
        Expect.same("each repository once, in the quests' order, home never",
            Sprouting.folders([
                { folder: "/home/j/src/core" },
                { folder: "/home/j" },
                { folder: "/home/j/src/harness" },
                { folder: "/home/j/src/core" },
                { folder: "" },
                {},
                { folder: "src/relative" }
            ], "/home/j"),
            [
                { folder: "/home/j/src/core", name: "~/src/core" },
                { folder: "/home/j/src/harness", name: "~/src/harness" }
            ]);
        Expect.same("home with a trailing slash is still home",
            Sprouting.folders([{ folder: "/home/j/" }], "/home/j"), []);
        Expect.same("a repository outside home reads as its whole path",
            Sprouting.folders([{ folder: "/srv/work" }], "/home/j"),
            [{ folder: "/srv/work", name: "/srv/work" }]);

        // How it ended, in the node's own words, or sprout's where it
        // refused before asking the node.
        Expect.same("started, on the machine the node chose",
            Sprouting.ended(JSON.stringify({ Ok: { Started: {
                machine: "kestrel", held: true, folder: "/home/j/src/core",
                attach: ["mycelium", "--realm", "harbour", "join", "ABC"] } } }), "", said),
            { started: true, machine: "kestrel" });
        Expect.same("refused by the node, in its words",
            Sprouting.ended(JSON.stringify({ Err: "no machine can take it" }), "", said),
            { started: false, why: "no machine can take it" });
        Expect.same("refused by sprout before it asked, in sprout's words",
            Sprouting.ended("", "mycelium: a session begun in a home folder has none of its "
                + "repository's skills: name the repository's folder\n", said),
            { started: false, why: "a session begun in a home folder has none of its "
                + "repository's skills: name the repository's folder" });
        Expect.same("of several lines said, the last is why",
            Sprouting.ended("", "warning: something\nmycelium: it isn't a folder here\n", said),
            { started: false, why: "it isn't a folder here" });
        Expect.same("nothing said back is no answer",
            Sprouting.ended("", "", said), { started: false, why: "its node didn't answer" });
        Expect.same("an answer that isn't the node's is unread",
            Sprouting.ended("{\"Ok\":{\"Placed\":{}}}", "", said),
            { started: false, why: "its node's answer couldn't be read" });
        Expect.same("an answer that isn't JSON is unread",
            Sprouting.ended("mycelium sprout: oops", "", said),
            { started: false, why: "its node's answer couldn't be read" });
    }
}

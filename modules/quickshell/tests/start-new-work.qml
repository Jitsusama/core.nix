// Start New Work, Agents' act that begins new work from a link or words, run
// in Qt's own JavaScript engine, the one the shell runs it in. The
// `start-new-work-asks-only-a-sprout` check copies
// modules/quickshell/launcher/Sprouting.js and tests/lib/Expect.js beside
// this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Sprouting.js" as Sprouting

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: Expect.run(cases)

    readonly property var said: ({
        started: "started on {machine}",
        unanswered: "its node didn't answer",
        unread: "its node's answer couldn't be read",
        unrun: "mycelium couldn't be run here",
        empty: "say what the work is",
        unrealmed: "no realm to start it in",
        unplaced: "that folder isn't a whole path"
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
        Expect.same("a realm opening with a dash would read as an option, so starts nothing",
            Sprouting.command("-x", "tidy it", "/home/j/src/core"), null);
        Expect.same("a folder that isn't a whole path is never named",
            Sprouting.command("harbour", "tidy it", "src/core"), null);

        // Why it asks nothing, said where Joel pressed.
        Expect.same("nothing said asks for the work",
            Sprouting.refused("harbour", " ", "", said), "say what the work is");
        Expect.same("no realm says so",
            Sprouting.refused("", "tidy it", "", said), "no realm to start it in");
        Expect.same("a folder not whole says so",
            Sprouting.refused("harbour", "tidy it", "src/core", said),
            "that folder isn't a whole path");
        Expect.same("a sprout it can ask is no refusal",
            Sprouting.refused("harbour", "tidy it", "/home/j/src/core", said), null);

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
        Expect.same("home spelled any other way is still home",
            Sprouting.folders([{ folder: "/home/j/" }, { folder: "/home//j" },
                { folder: "/home/j/." }, { folder: "/home/j/src/.." }], "/home/j"), []);
        Expect.same("home said with a slash after it is still home",
            Sprouting.folders([{ folder: "/home/j" }, { folder: "/home/j/src/core/" }],
                "/home/j/"),
            [{ folder: "/home/j/src/core", name: "~/src/core" }]);
        Expect.same("a folder only beginning as home does isn't under it",
            Sprouting.folders([{ folder: "/home/jo/src" }], "/home/j"),
            [{ folder: "/home/jo/src", name: "/home/jo/src" }]);
        Expect.same("a repository outside home reads as its whole path",
            Sprouting.folders([{ folder: "/srv/work" }], "/home/j"),
            [{ folder: "/srv/work", name: "/srv/work" }]);
        Expect.same("the root is never offered",
            Sprouting.folders([{ folder: "/" }], "/home/j"), []);
        Expect.same("with home unknown, no folder is offered",
            Sprouting.folders([{ folder: "/home/j/src/core" }], ""), []);

        // Both streams are heard before how it ended is said.
        const out = Sprouting.hear({}, "out", "{}");
        Expect.same("stdout alone isn't the whole of it", Sprouting.whole(out), false);
        Expect.same("stderr alone isn't the whole of it",
            Sprouting.whole(Sprouting.hear({}, "errors", "")), false);
        Expect.same("both, either way round, are",
            [Sprouting.whole(Sprouting.hear(out, "errors", "")),
                Sprouting.whole(Sprouting.hear(Sprouting.hear({}, "errors", ""), "out", ""))],
            [true, true]);
        Expect.same("hearing leaves what was heard before as it was", out, { out: "{}" });
        Expect.same("nothing heard isn't whole", Sprouting.whole(null), false);

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
        Expect.same("of several lines said, the last mycelium said is why",
            Sprouting.ended("", "warning: something\nmycelium: it isn't a folder here\n", said),
            { started: false, why: "it isn't a folder here" });
        Expect.same("a usage footer isn't the reason; the error before it is",
            Sprouting.ended("", "error: unexpected argument '--x' found\n\nUsage: mycelium "
                + "sprout [OPTIONS]\n\nFor more information, try '--help'.\n", said),
            { started: false, why: "unexpected argument '--x' found" });
        Expect.same("a line no one signed is the reason when it's all there is",
            Sprouting.ended("", "thread 'main' panicked\n", said),
            { started: false, why: "thread 'main' panicked" });
        Expect.same("nothing said back is no answer",
            Sprouting.ended("", "", said), { started: false, why: "its node didn't answer" });
        Expect.same("an answer that isn't the node's is unread",
            Sprouting.ended("{\"Ok\":{\"Placed\":{}}}", "", said),
            { started: false, why: "its node's answer couldn't be read" });
        Expect.same("an answer that isn't JSON is unread",
            Sprouting.ended("mycelium sprout: oops", "", said),
            { started: false, why: "its node's answer couldn't be read" });

        // A run that ended neither stream.
        Expect.same("one that couldn't run says so",
            Sprouting.unended(false, said), { started: false, why: "mycelium couldn't be run here" });
        Expect.same("one its node never answered says so",
            Sprouting.unended(true, said), { started: false, why: "its node didn't answer" });

        // What the act that ran says.
        Expect.same("started names the machine",
            Sprouting.told({ started: true, machine: "kestrel" }, said), "started on kestrel");
        Expect.same("refused says why",
            Sprouting.told({ started: false, why: "no machine can take it" }, said),
            "no machine can take it");
    }
}

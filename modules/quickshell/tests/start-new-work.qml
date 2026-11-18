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
        empty: "say what the work is",
        unplaced: "that folder isn't a whole path"
    })

    function cases() {
        // What it asks the agents daemon.
        Expect.same("a link alone is sprouted where its repository is checked out",
            Sprouting.ask("https://github.com/owner/repo/pull/7", ""),
            { ask: "sprout", text: "https://github.com/owner/repo/pull/7" });
        Expect.same("words are sprouted in the repository they're said in, as one",
            Sprouting.ask("  tidy the launcher's rows ", "/home/j/src/core"),
            { ask: "sprout", text: "tidy the launcher's rows", folder: "/home/j/src/core" });
        Expect.same("words opening with a dash are words",
            Sprouting.ask("--join now", "/home/j/src/core"),
            { ask: "sprout", text: "--join now", folder: "/home/j/src/core" });
        Expect.same("nothing said starts nothing", Sprouting.ask("   ", "/home/j/src/core"), null);
        Expect.same("a folder that isn't a whole path is never named",
            Sprouting.ask("tidy it", "src/core"), null);

        // Why it asks nothing, said where Joel pressed.
        Expect.same("nothing said asks for the work",
            Sprouting.refused(" ", "", said), "say what the work is");
        Expect.same("a folder not whole says so",
            Sprouting.refused("tidy it", "src/core", said), "that folder isn't a whole path");
        Expect.same("a sprout it can ask is no refusal",
            Sprouting.refused("tidy it", "/home/j/src/core", said), null);

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

        // How it ended, as the daemon answers (modules/agents/src/sprout.rs).
        Expect.same("started, on the machine the node chose",
            Sprouting.ended({ id: "ask-3", sprouted: { machine: "kestrel", held: "h1",
                folder: "/home/j/src/core", attach: ["mycelium", "join", "h1"] } }, said),
            { started: true, machine: "kestrel" });
        Expect.same("refused, in the node's or the daemon's words",
            Sprouting.ended({ id: "ask-3", error: { code: "unsprouted",
                message: "only the launcher starts new work here" } }, said),
            { started: false, why: "only the launcher starts new work here" });
        Expect.same("no daemon to ask is no answer",
            Sprouting.ended(null, said), { started: false, why: "its node didn't answer" });
        Expect.same("an answer that isn't a sprout's is unread",
            Sprouting.ended({ id: "ask-3", sprouted: {} }, said),
            { started: false, why: "its node's answer couldn't be read" });
        Expect.same("an error with no words is unread",
            Sprouting.ended({ error: { code: "unsprouted" } }, said),
            { started: false, why: "its node's answer couldn't be read" });

        // What the act that ran says.
        Expect.same("started names the machine",
            Sprouting.told({ started: true, machine: "kestrel" }, said), "started on kestrel");
        Expect.same("refused says why",
            Sprouting.told({ started: false, why: "no machine can take it" }, said),
            "no machine can take it");
    }
}

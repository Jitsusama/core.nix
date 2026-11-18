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
        // Only Joel's own Return sprouts, failing closed.
        Expect.same("his Return in the launcher sprouts",
            Sprouting.fromJoel({ cataloguing: false }), true);
        Expect.same("a walk from outside starts nothing",
            Sprouting.fromJoel({ cataloguing: true }), false);
        Expect.same("no launcher starts nothing", Sprouting.fromJoel(null), false);
        Expect.same("an unset launcher starts nothing", Sprouting.fromJoel(undefined), false);
        Expect.same("a launcher that doesn't say it isn't walked starts nothing",
            Sprouting.fromJoel({}), false);

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

        // How it ended, as the daemon answers (modules/agents/src/sprout.rs).
        Expect.same("started, on the machine the node chose, held by the id it gave",
            Sprouting.ended({ id: "ask-3", sprouted: { machine: "kestrel", held: "h1",
                folder: "/home/j/src/core", attach: ["mycelium", "join", "h1"] } }, said),
            { started: true, machine: "kestrel", held: "h1" });
        Expect.same("started with no holder id says no id",
            Sprouting.ended({ sprouted: { machine: "kestrel" } }, said),
            { started: true, machine: "kestrel", held: "" });
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

        // Which running session is the one it started, to open for him (Joel, 18 November:
        // "start new work does not seem to work from the launcher"): here, by its holder's
        // id; on the machine the node named, by the words that attach it, which end in that
        // id, as PiSource.js keeps a session elsewhere (here: false, held: null).
        const here = { address: "ONE", here: true, held: "h1", attach: null };
        const there = { address: "TWO", here: false, machine: "kestrel", held: null,
            attach: ["mycelium", "--realm", "work", "attach", "h2"] };
        const other = { address: "THREE", here: true, held: "h3",
            attach: ["mycelium", "attach", "h3"] };
        Expect.same("one held here is found by its holder's id",
            Sprouting.sproutedOf([other, here], "h1", "optimus")?.address, "ONE");
        Expect.same("one on another machine is found by its attach words",
            Sprouting.sproutedOf([other, there], "h2", "kestrel")?.address, "TWO");
        Expect.same("none is found before it's up",
            Sprouting.sproutedOf([other], "h1", "optimus"), null);
        Expect.same("no id finds none, never the first held",
            Sprouting.sproutedOf([{ address: "FOUR", here: true, held: "", attach: [] }], "",
                "optimus"), null);
        Expect.same("the same id on another machine than the node named finds none",
            Sprouting.sproutedOf([there], "h2", "falcon"), null);
        Expect.same("a session here whose attach words name the id isn't it",
            Sprouting.sproutedOf([{ address: "FIVE", here: true, held: "h9",
                attach: ["mycelium", "attach", "h2"] }], "h2", "kestrel"), null);
        Expect.same("attach words naming the id but not at their end aren't it",
            Sprouting.sproutedOf([{ address: "SIX", here: false, machine: "kestrel", held: null,
                attach: ["mycelium", "attach", "h2", "--then", "x"] }], "h2", "kestrel"), null);

        // Whether Joel still waits on the act in the launcher, so it may close for its
        // window and take the keys: open, on the act itself or held in its ask, where
        // Return leaves him with the field emptied for another try.
        const act = "verb:agents:start-new-work:link";
        Expect.same("held in the act's ask, it waits",
            Sprouting.waitsOn({ open: true, asking: { key: act }, selected: null }, act), true);
        Expect.same("on the act's row, it waits",
            Sprouting.waitsOn({ open: true, asking: null, selected: { key: act } }, act), true);
        Expect.same("on another row, it doesn't",
            Sprouting.waitsOn({ open: true, asking: null, selected: { key: "other" } }, act),
            false);
        Expect.same("closed, it doesn't",
            Sprouting.waitsOn({ open: false, asking: { key: act }, selected: null }, act), false);
        Expect.same("no launcher, it doesn't", Sprouting.waitsOn(null, act), false);

        // Which waiting to open are past the minute, at `now`, and which still wait.
        const openings = [{ key: "a", held: "h1", since: 1000 }, { key: "b", held: "h2",
            since: 50000 }];
        Expect.same("past the minute, one goes",
            Sprouting.overdue(openings, 62000, 60000).map(one => one.key), ["a"]);
        Expect.same("within it, none goes", Sprouting.overdue(openings, 30000, 60000), []);

        // What the act that ran says.
        Expect.same("started names the machine",
            Sprouting.told({ started: true, machine: "kestrel" }, said), "started on kestrel");
        Expect.same("refused says why",
            Sprouting.told({ started: false, why: "no machine can take it" }, said),
            "no machine can take it");
    }
}

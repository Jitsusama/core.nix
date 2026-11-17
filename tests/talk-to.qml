// Whom `qs ipc call launcher talkTo` opens a talk to, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `talk-to-opens-only-on-whom-it-names` check copies
// modules/quickshell/launcher/TalkTo.js and tests/lib/Expect.js beside this
// file and runs it offscreen. That the launcher then opens on the
// session's Talk, its field empty, is
// tests/agents-on-niri-test.py's.
import QtQuick
import "Expect.js" as Expect
import "launcher/TalkTo.js" as TalkTo

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

    // What the launcher holds here: the void at VOID01, two sessions and
    // a repository's network, named as the agents daemon names it.
    readonly property var known: ({
        isVoid: said => said.toUpperCase() === "VOID01",
        session: address => ["VOID01", "LISTEN", "PAUSED"].includes(address),
        network: name => name === "repo.world"
    })

    function cases() {
        Expect.same("a session's address talks to it",
            TalkTo.whom("LISTEN", known), { to: "session", address: "LISTEN" });
        Expect.same("an address in lower case is the same session's",
            TalkTo.whom("listen", known), { to: "session", address: "LISTEN" });
        Expect.same("words after an address are left behind, so nothing is filled in",
            TalkTo.whom("  PAUSED send this now", known), { to: "session", address: "PAUSED" });
        Expect.same("the void is talked to on its line, never in a field of its own",
            TalkTo.whom("void01", known), { to: "void" });
        Expect.same("a network is told apart, since it has no one to talk to",
            TalkTo.whom("repo.world", known), { to: "network", name: "repo.world" });
        Expect.same("an address no session has is nobody",
            TalkTo.whom("GONE42", known), { to: "none", said: "GONE42" });
        Expect.same("nothing said is nobody",
            TalkTo.whom("   ", known), { to: "none", said: "" });
        Expect.same("a network is named by the first word said alone, as network reads it",
            TalkTo.named(" repo.world and more\tstill"), "repo.world");
        Expect.same("nothing said names nothing", TalkTo.named(undefined), "");

        // Whom `qs ipc call launcher goTo` goes to, as a notice's Go To
        // asks it: a session alone, since only a session has a window.
        Expect.same("a session's address goes to it, in any case",
            TalkTo.goesTo("listen", known), "LISTEN");
        Expect.same("words after an address are left behind",
            TalkTo.goesTo("PAUSED and type this", known), "PAUSED");
        Expect.same("the void has no window to go to", TalkTo.goesTo("VOID01", known), "");
        Expect.same("nor has a network", TalkTo.goesTo("repo.world", known), "");
        Expect.same("an address no session has goes nowhere",
            TalkTo.goesTo("GONE42", known), "");
        Expect.same("nothing said goes nowhere", TalkTo.goesTo("", known), "");
    }
}

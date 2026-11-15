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
    }
}

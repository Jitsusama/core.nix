// What Carry Back asks the realm's node and how its answer ends, run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `carry-back-asks-the-node-only-a-carry` check copies
// modules/quickshell/launcher/CarryBack.js and tests/lib/Expect.js beside
// this file and runs it offscreen. That Return on a carried row sends it
// is tests/agents-on-niri-test.py's.
import QtQuick
import "Expect.js" as Expect
import "launcher/CarryBack.js" as CarryBack

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
        unanswered: "its machine didn't answer",
        unread: "its refusal couldn't be read"
    })

    function cases() {
        Expect.same("a carry of a quest back to a machine is asked as served",
            CarryBack.asked({ ask: "carry", quest: "QEST-20261115-HARBOR", to: "skiff" }),
            { ask: "carry", quest: "QEST-20261115-HARBOR", to: "skiff" });
        Expect.same("and nothing else it holds is sent",
            CarryBack.asked({ ask: "carry", quest: "QEST-20261115-HARBOR", to: "skiff",
                checked: true, by: "KELP01" }),
            { ask: "carry", quest: "QEST-20261115-HARBOR", to: "skiff" });
        Expect.same("any other ask of the node is never sent",
            CarryBack.asked({ ask: "update", quest: "QEST-20261115-HARBOR", to: "skiff" }), null);
        Expect.same("nor one naming no quest",
            CarryBack.asked({ ask: "carry", quest: "", to: "skiff" }), null);
        Expect.same("nor a quest's id it can't be",
            CarryBack.asked({ ask: "carry", quest: "QEST-1 --all", to: "skiff" }), null);
        Expect.same("nor a machine's name it can't be",
            CarryBack.asked({ ask: "carry", quest: "QEST-20261115-HARBOR", to: "-skiff" }),
            null);
        Expect.same("nor nothing", CarryBack.asked(undefined), null);

        Expect.same("a carry that ended carried is done",
            CarryBack.ended({ id: 3, carried: { step: "carried", state: "done",
                machine: "skiff", sessions: [] } }, said), { done: true });
        Expect.same("a refusal says the node's own why",
            CarryBack.ended({ id: 3, refused: true, error: {
                message: "Q's sessions run on a and b, so which machine to carry it from can't be told"
            } }, said),
            { done: false,
                why: "Q's sessions run on a and b, so which machine to carry it from can't be told" });
        Expect.same("one whose why can't be read says so",
            CarryBack.ended({ id: 3, refused: true }, said),
            { done: false, why: "its refusal couldn't be read" });
        Expect.same("a carry that ended otherwise is no carry back",
            CarryBack.ended({ id: 3, carried: { step: "started", state: "failed" } }, said),
            { done: false, why: "its refusal couldn't be read" });
        Expect.same("no answer at all says its machine didn't answer",
            CarryBack.ended(null, said), { done: false, why: "its machine didn't answer" });

        const carry = (state, at) => ({ kind: "carry", held: { state, at,
            quest: "QEST-20261115-HARBOR", from: "skiff", to: "tern" } });
        Expect.same("a carry ordered is under way, a thing to read since it was made",
            CarryBack.underWay(carry("ordered", "2026-11-16T01:00:00Z")),
            { kind: "read", since: "2026-11-16T01:00:00Z" });
        Expect.same("and so is one carrying",
            CarryBack.underWay(carry("carrying", "2026-11-16T01:00:00Z")),
            { kind: "read", since: "2026-11-16T01:00:00Z" });
        Expect.same("one with no time is under way since none",
            CarryBack.underWay(carry("carrying", undefined)), { kind: "read", since: null });
        for (const state of ["arrived", "refused", "back", ""])
            Expect.same(`one ${state || "of no state"} is under way no longer`,
                CarryBack.underWay(carry(state, "2026-11-16T01:00:00Z")), null);
        Expect.same("nor is a row that is no carry",
            CarryBack.underWay({ kind: "session", held: { state: "carrying" } }), null);
        Expect.same("nor nothing", CarryBack.underWay(null), null);
        Expect.same("nor a carry Joel made himself, since he knows it",
            CarryBack.underWay({ kind: "carry", held: { state: "carrying", by: "person",
                at: "2026-11-16T01:00:00Z" } }), null);
    }
}

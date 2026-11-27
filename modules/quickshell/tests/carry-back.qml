// What Carry Back asks the realm's node and how its answer ends, run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `carry-back-asks-the-node-only-a-carry` check copies
// modules/quickshell/launcher/CarryBack.js and tests/lib/Expect.js beside
// this file and runs it offscreen. That Return on a carried row sends it
// is tests/agents-on-niri-test.py's, whose carries' things claim no kind of
// the launcher's own and match their keys, so the key a carried row reads
// as carrying back under, and has its refusal said on, is pinned here,
// through CarryBack.carry, which Agents.qml's carryBack hands everything
// to but the provider's and the node's calls.
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

        Expect.same("a carried row is keyed by its thing, as ReadyRows keyed it",
            CarryBack.keyed({ thing: "carry:7", key: "carry:other" }, "carry:other"),
            "carry:7");
        Expect.same("one with no thing by its record's key, not the key it was served with",
            CarryBack.keyed({ key: "carry:served" }, "carry:7"), "carry:7");
        Expect.same("and one whose thing claims the launcher's own place stays under row:",
            CarryBack.keyed({ thing: "place:Harbour" }, "carry:7"), "row:place:Harbour");

        carrying();
    }

    // A shell's calls recorded in order, and the node's answer held to be
    // given when a case says.
    function recording() {
        const shell = { told: [], until: null, answer: null };
        shell.becomes = (shown, side, name, doing, until) => {
            shell.told.push(["becomes", shown, side, name, doing]);
            shell.until = until;
        };
        shell.refused = (shown, why) => shell.told.push(["refused", shown, why]);
        shell.find = (asked, answered) => {
            shell.told.push(["find", asked]);
            shell.answer = answered;
        };
        return shell;
    }

    function carrying() {
        const ask = { ask: "carry", quest: "QEST-20261115-HARBOR", to: "skiff" };
        const words = { carrying: "carrying back", unread: said.unread };
        const side = { side: "carrying back", sideItems: [{ said: "carrying back", keep: 4 }] };

        const done = recording();
        CarryBack.carry({
            row: { thing: "carry:7", key: "carry:other", name: "Harbour" },
            key: "carry:other",
            ask
        }, words, done);
        Expect.same("Carry Back has its row, keyed by its thing, read as carrying back, then asks",
            done.told, [["becomes", "carry:7", side, "Harbour", "carry back"], ["find", ask]]);
        Expect.same("which stays carrying back while the node hasn't answered",
            done.until.done(), false);
        done.answer({ id: 3, carried: { step: "carried", state: "done" } });
        Expect.same("and is done once the carry ended carried", done.until.done(), true);
        Expect.same("with no refusal said", done.told.length, 2);

        const refused = recording();
        CarryBack.carry({ row: { thing: "place:Harbour", name: "Harbour" }, key: "carry:7", ask },
            words, refused);
        refused.answer({ id: 3, refused: true, error: { message: "skiff is out of reach" } });
        Expect.same("a refusal is said on the row it read as carrying back, row: kept",
            refused.told.slice(0, 1).concat(refused.told.slice(2)),
            [["becomes", "row:place:Harbour", side, "Harbour", "carry back"],
                ["refused", "row:place:Harbour", "skiff is out of reach"]]);
        Expect.same("and never ends carrying back as done", refused.until.done(), false);

        const kept = recording();
        CarryBack.carry({
            row: { thing: "carry:7", name: "Harbour" },
            key: "carry:7",
            ask: { ask: "carry", quest: "QEST-20261115-HARBOR", to: "skiff",
                checked: true, by: "KELP01" }
        }, words, kept);
        Expect.same("a served ask is sent as asked rebuilds it, nothing else it holds",
            kept.told.slice(1), [["find", ask]]);

        const unasked = recording();
        CarryBack.carry({
            row: { thing: "carry:7", name: "Harbour" },
            key: "carry:7",
            ask: { ask: "update", quest: "QEST-20261115-HARBOR", to: "skiff" }
        }, { carrying: "carrying back", unasked: "it named no carry to send" }, unasked);
        Expect.same("one that is no carry is never sent, and is refused on its row",
            unasked.told,
            [["becomes", "carry:7", side, "Harbour", "carry back"],
                ["refused", "carry:7", "it named no carry to send"]]);
        Expect.same("nor ever ends carrying back as done", unasked.until.done(), false);
    }
}

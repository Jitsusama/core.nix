// What Daemon.qml's askTillTaken decides of an ask refused for now, run in Qt's own JavaScript
// engine, the one the shell runs it in: two Forget Its given as words each wait their own try;
// two kept places given as their latest wait as one, with the most tries either had left; no
// answer at the last try gives it up, warned and said with no refusal; `refused` is asked again
// and any other code is done; and a try falling due with no connection asks nothing and spends
// no try. The `an-ask-refused-for-now-is-asked-again` check copies modules/quickshell/Retrying.js
// and tests/lib/Expect.js beside this file and runs `qml -platform offscreen retrying.qml`.
// tests/daemon.qml holds the path through a real socket.
import QtQuick
import "Expect.js" as Expect
import "Retrying.js" as Retrying

QtObject {
    Component.onCompleted: {
        try {
            const refused = { error: { code: "refused", message: "not yet" } };

            // Each Forget It names the session it forgets, so neither stands for the other
            // (round 000036 on #6796, F118 and F123).
            const a = { ask: "forget", session: "a" };
            const b = { ask: "forget", session: "b" };
            const forgets = Retrying.waiting(Retrying.waiting([], a, "forget", 5), b, "forget",
                5);
            Expect.same("two Forget Its given as words both wait, each with its tries",
                forgets, [{ said: a, ask: "forget", tries: 4 }, { said: b, ask: "forget",
                    tries: 4 }]);

            // Kept places say their latest when asked, so one try stands for both (F111), with
            // the fresher's tries (F139).
            const places = () => ({ ask: "keep-places", places: {} });
            const older = Retrying.waiting([], places, "keep-places", 2);
            const kept = Retrying.waiting(older.concat(forgets), places, "keep-places", 5);
            Expect.same("two kept places given as their latest wait as one, the most tries left",
                kept.filter(one => one.ask === "keep-places").map(one => one.tries), [4]);
            Expect.expect("the one kept asks its latest, as a function",
                kept.find(one => one.ask === "keep-places").said === places, "it didn't");
            Expect.same("and leave the Forget Its waiting as they were",
                kept.filter(one => one.ask === "forget"), forgets);
            const sharing = () => ({ ask: "sharing", sharing: false });
            Expect.same("one given as its latest stands only for those of its own ask",
                Retrying.waiting(older, sharing, "sharing", 5).map(one => one.ask),
                ["keep-places", "sharing"]);
            Expect.same("a function says its latest when asked, words as they are",
                [Retrying.asked(places), Retrying.asked(a)],
                [{ ask: "keep-places", places: {} }, a]);

            // The daemon gone with a switch's restart, at the last of the five tries.
            for (const none of [null, undefined]) {
                Expect.same(`${none} at the last try gives it up, said with no refusal`,
                    Retrying.after(none, 1), { next: "givenUp", said: "no answer", why: "" });
            }
            Expect.same("no answer with tries left is asked again",
                Retrying.after(null, 2), { next: "again" });
            Expect.same("refused for now with tries left is asked again",
                Retrying.after(refused, 5), { next: "again" });
            Expect.same("refused at the last try gives it up with the daemon's refusal",
                Retrying.after(refused, 1), { next: "givenUp", said: "not yet", why: "not yet" });
            Expect.same("a refusal with no words gives it up with none",
                Retrying.after({ error: { code: "refused" } }, 1).why, "");
            for (const [why, answer] of [
                ["another code", { error: { code: "not-an-ask", message: "no places" } }],
                ["an answer taken", { kept: 2 }]
            ]) {
                Expect.same(`${why} is done, though tries are left`, Retrying.after(answer, 5),
                    { next: "done" });
            }

            // Nothing answers: the connection's return starts the try again.
            Expect.same("a try due with no connection asks nothing and keeps all as they were",
                Retrying.due(kept, false), { now: [], kept: kept });
            Expect.same("a try due with one asks all that waited",
                Retrying.due(kept, true), { now: kept, kept: [] });
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}

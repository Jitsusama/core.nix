// What the launcher keeps to say as it next opens, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-word-told-shut-waits-beside-the-others` check copies
// modules/quickshell/launcher/ToldShut.js and tests/lib/Expect.js beside
// this file and runs `qml -platform offscreen told-shut.qml`. Each case is
// a press told while the launcher was shut, and what its next opening says.
import QtQuick
import "ToldShut.js" as ToldShut
import "Expect.js" as Expect

QtObject {
    // What waits once each of `tells` (told, at, loose, and what it's
    // about where its teller says) was told in turn, as tellOnOpen keeps it.
    function kept(tells) {
        let waiting = [];
        for (const one of tells)
            waiting = ToldShut.waiting(waiting, { told: one.told, at: one.at,
                about: one.about ?? ToldShut.aboutOf(one.told), loose: one.loose === true });
        return waiting;
    }
    // What an opening at `now` says after `tells`, as the foot's words in
    // one line.
    function opened(tells, now) {
        const due = ToldShut.due(kept(tells), now);
        return { words: due.said.map(said => said.slice(0, 2).join(" ")).join(" | "),
            loose: due.loose, lapsed: due.lapsed };
    }

    readonly property real minute: 60 * 1000
    readonly property var coming: [["Quest A", "may yet come", "alert"]]
    readonly property var asleep: [["Quest B", "is asleep", "alert"]]
    readonly property var came: [["Quest A", "came after all", "text"]]

    Component.onCompleted: Expect.run(() => {
        Expect.same("a word told shut is said as it opens",
            opened([{ told: asleep, at: 0 }], minute).words, "Quest B is asleep");
        Expect.same("a late word about one press leaves a newer press's about another",
            opened([{ told: asleep, at: 0 }, { told: came, at: minute }], 2 * minute).words,
            "Quest B is asleep | Quest A came after all");
        Expect.same("a newer word about the same session takes the older's place",
            opened([{ told: coming, at: 0 }, { told: came, at: minute }], 2 * minute).words,
            "Quest A came after all");
        Expect.same("a word told more than a quarter hour before it opens lapses",
            opened([{ told: asleep, at: 0 }], 15 * minute + 1),
            { words: "", loose: false, lapsed: 1 });
        Expect.same("one told a quarter hour before is said still",
            opened([{ told: asleep, at: 0 }], 15 * minute).words, "Quest B is asleep");
        Expect.same("only the lapsed word goes",
            opened([{ told: asleep, at: 0 }, { told: came, at: 10 * minute }], 20 * minute),
            { words: "Quest A came after all", loose: false, lapsed: 1 });
        Expect.same("words are loose only where each is",
            [opened([{ told: asleep, at: 0, loose: true }], minute).loose,
                opened([{ told: asleep, at: 0, loose: true }, { told: came, at: 0 }],
                    minute).loose],
            [true, false]);
        const started = [["Start New Work", "asked", "text"], ["Start New Work", "up", "text"]];
        Expect.same("a start's words under one mark are each kept",
            opened([{ told: started, at: 0 }], minute).words,
            "Start New Work asked | Start New Work up");
        const both = kept([{ told: asleep, at: 0 }, { told: started, at: 0 }]);
        Expect.same("taking a start's words back leaves another's",
            ToldShut.said(ToldShut.without(both, started)).map(one => one[0]), ["Quest B"]);
        Expect.same("and so does taking them back from what an opening said",
            ToldShut.said(ToldShut.without(ToldShut.due(both, minute).tells, started))
                .map(one => one[0]),
            ["Quest B"]);
        // An act refused says its why, then the failed mark every refusal
        // shares (Becoming.js's told), and is about the act it answers.
        const refused = (why, what) => [[why, "", "muted"], ["x", what, "alert"]];
        Expect.same("a second act refused while shut leaves the first's whole",
            opened([{ told: refused("timed out", "VPN not connected"), at: 0,
                about: "VPN not connected" },
            { told: refused("timed out", "Home not disconnected"), at: minute,
                about: "Home not disconnected" }], 2 * minute).words,
            "timed out  | x VPN not connected | timed out  | x Home not disconnected");
        Expect.same("and the same act refused again takes its place whole",
            opened([{ told: refused("no answer", "VPN not connected"), at: 0,
                about: "VPN not connected" },
            { told: refused("timed out", "VPN not connected"), at: minute,
                about: "VPN not connected" }], 2 * minute).words,
            "timed out  | x VPN not connected");
        // A walk's tallies mark each count by the count, and are about the
        // walk (Agents.qml's tellAll), so only its newest is said.
        const tally = (gone, reopened) => [[String(gone), "gone", "alert"],
            [String(reopened), "reopened"]];
        Expect.same("a walk's newest tally alone is said",
            opened([{ told: tally(1, 3), at: 0, about: "reopen them all" },
                { told: tally(2, 2), at: 0, about: "reopen them all" },
                { told: tally(3, 1), at: 0, about: "reopen them all" }], minute).words,
            "3 gone | 1 reopened");
        Expect.same("a lapsed tell counts its words",
            opened([{ told: tally(1, 3), at: 0, about: "reopen them all" }],
                15 * minute + 1).lapsed,
            2);
    })
}

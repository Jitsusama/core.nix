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
            waiting = ToldShut.waiting(waiting,
                ToldShut.tell(one.told, one.loose, one.about, one.at));
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
        // Taking one back from what an opening said, as a start takes back
        // its words (Launcher.qml's untell), with the foot showing them still
        // or not.
        const two = ToldShut.due(both, minute);
        const openedTwo = { tells: two.tells, said: two.said };
        const shownStill = ToldShut.untold(openedTwo, two.said, started, true);
        Expect.same("one taken back while the foot shows them says the rest",
            [shownStill.foot.map(one => one[0]), shownStill.state.said === shownStill.foot],
            [["Quest B"], true]);
        Expect.same("one taken back after a key let them go says nothing again",
            ToldShut.untold(openedTwo, two.said, started, false),
            { state: { tells: [], said: null }, foot: [] });
        Expect.same("one taken back from a foot saying something else leaves it",
            ToldShut.untold(openedTwo, [["q", "does nothing"]], started, true).foot, null);
        Expect.same("words said alone are cleared when taken back",
            ToldShut.untold(openedTwo, started, started, true).foot, []);
        Expect.same("words no tell holds leave the foot",
            ToldShut.untold(openedTwo, two.said, coming, true).foot, null);
        // An offer's foot said beside the tells that waited with it.
        const offer = [["link", "from Quest C", "text"]];
        const withOffer = ToldShut.due(kept([{ told: asleep, at: 0 },
            { told: offer, at: 0, about: "from Quest C" }]), minute);
        const offered = { tells: withOffer.tells, said: withOffer.said };
        Expect.same("an offer said among other tells is said",
            [ToldShut.among(offered, withOffer.said, offer),
                ToldShut.among(offered, [["q", "does nothing"]], offer),
                ToldShut.among(offered, withOffer.said, came)],
            [true, false, false]);
        // Fitted into a foot with room for `words` words, as the foot
        // weighs them (Launcher.qml's keysFittedIn).
        const roomFor = words => keys => keys.length <= words;
        const refusals = ToldShut.due(kept([
            { told: refused("timed out", "VPN not connected"), at: 0,
                about: "VPN not connected" },
            { told: refused("no answer", "Home not disconnected"), at: 0,
                about: "Home not disconnected" }]), minute).tells;
        const firstWhole = ["timed out", "x"];
        Expect.same("a narrow foot gives the newest refusal way whole",
            ToldShut.fitting(refusals, roomFor(3)).map(one => one[0]), firstWhole);
        Expect.same("and says both where both fit",
            ToldShut.fitting(refusals, roomFor(4)).map(one => one[0]),
            ["timed out", "x", "no answer", "x"]);
        Expect.same("the one tell left is left for the foot to cut",
            ToldShut.fitting(refusals, roomFor(1)).map(one => one[0]), firstWhole);
        Expect.same("a lapsed tell counts its words",
            opened([{ told: tally(1, 3), at: 0, about: "reopen them all" }],
                15 * minute + 1).lapsed,
            2);
    })
}

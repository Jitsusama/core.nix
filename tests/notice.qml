// What a notice's card offers, its sender's own Read, and the line of a
// notice a search names, run in Qt's own JavaScript engine, the one the
// shell runs it in. Its check copies Notice.js beside this file and runs
// `qml -platform offscreen notice.qml`.
import QtQuick
import "Notice.js" as Notice

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // A sender's actions as Quickshell gives them, cut to what's read.
    readonly property var slack: [
        { identifier: "default", text: "View" },
        { identifier: "reply", text: "Reply" }
    ]

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        // Its sender's own opening is Open, never listed as an offer.
        expect("what it's about isn't an offer", Notice.offers(slack),
            [{ identifier: "reply", text: "Reply" }]);

        // Its sender's own Read, where it offers one, by its identifier or
        // its words, which d sends at once on the card.
        const marking = { identifier: "mark-as-read", text: "Mark as Read" };
        expect("a sender's Read, by its identifier",
            Notice.readOffer(slack.concat([marking])), marking);
        expect("or by its words",
            Notice.readOffer([{ identifier: "2", text: "Read" }]), { identifier: "2", text: "Read" });
        expect("and none where it offers none, a word holding read among them",
            Notice.readOffer(slack.concat([{ identifier: "readme", text: "Readme" }])), null);

        // Its row's name and note: its title, then the line of its body
        // that holds what was typed, its first line while nothing typed is
        // only in the body, and with no title its first line alone. A word
        // is held where it starts one of the text's words, as the search
        // finds it.
        const holds = (text, word) => text.toLowerCase().split(/\W+/)
            .some(one => one.startsWith(word));
        const log = "line 1 of the deploy went by\n\nline 7 of the deploy went by\n";
        expect("nothing typed", Notice.named("Deploy log", log, [], holds),
            { name: "Deploy log", note: "line 1 of the deploy went by" });
        expect("typed only in its title",
            Notice.named("Deploy log", log, ["deploy", "log"], holds),
            { name: "Deploy log", note: "line 1 of the deploy went by" });
        expect("typed in its body", Notice.named("Deploy log", log, ["log", "7"], holds),
            { name: "Deploy log", note: "line 7 of the deploy went by" });
        expect("the line holding most of it",
            Notice.named("Build", "it went by\nthe deploy failed", ["deploy", "fail"], holds),
            { name: "Build", note: "the deploy failed" });
        expect("no title", Notice.named("", "  first line\nsecond", ["sec"], holds),
            { name: "first line", note: "" });
        expect("no body", Notice.named("Battery low", "", ["low"], holds),
            { name: "Battery low", note: "" });

        // Lined, each row's note is worked out only once it's read, so a
        // search finding hundreds lines only the one the main search shows.
        let asked = 0;
        const counted = (text, word) => {
            asked += 1;
            return holds(text, word);
        };
        const rows = ["7", "1", "7"].map(line => ({
            key: line, title: "Deploy log", wrote: log, count: "", follows: "program:deploy"
        }));
        const lined = Notice.lined(rows, ["log", "7"], counted);
        expect("lined, no note is worked out", asked, 0);
        expect("until a note is read", lined[0].count, "line 7 of the deploy went by");
        const once = asked;
        void lined[0].count;
        expect("a note read twice is worked out once", asked, once);
        void lined[1].count;
        expect("another's was left till it was read", asked > once, true);
        expect("its row's own note stays its", rows.map(row => row.count), ["", "", ""]);
        expect("the rest is its row's", [lined[1].follows, lined[1].key],
            ["program:deploy", "1"]);
        lined[2].count = "said";
        expect("a note set holds", lined[2].count, "said");

        // A row lined, then given a field more, as History's age gives its
        // rows their side, keeps that field when it's lined again. Qt 6.12
        // names the slot of a getter's setter once per shape, so a field
        // turned into a getter on a shape grown since writes its setter
        // over the new field, or past the copy's end. That wrote over the
        // shell's memory until it crashed in Notice.lined, at 11:09 on
        // 2026-11-11.
        const grown = { key: "notice:grown", count: "", title: "Deploy log", wrote: log,
            atGrown: 1 };
        Notice.lined([grown], ["log"], holds);
        grown.sideGrown = "unread";
        const regrown = Notice.lined([grown], ["log"], holds)[0];
        expect("a field given between linings is kept",
            Object.assign({}, regrown).sideGrown, "unread");

        // A search's one copy of a row says the line for the words its
        // last search typed, worked out once for them, so it's never made
        // again for a key.
        asked = 0;
        const searched = { words: ["log", "7"] };
        const copy = Notice.lining(rows[0], searched, counted);
        expect("a copy lining works out no note", asked, 0);
        expect("its note is for the words typed", copy.count, "line 7 of the deploy went by");
        const linedOnce = asked;
        void copy.count;
        expect("read again for the same words, it's worked out once", asked, linedOnce);
        searched.words = ["log", "1"];
        expect("and again once the words change", copy.count, "line 1 of the deploy went by");
        expect("the rest is its row's too", [copy.follows, copy.key], ["program:deploy", "7"]);
        copy.count = "said";
        expect("a note set holds for its words", copy.count, "said");

        // How many each sender sent, and how many of those are unread,
        // counted once over every notice, so a program's row sums its own
        // senders' without copying a notice; one with no sender counts
        // under "".
        const tally = Notice.bySender([{ sender: "Slack", unread: true },
            { sender: "slack", unread: false }, { sender: "Slack", unread: false },
            { sender: "Mail", unread: true }, {}]);
        expect("each sender's notices counted, the unread apart",
            [tally.Slack, tally.slack, tally.Mail, tally[""]],
            [{ count: 2, unread: 1 }, { count: 1, unread: 0 }, { count: 1, unread: 1 },
                { count: 1, unread: 0 }]);
        expect("a sender that sent nothing isn't there", "Chrome" in tally, false);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}

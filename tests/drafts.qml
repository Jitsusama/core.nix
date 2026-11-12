// What Joel's words in a talk's field become as he types, sends and is
// delivered to, and as the shell starts again, run in Qt's own JavaScript
// engine, the one the shell runs it in. The
// `a-talks-words-outlive-the-shell` check copies
// modules/quickshell/launcher/Drafts.js and tests/lib/Expect.js beside this
// file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Drafts.js" as Drafts

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

    readonly property real day: 24 * 60 * 60 * 1000
    readonly property real now: 1e12

    function cases() {
        const none = Drafts.empty();
        Expect.same("an address never typed to opens empty",
            Drafts.kept(none, "8GYNJ6", now), { text: "", caret: 0 });

        const typing = Drafts.typed(none, "8GYNJ6", { text: "what's on screen", caret: 7 },
            now);
        Expect.same("what's being typed opens again with its caret",
            Drafts.kept(typing, "8GYNJ6", now + 1000), { text: "what's on screen", caret: 7 });
        Expect.same("another address's field is its own",
            Drafts.kept(typing, "P9R1KI", now), { text: "", caret: 0 });
        Expect.same("a field emptied keeps nothing",
            Drafts.kept(Drafts.typed(typing, "8GYNJ6", { text: "", caret: 0 }, now), "8GYNJ6",
                now),
            { text: "", caret: 0 });
        Expect.same("a draft a week old isn't given back",
            Drafts.kept(typing, "8GYNJ6", now + 7 * day), { text: "", caret: 0 });

        // Return never clears his words: until his talk says they were
        // delivered, a shell started again gives them back.
        const sending = Drafts.sent(typing, "8GYNJ6", "what's on screen ", now);
        Expect.same("words sent and not yet delivered open again",
            Drafts.kept(sending, "8GYNJ6", now), { text: "what's on screen", caret: 16 });
        Expect.same("what he types after sending opens rather than what he sent",
            Drafts.kept(Drafts.typed(sending, "8GYNJ6", { text: "and", caret: 3 }, now),
                "8GYNJ6", now),
            { text: "and", caret: 3 });

        const delivered = { "8GYNJ6": [{ words: "what's on screen", stage: "delivered" }] };
        Expect.same("words delivered go",
            Drafts.kept(Drafts.settled(sending, delivered), "8GYNJ6", now),
            { text: "", caret: 0 });
        Expect.same("words replied to go, a step past delivered",
            Drafts.settled(sending, { "8GYNJ6": [{ words: "what's on screen", stage: "replied" }] })
                .sending, {});
        Expect.same("words that failed to go are kept",
            Drafts.settled(sending, { "8GYNJ6": [{ words: "what's on screen", stage: "failed" }] })
                .sending["8GYNJ6"].length, 1);
        Expect.same("another talk's words leave these kept",
            Drafts.settled(sending, { "8GYNJ6": [{ words: "earlier", stage: "delivered" }] })
                .sending["8GYNJ6"].length, 1);
        Expect.expect("nothing settled is the same state, so nothing is written",
            Drafts.settled(sending, {}) === sending, "a new state for nothing settled");

        // As written and read again by a shell starting.
        const back = Drafts.read(
            JSON.stringify(Drafts.typed(sending, "8GYNJ6", { text: "and", caret: 3 }, now)),
            now + day);
        Expect.same("a shell starting again reads the draft",
            Drafts.kept(back, "8GYNJ6", now + day), { text: "and", caret: 3 });
        Expect.same("and the words still being delivered",
            back.sending["8GYNJ6"].map(one => one.text), ["what's on screen"]);
        Expect.same("a week-old file gives nothing back",
            Drafts.read(JSON.stringify(sending), now + 8 * day), Drafts.empty());
        Expect.same("a file not written here gives nothing back",
            Drafts.read("not json", now), Drafts.empty());
        Expect.same("nor does none at all", Drafts.read("", now), Drafts.empty());
    }
}
